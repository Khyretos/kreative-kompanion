use std::fs::{self, canonicalize};
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::grants::{Grants, Right};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum Tool {
    /// `offset`/`length` (bytes) read one slice of a big file; without them the whole file is read.
    ReadFile { path: String, #[serde(default)] offset: Option<u64>, #[serde(default)] length: Option<u64> },
    /// IMAGE-01: a PNG, JPEG, GIF or WebP as a data URL (`data:image/png;base64,...`) for a model that can see.
    ReadImage { path: String },
    /// WRITE-CHUNKS: a big file is written in parts: `append` adds to the end (making the file if
    /// needed); `offset` cuts the file to that many bytes first, so a part sent again lands in the
    /// same place. Neither: create or replace.
    WriteFile { path: String, content: String, #[serde(default)] append: bool, #[serde(default)] offset: Option<u64> },
    ListDir { path: String },
    Shell { cwd: String, command: String },
    EditFile { path: String, old: String, new: String },
    Service { action: String, #[serde(default)] unit: Option<String> },
    Package { manager: String, action: String, names: Vec<String> },
    Reload { what: String },
    /// GPU-01: start, stop or status of the studio apps, or Ollama unload.
    GpuApps { action: String, #[serde(default)] app: Option<String> },
    SystemInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome { pub ok: bool, pub output: String }

/// Runs one tool call if the grants allow it. Never panics.
pub fn run(grants: &Grants, tool: &Tool, now: &str) -> Outcome {
    match tool {
        Tool::ReadFile { path, offset, length } => {
            let path = Path::new(path);
            let canonical_path = match canonicalize(path) {
                Ok(p) => p,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Outcome { ok: false, output: format!("no such file or folder: {}", path.display()) },
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, path.display()) },
            };

            if !grants.allows(&canonical_path, &Right::Read, now) {
                return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) };
            }

            if canonical_path.is_file() {
                let metadata = match fs::metadata(&canonical_path) {
                    Ok(m) => m,
                    Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) },
                };

                const CAP: u64 = 256 * 1024;
                if offset.is_some() || length.is_some() {
                    use std::io::{Read, Seek, SeekFrom};
                    let start = offset.unwrap_or(0);
                    // The server keeps only the first 64 KiB of a result, so a slice stays under
                    // that with room for the trailing "continue with offset" note.
                    const SLICE_CAP: u64 = 60 * 1024;
                    let want = length.unwrap_or(SLICE_CAP).min(SLICE_CAP);
                    let size = metadata.len();
                    if start > size {
                        return Outcome { ok: false, output: format!("offset {start} is past the end of the file ({size} bytes): {}", canonical_path.display()) };
                    }
                    let mut f = match fs::File::open(&canonical_path) {
                        Ok(f) => f,
                        Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) },
                    };
                    if f.seek(SeekFrom::Start(start)).is_err() {
                        return Outcome { ok: false, output: format!("cannot seek in: {}", canonical_path.display()) };
                    }
                    let mut buf = Vec::new();
                    if f.take(want).read_to_end(&mut buf).is_err() {
                        return Outcome { ok: false, output: format!("cannot read: {}", canonical_path.display()) };
                    }
                    let end = start + buf.len() as u64;
                    let text = String::from_utf8_lossy(&buf).into_owned();
                    let note = if end < size { format!("\n[bytes {start}-{end} of {size}; more follows, continue with offset {end}]") } else { format!("\n[bytes {start}-{end} of {size}; end of file]") };
                    return Outcome { ok: true, output: format!("{text}{note}") };
                }

                if metadata.len() > CAP {
                    return Outcome { ok: false, output: format!("file too big ({} bytes, limit {CAP}): read it in slices with offset and length: {}", metadata.len(), canonical_path.display()) };
                }

                let content = match fs::read_to_string(&canonical_path) {
                    Ok(c) => c,
                    Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) },
                };

                Outcome { ok: true, output: content }
            } else {
                Outcome { ok: false, output: format!("not a file: {}", canonical_path.display()) }
            }
        },
        Tool::ReadImage { path } => {
            let path = Path::new(path);
            let canonical_path = match canonicalize(path) {
                Ok(p) => p,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Outcome { ok: false, output: format!("no such file or folder: {}", path.display()) },
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, path.display()) },
            };
            if !grants.allows(&canonical_path, &Right::Read, now) {
                return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) };
            }
            let size = match fs::metadata(&canonical_path) {
                Ok(m) if m.is_file() => m.len(),
                Ok(_) => return Outcome { ok: false, output: format!("not a file: {}", canonical_path.display()) },
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) },
            };
            if size > IMAGE_CAP {
                return Outcome { ok: false, output: format!("picture too big ({size} bytes, limit {IMAGE_CAP}): make a smaller copy first, then read that: {}", canonical_path.display()) };
            }
            let bytes = match fs::read(&canonical_path) {
                Ok(b) => b,
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) },
            };
            match image_mime(&bytes) {
                Some(mime) => Outcome { ok: true, output: format!("data:{mime};base64,{}", base64(&bytes)) },
                None => Outcome { ok: false, output: format!("not a PNG, JPEG, GIF or WebP picture: {}", canonical_path.display()) },
            }
        },
        Tool::WriteFile { path, content, append, offset } => {
            let path = Path::new(path);
            let parent_path = path.parent().unwrap_or(Path::new("."));
            let canonical_parent = match canonicalize(parent_path) {
                Ok(p) => p,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Outcome { ok: false, output: format!("no such folder: {}", parent_path.display()) },
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Write, path.display()) },
            };

            if !grants.allows(&canonical_parent, &Right::Write, now) {
                return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Write, canonical_parent.display()) };
            }

            if content.len() > 1024 * 1024 {
                return Outcome { ok: false, output: format!("content too big: {}", path.display()) };
            }

            let canonical_path = canonical_parent.join(path.file_name().unwrap_or_default());
            if *append && offset.is_some() {
                return Outcome { ok: false, output: "give append or offset, not both".into() };
            }
            // WRITE-CHUNKS: a file grown in parts stays within this.
            const TOTAL_CAP: u64 = 64 * 1024 * 1024;
            let size = fs::metadata(&canonical_path).map(|m| m.len()).ok();
            if let Some(at) = offset {
                match size {
                    None => return Outcome { ok: false, output: format!("no such file to write at offset {at} (write the first part without offset): {}", canonical_path.display()) },
                    Some(len) if *at > len => return Outcome { ok: false, output: format!("offset {at} is past the end of the file ({len} bytes): {}", canonical_path.display()) },
                    _ => {}
                }
            }
            let start = if *append { size.unwrap_or(0) } else { offset.unwrap_or(0) };
            if start + content.len() as u64 > TOTAL_CAP {
                return Outcome { ok: false, output: format!("file would pass {} MiB: {}", TOTAL_CAP / 1024 / 1024, canonical_path.display()) };
            }
            let opened = if *append {
                fs::OpenOptions::new().append(true).create(true).open(&canonical_path)
            } else if let Some(at) = offset {
                fs::OpenOptions::new().write(true).open(&canonical_path).and_then(|f| f.set_len(*at).map(|_| f)).and_then(|mut f| {
                    use std::io::{Seek, SeekFrom};
                    f.seek(SeekFrom::Start(*at)).map(|_| f)
                })
            } else {
                fs::File::create(&canonical_path)
            };
            let mut file = match opened {
                Ok(f) => f,
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Write, canonical_path.display()) },
            };

            if file.write_all(content.as_bytes()).is_err() {
                return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Write, canonical_path.display()) };
            }

            let now_size = start + content.len() as u64;
            if *append || offset.is_some() {
                Outcome { ok: true, output: format!("wrote {} bytes at byte {start} of {}; the file is {now_size} bytes now (next part: append, or offset {now_size})", content.len(), canonical_path.display()) }
            } else {
                Outcome { ok: true, output: format!("wrote {}", canonical_path.display()) }
            }
        },
        Tool::ListDir { path } => {
            let path = Path::new(path);
            let canonical_path = match canonicalize(path) {
                Ok(p) => p,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Outcome { ok: false, output: format!("no such file or folder: {}", path.display()) },
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, path.display()) },
            };

            if !grants.allows(&canonical_path, &Right::Read, now) {
                return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Read, canonical_path.display()) };
            }

            if !canonical_path.is_dir() {
                return Outcome { ok: false, output: format!("not a directory: {}", canonical_path.display()) };
            }

            let mut entries = Vec::new();
            if let Ok(dir) = fs::read_dir(&canonical_path) {
                for entry in dir {
                    if let Ok(entry) = entry {
                        let entry_path = entry.path();
                        let entry_name = entry.file_name();
                        let entry_type = if entry_path.is_dir() { "dir" } else { "file" };
                        entries.push((entry_name.to_string_lossy().into_owned(), entry_type));
                    }
                }
            }

            entries.sort();

            let mut output = String::new();
            for (i, (name, entry_type)) in entries.iter().enumerate() {
                if i >= 500 {
                    break;
                }

                let name = if *entry_type == "dir" {
                    format!("{}{}", name, '/')
                } else {
                    name.to_string()
                };

                output.push_str(&name);
                output.push('\n');
            }

            Outcome { ok: true, output }
        },
        Tool::Shell { cwd, command } => {
            let cwd = Path::new(cwd);
            let canonical_cwd = match canonicalize(cwd) {
                Ok(p) => p,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Outcome { ok: false, output: format!("no such folder: {}", cwd.display()) },
                Err(_) => return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Shell, cwd.display()) },
            };

            if !grants.allows(&canonical_cwd, &Right::Shell, now) {
                return Outcome { ok: false, output: format!("not granted: {:?} on {}", Right::Shell, canonical_cwd.display()) };
            }

            let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
            crate::proc::run_cmd(shell, &[flag.to_string(), command.clone()], Some(&canonical_cwd), &[], 60)
        },
        Tool::EditFile { path, old, new } => crate::edit::edit_file(grants, path, old, new, now),
        Tool::Service { action, unit } => crate::systools::service(grants, action, unit.as_deref(), now),
        Tool::Package { manager, action, names } => crate::systools::package(grants, manager, action, names, now),
        Tool::Reload { what } => crate::systools::reload(grants, what, now),
        Tool::SystemInfo => crate::sysinfo::system_info(),
        Tool::GpuApps { action, app } => crate::gpuapps::gpu_apps(grants, crate::gpuapps::apps(), action, app.as_deref(), now),
    }
}

/// IMAGE-01: the biggest picture `read_image` sends (about 4 MiB as base64; the server takes 6 MiB).
const IMAGE_CAP: u64 = 3 * 1024 * 1024;

/// The picture type by its first bytes (never by the file name), None for anything else.
pub fn image_mime(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some("image/png")
    } else if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Standard base64 (RFC 4648, section 4) with `=` padding. The table holds the 64 alphabet symbols only;
/// `=` is padding, written by the last two lines of the loop, and is not part of the alphabet.
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
