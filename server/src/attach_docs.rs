//! CHAT-09: a text view of any attached file, so a model of any size can read it in parts (attach_read). Documents give
//! their text (Word, Excel, PowerPoint, OpenDocument, ePub, RTF, PDF pages), archives their listing, media their
//! properties, and any other binary the readable strings in it. The original is kept next to the view for download.

use std::{io::Read, path::Path, sync::LazyLock};
use regex::Regex;
use serde_json::Value;
use crate::assets::zipindex::{self, Entry};

/// The most of one document part (an XML file in a zip, a decompressed file) that is read into memory.
const PART: u64 = 64 << 20;
/// Entries one archive listing shows at most.
const LISTED: usize = 100_000;
/// Bytes of an unknown binary that are scanned for readable strings, and the most such text kept.
const SCAN: usize = 4 << 20;
const STRINGS_KEPT: usize = 100_000;

pub struct View {
    pub text: String,
}

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid regex")
}

pub fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

pub fn human(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") } else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) } else if bytes < 1 << 30 { format!("{:.1} MB", bytes as f64 / 1048576.0) } else { format!("{:.2} GB", bytes as f64 / 1073741824.0) }
}

/// Files that have a reader here (kept as they are, with a text view next to them), whatever their content looks like.
pub fn has_reader(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".tar.gz") || matches!(ext_of(name).as_str(),
        "docx" | "docm" | "dotx" | "xlsx" | "xlsm" | "xltx" | "pptx" | "pptm" | "ppsx" | "odt" | "ods" | "odp" | "epub" | "rtf"
        | "zip" | "jar" | "apk" | "tar" | "tgz" | "gz")
}

const MEDIA: [&str; 30] = ["mp3", "wav", "flac", "ogg", "oga", "opus", "m4a", "aac", "wma", "mp4", "m4v", "mkv", "mov", "avi", "webm", "wmv", "flv", "mpg", "mpeg", "ts", "3gp", "gif", "bmp", "tif", "tiff", "heic", "avif", "ico", "aiff", "mid"];

// ---------------------------------------------------------------- XML text

pub fn unescape(s: &str) -> String {
    static ENT: LazyLock<Regex> = LazyLock::new(|| re(r"&(#x[0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos);"));
    if !s.contains('&') { return s.to_string() }
    ENT.replace_all(s, |c: &regex::Captures| {
        let e = &c[1];
        match e {
            "amp" => "&".to_string(), "lt" => "<".to_string(), "gt" => ">".to_string(), "quot" => "\"".to_string(), "apos" => "'".to_string(),
            _ => {
                let n = if let Some(h) = e.strip_prefix("#x") { u32::from_str_radix(h, 16).ok() } else { e[1..].parse().ok() };
                n.and_then(char::from_u32).map(String::from).unwrap_or_default()
            }
        }
    }).into_owned()
}

/// The text of an XML fragment: tags dropped, entities decoded.
pub fn xml_text(xml: &str) -> String {
    static TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<[^>]*>"));
    unescape(&TAG.replace_all(xml, ""))
}

fn push_line(out: &mut String, line: &str) {
    let line = line.trim_end();
    if line.is_empty() {
        if !out.is_empty() && !out.ends_with("\n\n") { out.push('\n') }
    } else {
        out.push_str(line);
        out.push('\n');
    }
}

fn entry_bytes(path: &Path, entries: &[Entry], name: &str) -> Option<Vec<u8>> {
    let e = entries.iter().find(|e| e.name == name)?;
    let mut buf = Vec::new();
    zipindex::copy_entry(path, e, &mut buf, PART).ok()?;
    Some(buf)
}

fn entry_text(path: &Path, entries: &[Entry], name: &str) -> Option<String> {
    entry_bytes(path, entries, name).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Entries named `<prefix><number><suffix>`, in numeric order.
fn numbered<'a>(entries: &'a [Entry], prefix: &str, suffix: &str) -> Vec<(u32, &'a Entry)> {
    let mut v: Vec<(u32, &Entry)> = entries.iter().filter_map(|e| {
        let n = e.name.strip_prefix(prefix)?.strip_suffix(suffix)?;
        Some((n.parse().ok()?, e))
    }).collect();
    v.sort_by_key(|(n, _)| *n);
    v
}

