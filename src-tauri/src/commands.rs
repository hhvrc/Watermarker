//! Tauri command surface.
//!
//! Metadata is returned as JSON; preview *pixels* are returned as raw bytes via
//! [`tauri::ipc::Response`] (an `ArrayBuffer` on the JS side, no base64). The
//! frontend wraps those bytes in a `Blob` URL for display.

use std::path::PathBuf;

use serde::Serialize;
use tauri::ipc::Response;
use tauri::State;

use std::sync::Arc;

use tauri::AppHandle;

use crate::compositor::composite;
use crate::decode;
use crate::placement::Placement;
use crate::preview;
use crate::settings::{self, Settings};
use crate::state::{AppState, ImageEntry, ImageSource, WatermarkData, WatermarkEntry};
use crate::watermark::WatermarkSource;

/// Metadata for an imported source image.
#[derive(Serialize)]
pub struct ImageMeta {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

/// Metadata for the current watermark.
#[derive(Serialize)]
pub struct WatermarkMeta {
    pub kind: String, // "svg" | "raster"
    pub width: u32,
    pub height: u32,
}

/// Watermark metadata plus the managed path the logo was copied to, so the
/// frontend can persist a reference that survives the original file moving.
#[derive(Serialize)]
pub struct WatermarkMetaStored {
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub stored_path: String,
}

fn base_key(id: &str) -> String {
    format!("base:{id}")
}

/// Decode a display preview, register the image, and return its metadata.
fn register_image(
    state: &AppState,
    source: ImageSource,
    name: String,
    img: image::RgbaImage,
) -> Result<ImageMeta, String> {
    let (w, h) = img.dimensions();
    let display = preview::downscale_for_display(&img);
    let bytes = preview::encode_preview(&display).map_err(|e| e.to_string())?;

    let id = state.next_id("img");
    state.put_preview(base_key(&id), bytes);
    state.images.lock().unwrap().insert(
        id.clone(),
        ImageEntry {
            source,
            name: name.clone(),
        },
    );

    Ok(ImageMeta {
        id,
        name,
        width: w,
        height: h,
    })
}

/// Import images by path: decode, downscale a display preview, and register them.
/// Unsupported paths are silently skipped. Returns metadata for the imported set.
#[tauri::command]
pub fn import_images(state: State<AppState>, paths: Vec<String>) -> Result<Vec<ImageMeta>, String> {
    let mut out = Vec::new();
    for path in paths {
        let pb = PathBuf::from(&path);
        if !decode::is_supported_input(&pb) {
            continue;
        }
        let img = decode::load_rgba(&pb).map_err(|e| e.to_string())?;
        let name = pb
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        out.push(register_image(&state, ImageSource::Path(pb), name, img)?);
    }
    Ok(out)
}

/// Read the raw IPC body bytes plus the URI-encoded `name` header.
fn read_raw(
    request: &tauri::ipc::Request,
    default_name: &str,
) -> Result<(Vec<u8>, String), String> {
    let bytes = match request.body() {
        tauri::ipc::InvokeBody::Raw(b) => b.clone(),
        _ => return Err("expected raw bytes".into()),
    };
    let name = request
        .headers()
        .get("name")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            urlencoding::decode(s)
                .map(|c| c.into_owned())
                .unwrap_or_else(|_| s.to_string())
        })
        .unwrap_or_else(|| default_name.to_string());
    Ok((bytes, name))
}

fn ext_of(name: &str) -> String {
    PathBuf::from(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Heuristic: does this byte stream look like an SVG document?
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(256)];
    let s = String::from_utf8_lossy(head);
    let t = s.trim_start();
    t.starts_with("<?xml") || t.starts_with("<svg") || t.contains("<svg")
}

/// Import a single image from raw bytes (a drag-dropped file or bitmap). The image
/// bytes are sent as the raw IPC body; the file name travels in a `name` header
/// (URI-encoded). Returns the registered image's metadata.
#[tauri::command]
pub fn import_image_bytes(
    state: State<AppState>,
    request: tauri::ipc::Request,
) -> Result<ImageMeta, String> {
    let (bytes, name) = read_raw(&request, "image.png")?;
    let img = decode::load_rgba_from_bytes(&bytes).map_err(|e| e.to_string())?;
    register_image(&state, ImageSource::Bytes(Arc::new(bytes)), name, img)
}

/// Remove an image from the registry and free its cached preview.
#[tauri::command]
pub fn remove_image(state: State<AppState>, id: String) {
    state.images.lock().unwrap().remove(&id);
    state.previews.lock().unwrap().remove(&base_key(&id));
}

/// Read persisted user settings, pruning any paths that no longer exist.
#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    settings::load_pruned(&app)
}

/// Persist user settings.
#[tauri::command]
pub fn set_settings(app: AppHandle, value: Settings) -> Result<(), String> {
    settings::save(&app, &value)
}

