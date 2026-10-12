//! KG-11: the asset import contract of an engine profile. `validate` refuses a contract-breaking asset (naming, type,
//! PNG header, atlas metadata, audio format, .glb budget and LODs) with one reason each, before the asset is committed;
//! `style_check` flags a sprite that does not look like the rest of its set.
use regex::Regex;
use serde::Serialize;

pub use crate::assetstyle::style_check;

/// One file of an import batch: its path inside the project and its bytes.
#[derive(Debug)]
pub struct Asset {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub path: String,
    pub rule: String,
    pub message: String,
}

impl Issue {
    pub fn new(path: &str, rule: &str, message: impl Into<String>) -> Issue {
        Issue { path: path.to_string(), rule: rule.to_string(), message: message.into() }
    }
}

/// The contract of one engine.
pub struct Profile {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
}

impl Profile {
    pub fn named(name: &str) -> Option<Profile> {
        match name {
            "kk-engine" => Some(Profile { name: "kk-engine", extensions: &["png", "ogg", "wav", "glb", "json"] }),
            _ => None,
        }
    }
}

/// Checks every asset against the profile; issues are sorted by path.
pub fn validate(p: &Profile, assets: &[Asset]) -> Vec<Issue> {
    let segment = Regex::new(r"^[a-z0-9][a-z0-9_]*(\.[a-z0-9]+)?$").expect("segment pattern");
    let mut issues = Vec::new();
    let mut named: Vec<&Asset> = Vec::new();
    for a in assets {
        if !a.path.split('/').all(|s| segment.is_match(s)) {
            issues.push(Issue::new(&a.path, "name", "names must be lowercase snake_case (a-z, 0-9, _), without spaces, capitals or '..'"));
            continue;
        }
        let ext = a.path.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        if !p.extensions.contains(&ext) {
            issues.push(Issue::new(&a.path, "type", format!("'.{ext}' is not an accepted type for {} (png, ogg, wav, glb, json)", p.name)));
            continue;
        }
        named.push(a);
    }
    let ok: Vec<Asset> = named.iter().map(|a| Asset { path: a.path.clone(), bytes: a.bytes.clone() }).collect();
    issues.extend(crate::assetimage::check_images(&ok));
    issues.extend(crate::assetaudio::check_audio(&ok));
    issues.extend(crate::assetmodel::check_models(&ok));
    issues.sort_by(|a, b| a.path.cmp(&b.path));
    issues
}

#[cfg(test)]
#[path = "assetcontract_tests.rs"]
mod tests;
