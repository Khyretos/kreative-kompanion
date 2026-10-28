//! CHAT-09: a model reads an attached file in parts of its own size: `read_attachment` (from a line, the first read with
//! a summary and the headings) and `search_attachment` (words to line numbers). Files of any type are read through their
//! text view (attach_docs). Sizes follow the model's context window, and a window that is not known counts as a small
//! one, so a 3B-9B model gets parts it can hold and a large model just gets bigger ones.

use std::{collections::HashSet, io::{BufRead, BufReader, Cursor, Read}, path::{Path, PathBuf}, sync::LazyLock};
use regex::Regex;
use serde_json::{Value, json};
use crate::{AppState, chat_attach::Attached, chat_files};

pub const READ: &str = "read_attachment";
pub const SEARCH: &str = "search_attachment";
/// The tools' server name in a chat's tool list.
pub const SERVER: &str = "Attachments";

pub fn is_tool(name: &str) -> bool {
    name == READ || name == SEARCH
}

/// Characters of text per token, set low on purpose so a budget never overshoots a window.
const CHARS_PER_TOKEN: usize = 3;
/// The window assumed when the provider has none set: the small-model baseline.
pub const DEFAULT_WINDOW: i64 = 8_192;
/// Longest line the reader returns; a longer one is split, so line numbers always work (minified files, one-line logs).
const WRAP: usize = 1_000;
const CHUNK: u64 = 64 * 1024;
const OUTLINE_MAX: usize = 60;
const HITS_MAX: usize = 15;
const LINE_SHOWN: usize = 240;
const MEMBER_MAX: u64 = 64 << 20;

/// What fits the model: characters put in a message (`inline`), returned by one tool call (`read`), and whether it has the tools.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub window: i64,
    pub inline: usize,
    pub read: usize,
    pub tools: bool,
}

impl Budget {
    pub fn new(window: Option<i64>, tools: bool) -> Self {
        let window = window.filter(|w| *w >= 1_000).unwrap_or(DEFAULT_WINDOW);
        let chars = (window as usize).saturating_mul(CHARS_PER_TOKEN);
        // A quarter of the window for the files of a message; an eighth for one read, since a few reads follow each other.
        Budget { window, inline: (chars / 4).clamp(1_500, 100_000), read: (chars / 8).clamp(1_200, 48_000), tools }
    }
}

/// The characters of attached text left for the messages of one prompt (newest first take theirs first), and whether
/// some file's text was not shown in full (only then does a plain chat answer offer the reading tools).
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub budget: Budget,
    pub left: usize,
    pub cut: bool,
}

impl Plan {
    pub fn new(window: Option<i64>, tools: bool) -> Self {
        let budget = Budget::new(window, tools);
        Plan { budget, left: budget.inline, cut: false }
    }
}

// ---------------------------------------------------------------- the chat's files

#[derive(Debug, Clone)]
pub struct Doc {
    pub name: String,
    pub kind: String,
    /// The stored original (a picture, the text of a document, or the file as it was attached).
    pub original: Option<PathBuf>,
    /// The text a model reads: the stored text, or the view next to a file kept as it is.
    pub text: Option<PathBuf>,
    pub size: u64,
}

pub fn doc_of(dir: &Path, chat_id: &str, f: &Attached) -> Doc {
    let original = chat_files::path_for(dir, chat_id, &f.file);
    let text = match (f.kind.as_str(), &original) {
        ("text", Some(p)) => Some(p.clone()),
        ("file", Some(p)) => Some(p.with_extension("txt")).filter(|t| t.is_file()),
        _ => None,
    };
    let size = original.as_ref().and_then(|p| std::fs::metadata(p).ok()).map_or(0, |m| m.len());
    Doc { name: f.name.clone(), kind: f.kind.clone(), original, text, size }
}

