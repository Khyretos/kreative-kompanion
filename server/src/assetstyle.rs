//! KG-11: style consistency of a sprite set. PNGs of one folder (atlases excluded, at least 3) are compared with the set's
//! median on mean luminance, mean saturation and the number of distinct colours; an outlier is flagged with the numbers.
use crate::assetcontract::{Asset, Issue};
use crate::assetimage;
use std::collections::{BTreeMap, HashSet};

const MIN_SET: usize = 3;
const LUMINANCE: f64 = 0.25;
const SATURATION: f64 = 0.3;
const COLOUR_LOG2: f64 = 2.0;

struct Stats {
    luminance: f64,
    saturation: f64,
    colours: f64,
}

fn stats(bytes: &[u8]) -> Option<Stats> {
    let (_, rgba) = assetimage::decode(bytes)?;
    let (mut n, mut lum, mut sat) = (0.0, 0.0, 0.0);
    let mut seen = HashSet::new();
    for px in rgba.chunks(4).filter(|p| p[3] > 0) {
        let (r, g, b) = (px[0] as f64 / 255.0, px[1] as f64 / 255.0, px[2] as f64 / 255.0);
        let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
        lum += 0.2126 * r + 0.7152 * g + 0.0722 * b;
        sat += if hi > 0.0 { (hi - lo) / hi } else { 0.0 };
        seen.insert((px[0] >> 4, px[1] >> 4, px[2] >> 4));
        n += 1.0;
    }
    (n > 0.0).then(|| Stats { luminance: lum / n, saturation: sat / n, colours: seen.len() as f64 })
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    let m = v.len() / 2;
    if v.len() % 2 == 1 { v[m] } else { (v[m - 1] + v[m]) / 2.0 }
}

pub fn style_check(assets: &[Asset]) -> Vec<Issue> {
    let mut sets: BTreeMap<&str, Vec<(&str, Stats)>> = BTreeMap::new();
    for a in assets.iter().filter(|a| a.path.ends_with(".png") && !a.path.ends_with("_atlas.png")) {
        if let Some(s) = stats(&a.bytes) {
            sets.entry(a.path.rsplit_once('/').map_or("", |(d, _)| d)).or_default().push((&a.path, s));
        }
    }
    let mut issues = Vec::new();
    for set in sets.values().filter(|s| s.len() >= MIN_SET) {
        let med = |f: fn(&Stats) -> f64| median(set.iter().map(|(_, s)| f(s)).collect());
        let (ml, ms, mc) = (med(|s| s.luminance), med(|s| s.saturation), med(|s| s.colours));
        for (path, s) in set {
            let problem = if (s.luminance - ml).abs() > LUMINANCE {
                Some(format!("brightness {:.2} against {:.2} for the rest of the set", s.luminance, ml))
            } else if (s.saturation - ms).abs() > SATURATION {
                Some(format!("colour saturation {:.2} against {:.2} for the rest of the set", s.saturation, ms))
            } else if (s.colours / mc).log2().abs() > COLOUR_LOG2 {
                Some(format!("{} distinct colours against {} for the rest of the set", s.colours, mc))
            } else {
                None
            };
            if let Some(m) = problem {
                issues.push(Issue::new(path, "style", format!("does not match the style of its set: {m}")));
            }
        }
    }
    issues.sort_by(|a, b| a.path.cmp(&b.path));
    issues
}