/// Raw bytes of the downscaled, watermark-free preview for an image.
#[tauri::command]
pub fn get_image_preview(state: State<AppState>, id: String) -> Result<Response, String> {
    let bytes = state
        .get_preview(&base_key(&id))
        .ok_or_else(|| format!("no preview for image {id}"))?;
    Ok(Response::new(bytes))
}

/// Build a raster/SVG watermark from a name + bytes (extension or content sniff).
fn watermark_data_from(
    name: &str,
    bytes: Vec<u8>,
) -> Result<(WatermarkData, &'static str), String> {
    if ext_of(name) == "svg" || looks_like_svg(&bytes) {
        // Validate it parses up front.
        WatermarkSource::from_svg_bytes(&bytes).map_err(|e| e.to_string())?;
        Ok((WatermarkData::Svg(bytes), "svg"))
    } else {
        let img = decode::load_rgba_from_bytes(&bytes).map_err(|e| e.to_string())?;
        // Trim transparent borders so the watermark box is the actual content.
        let trimmed = crate::watermark::trim_transparent(&img);
        Ok((WatermarkData::Raster(Arc::new(trimmed)), "raster"))
    }
}

/// Render and cache the watermark preview, store it, and return its metadata.
fn register_watermark(
    state: &AppState,
    data: WatermarkData,
    kind: &str,
) -> Result<WatermarkMeta, String> {
    let src = data.source().map_err(|e| e.to_string())?;
    let (width, height) = src.content_size();
    let aspect = src.aspect();

    // Render a capped preview of the watermark (with alpha) for the canvas.
    let tw = width.clamp(1, 1024);
    let th = ((tw as f32 / aspect).round() as u32).max(1);
    let stamp = src.rasterize(tw, th);
    let bytes = preview::encode_preview(&stamp).map_err(|e| e.to_string())?;

    state.put_preview("watermark".into(), bytes);
    *state.watermark.lock().unwrap() = Some(WatermarkEntry { data });

    Ok(WatermarkMeta {
        kind: kind.into(),
        width,
        height,
    })
}

/// Load a watermark (SVG or raster) from a file path and cache a preview of it.
#[tauri::command]
pub fn set_watermark(state: State<AppState>, path: String) -> Result<WatermarkMeta, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let (data, kind) = watermark_data_from(&path, bytes)?;
    register_watermark(&state, data, kind)
}

/// Like [`set_watermark`], but also copies the logo into the app's managed
/// `watermarks/` dir and returns that durable path. The frontend persists the
/// returned `stored_path` so the watermark survives the original file moving.
#[tauri::command]
pub fn set_watermark_persisted(
    state: State<AppState>,
    app: AppHandle,
    path: String,
) -> Result<WatermarkMetaStored, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let mut ext = ext_of(&path);
    if ext.is_empty() {
        ext = if looks_like_svg(&bytes) { "svg" } else { "png" }.to_string();
    }
    let (data, kind) = watermark_data_from(&path, bytes.clone())?;
    let meta = register_watermark(&state, data, kind)?;
    let stored = settings::store_watermark_bytes(&app, &bytes, &ext)?;
    Ok(WatermarkMetaStored {
        kind: meta.kind,
        width: meta.width,
        height: meta.height,
        stored_path: stored.to_string_lossy().into_owned(),
    })
}

/// Load a watermark from raw dropped bytes (file or bitmap).
#[tauri::command]
pub fn set_watermark_bytes(
    state: State<AppState>,
    request: tauri::ipc::Request,
) -> Result<WatermarkMeta, String> {
    let (bytes, name) = read_raw(&request, "watermark.png")?;
    let (data, kind) = watermark_data_from(&name, bytes)?;
    register_watermark(&state, data, kind)
}

/// Raw bytes of the current watermark preview (with alpha).
#[tauri::command]
pub fn get_watermark_preview(state: State<AppState>) -> Result<Response, String> {
    let bytes = state
        .get_preview("watermark")
        .ok_or("no watermark loaded")?;
    Ok(Response::new(bytes))
}

/// Render an exact, Rust-composited preview for one image at the given placement.
///
/// Composites on the already-downscaled base (placement is resolution-independent,
/// so the result matches full-res export), avoiding a full-res decode per drag.
#[tauri::command]
pub fn render_exact_preview(
    state: State<AppState>,
    image_id: String,
    placement: Placement,
) -> Result<Response, String> {
    let base_bytes = state
        .get_preview(&base_key(&image_id))
        .ok_or_else(|| format!("no preview for image {image_id}"))?;
    let base = image::load_from_memory(&base_bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();

    let wm_src = {
        let guard = state.watermark.lock().unwrap();
        let entry = guard.as_ref().ok_or("no watermark loaded")?;
        entry.data.source().map_err(|e| e.to_string())?
    };

    let composed = composite(&base, &wm_src, &placement);
    let bytes = preview::encode_preview(&composed).map_err(|e| e.to_string())?;
    Ok(Response::new(bytes))
}