/// The files attached in this chat, oldest first.
pub async fn chat_docs(db: &sqlx::SqlitePool, dir: &Path, chat_id: &str) -> Vec<Doc> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT text FROM messages WHERE chat_id = ?1 AND author = 'user' AND text LIKE '%:::files%' ORDER BY at, rowid")
        .bind(chat_id).fetch_all(db).await.unwrap_or_default();
    rows.iter().flat_map(|t| crate::chat_attach::split(t).1).map(|f| doc_of(dir, chat_id, &f)).filter(|d| d.original.is_some()).collect()
}

/// Whether the chat has a file other than a picture (the tools are then offered).
pub async fn has_docs(db: &sqlx::SqlitePool, dir: &Path, chat_id: &str) -> bool {
    chat_docs(db, dir, chat_id).await.iter().any(|d| d.kind != "image")
}

fn first_line(path: &Path) -> String {
    let mut s = String::new();
    if let Ok(f) = std::fs::File::open(path) { BufReader::new(f.take(600)).read_line(&mut s).ok(); }
    s.trim().to_string()
}

fn listing(docs: &[Doc]) -> String {
    docs.iter().map(|d| match (&d.text, d.kind.as_str()) {
        (Some(t), "file") => format!("- {}", first_line(t)),
        (Some(_), _) => format!("- {} (text, {})", d.name, crate::attach_docs::human(d.size)),
        (None, "image") => format!("- {} (picture)", d.name),
        (None, _) => format!("- {} (no text to read, {})", d.name, crate::attach_docs::human(d.size)),
    }).collect::<Vec<_>>().join("\n")
}

fn pick<'a>(docs: &'a [Doc], name: Option<&str>) -> Result<&'a Doc, String> {
    let readable: Vec<&Doc> = docs.iter().filter(|d| d.text.is_some()).collect();
    let Some(name) = name else {
        return match readable.as_slice() {
            [one] => Ok(one),
            [] => Err(format!("None of the attached files has text to read.\n{}", listing(docs))),
            _ => Err(format!("Which file? Give its name:\n{}", listing(docs))),
        };
    };
    let lower = name.to_lowercase();
    readable.iter().rev().find(|d| d.name.to_lowercase() == lower)
        .or_else(|| readable.iter().rev().find(|d| d.name.to_lowercase().contains(&lower)))
        .copied()
        .ok_or_else(|| format!("No attached file is called {name}. The files:\n{}", listing(docs)))
}

// ---------------------------------------------------------------- lines

/// Calls `f(line number, text)` for each line, from 1; `f` returns false to stop. A line over `WRAP` characters counts as
/// several lines. Reads in chunks, so a file of any size (or one huge line) never sits in memory. Returns the lines seen.
pub fn scan<R: BufRead>(mut r: R, mut f: impl FnMut(usize, &str) -> bool) -> std::io::Result<usize> {
    let (mut n, mut carry, mut buf) = (0usize, Vec::<u8>::new(), Vec::<u8>::new());
    let mut emit = |text: &str, n: &mut usize, blank_ok: bool| -> bool {
        if text.is_empty() {
            if blank_ok { *n += 1; return f(*n, "") }
            return true;
        }
        let mut rest = text;
        while !rest.is_empty() {
            let end = rest.char_indices().nth(WRAP).map_or(rest.len(), |(i, _)| i);
            *n += 1;
            if !f(*n, &rest[..end]) { return false }
            rest = &rest[end..];
        }
        true
    };
    // True while the line being read began in an earlier chunk: its newline then ends that line, it is not a blank one.
    let mut continued = false;
    loop {
        buf.clear();
        let got = (&mut r).take(CHUNK).read_until(b'\n', &mut buf)?;
        if got == 0 { break }
        let ended = buf.last() == Some(&b'\n');
        let mut data = std::mem::take(&mut carry);
        data.extend_from_slice(&buf);
        if ended {
            data.pop();
            if data.last() == Some(&b'\r') { data.pop(); }
        } else if let Err(e) = std::str::from_utf8(&data) {
            // The line goes on in the next chunk: keep a character cut in half for it.
            if e.error_len().is_none() { carry = data.split_off(e.valid_up_to()); }
        }
        let blank_ok = ended && !(data.is_empty() && continued);
        continued = !ended;
        if !emit(&String::from_utf8_lossy(&data), &mut n, blank_ok) { return Ok(n) }
    }
    if !carry.is_empty() { emit(&String::from_utf8_lossy(&carry), &mut n, false); }
    Ok(n)
}

