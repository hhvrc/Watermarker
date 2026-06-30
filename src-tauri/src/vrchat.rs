//! Best-effort parsing of VRChat screenshot metadata, for display only.
//!
//! VRChat (and the companion tool VRCX) embed capture metadata into PNG text
//! chunks in one of three shapes:
//!
//!   * **line**  — a pipe-delimited record under the `Description` keyword, e.g.
//!     `lfs|2|author:usr_x,Alice|world:wrld_y,inst,Cool World|pos:...`.
//!   * **json**  — a VRCX JSON object under `Description`, with `world`/`author`.
//!   * **xmp**   — an Adobe XMP packet under `XML:com.adobe.xmp`, carrying the
//!     `vrc:` namespace (and a real `xmp:CreateDate`).
//!
//! We extract only the few human-facing fields the UI shows (world, author,
//! capture date). Parsing never fails: a non-VRChat or malformed PNG yields
//! `None` and the panel simply isn't shown.

use serde::Serialize;

/// The PNG 8-byte signature.
const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";

const KEY_DESCRIPTION: &str = "Description";
const KEY_XMP: &str = "XML:com.adobe.xmp";

/// Key VRChat fields extracted for display. Every field is best-effort and may
/// be absent. Serialized to the frontend as part of an image's metadata.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct VrcMetadata {
    /// Name of the world the photo was taken in.
    pub world_name: Option<String>,
    /// Display name of the photographer (the local player who took the shot).
    pub author_name: Option<String>,
    /// Capture timestamp, display-ready (`YYYY-MM-DD HH:MM:SS`). Taken from the
    /// XMP `CreateDate` when present, otherwise from the VRChat filename.
    pub captured_at: Option<String>,
    /// Which embedded format the data came from: `"line"`, `"json"`, or `"xmp"`.
    pub source_format: &'static str,
}

impl VrcMetadata {
    /// True once we've recovered an identifying field, i.e. this really is a
    /// VRChat image and the panel is worth showing.
    fn is_vrchat(&self) -> bool {
        self.world_name.is_some() || self.author_name.is_some()
    }
}

/// Parse VRChat metadata out of PNG `bytes`. `file_name` is used only as a
/// fallback source for the capture date (VRChat encodes it in the filename).
/// Returns `None` for non-PNG, non-VRChat, or unparseable input.
pub fn parse_png(bytes: &[u8], file_name: &str) -> Option<VrcMetadata> {
    let chunks = read_text_chunks(bytes);

    let mut out = VrcMetadata::default();

    // Pass 1: the `Description` record (VRCX JSON preferred over the line form).
    for (keyword, text) in &chunks {
        if keyword != KEY_DESCRIPTION {
            continue;
        }
        if let Some(f) = parse_json(text) {
            merge(&mut out, f);
            break;
        }
        if let Some(f) = parse_line(text) {
            merge(&mut out, f);
            break;
        }
    }

    // Pass 2: XMP fills any gaps (and is the only reliable source of a date).
    for (keyword, text) in &chunks {
        if keyword == KEY_XMP || looks_like_xml(text) {
            merge(&mut out, parse_xmp(text));
        }
    }

    if !out.is_vrchat() {
        return None;
    }

    if out.captured_at.is_none() {
        out.captured_at = date_from_filename(file_name);
    }

    Some(out)
}

/// Fields recovered from a single chunk, before merging into [`VrcMetadata`].
#[derive(Default)]
struct Fields {
    world_name: Option<String>,
    author_name: Option<String>,
    captured_at: Option<String>,
    source_format: &'static str,
}

/// Fill empty fields of `out` from `f` (first writer wins) and remember the
/// format that supplied the first identifying field.
fn merge(out: &mut VrcMetadata, f: Fields) {
    let had_id = out.is_vrchat();
    fill(&mut out.world_name, f.world_name);
    fill(&mut out.author_name, f.author_name);
    fill(&mut out.captured_at, f.captured_at);
    if !had_id && out.is_vrchat() {
        out.source_format = f.source_format;
    }
}

fn fill(slot: &mut Option<String>, value: Option<String>) {
    if slot.is_none() {
        if let Some(v) = value {
            let v = v.trim().to_string();
            if !v.is_empty() {
                *slot = Some(v);
            }
        }
    }
}

// ── PNG chunk reader ────────────────────────────────────────────────────────

