//! KG-11: audio rules (.wav PCM 16-bit, .ogg Vorbis, 44.1 or 48 kHz, mono or stereo, 20 MB).
use crate::assetcontract::{Asset, Issue};

const MAX_BYTES: usize = 20 << 20;

fn u16le(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn wav(b: &[u8]) -> Result<(u32, u16), String> {
    if b.get(..4) != Some(&b"RIFF"[..]) || b.get(8..16) != Some(&b"WAVEfmt "[..]) {
        return Err("not a RIFF/WAVE file with a fmt chunk".into());
    }
    let (format, channels, rate, bits) = (u16le(b, 20), u16le(b, 22), u32le(b, 24), u16le(b, 34));
    match (format, channels, rate, bits) {
        (Some(1), Some(c), Some(r), Some(16)) => Ok((r, c)),
        _ => Err("must be 16-bit PCM".into()),
    }
}

fn ogg(b: &[u8]) -> Result<(u32, u16), String> {
    let at = b.windows(7).position(|w| w == b"\x01vorbis").filter(|_| b.starts_with(b"OggS"));
    let at = at.ok_or("not an Ogg Vorbis file")?;
    match (b.get(at + 11), u32le(b, at + 12)) {
        (Some(c), Some(r)) => Ok((r, *c as u16)),
        _ => Err("Vorbis header is cut off".into()),
    }
}

pub fn check_audio(assets: &[&Asset]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for a in assets.iter().filter(|a| a.path.ends_with(".wav") || a.path.ends_with(".ogg")) {
        let parsed = if a.path.ends_with(".wav") { wav(&a.bytes) } else { ogg(&a.bytes) };
        let problem = match parsed {
            Err(e) => Some(e),
            Ok((rate, ch)) if rate != 44100 && rate != 48000 => Some(format!("sample rate {rate} Hz; use 44100 or 48000 Hz (ch {ch})")),
            Ok((_, ch)) if !(1..=2).contains(&ch) => Some(format!("{ch} channels; use mono or stereo")),
            _ if a.bytes.len() > MAX_BYTES => Some("larger than 20 MB".into()),
            _ => None,
        };
        if let Some(m) = problem {
            issues.push(Issue::new(&a.path, "audio", m));
        }
    }
    issues
}