pub struct Page {
    pub text: String,
    pub last: usize,
    pub more: bool,
}

/// Lines from `from` on, up to about `max_chars` (always one line); `more` says the file goes on.
pub fn read_page<R: BufRead>(r: R, from: usize, max_chars: usize) -> std::io::Result<Page> {
    let (mut page, mut chars) = (Page { text: String::new(), last: from.saturating_sub(1), more: false }, 0usize);
    scan(r, |n, line| {
        if n < from { return true }
        let len = line.chars().count() + 1;
        if page.last >= from && chars + len > max_chars {
            page.more = true;
            return false;
        }
        page.text.push_str(line);
        page.text.push('\n');
        chars += len;
        page.last = n;
        true
    })?;
    Ok(page)
}

// ---------------------------------------------------------------- summary of a file

const CODE: [&str; 21] = ["rs", "py", "js", "jsx", "ts", "tsx", "java", "go", "c", "h", "cpp", "hpp", "cc", "cs", "kt", "rb", "gd", "lua", "php", "swift", "sh"];

pub struct Overview {
    pub lines: usize,
    pub pages: usize,
    pub outline: Vec<(usize, String)>,
    pub outline_more: usize,
    pub first: String,
}

fn short(s: &str, n: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= n { s.to_string() } else { format!("{}...", s.chars().take(n).collect::<String>()) }
}

/// Line count, page marks, and the headings (markdown headings, code definitions, sheets, slides, chapters).
pub fn overview<R: BufRead>(r: R, ext: &str) -> std::io::Result<Overview> {
    static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#{1,6}\s+\S").expect("regex"));
    static DEF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*(?:(?:pub|export|public|private|protected|static|async|final|abstract)(?:\([^)]*\))?\s+)*(?:fn|def|class|struct|enum|impl|trait|interface|function|func|module|namespace|type|object|record)\s+\w+").expect("regex"));
    let code = CODE.contains(&ext);
    let mut o = Overview { lines: 0, pages: 0, outline: Vec::new(), outline_more: 0, first: String::new() };
    o.lines = scan(r, |n, line| {
        if n == 1 { o.first = short(line, 300) }
        let entry = if line.starts_with("=== page ") { o.pages += 1; false } else if line.starts_with("=== ") { true } else if code { DEF.is_match(line) } else { HEADING.is_match(line) };
        if entry {
            if o.outline.len() < OUTLINE_MAX { o.outline.push((n, short(line, 90))) } else { o.outline_more += 1 }
        }
        true
    })?;
    Ok(o)
}

fn overview_text(name: &str, ext: &str, o: &Overview, room: usize) -> String {
    let mut s = format!("File: {name}, {} lines", o.lines);
    if o.pages > 0 { s.push_str(&format!(", {} pages (to go to one: search_attachment \"=== page 5 ===\")", o.pages)) }
    s.push('\n');
    if matches!(ext, "csv" | "tsv" | "tab") && !o.first.is_empty() { s.push_str(&format!("Columns: {}\n", o.first)) }
    if !o.outline.is_empty() {
        s.push_str("Contents (line: heading):\n");
        let (mut used, mut shown) = (0usize, 0usize);
        for (n, t) in &o.outline {
            let line = format!("  {n}: {t}\n");
            if used + line.len() > room / 3 { break }
            used += line.len();
            shown += 1;
            s.push_str(&line);
        }
        let more = o.outline.len() - shown + o.outline_more;
        if more > 0 { s.push_str(&format!("  ... {more} more (use search_attachment)\n")) }
    }
    s.push('\n');
    s
}