/// Scan a PNG and return `(keyword, text)` for each readable text chunk
/// (`tEXt` and uncompressed `iTXt`). Stops at `IEND`. Best-effort: a truncated
/// or non-PNG stream just yields whatever was readable (often nothing).
fn read_text_chunks(bytes: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if bytes.len() < PNG_SIG.len() || &bytes[..PNG_SIG.len()] != PNG_SIG {
        return out;
    }

    let mut pos = PNG_SIG.len();
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
            as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = match data_start.checked_add(len) {
            Some(e) if e <= bytes.len() => e,
            _ => break, // declared length runs past the buffer; give up.
        };
        let data = &bytes[data_start..data_end];

        match kind {
            b"IEND" => break,
            b"tEXt" => {
                if let Some(c) = parse_text_chunk(data) {
                    out.push(c);
                }
            }
            b"iTXt" => {
                if let Some(c) = parse_itxt_chunk(data) {
                    out.push(c);
                }
            }
            _ => {}
        }

        // Advance past data + 4-byte CRC.
        pos = data_end + 4;
    }
    out
}

/// `tEXt`: `keyword \0 text` (text is Latin-1; we decode lossily as UTF-8).
fn parse_text_chunk(data: &[u8]) -> Option<(String, String)> {
    let nul = data.iter().position(|&b| b == 0)?;
    let keyword = String::from_utf8_lossy(&data[..nul]).into_owned();
    let text = String::from_utf8_lossy(&data[nul + 1..]).into_owned();
    Some((keyword, text))
}

/// `iTXt`: `keyword \0 comp_flag comp_method lang \0 translated \0 text`.
/// Compressed iTXt (`comp_flag == 1`) is skipped — VRChat/Adobe write it raw.
fn parse_itxt_chunk(data: &[u8]) -> Option<(String, String)> {
    let nul = data.iter().position(|&b| b == 0)?;
    if nul + 2 >= data.len() {
        return None;
    }
    let keyword = String::from_utf8_lossy(&data[..nul]).into_owned();
    let comp_flag = data[nul + 1];
    if comp_flag != 0 {
        return None;
    }
    // Skip comp_flag + comp_method, then language \0 translated \0 text.
    let rest = &data[nul + 3..];
    let lang_end = rest.iter().position(|&b| b == 0)?;
    let after_lang = &rest[lang_end + 1..];
    let trans_end = after_lang.iter().position(|&b| b == 0)?;
    let text = &after_lang[trans_end + 1..];
    Some((keyword, String::from_utf8_lossy(text).into_owned()))
}

// ── Format parsers ──────────────────────────────────────────────────────────

/// VRCX JSON: `{ "world": {"name": ...}, "author": {"displayName": ...}, ... }`.
fn parse_json(text: &str) -> Option<Fields> {
    let v: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
    let obj = v.as_object()?;

    let world_name = obj
        .get("world")
        .and_then(|w| w.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string);
    let author = obj.get("author");
    let author_name = author
        .and_then(|a| a.get("displayName"))
        .or_else(|| author.and_then(|a| a.get("name")))
        .and_then(|n| n.as_str())
        .map(str::to_string);

    let f = Fields {
        world_name,
        author_name,
        captured_at: None,
        source_format: "json",
    };
    // Only treat it as VRCX JSON if it actually carried an id-ish field.
    (f.world_name.is_some() || f.author_name.is_some()).then_some(f)
}

/// Line form: `type|index|author:id,Name|world:id,inst,Name|...`. Bare
/// `wrld_...` segments (older screenshotmanager) are also accepted.
fn parse_line(text: &str) -> Option<Fields> {
    let line = text.trim();
    let mut parts = line.split('|');
    let meta_type = parts.next()?.trim();
    if meta_type != "screenshotmanager" && meta_type != "lfs" {
        return None;
    }

    let mut world_name = None;
    let mut author_name = None;
    for seg in parts {
        let seg = seg.trim();
        if let Some(rest) = seg.strip_prefix("wrld_") {
            // bare world: `wrld_<id>,<inst>,<name>`
            world_name = nth_after_commas(rest, 2);
        } else if let Some((key, val)) = seg.split_once(':') {
            match key {
                "author" => author_name = val.split_once(',').map(|(_, n)| n.to_string()),
                "world" => world_name = nth_after_commas(val, 2),
                _ => {}
            }
        }
    }

    let f = Fields {
        world_name,
        author_name,
        captured_at: None,
        source_format: "line",
    };
    (f.world_name.is_some() || f.author_name.is_some()).then_some(f)
}