// ---------------------------------------------------------------- Office Open XML

fn heading_level(style: &str) -> Option<usize> {
    let s = style.to_ascii_lowercase().replace(' ', "");
    if s == "title" { return Some(1) }
    let n = s.strip_prefix("heading")?;
    Some(n.parse::<usize>().unwrap_or(1).clamp(1, 6))
}

/// The text of a Word document's body: a paragraph a line, headings as markdown headings.
pub fn docx_text(xml: &str) -> String {
    static PARA: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<w:p\b[^>]*?(?:/>|>(.*?)</w:p>)"));
    static TOKEN: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<w:t(?:\s[^>]*)?>(.*?)</w:t>|<w:tab\s*/>|<w:(?:br|cr)\b[^>]*/>"));
    static STYLE: LazyLock<Regex> = LazyLock::new(|| re(r#"<w:pStyle\s+w:val="([^"]*)""#));
    let mut out = String::new();
    for p in PARA.captures_iter(xml) {
        let inner = p.get(1).map_or("", |m| m.as_str());
        let mut line = String::new();
        for t in TOKEN.captures_iter(inner) {
            match t.get(1) {
                Some(text) => line.push_str(&unescape(text.as_str())),
                None => line.push(if t[0].starts_with("<w:tab") { '\t' } else { ' ' }),
            }
        }
        if let Some(level) = STYLE.captures(inner).and_then(|c| heading_level(&c[1])).filter(|_| !line.trim().is_empty()) {
            line = format!("{} {}", "#".repeat(level), line.trim());
        }
        push_line(&mut out, &line);
    }
    out
}

fn shared_strings(xml: &str) -> Vec<String> {
    static SI: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<si\b[^>]*>(.*?)</si>"));
    static PHONETIC: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<rPh\b.*?</rPh>"));
    static T: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<t(?:\s[^>]*)?>(.*?)</t>"));
    SI.captures_iter(xml).map(|c| {
        let s = PHONETIC.replace_all(&c[1], "");
        T.captures_iter(&s).map(|t| unescape(&t[1])).collect::<String>()
    }).collect()
}

fn column_index(letters: &str) -> usize {
    letters.bytes().fold(0usize, |n, b| n * 26 + (b.to_ascii_uppercase() - b'A' + 1) as usize).saturating_sub(1)
}

/// One worksheet as tab-separated rows (empty rows left out, cells placed in their columns).
pub fn sheet_rows(xml: &str, shared: &[String]) -> String {
    static ROW: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<row\b[^>]*?(?:/>|>(.*?)</row>)"));
    static CELL: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<c\b([^>]*?)(?:/>|>(.*?)</c>)"));
    static REF: LazyLock<Regex> = LazyLock::new(|| re(r#"\br="([A-Za-z]+)\d+""#));
    static TYPE: LazyLock<Regex> = LazyLock::new(|| re(r#"\bt="(\w+)""#));
    static VALUE: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<v>(.*?)</v>"));
    static INLINE: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<is>(.*?)</is>"));
    let mut out = String::new();
    for row in ROW.captures_iter(xml) {
        let mut cells: Vec<(usize, String)> = Vec::new();
        for (i, c) in CELL.captures_iter(row.get(1).map_or("", |m| m.as_str())).enumerate() {
            let attrs = &c[1];
            let body = c.get(2).map_or("", |m| m.as_str());
            let col = REF.captures(attrs).map_or(i, |r| column_index(&r[1]));
            let kind = TYPE.captures(attrs).map(|t| t[1].to_string()).unwrap_or_default();
            let value = match kind.as_str() {
                "inlineStr" => INLINE.captures(body).map(|m| xml_text(&m[1])).unwrap_or_default(),
                "s" => VALUE.captures(body).and_then(|v| v[1].trim().parse::<usize>().ok()).and_then(|n| shared.get(n).cloned()).unwrap_or_default(),
                "b" => VALUE.captures(body).map(|v| if v[1].trim() == "1" { "TRUE" } else { "FALSE" }.to_string()).unwrap_or_default(),
                _ => VALUE.captures(body).map(|v| unescape(&v[1])).unwrap_or_default(),
            };
            if !value.is_empty() { cells.push((col, value.replace(['\t', '\n', '\r'], " "))) }
        }
        let Some(&(last, _)) = cells.last() else { continue };
        let mut line = vec![String::new(); last + 1];
        for (col, v) in cells { if col < line.len() { line[col] = v } }
        out.push_str(&line.join("\t"));
        out.push('\n');
    }
    out
}

fn xlsx(path: &Path) -> Result<String, String> {
    let entries = zipindex::zip_entries(path).map_err(|e| e.to_string())?;
    static SHEET: LazyLock<Regex> = LazyLock::new(|| re(r#"<sheet\b[^>]*?\bname="([^"]*)""#));
    let shared = entry_text(path, &entries, "xl/sharedStrings.xml").map(|x| shared_strings(&x)).unwrap_or_default();
    let names: Vec<String> = entry_text(path, &entries, "xl/workbook.xml").map(|x| SHEET.captures_iter(&x).map(|c| unescape(&c[1])).collect()).unwrap_or_default();
    let mut out = String::new();
    for (i, (n, e)) in numbered(&entries, "xl/worksheets/sheet", ".xml").into_iter().enumerate() {
        let mut xml = Vec::new();
        zipindex::copy_entry(path, e, &mut xml, PART).map_err(|e| e.to_string())?;
        let name = names.get(i).cloned().unwrap_or_else(|| format!("sheet{n}"));
        out.push_str(&format!("=== sheet {name} ===\n{}\n", sheet_rows(&String::from_utf8_lossy(&xml), &shared)));
    }
    if out.is_empty() { return Err("no worksheets found".into()) }
    Ok(out)
}

fn docx(path: &Path) -> Result<String, String> {
    let entries = zipindex::zip_entries(path).map_err(|e| e.to_string())?;
    let xml = entry_text(path, &entries, "word/document.xml").ok_or("no word/document.xml in the file")?;
    let mut out = docx_text(&xml);
    for (what, part) in [("Footnotes", "word/footnotes.xml"), ("Comments", "word/comments.xml")] {
        if let Some(x) = entry_text(path, &entries, part) {
            let t = docx_text(&x);
            if !t.trim().is_empty() { out.push_str(&format!("\n=== {what} ===\n{t}")) }
        }
    }
    Ok(out)
}

pub fn slide_text(xml: &str) -> String {
    static PARA: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<a:p\b[^>]*?(?:/>|>(.*?)</a:p>)"));
    static TOKEN: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<a:t>(.*?)</a:t>|<a:br\s*/>"));
    let mut out = String::new();
    for p in PARA.captures_iter(xml) {
        let line: String = TOKEN.captures_iter(p.get(1).map_or("", |m| m.as_str())).map(|t| t.get(1).map_or("\n".to_string(), |x| unescape(x.as_str()))).collect();
        push_line(&mut out, &line);
    }
    out
}

fn pptx(path: &Path) -> Result<String, String> {
    let entries = zipindex::zip_entries(path).map_err(|e| e.to_string())?;
    let notes = numbered(&entries, "ppt/notesSlides/notesSlide", ".xml");
    let mut out = String::new();
    for (n, e) in numbered(&entries, "ppt/slides/slide", ".xml") {
        let mut xml = Vec::new();
        zipindex::copy_entry(path, e, &mut xml, PART).map_err(|e| e.to_string())?;
        out.push_str(&format!("=== slide {n} ===\n{}", slide_text(&String::from_utf8_lossy(&xml))));
        if let Some((_, ne)) = notes.iter().find(|(k, _)| *k == n) {
            let mut nx = Vec::new();
            if zipindex::copy_entry(path, ne, &mut nx, PART).is_ok() {
                let t = slide_text(&String::from_utf8_lossy(&nx));
                let t = t.lines().filter(|l| !l.trim().chars().all(|c| c.is_ascii_digit())).collect::<Vec<_>>().join("\n");
                if !t.trim().is_empty() { out.push_str(&format!("Notes: {}\n", t.trim())) }
            }
        }
        out.push('\n');
    }
    if out.is_empty() { return Err("no slides found".into()) }
    Ok(out)
}

// ---------------------------------------------------------------- OpenDocument, ePub, RTF

fn odf_inline(s: &str) -> String {
    static TAB: LazyLock<Regex> = LazyLock::new(|| re(r"<text:tab\s*/>"));
    static BREAK: LazyLock<Regex> = LazyLock::new(|| re(r"<text:line-break\s*/>"));
    static SPACES: LazyLock<Regex> = LazyLock::new(|| re(r#"<text:s(?:\s+text:c="(\d+)")?\s*/>"#));
    let s = TAB.replace_all(s, "\t");
    let s = BREAK.replace_all(&s, "\n");
    let s = SPACES.replace_all(&s, |c: &regex::Captures| " ".repeat(c.get(1).and_then(|n| n.as_str().parse().ok()).unwrap_or(1usize).min(200)));
    xml_text(&s)
}

/// The text of an OpenDocument text or presentation: headings as markdown headings, a paragraph a line, slides marked.
pub fn odf_text(xml: &str) -> String {
    static TOKEN: LazyLock<Regex> = LazyLock::new(|| re(r#"(?s)<text:h\b([^>]*?)>(.*?)</text:h>|<text:p\b[^>]*?>(.*?)</text:p>|<draw:page\b[^>]*?draw:name="([^"]*)""#));
    static LEVEL: LazyLock<Regex> = LazyLock::new(|| re(r#"text:outline-level="(\d+)""#));
    let (mut out, mut slides) = (String::new(), 0);
    for t in TOKEN.captures_iter(xml) {
        if let Some(name) = t.get(4) {
            slides += 1;
            out.push_str(&format!("\n=== slide {slides}: {} ===\n", unescape(name.as_str())));
        } else if let Some(h) = t.get(2) {
            let level = LEVEL.captures(&t[1]).and_then(|l| l[1].parse::<usize>().ok()).unwrap_or(1).clamp(1, 6);
            push_line(&mut out, &format!("{} {}", "#".repeat(level), odf_inline(h.as_str()).trim()));
        } else if let Some(p) = t.get(3) {
            push_line(&mut out, &odf_inline(p.as_str()));
        }
    }
    out
}

/// The sheets of an OpenDocument spreadsheet as tab-separated rows.
pub fn ods_text(xml: &str) -> String {
    static TABLE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?s)<table:table\s[^>]*?table:name="([^"]*)"[^>]*>(.*?)</table:table>"#));
    static ROW: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<table:table-row\b[^>]*?(?:/>|>(.*?)</table:table-row>)"));
    static CELL: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)<table:table-cell\b[^>]*?(?:/>|>(.*?)</table:table-cell>)"));
    static PARA_END: LazyLock<Regex> = LazyLock::new(|| re(r"</text:p>"));
    let mut out = String::new();
    for t in TABLE.captures_iter(xml) {
        out.push_str(&format!("=== sheet {} ===\n", unescape(&t[1])));
        for r in ROW.captures_iter(&t[2]) {
            let mut cells: Vec<String> = CELL.captures_iter(r.get(1).map_or("", |m| m.as_str()))
                .map(|c| odf_inline(&PARA_END.replace_all(c.get(1).map_or("", |m| m.as_str()), " ")).trim().replace(['\t', '\n'], " "))
                .collect();
            while cells.last().is_some_and(|c| c.is_empty()) { cells.pop(); }
            if !cells.is_empty() { out.push_str(&cells.join("\t")); out.push('\n') }
        }
        out.push('\n');
    }
    out
}

fn odf(path: &Path, ext: &str) -> Result<String, String> {
    let entries = zipindex::zip_entries(path).map_err(|e| e.to_string())?;
    let xml = entry_text(path, &entries, "content.xml").ok_or("no content.xml in the file")?;
    Ok(if ext == "ods" { ods_text(&xml) } else { odf_text(&xml) })
}

fn epub(path: &Path) -> Result<String, String> {
    let entries = zipindex::zip_entries(path).map_err(|e| e.to_string())?;
    let mut pages: Vec<&Entry> = entries.iter().filter(|e| matches!(ext_of(&e.name).as_str(), "xhtml" | "html" | "htm")).collect();
    pages.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = String::new();
    for e in pages {
        let mut buf = Vec::new();
        if zipindex::copy_entry(path, e, &mut buf, PART).is_err() { continue }
        let text = crate::web::html_text(&String::from_utf8_lossy(&buf));
        if !text.trim().is_empty() { out.push_str(&format!("=== chapter: {} ===\n{}\n\n", e.name, text.trim())) }
    }
    if out.is_empty() { return Err("no chapters found".into()) }
    Ok(out)
}

/// The text of an RTF document (control words dropped, fonts, styles and pictures skipped).
pub fn rtf_text(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let (mut out, mut i, mut depth, mut skip) = (String::new(), 0usize, 0usize, None::<usize>);
    while i < c.len() {
        match c[i] {
            '{' => {
                depth += 1;
                let rest: String = c[i + 1..(i + 14).min(c.len())].iter().collect();
                if skip.is_none() && ["\\*", "\\fonttbl", "\\colortbl", "\\stylesheet", "\\info", "\\pict", "\\header", "\\footer"].iter().any(|d| rest.starts_with(d)) { skip = Some(depth) }
                i += 1;
            }
            '}' => {
                if skip == Some(depth) { skip = None }
                depth = depth.saturating_sub(1);
                i += 1;
            }
            '\\' => {
                i += 1;
                let Some(&n) = c.get(i) else { break };
                if matches!(n, '\\' | '{' | '}') {
                    if skip.is_none() { out.push(n) }
                    i += 1;
                } else if n == '\'' {
                    let hex: String = c[i + 1..(i + 3).min(c.len())].iter().collect();
                    if let (None, Ok(b)) = (skip, u8::from_str_radix(&hex, 16)) { out.push(char::from(b)) }
                    i += 3;
                } else if n.is_ascii_alphabetic() {
                    let start = i;
                    while i < c.len() && c[i].is_ascii_alphabetic() { i += 1 }
                    let word: String = c[start..i].iter().collect();
                    let ns = i;
                    if i < c.len() && c[i] == '-' { i += 1 }
                    while i < c.len() && c[i].is_ascii_digit() { i += 1 }
                    let num: String = c[ns..i].iter().collect();
                    if i < c.len() && c[i] == ' ' { i += 1 }
                    if skip.is_none() {
                        match word.as_str() {
                            "par" | "line" | "sect" | "page" => out.push('\n'),
                            "tab" => out.push('\t'),
                            "u" => {
                                if let Some(ch) = num.parse::<i32>().ok().and_then(|v| char::from_u32(if v < 0 { (v + 65536) as u32 } else { v as u32 })) { out.push(ch) }
                                if i < c.len() && c[i] == '?' { i += 1 }
                            }
                            _ => {}
                        }
                    }
                } else {
                    i += 1;
                }
            }
            '\r' | '\n' => i += 1,
            ch => {
                if skip.is_none() { out.push(ch) }
                i += 1;
            }
        }
    }
    out.lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
}

// ---------------------------------------------------------------- archives

fn zip_listing(path: &Path) -> Result<String, String> {
    let entries = zipindex::zip_entries(path).map_err(|e| e.to_string())?;
    let total: u64 = entries.iter().map(|e| e.size).sum();
    let mut out = format!("{} entries, {} unpacked. Name, size:\n", entries.len(), human(total));
    for e in entries.iter().take(LISTED) {
        out.push_str(&format!("{}  ({}){}\n", e.name, human(e.size), if e.encrypted { " encrypted" } else { "" }));
    }
    if entries.len() > LISTED { out.push_str(&format!("... and {} more entries\n", entries.len() - LISTED)) }
    Ok(out)
}

fn tar_list<R: Read>(r: R) -> Result<String, String> {
    let mut archive = tar::Archive::new(r);
    let (mut out, mut n, mut total) = (String::new(), 0usize, 0u64);
    for e in archive.entries().map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let (size, name) = (e.size(), e.path().map(|p| p.display().to_string()).unwrap_or_default());
        n += 1;
        total += size;
        if n <= LISTED { out.push_str(&format!("{name}  ({})\n", human(size))) }
    }
    let more = if n > LISTED { format!("... and {} more entries\n", n - LISTED) } else { String::new() };
    Ok(format!("{n} entries, {} unpacked. Name, size:\n{out}{more}", human(total)))
}

fn tar_listing(path: &Path, gz: bool) -> Result<String, String> {
    let f = std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
    if gz { tar_list(flate2::read::GzDecoder::new(f)) } else { tar_list(f) }
}

fn gz_text(path: &Path) -> Option<(String, String)> {
    let f = std::io::BufReader::new(std::fs::File::open(path).ok()?);
    let mut buf = Vec::new();
    flate2::read::GzDecoder::new(f).take(PART).read_to_end(&mut buf).ok()?;
    if !crate::chat_attach::looks_like_text(&buf, true) { return Some(("gzip file".into(), format!("compressed data that is not text ({} unpacked, at least)", human(buf.len() as u64)))) }
    let cut = if buf.len() as u64 >= PART { format!("\n[only the first {} of the unpacked text is shown]\n", human(PART)) } else { String::new() };
    Some(("Compressed text file".into(), format!("{}{cut}", String::from_utf8_lossy(&buf))))
}

// ---------------------------------------------------------------- media and unknown binaries

fn magic_label(h: &[u8]) -> &'static str {
    let starts = |m: &[u8]| h.starts_with(m);
    if starts(b"%PDF") { "PDF document" } else if starts(b"PK\x03\x04") { "ZIP-based file" } else if starts(b"\x7fELF") { "ELF executable" }
    else if starts(b"MZ") { "Windows executable (PE)" } else if starts(b"\x1f\x8b") { "gzip data" } else if starts(b"BZh") { "bzip2 data" }
    else if starts(b"7z\xbc\xaf\x27\x1c") { "7-Zip archive (its content cannot be listed here)" } else if starts(b"Rar!") { "RAR archive (its content cannot be listed here)" }
    else if starts(b"SQLite format 3\0") { "SQLite database" } else if starts(b"\xd0\xcf\x11\xe0") { "Microsoft Office file in the old binary format" }
    else if starts(b"\x89PNG") { "PNG picture" } else if starts(b"GIF8") { "GIF picture" } else if starts(b"\xff\xd8\xff") { "JPEG picture" }
    else if starts(b"BLENDER") { "Blender scene" } else if starts(b"glTF") { "glTF model" } else if starts(b"Kaydara FBX Binary") { "FBX model" }
    else if starts(b"OggS") || starts(b"fLaC") || starts(b"ID3") { "audio" } else if h.len() > 11 && &h[4..8] == b"ftyp" { "MP4 or QuickTime media" }
    else if starts(b"\x1a\x45\xdf\xa3") { "Matroska or WebM media" } else if starts(b"RIFF") { "RIFF media (WAV or AVI)" }
    else if starts(b"wOFF") || starts(b"wOF2") || starts(b"OTTO") || starts(b"\0\x01\0\0") { "font" } else { "Binary file" }
}

fn strings_view(path: &Path, head: &[u8]) -> (String, String) {
    let mut buf = Vec::new();
    if let Ok(f) = std::fs::File::open(path) { f.take(SCAN as u64).read_to_end(&mut buf).ok(); }
    let (mut out, mut run) = (String::new(), String::new());
    for &b in buf.iter().chain(std::iter::once(&0u8)) {
        if (0x20..0x7f).contains(&b) || b == b'\t' {
            run.push(b as char);
        } else {
            if run.trim().len() >= 6 && out.len() < STRINGS_KEPT { out.push_str(run.trim()); out.push('\n') }
            run.clear();
        }
    }
    let body = if out.is_empty() {
        "no readable text found; the content is binary data".to_string()
    } else {
        format!("The text found inside the file (a rough view of a binary, not a document):\n{out}")
    };
    (magic_label(head).to_string(), body)
}

fn duration(secs: f64) -> String {
    let s = secs.round() as u64;
    format!("{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60)
}

/// What ffprobe (-print_format json) found, as a few lines.
pub fn probe_summary(v: &Value) -> Option<String> {
    let f = &v["format"];
    let mut out = String::new();
    let num = |x: &Value| x.as_str().and_then(|s| s.parse::<f64>().ok());
    out.push_str(&format!("Format: {}", f["format_long_name"].as_str().or(f["format_name"].as_str())?));
    if let Some(d) = num(&f["duration"]) { out.push_str(&format!(", duration {}", duration(d))) }
    if let Some(b) = num(&f["bit_rate"]) { out.push_str(&format!(", {:.0} kbit/s", b / 1000.0)) }
    out.push('\n');
    for (k, label) in [("title", "Title"), ("artist", "Artist"), ("album", "Album"), ("date", "Date"), ("genre", "Genre"), ("comment", "Comment")] {
        let tag = f["tags"].as_object().and_then(|t| t.iter().find(|(n, _)| n.eq_ignore_ascii_case(k))).and_then(|(_, v)| v.as_str());
        if let Some(t) = tag { out.push_str(&format!("{label}: {}\n", t.chars().take(200).collect::<String>())) }
    }
    for s in v["streams"].as_array().into_iter().flatten() {
        let codec = s["codec_name"].as_str().unwrap_or("?");
        let line = match s["codec_type"].as_str().unwrap_or("") {
            "video" => {
                let fps = s["r_frame_rate"].as_str().and_then(|r| r.split_once('/')).and_then(|(a, b)| Some(a.parse::<f64>().ok()? / b.parse::<f64>().ok().filter(|b| *b > 0.0)?));
                let size = match (s["width"].as_u64(), s["height"].as_u64()) { (Some(w), Some(h)) => format!(", {w}x{h}"), _ => String::new() };
                format!("Video: {codec}{size}{}", fps.filter(|f| *f > 1.0 && *f < 1000.0).map(|f| format!(", {f:.2} fps")).unwrap_or_default())
            }
            "audio" => format!("Audio: {codec}{}{}", num(&s["sample_rate"]).map(|r| format!(", {r:.0} Hz")).unwrap_or_default(), s["channels"].as_u64().map(|c| format!(", {c} channel(s)")).unwrap_or_default()),
            "subtitle" => format!("Subtitles: {codec}"),
            other => format!("{other}: {codec}"),
        };
        out.push_str(&line);
        out.push('\n');
    }
    Some(out)
}

async fn probe(path: &Path) -> Option<String> {
    let run = tokio::process::Command::new("ffprobe").args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"]).arg(path).kill_on_drop(true).output();
    let out = tokio::time::timeout(std::time::Duration::from_secs(20), run).await.ok()?.ok()?;
    if !out.status.success() { return None }
    probe_summary(&serde_json::from_slice(&out.stdout).ok()?)
}

// ---------------------------------------------------------------- entry points

/// The page marks of a PDF's text (pdftotext ends a page with a form feed), so a model can go to a page.
pub fn mark_pages(text: &str) -> String {
    let pages: Vec<&str> = text.split('\x0c').collect();
    if pages.len() < 2 { return text.to_string() }
    let mut out = String::new();
    for (i, p) in pages.iter().enumerate() {
        if p.trim().is_empty() && i + 1 == pages.len() { break }
        out.push_str(&format!("=== page {} ===\n{}\n", i + 1, p.trim_matches('\n')));
    }
    out
}

fn done(label: &str, r: Result<String, String>) -> Option<(String, String)> {
    Some((label.to_string(), r.unwrap_or_else(|e| format!("(the contents could not be read: {e})"))))
}

fn sync_view(name: &str, path: &Path, head: &[u8]) -> Option<(String, String)> {
    let (ext, lower) = (ext_of(name), name.to_ascii_lowercase());
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        return match ext.as_str() {
            "docx" | "docm" | "dotx" => done("Word document", docx(path)),
            "xlsx" | "xlsm" | "xltx" => done("Excel workbook", xlsx(path)),
            "pptx" | "pptm" | "ppsx" => done("PowerPoint presentation", pptx(path)),
            "odt" | "odp" | "ods" => done(match ext.as_str() { "odt" => "OpenDocument text", "odp" => "OpenDocument presentation", _ => "OpenDocument spreadsheet" }, odf(path, &ext)),
            "epub" => done("ePub book", epub(path)),
            _ => done("ZIP archive", zip_listing(path)),
        };
    }
    if lower.ends_with(".tar.gz") || ext == "tgz" { return done("Compressed tar archive", tar_listing(path, true)) }
    if ext == "tar" { return done("Tar archive", tar_listing(path, false)) }
    if ext == "gz" || head.starts_with(b"\x1f\x8b") { return gz_text(path) }
    if ext == "rtf" || head.starts_with(b"{\\rtf") {
        let mut s = String::new();
        std::fs::File::open(path).ok()?.take(PART).read_to_string(&mut s).ok();
        return Some(("RTF document".into(), rtf_text(&s)));
    }
    None
}

/// The text view of an attached file kept at `path`, named `name` by the user: what a model reads in its place.
pub async fn view(name: &str, path: &Path, size: u64, head: &[u8]) -> View {
    let (n, p, h) = (name.to_string(), path.to_path_buf(), head.to_vec());
    let found = tokio::task::spawn_blocking(move || sync_view(&n, &p, &h)).await.ok().flatten();
    let (label, body) = match found {
        Some(x) => x,
        None => {
            let media = if MEDIA.contains(&ext_of(name).as_str()) { probe(path).await } else { None };
            match media {
                Some(b) => ("Media file".to_string(), b),
                None => {
                    let (p, h) = (path.to_path_buf(), head.to_vec());
                    tokio::task::spawn_blocking(move || strings_view(&p, &h)).await.unwrap_or_else(|_| ("Binary file".into(), String::new()))
                }
            }
        }
    };
    View { text: format!("{label}: {name} ({})\n{body}", human(size)) }
}

/// A zip with stored (uncompressed) entries, for tests here and in attach_read.
#[cfg(test)]
pub fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let (mut out, mut central) = (Vec::<u8>::new(), Vec::<u8>::new());
    for (name, data) in entries {
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let offset = out.len() as u32;
        for (b, wide) in [(&mut out, false), (&mut central, true)] {
            b.extend_from_slice(&(if wide { 0x0201_4b50u32 } else { 0x0403_4b50 }).to_le_bytes());
            if wide { b.extend_from_slice(&20u16.to_le_bytes()) }
            b.extend_from_slice(&20u16.to_le_bytes());
            b.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0x21, 0]);
            b.extend_from_slice(&crc.sum().to_le_bytes());
            b.extend_from_slice(&(data.len() as u32).to_le_bytes());
            b.extend_from_slice(&(data.len() as u32).to_le_bytes());
            b.extend_from_slice(&(name.len() as u16).to_le_bytes());
            b.extend_from_slice(&0u16.to_le_bytes());
            if wide { b.extend_from_slice(&[0u8; 10]); b.extend_from_slice(&offset.to_le_bytes()) }
            b.extend_from_slice(name.as_bytes());
        }
        out.extend_from_slice(data);
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[cfg(test)]
#[path = "attach_docs_tests.rs"]
mod tests;