// ---------------------------------------------------------------- search

fn words_of(q: &str) -> (String, Vec<String>) {
    let phrase = q.trim().to_lowercase();
    let words = phrase.split_whitespace().map(String::from).collect();
    (phrase, words)
}

fn matches(lower: &str, phrase: &str, words: &[String]) -> bool {
    lower.contains(phrase) || (words.len() > 1 && words.iter().all(|w| lower.contains(w.as_str())))
}

/// How many lines match, and the numbers of the first `max`.
pub fn find<R: BufRead>(r: R, query: &str, max: usize) -> std::io::Result<(usize, Vec<usize>)> {
    let (phrase, words) = words_of(query);
    let (mut total, mut first) = (0usize, Vec::new());
    if phrase.is_empty() { return Ok((0, first)) }
    scan(r, |n, line| {
        if matches(&line.to_lowercase(), &phrase, &words) {
            total += 1;
            if first.len() < max { first.push(n) }
        }
        true
    })?;
    Ok((total, first))
}

/// The lines inside the (ascending, not overlapping) ranges.
fn lines_in<R: BufRead>(r: R, ranges: &[(usize, usize)]) -> std::io::Result<Vec<(usize, String)>> {
    let (mut out, end) = (Vec::new(), ranges.last().map_or(0, |r| r.1));
    scan(r, |n, line| {
        if ranges.iter().any(|&(a, b)| n >= a && n <= b) { out.push((n, line.to_string())) }
        n < end
    })?;
    Ok(out)
}

fn merge(hits: &[usize], ctx: usize) -> Vec<(usize, usize)> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &h in hits {
        let (a, b) = (h.saturating_sub(ctx).max(1), h + ctx);
        match ranges.last_mut() {
            Some(last) if a <= last.1 + 1 => last.1 = last.1.max(b),
            _ => ranges.push((a, b)),
        }
    }
    ranges
}

// ---------------------------------------------------------------- one source (a file's text, or a member of an archive)

enum Source {
    File(PathBuf),
    Text(String),
}

impl Source {
    fn open(&self) -> std::io::Result<Box<dyn BufRead + Send>> {
        Ok(match self {
            Source::File(p) => Box::new(BufReader::with_capacity(1 << 16, std::fs::File::open(p)?)),
            Source::Text(t) => Box::new(Cursor::new(t.clone().into_bytes())),
        })
    }
}

/// A text file inside a zip or tar archive, as text.
fn member_text(doc: &Doc, member: &str) -> Result<(String, String), String> {
    let path = doc.original.as_ref().ok_or("the file is not stored")?;
    let lower = doc.name.to_lowercase();
    let want = member.trim().trim_start_matches("./");
    let hit = |name: &str| name == want || name.eq_ignore_ascii_case(want) || name.ends_with(&format!("/{want}"));
    let mut bytes = Vec::new();
    let found = if lower.ends_with(".tar") || lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        let f = BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?);
        let r: Box<dyn Read> = if lower.ends_with(".tar") { Box::new(f) } else { Box::new(flate2::read::GzDecoder::new(f)) };
        let mut archive = tar::Archive::new(r);
        let mut found = None;
        for e in archive.entries().map_err(|e| e.to_string())? {
            let mut e = e.map_err(|e| e.to_string())?;
            let name = e.path().map(|p| p.display().to_string()).unwrap_or_default();
            if hit(&name) && e.header().entry_type().is_file() {
                e.by_ref().take(MEMBER_MAX).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                found = Some(name);
                break;
            }
        }
        found
    } else {
        let entries = crate::assets::zipindex::zip_entries(path).map_err(|_| "this file is not an archive".to_string())?;
        match entries.iter().find(|e| hit(&e.name)) {
            Some(e) => {
                crate::assets::zipindex::copy_entry(path, e, &mut bytes, MEMBER_MAX).map_err(|e| e.to_string())?;
                Some(e.name.clone())
            }
            None => None,
        }
    };
    let name = found.ok_or_else(|| format!("{member} is not in the archive; the listing is in the file's first lines"))?;
    if !crate::chat_attach::looks_like_text(&bytes[..bytes.len().min(crate::chat_attach::SNIFF)], bytes.len() > crate::chat_attach::SNIFF) {
        return Err(format!("{name} is not text ({}); only text files inside an archive can be read", crate::attach_docs::human(bytes.len() as u64)));
    }
    Ok((name, String::from_utf8_lossy(&bytes).into_owned()))
}

