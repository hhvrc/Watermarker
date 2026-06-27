//! Batch export: full-resolution composite -> PNG -> metadata transplant -> disk.
//!
//! Runs the batch in parallel (rayon) and streams a [`ProgressEvent`] per item over
//! the `wm://progress` Tauri event channel.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::compositor::composite;
use crate::placement::Placement;
use crate::state::{AppState, ImageSource, WatermarkData};
use crate::{metadata, preview};

/// Where and how exported files are written.
#[derive(Deserialize)]
pub struct OutputSettings {
    pub dir: String,
    #[serde(default = "default_suffix")]
    pub suffix: String,
    #[serde(default)]
    pub overwrite: bool,
    /// When true, no source metadata is carried over, so the output PNG is written
    /// with no EXIF/ICC/etc. (privacy).
    #[serde(default)]
    pub strip_metadata: bool,
    /// When true, also write a non-watermarked PNG (`<stem>.png`) alongside the
    /// watermarked output. Handy for converting TIFF/other inputs to a clean PNG.
    #[serde(default)]
    pub export_clean: bool,
}

fn default_suffix() -> String {
    "_wm".into()
}

/// One image plus its own placement. Each image in a batch is positioned
/// independently (the frontend carries the previous image's placement forward as a
/// sticky default, but the user may adjust it per image).
#[derive(Deserialize)]
pub struct ImageJob {
    pub image_id: String,
    pub placement: Placement,
}

/// A batch export request from the frontend. Only the images the user has staged
/// are included.
#[derive(Deserialize)]
pub struct JobSpec {
    pub items: Vec<ImageJob>,
    pub output: OutputSettings,
}

/// Emitted once per processed item on `wm://progress`.
#[derive(Serialize, Clone)]
pub struct ProgressEvent {
    pub completed: usize,
    pub total: usize,
    pub image_id: String,
    pub name: String,
    pub status: String, // "done" | "error"
    pub error: Option<String>,
    pub output_path: Option<String>,
}

/// Returned when the batch finishes.
#[derive(Serialize)]
pub struct BatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}

struct Item {
    id: String,
    source: ImageSource,
    name: String,
    placement: Placement,
}

/// Process a batch of images, writing watermarked PNGs to the output directory.
#[tauri::command]
pub fn process_batch(
    app: AppHandle,
    state: State<AppState>,
    job: JobSpec,
) -> Result<BatchSummary, String> {
    // Snapshot the work list and watermark up front so the parallel section never
    // touches Tauri state locks.
    let items: Vec<Item> = {
        let images = state.images.lock().unwrap();
        job.items
            .iter()
            .filter_map(|j| {
                images.get(&j.image_id).map(|e| Item {
                    id: j.image_id.clone(),
                    source: e.source.clone(),
                    name: e.name.clone(),
                    placement: j.placement,
                })
            })
            .collect()
    };

    // The suffix is user-editable text that gets concatenated into the output file
    // name; reject anything that could escape the chosen output directory.
    validate_suffix(&job.output.suffix)?;

    let wm_data: Arc<WatermarkData> = {
        let guard = state.watermark.lock().unwrap();
        let entry = guard.as_ref().ok_or("no watermark loaded")?;
        Arc::new(entry.data.clone())
    };
    // Surface a bad watermark before doing any work.
    wm_data.source().map_err(|e| e.to_string())?;

    let out_dir = PathBuf::from(&job.output.dir);
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let total = items.len();
    let completed = AtomicUsize::new(0);
    let succeeded = AtomicUsize::new(0);

    items.par_iter().for_each(|item| {
        let result = process_one(item, &wm_data, &out_dir, &job.output);
        let n = completed.fetch_add(1, Ordering::Relaxed) + 1;

        let event = match result {
            Ok(output_path) => {
                succeeded.fetch_add(1, Ordering::Relaxed);
                ProgressEvent {
                    completed: n,
                    total,
                    image_id: item.id.clone(),
                    name: item.name.clone(),
                    status: "done".into(),
                    error: None,
                    output_path: Some(output_path),
                }
            }
            Err(error) => ProgressEvent {
                completed: n,
                total,
                image_id: item.id.clone(),
                name: item.name.clone(),
                status: "error".into(),
                error: Some(error),
                output_path: None,
            },
        };
        let _ = app.emit("wm://progress", event);
    });

    let succeeded = succeeded.load(Ordering::Relaxed);
    Ok(BatchSummary {
        total,
        succeeded,
        failed: total - succeeded,
    })
}

