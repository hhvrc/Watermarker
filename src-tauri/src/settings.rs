//! Persistent app settings, stored as `settings.json` in the platform app-config
//! directory (e.g. `%APPDATA%/com.watermarker.app/`). This is the idiomatic place
//! for small persistent prefs in a Tauri app, not webview localStorage.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::placement::Placement;

/// A named, quickly-recallable combination of watermark + placement.
///
/// `watermark_path` points at a *managed* copy of the logo inside the app's
/// `watermarks/` dir (see [`store_watermark_bytes`]), so the preset keeps working
/// even if the user moves or deletes the original file.
#[derive(Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub placement: Placement,
    #[serde(default)]
    pub watermark_path: Option<String>,
}

/// User preferences that persist across launches.
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Last watermark, stored as a managed copy in the app's `watermarks/` dir.
    #[serde(default)]
    pub watermark_path: Option<String>,
    /// Last chosen output directory.
    #[serde(default)]
    pub output_dir: Option<String>,
    /// Whether to strip all metadata from outputs (privacy).
    #[serde(default)]
    pub strip_metadata: bool,
    /// Last-used placement, restored as the sticky default on next launch.
    #[serde(default)]
    pub placement: Option<Placement>,
    /// Saved presets, in user order.
    #[serde(default)]
    pub presets: Vec<Preset>,
    /// A release version the user chose to skip; the update prompt stays hidden
    /// for exactly this version but reappears for any newer one.
    #[serde(default)]
    pub skipped_version: Option<String>,
    /// When true, the app never checks for or prompts about updates.
    #[serde(default)]
    pub updates_disabled: bool,
}

/// The directory holding managed watermark copies, created on demand.
fn watermarks_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("watermarks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Copy watermark bytes into the app's managed `watermarks/` dir, named by a
/// content hash so re-importing the same logo dedupes. Returns the stored path.
///
/// This is what makes a logo durable: the original file can move or vanish and
/// the managed copy still drives previews, exports, and presets.
pub fn store_watermark_bytes(app: &AppHandle, bytes: &[u8], ext: &str) -> Result<PathBuf, String> {
    let dir = watermarks_dir(app)?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    let id = hasher.finish();
    let ext = if ext.is_empty() { "bin" } else { ext };
    let path = dir.join(format!("{id:016x}.{ext}"));
    if !path.exists() {
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

/// Load settings, returning defaults on any missing-file/parse error.
pub fn load(app: &AppHandle) -> Settings {
    settings_path(app)
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Load settings and drop any persisted paths that no longer exist on disk,
/// re-saving if anything was pruned.
pub fn load_pruned(app: &AppHandle) -> Settings {
    let mut s = load(app);
    let mut changed = false;

    if let Some(p) = &s.watermark_path {
        if !Path::new(p).is_file() {
            s.watermark_path = None;
            changed = true;
        }
    }
    if let Some(p) = &s.output_dir {
        if !Path::new(p).is_dir() {
            s.output_dir = None;
            changed = true;
        }
    }

    if changed {
        let _ = save(app, &s);
    }
    s
}

/// Persist settings to disk, creating the config directory if needed.
pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