// ---------------------------------------------------------------- the tools

fn read_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "description": "The file's name as attached. Leave out to list the files." },
            "from_line": { "type": "integer", "description": "First line to read. Leave out for the start, with a summary of the file." },
            "member": { "type": "string", "description": "A text file inside an archive (zip, tar) to read instead." }
        },
        "required": []
    })
}

fn search_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Words to find on a line (any case; all of them)." },
            "name": { "type": "string", "description": "The file's name. Leave out to search every attached file." },
            "context": { "type": "integer", "description": "Lines to show around each match, 0 to 5 (default 1)." }
        },
        "required": ["query"]
    })
}

const READ_DESC: &str = "Read a file the user attached, a part at a time: text, PDF, Word, Excel, PowerPoint, OpenDocument, ePub, archives, media details. The answer ends with the line to continue from.";
const SEARCH_DESC: &str = "Find lines with given words in the files the user attached, with their line numbers. Use it first on a big file, then read_attachment from a line near a match.";

/// The tools as an MCP server lists them (for chat tool rounds).
pub fn tools() -> Vec<Value> {
    vec![
        json!({ "name": READ, "description": READ_DESC, "inputSchema": read_params() }),
        json!({ "name": SEARCH, "description": SEARCH_DESC, "inputSchema": search_params() }),
    ]
}

/// The tools as function specs (for the PC agent).
pub fn specs() -> Vec<Value> {
    vec![
        json!({ "type": "function", "function": { "name": READ, "description": READ_DESC, "parameters": read_params() } }),
        json!({ "type": "function", "function": { "name": SEARCH, "description": SEARCH_DESC, "parameters": search_params() } }),
    ]
}

/// Runs one of the tools for the files of `chat_id`. `cap` is the most characters the caller lets a result hold.
pub async fn call(s: &AppState, chat_id: &str, tool: &str, args: &Value, cap: usize, budget: Budget) -> String {
    let dir = chat_files::dir_for(&s.config.database);
    let docs = chat_docs(&s.db, &dir, chat_id).await;
    if !docs.iter().any(|d| d.kind != "image") { return "There are no attached files in this chat to read.".into() }
    let room = budget.read.min(cap).max(1_200);
    let name = args["name"].as_str().map(str::trim).filter(|n| !n.is_empty()).map(String::from);
    let (args, tool) = (args.clone(), tool.to_string());
    let job = move || -> String {
        let r = if tool == READ { read_tool(&docs, name.as_deref(), &args, room) } else { search_tool(&docs, name.as_deref(), &args, room) };
        r.unwrap_or_else(|e| e)
    };
    tokio::task::spawn_blocking(job).await.unwrap_or_else(|_| "error: the file could not be read".into())
}

