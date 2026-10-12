//! KG-11: .glb rules (header, 32 MB, 100000 triangles) and LOD chains (`name_lod0.glb`, `name_lod1.glb`, ...).
use crate::assetcontract::{Asset, Issue};
use regex::Regex;
use serde_json::Value;
use std::collections::BTreeMap;

const MAX_BYTES: usize = 32 << 20;
const MAX_TRIANGLES: u64 = 100_000;

fn le32(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn triangles(b: &[u8]) -> Result<u64, String> {
    if b.get(..4) != Some(&b"glTF"[..]) || le32(b, 4) != Some(2) {
        return Err("not a glTF 2 binary (.glb) file".into());
    }
    if le32(b, 8) != Some(b.len()) {
        return Err("the length in the header does not match the file".into());
    }
    let len = le32(b, 12).ok_or("cut off")?;
    if b.get(16..20) != Some(&b"JSON"[..]) {
        return Err("first chunk is not JSON".into());
    }
    let json: Value = serde_json::from_slice(b.get(20..20 + len).ok_or("JSON chunk is cut off")?).map_err(|e| format!("JSON chunk: {e}"))?;
    let count = |i: Option<u64>| i.and_then(|i| json["accessors"].get(i as usize)).and_then(|a| a["count"].as_u64());
    let mut total = 0;
    for prim in json["meshes"].as_array().into_iter().flatten().flat_map(|m| m["primitives"].as_array().into_iter().flatten()) {
        if prim["mode"].as_u64().is_some_and(|m| m != 4) {
            continue;
        }
        total += count(prim["indices"].as_u64()).or_else(|| count(prim["attributes"]["POSITION"].as_u64())).unwrap_or(0) / 3;
    }
    Ok(total)
}

pub fn check_models(assets: &[&Asset]) -> Vec<Issue> {
    let lod = Regex::new(r"^(.*)_lod(\d+)\.glb$").expect("lod pattern");
    let mut issues = Vec::new();
    let mut groups: BTreeMap<String, Vec<(u32, &str, u64)>> = BTreeMap::new();
    for a in assets.iter().filter(|a| a.path.ends_with(".glb")) {
        let tris = match triangles(&a.bytes) {
            Err(e) => {
                issues.push(Issue::new(&a.path, "glb", e));
                continue;
            }
            Ok(t) if t > MAX_TRIANGLES || a.bytes.len() > MAX_BYTES => {
                issues.push(Issue::new(&a.path, "glb", format!("{t} triangles or {} bytes is over the budget (100000 triangles, 32 MB)", a.bytes.len())));
                continue;
            }
            Ok(t) => t,
        };
        if let Some(c) = lod.captures(&a.path) {
            groups.entry(c[1].to_string()).or_default().push((c[2].parse().unwrap_or(u32::MAX), &a.path, tris));
        }
    }
    for lods in groups.values_mut() {
        lods.sort();
        for (i, w) in lods.iter().enumerate() {
            let problem = match i {
                0 if w.0 != 0 => Some("a LOD chain must start with lod0".to_string()),
                0 => None,
                _ if w.0 != lods[i - 1].0 + 1 => Some(format!("lod{} is missing before lod{}", lods[i - 1].0 + 1, w.0)),
                _ if w.2 >= lods[i - 1].2 => Some(format!("lod{} has {} triangles, not fewer than lod{} ({})", w.0, w.2, lods[i - 1].0, lods[i - 1].2)),
                _ => None,
            };
            if let Some(m) = problem {
                issues.push(Issue::new(w.1, "lod", m));
            }
        }
    }
    issues
}