/// Composite and write a single image. Returns the written (watermarked) path on
/// success. When `export_clean` is set, also writes a non-watermarked `<stem>.png`.
fn process_one(
    item: &Item,
    wm_data: &WatermarkData,
    out_dir: &Path,
    output: &OutputSettings,
) -> Result<String, String> {
    // When exporting both versions, disambiguate with a type tag after the user's
    // suffix (`<stem><suffix>_watermarked.png` / `_clean.png`); a lone watermarked
    // export keeps the plain `<stem><suffix>.png` name.
    let (wm_tag, clean_tag) = if output.export_clean {
        ("_watermarked", "_clean")
    } else {
        ("", "")
    };
    let dest = output_path(&item.name, out_dir, &output.suffix, wm_tag);
    let clean_dest = output
        .export_clean
        .then(|| output_path(&item.name, out_dir, &output.suffix, clean_tag));

    // Check both destinations up front so we don't write one then fail on the other.
    if dest.exists() && !output.overwrite {
        return Err(format!("output already exists: {}", dest.display()));
    }
    if let Some(clean) = &clean_dest {
        if clean.exists() && !output.overwrite {
            return Err(format!("output already exists: {}", clean.display()));
        }
    }

    let base = item.source.decode().map_err(|e| e.to_string())?;

    // With `strip_metadata` set the output carries nothing: no transplanted
    // ICC/EXIF and no Software tag. Otherwise transplant source metadata (when the
    // container has any) and record the editor's Software tag.
    let tag_software = !output.strip_metadata;
    let meta = if output.strip_metadata {
        None
    } else {
        let m = source_metadata(&item.source, &item.name);
        if m.is_empty() {
            None
        } else {
            Some(m)
        }
    };

    // Optional clean (non-watermarked) copy of the decoded source.
    if let Some(clean) = &clean_dest {
        write_png(&base, clean, meta.as_ref(), tag_software)?;
    }

    let wm = wm_data.source().map_err(|e| e.to_string())?;
    let composed = composite(&base, &wm, &item.placement);
    write_png(&composed, &dest, meta.as_ref(), tag_software)?;

    Ok(dest.to_string_lossy().into_owned())
}

/// Encode an image to PNG, transplant `meta` (ICC/EXIF), optionally tag it with the
/// editor's Software identifier, and write it.
fn write_png(
    pixels: &image::RgbaImage,
    dest: &Path,
    meta: Option<&metadata::Metadata>,
    tag_software: bool,
) -> Result<(), String> {
    let png = preview::encode_png(pixels).map_err(|e| e.to_string())?;
    let final_bytes = metadata::finalize_png(&png, meta, tag_software).unwrap_or(png);
    std::fs::write(dest, final_bytes).map_err(|e| e.to_string())
}

/// Extract source metadata from whichever backing the image has.
fn source_metadata(source: &ImageSource, name: &str) -> metadata::Metadata {
    match source {
        ImageSource::Path(p) => metadata::extract(p),
        ImageSource::Bytes(b) => metadata::extract_from_bytes(b, &ext_of(name)),
    }
}

fn ext_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Reject output suffixes that contain path separators or `..`, which would let
/// the concatenated file name escape the output directory (arbitrary `.png` write).
fn validate_suffix(suffix: &str) -> Result<(), String> {
    if suffix.contains('/') || suffix.contains('\\') || suffix.contains("..") {
        return Err("output suffix must not contain path separators or '..'".into());
    }
    Ok(())
}

/// Build `<out_dir>/<stem><suffix><tag>.png` from a source file name. `tag` is an
/// export-type marker (e.g. `_watermarked`) used only when both versions are written.
fn output_path(name: &str, out_dir: &Path, suffix: &str, tag: &str) -> PathBuf {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    out_dir.join(format!("{stem}{suffix}{tag}.png"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_path_uses_stem_suffix_and_png_ext() {
        let p = output_path("IMG_001.jpg", Path::new("/out"), "_wm", "");
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), "IMG_001_wm.png");
    }

    #[test]
    fn output_path_replaces_any_input_extension() {
        let p = output_path("a.tiff", Path::new("out"), "", "");
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), "a.png");
    }

    #[test]
    fn both_exports_share_suffix_and_differ_by_type_tag() {
        // A TIFF input with the default suffix, exporting both versions.
        let wm = output_path("scan.tiff", Path::new("/out"), "_wm", "_watermarked");
        let clean = output_path("scan.tiff", Path::new("/out"), "_wm", "_clean");
        assert_eq!(
            wm.file_name().unwrap().to_str().unwrap(),
            "scan_wm_watermarked.png"
        );
        assert_eq!(
            clean.file_name().unwrap().to_str().unwrap(),
            "scan_wm_clean.png"
        );
        assert_ne!(wm, clean);
    }

    #[test]
    fn ext_of_lowercases() {
        assert_eq!(ext_of("Photo.JPG"), "jpg");
        assert_eq!(ext_of("no-ext"), "");
    }

    #[test]
    fn validate_suffix_blocks_path_traversal() {
        assert!(validate_suffix("_wm").is_ok());
        assert!(validate_suffix("").is_ok());
        assert!(validate_suffix("-final_v2").is_ok());
        assert!(validate_suffix("/etc/evil").is_err());
        assert!(validate_suffix("..\\..\\Windows\\x").is_err());
        assert!(validate_suffix("_wm/../../sneaky").is_err());
    }
}