fn read_tool(docs: &[Doc], name: Option<&str>, args: &Value, room: usize) -> Result<String, String> {
    let doc = pick(docs, name)?;
    let from = args["from_line"].as_u64().map(|n| n.max(1) as usize);
    let member = args["member"].as_str().map(str::trim).filter(|m| !m.is_empty());
    let text = doc.text.clone().ok_or("this file has no text to read")?;
    let (label, ext, source) = match member {
        Some(m) => {
            let (mname, mtext) = member_text(doc, m)?;
            (format!("{} in {}", mname, doc.name), crate::attach_docs::ext_of(&mname), Source::Text(mtext))
        }
        None => (doc.name.clone(), crate::attach_docs::ext_of(&doc.name), Source::File(text)),
    };
    // The text of a PDF or Word file sits in a .txt; its kind comes from the name the user gave it.
    let ext = if member.is_none() && doc.kind == "file" { "doc".to_string() } else { ext };
    let io = |e: std::io::Error| format!("error: {e}");
    let mut out = String::new();
    if from.is_none() {
        let o = overview(source.open().map_err(io)?, &ext).map_err(io)?;
        out.push_str(&overview_text(&label, &ext, &o, room));
    }
    let first = from.unwrap_or(1);
    let page = read_page(source.open().map_err(io)?, first, room.saturating_sub(out.len()).max(1_200)).map_err(io)?;
    if page.text.is_empty() {
        let total = scan(source.open().map_err(io)?, |_, _| true).map_err(io)?;
        return Ok(format!("{out}{label} has {total} lines; there is nothing from line {first} on."));
    }
    out.push_str(&page.text);
    if page.more {
        let m = member.map(|m| format!(" member=\"{m}\"")).unwrap_or_default();
        out.push_str(&format!("[Lines {first}-{} of {label}. To continue: {READ} name=\"{}\"{m} from_line={}]", page.last, doc.name, page.last + 1));
    } else {
        out.push_str(&format!("[Lines {first}-{} of {label}: the end of the file.]", page.last));
    }
    Ok(out)
}

fn search_tool(docs: &[Doc], name: Option<&str>, args: &Value, room: usize) -> Result<String, String> {
    let query = args["query"].as_str().map(str::trim).filter(|q| !q.is_empty()).ok_or("Give the words to look for in `query`.")?;
    let ctx = args["context"].as_u64().unwrap_or(1).min(5) as usize;
    let targets: Vec<&Doc> = match name {
        Some(_) => vec![pick(docs, name)?],
        None => docs.iter().filter(|d| d.text.is_some()).collect(),
    };
    if targets.is_empty() { return Err(format!("None of the attached files has text to search.\n{}", listing(docs))) }
    let per = room / targets.len();
    let io = |e: std::io::Error| format!("error: {e}");
    let mut out = String::new();
    for d in targets {
        let path = d.text.as_ref().ok_or("no text")?;
        let open = || std::fs::File::open(path).map(|f| BufReader::with_capacity(1 << 16, f));
        let (total, hits) = find(open().map_err(io)?, query, HITS_MAX).map_err(io)?;
        if total == 0 {
            out.push_str(&format!("No line of {} has \"{query}\".\n", d.name));
            continue;
        }
        let ranges = merge(&hits, ctx);
        let lines = lines_in(open().map_err(io)?, &ranges).map_err(io)?;
        let marked: HashSet<usize> = hits.iter().copied().collect();
        let mut block = format!("\"{query}\" in {}: {total} matching line(s){}. Lines marked * match; read around one with {READ} from_line.\n", d.name, if total > hits.len() { format!(", the first {} shown", hits.len()) } else { String::new() });
        let mut prev = 0usize;
        for (n, line) in lines {
            if prev != 0 && n > prev + 1 { block.push_str("--\n") }
            let row = format!("{n}{} {}\n", if marked.contains(&n) { '*' } else { ':' }, short(&line, LINE_SHOWN));
            if block.len() + row.len() > per.max(1_200) {
                block.push_str("... (more matches; search for a more exact phrase)\n");
                break;
            }
            block.push_str(&row);
            prev = n;
        }
        out.push_str(&block);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "attach_read_tests.rs"]
mod tests;