/// Return the comma-separated field at index `n`, joining any trailing
/// remainder (world names may contain commas).
fn nth_after_commas(s: &str, n: usize) -> Option<String> {
    let mut it = s.splitn(n + 1, ',');
    let val = it.nth(n)?;
    Some(val.to_string())
}

/// Best-effort XMP scrape for the handful of fields we display. Matches by XML
/// local name across both element (`<vrc:WorldDisplayName>X</…>`) and attribute
/// (`vrc:WorldDisplayName="X"`) forms, since Adobe re-saves use the latter.
fn parse_xmp(xml: &str) -> Fields {
    let world_name = xmp_value(xml, "WorldDisplayName");
    let author_name = xmp_value(xml, "Author");
    let captured_at = xmp_value(xml, "CreateDate").map(|s| clean_datetime(&s));

    Fields {
        world_name,
        author_name,
        captured_at,
        source_format: "xmp",
    }
}

/// Find the first value for XML local name `local`, trying the element body
/// then the attribute form. Returns the entity-decoded, trimmed text.
fn xmp_value(xml: &str, local: &str) -> Option<String> {
    // Element form: `…:Local>VALUE</…` — find an opening tag ending in `Local>`.
    let open = format!("{local}>");
    if let Some(i) = xml.find(&open) {
        let before = xml[..i].chars().last();
        if matches!(before, Some(c) if c == ':' || c == '<') {
            let start = i + open.len();
            if let Some(end_rel) = xml[start..].find('<') {
                let raw = &xml[start..start + end_rel];
                let val = decode_entities(raw.trim());
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    // Attribute form: `…:Local="VALUE"`.
    let attr = format!("{local}=\"");
    if let Some(i) = xml.find(&attr) {
        let before = xml[..i].chars().last();
        if matches!(before, Some(c) if c == ':' || c == ' ') {
            let start = i + attr.len();
            if let Some(end_rel) = xml[start..].find('"') {
                let val = decode_entities(xml[start..start + end_rel].trim());
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Decode the five predefined XML entities. Sufficient for display text.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn looks_like_xml(text: &str) -> bool {
    let t = text.trim_start();
    t.starts_with("<?xpacket") || t.starts_with("<x:xmpmeta") || t.starts_with("<?xml")
}

// ── Date helpers ────────────────────────────────────────────────────────────

/// Reduce an ISO-8601 timestamp to `YYYY-MM-DD HH:MM:SS`, dropping fractional
/// seconds and timezone. Leaves anything that doesn't match untouched.
fn clean_datetime(iso: &str) -> String {
    let s = iso.trim();
    // Replace the date/time separator and cut at the first `.`, `+`, or `Z`.
    let s = s.replacen('T', " ", 1);
    let cut = s.find(['.', '+', 'Z']).unwrap_or(s.len());
    // Keep a trailing ` -` zone offset out too (rare in XMP, but harmless).
    s[..cut].trim().to_string()
}

/// Pull the capture time out of a `VRChat_2025-01-02_15-04-05.123...` filename.
/// Returns `YYYY-MM-DD HH:MM:SS`, or `None` if no such stamp is present.
fn date_from_filename(name: &str) -> Option<String> {
    let i = name.find("VRChat_")? + "VRChat_".len();
    let rest = name.get(i..)?;
    // Expect `YYYY-MM-DD_HH-MM-SS` = 19 chars.
    let stamp: &str = rest.get(..19)?;
    let bytes = stamp.as_bytes();
    // Positions of the separators in the fixed-width stamp.
    let ok = bytes.iter().enumerate().all(|(k, &b)| match k {
        4 | 7 => b == b'-',
        10 => b == b'_',
        13 | 16 => b == b'-',
        _ => b.is_ascii_digit(),
    });
    if !ok {
        return None;
    }
    let date = &stamp[..10];
    let time = stamp[11..].replace('-', ":");
    Some(format!("{date} {time}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a minimal valid PNG carrying the given `(keyword, text)` iTXt chunks.
    fn png_with_itxt(chunks: &[(&str, &str)]) -> Vec<u8> {
        fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(data);
            // CRC value is irrelevant to our reader; zero it.
            out.extend_from_slice(&[0, 0, 0, 0]);
            out
        }
        let mut png = Vec::new();
        png.write_all(PNG_SIG).unwrap();
        // A token IHDR so the stream looks plausible (contents unused by us).
        png.extend(chunk(b"IHDR", &[0; 13]));
        for (kw, text) in chunks {
            let mut data = Vec::new();
            data.extend_from_slice(kw.as_bytes());
            data.push(0); // keyword terminator
            data.push(0); // comp flag (uncompressed)
            data.push(0); // comp method
            data.push(0); // empty language + terminator
            data.push(0); // empty translated keyword + terminator
            data.extend_from_slice(text.as_bytes());
            png.extend(chunk(b"iTXt", &data));
        }
        png.extend(chunk(b"IEND", &[]));
        png
    }

    #[test]
    fn parses_line_format() {
        let line = "lfs|2|author:usr_abc,Alice|world:wrld_xyz,12345~private,Cool World|pos:1,2,3";
        let png = png_with_itxt(&[("Description", line)]);
        let m = parse_png(&png, "shot.png").unwrap();
        assert_eq!(m.world_name.as_deref(), Some("Cool World"));
        assert_eq!(m.author_name.as_deref(), Some("Alice"));
        assert_eq!(m.source_format, "line");
    }

    #[test]
    fn world_name_with_comma_is_kept_whole() {
        let line = "screenshotmanager|1|world:wrld_xyz,inst,Hello, World";
        let png = png_with_itxt(&[("Description", line)]);
        let m = parse_png(&png, "x.png").unwrap();
        assert_eq!(m.world_name.as_deref(), Some("Hello, World"));
    }

    #[test]
    fn parses_vrcx_json() {
        let json = r#"{"world":{"id":"wrld_x","name":"Json World"},"author":{"id":"usr_y","displayName":"Bob"}}"#;
        let png = png_with_itxt(&[("Description", json)]);
        let m = parse_png(&png, "VRChat_2025-01-02_15-04-05.123_1920x1080.png").unwrap();
        assert_eq!(m.world_name.as_deref(), Some("Json World"));
        assert_eq!(m.author_name.as_deref(), Some("Bob"));
        assert_eq!(m.source_format, "json");
        // No date in JSON → recovered from the filename.
        assert_eq!(m.captured_at.as_deref(), Some("2025-01-02 15:04:05"));
    }

    #[test]
    fn json_preferred_over_xmp_but_xmp_supplies_date() {
        let json = r#"{"world":{"name":"Json World"},"author":{"displayName":"Bob"}}"#;
        let xmp = r#"<x:xmpmeta xmlns:vrc="http://ns.vrchat.com/vrc/1.0/" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
            <vrc:WorldDisplayName>Xmp World</vrc:WorldDisplayName>
            <xmp:CreateDate>2024-12-31T23:59:58.500+09:00</xmp:CreateDate></x:xmpmeta>"#;
        let png = png_with_itxt(&[("Description", json), ("XML:com.adobe.xmp", xmp)]);
        let m = parse_png(&png, "no-date.png").unwrap();
        assert_eq!(m.world_name.as_deref(), Some("Json World")); // JSON wins
        assert_eq!(m.captured_at.as_deref(), Some("2024-12-31 23:59:58")); // XMP date
    }

    #[test]
    fn parses_xmp_attribute_form() {
        let xmp = r#"<x:xmpmeta><rdf:Description vrc:WorldDisplayName="Attr World" xmp:Author="Carol"
            xmp:CreateDate="2025-06-01T08:00:00Z"/></x:xmpmeta>"#;
        let png = png_with_itxt(&[("XML:com.adobe.xmp", xmp)]);
        let m = parse_png(&png, "x.png").unwrap();
        assert_eq!(m.world_name.as_deref(), Some("Attr World"));
        assert_eq!(m.author_name.as_deref(), Some("Carol"));
        assert_eq!(m.captured_at.as_deref(), Some("2025-06-01 08:00:00"));
        assert_eq!(m.source_format, "xmp");
    }

    #[test]
    fn non_vrchat_png_returns_none() {
        let png = png_with_itxt(&[("Comment", "just a regular screenshot")]);
        assert!(parse_png(&png, "photo.png").is_none());
    }

    #[test]
    fn non_png_returns_none() {
        assert!(parse_png(b"not a png at all", "x.png").is_none());
    }

    #[test]
    fn xml_entities_are_decoded() {
        let xmp = r#"<x:xmpmeta><vrc:WorldDisplayName>Tom &amp; Jerry&apos;s</vrc:WorldDisplayName></x:xmpmeta>"#;
        let png = png_with_itxt(&[("XML:com.adobe.xmp", xmp)]);
        let m = parse_png(&png, "x.png").unwrap();
        assert_eq!(m.world_name.as_deref(), Some("Tom & Jerry's"));
    }
}
