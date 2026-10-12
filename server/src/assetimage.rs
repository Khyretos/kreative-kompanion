//! KG-11: PNG header rules and atlas metadata for the asset contract, plus the PNG decoder the style check uses.
use crate::assetcontract::{Asset, Issue};
use flate2::read::ZlibDecoder;
use serde_json::Value;
use std::collections::HashSet;
use std::io::Read;

const MAX_DIM: u32 = 4096;
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

pub struct Header {
    pub width: u32,
    pub height: u32,
    pub channels: usize,
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Reads IHDR; an error says what is wrong with the file.
pub fn header(bytes: &[u8]) -> Result<Header, String> {
    if bytes.get(..8) != Some(&SIGNATURE[..]) || bytes.get(12..16) != Some(&b"IHDR"[..]) {
        return Err("not a PNG file (signature or IHDR missing)".into());
    }
    let (w, h) = (be32(bytes, 16).unwrap_or(0), be32(bytes, 20).unwrap_or(0));
    let (depth, colour, interlace) = (bytes.get(24).copied().unwrap_or(0), bytes.get(25).copied().unwrap_or(0), bytes.get(28).copied().unwrap_or(0));
    if w == 0 || h == 0 || w > MAX_DIM || h > MAX_DIM {
        return Err(format!("size {w}x{h} is outside 1..{MAX_DIM} pixels"));
    }
    if depth != 8 || !matches!(colour, 2 | 6) || interlace != 0 {
        return Err("must be an 8-bit RGB or RGBA PNG without interlacing".into());
    }
    Ok(Header { width: w, height: h, channels: if colour == 6 { 4 } else { 3 } })
}

/// RGBA pixels (4 bytes each) of a PNG that passes `header`.
pub fn decode(bytes: &[u8]) -> Option<(Header, Vec<u8>)> {
    let head = header(bytes).ok()?;
    let (mut at, mut idat) = (8, Vec::new());
    while let Some(len) = be32(bytes, at) {
        let kind = bytes.get(at + 4..at + 8)?;
        if kind == b"IDAT" {
            idat.extend(bytes.get(at + 8..at + 8 + len as usize)?);
        }
        at += 12 + len as usize;
    }
    let mut raw = Vec::new();
    ZlibDecoder::new(&idat[..]).take(64 << 20).read_to_end(&mut raw).ok()?;
    let (w, c) = (head.width as usize, head.channels);
    let stride = w * c;
    if raw.len() < (stride + 1) * head.height as usize {
        return None;
    }
    let mut rows: Vec<Vec<u8>> = Vec::new();
    for y in 0..head.height as usize {
        let line = &raw[y * (stride + 1)..(y + 1) * (stride + 1)];
        let mut cur = line[1..].to_vec();
        let prev = rows.last();
        for i in 0..stride {
            let a = if i >= c { cur[i - c] as i32 } else { 0 };
            let b = prev.map_or(0, |p| p[i] as i32);
            let cc = if i >= c { prev.map_or(0, |p| p[i - c] as i32) } else { 0 };
            let add = match line[0] {
                0 => 0,
                1 => a,
                2 => b,
                3 => (a + b) / 2,
                4 => {
                    let p = a + b - cc;
                    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - cc).abs());
                    if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { cc }
                }
                _ => return None,
            };
            cur[i] = cur[i].wrapping_add(add as u8);
        }
        rows.push(cur);
    }
    let mut rgba = Vec::with_capacity(w * head.height as usize * 4);
    for row in &rows {
        for px in row.chunks(c) {
            rgba.extend(&px[..3]);
            rgba.push(if c == 4 { px[3] } else { 255 });
        }
    }
    Some((head, rgba))
}

fn stem(path: &str) -> &str {
    path.rsplit_once('.').map_or(path, |(s, _)| s)
}

fn frame_problem(f: &Value, w: u64, h: u64, names: &mut HashSet<String>) -> Option<String> {
    let num = |k: &str| f.get(k).and_then(Value::as_u64);
    let name = f.get("name").and_then(Value::as_str).unwrap_or("");
    let (Some(x), Some(y), Some(fw), Some(fh)) = (num("x"), num("y"), num("w"), num("h")) else {
        return Some(format!("frame '{name}' needs name, x, y, w and h"));
    };
    if name.is_empty() || !names.insert(name.to_string()) {
        return Some(format!("frame name '{name}' is empty or used twice"));
    }
    if fw == 0 || fh == 0 || x + fw > w || y + fh > h {
        return Some(format!("frame '{name}' lies outside the {w}x{h} image"));
    }
    None
}

/// Header rules for every PNG; atlas rules for `*_atlas.png` and every .json.
pub fn check_images(assets: &[&Asset]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for a in assets.iter().filter(|a| a.path.ends_with(".png")) {
        let head = header(&a.bytes);
        if let Err(e) = &head {
            issues.push(Issue::new(&a.path, "png", e.clone()));
        }
        let json = format!("{}.json", stem(&a.path));
        if stem(&a.path).ends_with("_atlas") && !assets.iter().any(|j| j.path == json) {
            issues.push(Issue::new(&a.path, "atlas", format!("an atlas needs its frame metadata '{json}' in the same import")));
        }
    }
    for a in assets.iter().filter(|a| a.path.ends_with(".json")) {
        let Ok(v) = serde_json::from_slice::<Value>(&a.bytes) else {
            issues.push(Issue::new(&a.path, "atlas", "metadata is not valid JSON"));
            continue;
        };
        let size = assets.iter().find(|p| p.path == format!("{}.png", stem(&a.path))).and_then(|p| header(&p.bytes).ok());
        let Some(frames) = v.get("frames").and_then(Value::as_array) else {
            if size.is_some() {
                issues.push(Issue::new(&a.path, "atlas", "atlas metadata needs a 'frames' list"));
            }
            continue;
        };
        let (w, h) = size.map_or((u64::MAX / 4, u64::MAX / 4), |s| (s.width as u64, s.height as u64));
        let mut names = HashSet::new();
        if let Some(msg) = frames.iter().find_map(|f| frame_problem(f, w, h, &mut names)) {
            issues.push(Issue::new(&a.path, "atlas", msg));
        }
    }
    issues
}
