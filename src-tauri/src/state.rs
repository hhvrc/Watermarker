//! Shared application state held by Tauri.
//!
//! Holds the registry of imported images (path + dimensions), the current watermark,
//! and an in-memory cache of encoded preview bytes keyed by string. Full-resolution
//! pixels are never kept here; they are re-decoded from disk for export.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use image::RgbaImage;

use crate::decode::{self, DecodeError};
use crate::watermark::{WatermarkError, WatermarkSource};

/// Where a registered image's full-resolution pixels come from: a file on disk
/// (re-decoded on demand) or in-memory bytes (a drag-dropped file or raw bitmap).
#[derive(Clone)]
pub enum ImageSource {
    Path(PathBuf),
    Bytes(Arc<Vec<u8>>),
}

impl ImageSource {
    /// Decode the full-resolution image from this source.
    pub fn decode(&self) -> Result<RgbaImage, DecodeError> {
        match self {
            ImageSource::Path(p) => decode::load_rgba(p),
            ImageSource::Bytes(b) => decode::load_rgba_from_bytes(b),
        }
    }
}

/// A registered source image. Dimensions are returned to the frontend at import
/// time (via `ImageMeta`); the source + name are kept for re-decode and output naming.
pub struct ImageEntry {
    pub source: ImageSource,
    pub name: String,
}

/// Watermark payload stored in a thread-safe, re-constructible form.
///
/// SVG is kept as raw bytes (parsed per use, since `usvg::Tree` is not `Sync`),
/// raster as a decoded image.
#[derive(Clone)]
pub enum WatermarkData {
    Raster(Arc<RgbaImage>),
    Svg(Vec<u8>),
}

impl WatermarkData {
    /// Construct a usable [`WatermarkSource`] (parses SVG on demand). Raster sources
    /// share the underlying pixels via `Arc`, so this is cheap to call per item.
    pub fn source(&self) -> Result<WatermarkSource, WatermarkError> {
        match self {
            WatermarkData::Raster(img) => Ok(WatermarkSource::Raster(Arc::clone(img))),
            WatermarkData::Svg(bytes) => WatermarkSource::from_svg_bytes(bytes),
        }
    }
}

/// The current watermark.
pub struct WatermarkEntry {
    pub data: WatermarkData,
}

/// Application-wide state.
#[derive(Default)]
pub struct AppState {
    /// Encoded preview bytes, keyed by an arbitrary string (e.g. `"base:<id>"`).
    pub previews: Mutex<HashMap<String, Vec<u8>>>,
    /// Imported images, keyed by image id.
    pub images: Mutex<HashMap<String, ImageEntry>>,
    /// The current watermark, if any.
    pub watermark: Mutex<Option<WatermarkEntry>>,
    counter: AtomicU64,
}

impl AppState {
    /// Allocate a process-unique id with the given prefix.
    pub fn next_id(&self, prefix: &str) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}{n}")
    }

    pub fn put_preview(&self, key: String, bytes: Vec<u8>) {
        self.previews.lock().unwrap().insert(key, bytes);
    }

    pub fn get_preview(&self, key: &str) -> Option<Vec<u8>> {
        self.previews.lock().unwrap().get(key).cloned()
    }
}
