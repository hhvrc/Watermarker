//! Best-effort metadata preservation from source into the exported PNG.
//!
//! Extracts the ICC colour profile and EXIF block from the source container and
//! re-embeds them into the output PNG (as `iCCP` / `eXIf` chunks). Sources whose
//! container img-parts cannot read (e.g. TIFF) simply carry no transplanted
//! metadata for now.

use std::path::Path;

use bytes::Bytes;
use img_parts::jpeg::Jpeg;
use img_parts::png::{Png, PngChunk};
use img_parts::webp::WebP;
use img_parts::{ImageEXIF, ImageICC};

/// Editor identifier written into exports as the PNG `Software` tag, the way
/// image tools conventionally record what produced a file. Skipped when the user
/// strips metadata.
pub const SOFTWARE: &str = "Watermarker (https://github.com/hhvrc)";

/// ICC profile and EXIF blocks extracted from a source image.
#[derive(Default, Clone)]
pub struct Metadata {
    pub icc: Option<Vec<u8>>,
    pub exif: Option<Vec<u8>>,
}

impl Metadata {
    pub fn is_empty(&self) -> bool {
        self.icc.is_none() && self.exif.is_none()
    }
}

/// Extract metadata from a source file. Never fails, returning empty metadata on
/// any read/parse error or unsupported container.
pub fn extract(path: &Path) -> Metadata {
    let Ok(raw) = std::fs::read(path) else {
        return Metadata::default();
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    from_container(Bytes::from(raw), &ext)
}

/// Extract metadata from in-memory source bytes (drag-dropped images), using
/// `ext` to pick the container parser.
pub fn extract_from_bytes(bytes: &[u8], ext: &str) -> Metadata {
    from_container(Bytes::copy_from_slice(bytes), &ext.to_ascii_lowercase())
}

fn from_container(b: Bytes, ext: &str) -> Metadata {
    let (icc, exif) = match ext {
        "jpg" | "jpeg" => Jpeg::from_bytes(b)
            .map(|j| (j.icc_profile(), j.exif()))
            .unwrap_or((None, None)),
        "png" => Png::from_bytes(b)
            .map(|p| (p.icc_profile(), p.exif()))
            .unwrap_or((None, None)),
        "webp" => WebP::from_bytes(b)
            .map(|w| (w.icc_profile(), w.exif()))
            .unwrap_or((None, None)),
        _ => (None, None),
    };
    Metadata {
        icc: icc.map(|b| b.to_vec()),
        exif: exif.map(|b| b.to_vec()),
    }
}

/// Build an uncompressed iTXt chunk (UTF-8 text under an ASCII keyword).
fn itxt_chunk(keyword: &str, text: &str) -> PngChunk {
    let mut data = Vec::new();
    data.extend_from_slice(keyword.as_bytes());
    data.push(0); // keyword null terminator
    data.push(0); // compression flag: uncompressed
    data.push(0); // compression method
    data.push(0); // empty language tag + null
    data.push(0); // empty translated keyword + null
    data.extend_from_slice(text.as_bytes());
    PngChunk::new(*b"iTXt", Bytes::from(data))
}

/// Re-embed source `meta` (ICC/EXIF) into exported PNG bytes and, when `tag_software`
/// is set, record the [`SOFTWARE`] identifier as the PNG `Software` tag. Returns new
/// PNG bytes.
pub fn finalize_png(
    png_bytes: &[u8],
    meta: Option<&Metadata>,
    tag_software: bool,
) -> Result<Vec<u8>, String> {
    let mut png = Png::from_bytes(Bytes::copy_from_slice(png_bytes)).map_err(|e| e.to_string())?;

    if let Some(m) = meta {
        png.set_icc_profile(m.icc.clone().map(Bytes::from));
        png.set_exif(m.exif.clone().map(Bytes::from));
    }

    if tag_software {
        // Insert the Software tag before IEND (or at the end if absent).
        let chunk = itxt_chunk("Software", SOFTWARE);
        let chunks = png.chunks_mut();
        let idx = chunks
            .iter()
            .position(|c| c.kind() == *b"IEND")
            .unwrap_or(chunks.len());
        chunks.insert(idx, chunk);
    }

    let mut out = Vec::new();
    png.encoder()
        .write_to(&mut out)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, RgbaImage};

    fn tiny_png() -> Vec<u8> {
        let img = RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        image::codecs::png::PngEncoder::new(&mut buf)
            .write_image(img.as_raw(), 8, 8, image::ExtendedColorType::Rgba8)
            .unwrap();
        buf.into_inner()
    }

    #[test]
    fn extract_missing_file_is_empty() {
        assert!(extract(Path::new("does-not-exist.jpg")).is_empty());
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn finalize_roundtrips_icc_and_exif() {
        let meta = Metadata {
            icc: Some(vec![9, 8, 7, 6, 5]),
            // EXIF blocks in PNG are stored with a leading TIFF header; img-parts
            // round-trips the raw bytes we hand it.
            exif: Some(b"MM\x00\x2a\x00\x00\x00\x08".to_vec()),
        };
        let out = finalize_png(&tiny_png(), Some(&meta), true).unwrap();

        let png = Png::from_bytes(Bytes::from(out.clone())).unwrap();
        assert_eq!(png.icc_profile().map(|b| b.to_vec()), meta.icc);
        assert_eq!(png.exif().map(|b| b.to_vec()), meta.exif);
        assert!(contains(&out, b"Software"), "software tag missing");
        // Still a valid, decodable PNG.
        assert_eq!(
            image::load_from_memory(&out)
                .unwrap()
                .to_rgba8()
                .dimensions(),
            (8, 8)
        );
    }

    #[test]
    fn finalize_without_software_tag_stays_clean() {
        let out = finalize_png(&tiny_png(), None, false).unwrap();
        let png = Png::from_bytes(Bytes::from(out.clone())).unwrap();
        assert!(png.icc_profile().is_none());
        assert!(png.exif().is_none());
        assert!(!contains(&out, b"Software"), "expected no software tag");
        let decoded = image::load_from_memory(&out).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (8, 8));
    }
}
