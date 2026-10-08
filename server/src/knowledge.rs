//! CHAT-03: knowledge collections for chats: documents split into chunks, searched with FTS5 (bm25).
//! CHAT-03b: uploads, collections per project, and vectors (OVMS Embedder, sqlite-vec) next to the words.

use anyhow::Result;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use crate::util;
use crate::assets::ai::{self, Ai};

#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub collection: String,
    pub doc: String,
    pub n: i64,
    pub text: String,
}

/// Paragraphs (split at blank lines) packed into chunks of at most `size` characters;
/// a longer paragraph is cut into `size`-character pieces.
pub fn chunks(text: &str, size: usize) -> Vec<String> {
    let mut paras: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !cur.trim().is_empty() {
                paras.push(cur.trim().to_string());
            }
            cur.clear();
        } else {
            if !cur.is_empty() {
                cur.push('\n');
            }
            cur.push_str(line);
        }
    }
    if !cur.trim().is_empty() {
        paras.push(cur.trim().to_string());
    }
    let mut out: Vec<String> = Vec::new();
    let mut chunk = String::new();
    for p in paras {
        let pieces: Vec<String> = if p.chars().count() > size {
            p.chars().collect::<Vec<_>>().chunks(size).map(|c| c.iter().collect()).collect()
        } else {
            vec![p]
        };
        for piece in pieces {
            let joined = if chunk.is_empty() { piece.chars().count() } else { chunk.chars().count() + 2 + piece.chars().count() };
            if !chunk.is_empty() && joined > size {
                out.push(std::mem::take(&mut chunk));
            }
            if !chunk.is_empty() {
                chunk.push_str("\n\n");
            }
            chunk.push_str(&piece);
        }
    }
    if !chunk.is_empty() {
        out.push(chunk);
    }
    out
}


/// Build an FTS5 query string from the input.
pub fn fts_query(q: &str) -> String {
    let q = q.to_lowercase();
    let words: Vec<&str> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2)
        .filter(|w| {
            let stop = ["a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "i", "in", "is", "it", "of", "on", "or", "the", "to", "with", "x"];
            !stop.contains(&w)
        })
        .collect();

    if words.is_empty() {
        return String::new();
    }

    words.iter().map(|w| format!("\"{}\"", w)).collect::<Vec<_>>().join(" OR ")
}

/// Ensure a knowledge collection exists for the user.
pub async fn ensure_collection(db: &SqlitePool, user_id: &str, name: &str, source: &str) -> Result<String> {
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM knowledge_collection WHERE user_id = ? AND name = ?",
    )
    .bind(user_id)
    .bind(name)
    .fetch_optional(db)
    .await?;

    let id = row.map(|r| r.0);

    match id {
        Some(id) => Ok(id),
        None => {
            let new_id = util::new_id();
            let created_at = util::now();
            sqlx::query(
                "INSERT INTO knowledge_collection (id, user_id, name, source, created_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&new_id)
            .bind(user_id)
            .bind(name)
            .bind(source)
            .bind(created_at)
            .execute(db)
            .await?;
            Ok(new_id)
        }
    }
}

/// Add a document to a collection and its chunks.
pub async fn add_doc(db: &SqlitePool, collection_id: &str, name: &str, text: &str) -> Result<i64> {
    let chars = text.chars().count();
    let added_at = util::now();

    let (doc_id,): (i64,) = sqlx::query_as(
        "INSERT INTO knowledge_doc (collection_id, name, chars, added_at, hash) VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(collection_id)
    .bind(name)
    .bind(chars as i64)
    .bind(added_at)
    .bind(util::sha256_hex(text))
    .fetch_one(db)
    .await?;

    let chunks = chunks(text, 1500);
    for (i, chunk_text) in chunks.into_iter().enumerate() {
        let n = (i + 1) as i64;
        sqlx::query(
            "INSERT INTO knowledge_chunk (doc_id, n, text) VALUES (?, ?, ?)",
        )
        .bind(doc_id)
        .bind(n)
        .bind(chunk_text)
        .execute(db)
        .await?;
    }

    Ok(doc_id)
}

/// CHAT-03b: words (FTS5, bm25) and meaning (`qvec`, the query's embedding) fused by `rrf`;
/// `collection` limits to one collection by name; in a project, its collections' hits come first.
pub async fn search(db: &SqlitePool, user_id: &str, query: &str, qvec: Option<&[f32]>, collection: Option<&str>, project: Option<&str>, limit: i64) -> Result<Vec<Hit>> {
    let q = fts_query(query);
    let words: Vec<i64> = if q.is_empty() { Vec::new() } else {
        sqlx::query_as::<_, (i64,)>("SELECT k.id FROM knowledge_fts JOIN knowledge_chunk k ON k.id = knowledge_fts.rowid JOIN knowledge_doc d ON d.id = k.doc_id JOIN knowledge_collection c ON c.id = d.collection_id WHERE knowledge_fts MATCH ? AND c.user_id = ? AND (? IS NULL OR c.name = ?) ORDER BY bm25(knowledge_fts) LIMIT 40")
            .bind(&q).bind(user_id).bind(collection).bind(collection).fetch_all(db).await?.into_iter().map(|r| r.0).collect()
    };
    let mut meaning: Vec<i64> = Vec::new();
    if let Some(v) = qvec {
        let blob: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes()).collect();
        let near: Vec<(i64,)> = sqlx::query_as("SELECT rowid FROM knowledge_vec WHERE embedding MATCH ? AND k = 100 ORDER BY distance").bind(blob).fetch_all(db).await.unwrap_or_default();
        let near_ids: Vec<i64> = near.iter().map(|r| r.0).collect();
        let ids = serde_json::to_string(&near_ids)?;
        let mine: Vec<(i64,)> = sqlx::query_as("SELECT k.id FROM knowledge_chunk k JOIN knowledge_doc d ON d.id = k.doc_id JOIN knowledge_collection c ON c.id = d.collection_id WHERE k.id IN (SELECT value FROM json_each(?)) AND c.user_id = ? AND (? IS NULL OR c.name = ?)")
            .bind(ids).bind(user_id).bind(collection).bind(collection).fetch_all(db).await?;
        meaning = near_ids.into_iter().filter(|id| mine.iter().any(|m| m.0 == *id)).take(40).collect();
    }
    let fused = rrf(&[words, meaning]);
    if fused.is_empty() { return Ok(Vec::new()); }
    let rows: Vec<(i64, String, String, String, i64, String)> = sqlx::query_as("SELECT k.id, c.id, c.name, d.name, k.n, k.text FROM knowledge_chunk k JOIN knowledge_doc d ON d.id = k.doc_id JOIN knowledge_collection c ON c.id = d.collection_id WHERE k.id IN (SELECT value FROM json_each(?))")
        .bind(serde_json::to_string(&fused)?).fetch_all(db).await?;
    let mut rows: Vec<_> = fused.iter().filter_map(|id| rows.iter().find(|r| r.0 == *id).cloned()).collect();
    if let (Some(p), None) = (project, collection) {
        let first: Vec<String> = sqlx::query_scalar("SELECT collection_id FROM knowledge_project WHERE project_id = ?").bind(p).fetch_all(db).await?;
        rows.sort_by_key(|r| !first.contains(&r.1));
    }
    Ok(rows.into_iter().take(limit.max(0) as usize).map(|(_, _, collection, doc, n, text)| Hit { collection, doc, n, text }).collect())
}

/// Format search hits as text.
pub fn hits_text(hits: &[Hit]) -> String {
    if hits.is_empty() {
        return "Nothing found in the knowledge collections.".into();
    }
    hits.iter().map(|h| format!("[{} / {}, part {}]\n{}", h.collection, h.doc, h.n, h.text)).collect::<Vec<_>>().join("\n\n")
}

/// CHAT-04: hits numbered from `first_n` ("[3] collection / doc, part 1"), so an answer can cite `[name](src:3)`.
pub fn hits_numbered(hits: &[Hit], first_n: usize) -> String {
    if hits.is_empty() {
        return "Nothing found in the knowledge collections.".into();
    }
    hits.iter().enumerate().map(|(i, h)| format!("[{}] {} / {}, part {}\n{}", first_n + i, h.collection, h.doc, h.n, h.text)).collect::<Vec<_>>().join("\n\n")
}

/// List user's knowledge collections with stats.
pub async fn collections(db: &SqlitePool, user_id: &str) -> Result<Vec<(String, i64, i64)>> {
    let rows = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT c.name, COUNT(DISTINCT d.id) as doc_count, COUNT(k.id) as chunk_count
         FROM knowledge_collection c
         LEFT JOIN knowledge_doc d ON d.collection_id = c.id
         LEFT JOIN knowledge_chunk k ON k.doc_id = d.id
         WHERE c.user_id = ?
         GROUP BY c.id, c.name
         ORDER BY c.name",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    Ok(rows)
}

/// CHAT-03 + CHAT-03b: Open WebUI's knowledge collections (their extracted text) into this user's
/// collections of source "openwebui": new files added, changed ones replaced, removed ones deleted.
pub async fn import_openwebui(db: &SqlitePool, webui: &str, user_name: &str) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE name = ?").bind(user_name).fetch_optional(db).await?;
    let Some((user_id,)) = user else { anyhow::bail!("no user {user_name}") };
    let src = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new().filename(webui).read_only(true))
        .await?;
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as("SELECT k.name, f.filename, f.data FROM knowledge k JOIN knowledge_file kf ON kf.knowledge_id = k.id JOIN file f ON f.id = kf.file_id ORDER BY k.name, f.filename")
        .fetch_all(&src)
        .await?;
    src.close().await;
    // (collection name, added, updated, removed, skipped), in the order the collections come.
    let mut counts: Vec<(String, i64, i64, i64, i64)> = Vec::new();
    // (collection name, file name) of every document Open WebUI has.
    let mut seen: Vec<(String, String)> = Vec::new();
    for (coll, file, data) in rows {
        if counts.last().map(|c| c.0 != coll).unwrap_or(true) {
            counts.push((coll.clone(), 0, 0, 0, 0));
        }
        let text = data
            .and_then(|d| serde_json::from_str::<Value>(&d).ok())
            .and_then(|v| v["content"].as_str().map(String::from))
            .unwrap_or_default();
        let cid = ensure_collection(db, &user_id, &coll, "openwebui").await?;
        let last = counts.last_mut().expect("pushed above");
        if text.trim().is_empty() || seen.iter().any(|s| s.0 == coll && s.1 == file) {
            last.4 += 1;
            continue;
        }
        seen.push((coll.clone(), file.clone()));
        let hash = util::sha256_hex(&text);
        let old: Option<(i64, String, i64)> = sqlx::query_as("SELECT id, hash, chars FROM knowledge_doc WHERE collection_id = ? AND name = ?").bind(&cid).bind(&file).fetch_optional(db).await?;
        match old {
            None => {
                add_doc(db, &cid, &file, &text).await?;
                last.1 += 1;
            }
            Some((_, h, _)) if h == hash => last.4 += 1,
            // Imported before CHAT-03b (no hash yet): the same length counts as unchanged.
            Some((id, h, chars)) if h.is_empty() && chars == text.chars().count() as i64 => {
                sqlx::query("UPDATE knowledge_doc SET hash = ? WHERE id = ?").bind(&hash).bind(id).execute(db).await?;
                last.4 += 1;
            }
            Some((id, _, _)) => {
                delete_doc(db, &user_id, id).await?;
                add_doc(db, &cid, &file, &text).await?;
                last.2 += 1;
            }
        }
    }
    // Documents Open WebUI no longer has, only in its own ("openwebui") collections.
    for c in counts.iter_mut() {
        let docs: Vec<(i64, String)> = sqlx::query_as("SELECT d.id, d.name FROM knowledge_doc d JOIN knowledge_collection c ON c.id = d.collection_id WHERE c.user_id = ? AND c.name = ? AND c.source = 'openwebui'")
            .bind(&user_id).bind(&c.0).fetch_all(db).await?;
        for (id, file) in docs {
            if !seen.iter().any(|s| s.0 == c.0 && s.1 == file) {
                delete_doc(db, &user_id, id).await?;
                c.3 += 1;
            }
        }
    }
    Ok(counts.iter().map(|(n, a, u, r, s)| format!("{n}: {a} added, {u} updated, {r} removed, {s} skipped")).collect::<Vec<_>>().join("\n"))
}

/// KNOW-01: a folder of documents (md, markdown, txt, html, htm, pdf; subfolders too) as the collection `name`
/// (source "folder") of the user called `user_name`: new files added, changed ones replaced, vanished ones removed.
pub async fn import_folder(db: &SqlitePool, user_name: &str, name: &str, dir: &str) -> Result<String> {
    let user: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE name = ?").bind(user_name).fetch_optional(db).await?;
    let Some((user_id,)) = user else { anyhow::bail!("no user {user_name}") };
    anyhow::ensure!(std::fs::metadata(dir)?.is_dir(), "{dir} is not a folder");
    let mut files: Vec<(String, std::path::PathBuf)> = Vec::new();
    let mut todo = vec![std::path::PathBuf::from(dir)];
    while let Some(d) = todo.pop() {
        for entry in std::fs::read_dir(&d)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                todo.push(path);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase) && ["md", "markdown", "txt", "html", "htm", "pdf"].contains(&ext.as_str()) {
                let rel = path.strip_prefix(dir).unwrap_or(&path).components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/");
                files.push((rel, path));
            }
        }
    }
    files.sort();
    let cid = ensure_collection(db, &user_id, name, "folder").await?;
    let (mut added, mut updated, mut removed, mut skipped) = (0, 0, 0, 0);
    let mut seen: Vec<String> = Vec::new();
    for (rel, path) in files {
        seen.push(rel.clone());
        let text = match std::fs::read(&path) {
            Ok(bytes) => extract(&rel, &bytes).await.unwrap_or_default(),
            Err(_) => String::new(),
        };
        if text.trim().is_empty() {
            skipped += 1;
            continue;
        }
        let hash = util::sha256_hex(&text);
        let old: Option<(i64, String)> = sqlx::query_as("SELECT id, hash FROM knowledge_doc WHERE collection_id = ? AND name = ?").bind(&cid).bind(&rel).fetch_optional(db).await?;
        match old {
            None => { add_doc(db, &cid, &rel, &text).await?; added += 1; }
            Some((_, h)) if h == hash => skipped += 1,
            Some((id, _)) => { delete_doc(db, &user_id, id).await?; add_doc(db, &cid, &rel, &text).await?; updated += 1; }
        }
    }
    let docs: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM knowledge_doc WHERE collection_id = ?").bind(&cid).fetch_all(db).await?;
    for (id, file) in docs {
        if !seen.contains(&file) {
            delete_doc(db, &user_id, id).await?;
            removed += 1;
        }
    }
    Ok(format!("{name}: {added} added, {updated} updated, {removed} removed, {skipped} skipped"))
}

/// Reciprocal rank fusion (k = 60): ids from several ranked lists, best first; ties keep first-seen order.
pub fn rrf(lists: &[Vec<i64>]) -> Vec<i64> {
    let mut scores: Vec<(i64, f64)> = Vec::new(); // first-seen order
    for list in lists {
        for (rank, id) in list.iter().enumerate() {
            let add = 1.0 / (60.0 + rank as f64 + 1.0);
            match scores.iter_mut().find(|s| s.0 == *id) { Some(s) => s.1 += add, None => scores.push((*id, add)) }
        }
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1)); // stable: ties keep first-seen order
    scores.into_iter().map(|s| s.0).collect()
}

/// Text of an uploaded file: Markdown and text as they are, HTML without tags, PDF through pdftotext.
pub async fn extract(name: &str, bytes: &[u8]) -> Result<String> {
    let ext = match name.rsplit_once('.') { Some((_, e)) => e.to_lowercase(), None => String::new() };
    match ext.as_str() {
        "md" | "markdown" | "txt" | "text" | "rst" | "csv" | "json" | "toml" | "yaml" | "yml" | "gd" | "rs" | "py" | "ts" | "js" => Ok(String::from_utf8_lossy(bytes).into_owned()),
        "html" | "htm" => Ok(crate::web::html_text(&String::from_utf8_lossy(bytes))),
        "pdf" => pdf_text(bytes).await,
        "" => anyhow::bail!("unsupported file type: the name has no extension"),
        other => anyhow::bail!("unsupported file type: {other}"),
    }
}

/// pdftotext reads the PDF from stdin and writes its text to stdout.
async fn pdf_text(bytes: &[u8]) -> Result<String> {
    use tokio::io::AsyncWriteExt;
    let mut child = tokio::process::Command::new("pdftotext")
        .arg("-enc")
        .arg("UTF-8")
        .arg("-q")
        .arg("-")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| anyhow::anyhow!("PDF needs pdftotext (poppler-utils): {e}"))?;

    let mut stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("pdftotext stdin missing"))?;
    let input_bytes = bytes.to_vec();
    let _write_task = tokio::spawn(async move {
        stdin.write_all(&input_bytes).await
    });

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        child.wait_with_output(),
    )
    .await;

    match output {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed = stdout.trim();
                if trimmed.is_empty() {
                    return Err(anyhow::anyhow!("the PDF has no text layer"));
                }
                Ok(trimmed.into())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(anyhow::anyhow!("pdftotext failed: {}", stderr.trim()))
            }
        }
        Ok(Err(e)) => Err(anyhow::anyhow!("pdftotext failed: {}", e)),
        Err(_) => Err(anyhow::anyhow!("pdftotext took too long")),
    }
}

/// The vector table lives outside the migrations: it needs sqlite-vec on the connection.
pub async fn ensure_vectors(db: &SqlitePool) -> Result<()> {
    sqlx::query(&format!("CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_vec USING vec0(embedding float[{}])", ai::DIMS))
        .execute(db)
        .await?;
    Ok(())
}

/// Up to `n` chunks without a vector yet: (chunk id, text), oldest first.
pub async fn pending_chunks(db: &SqlitePool, n: i64) -> Result<Vec<(i64, String)>> {
    Ok(sqlx::query_as("SELECT k.id, k.text FROM knowledge_chunk k WHERE NOT EXISTS (SELECT 1 FROM knowledge_vec v WHERE v.rowid = k.id) ORDER BY k.id LIMIT ?")
        .bind(n)
        .fetch_all(db)
        .await?)
}

/// Stores (or replaces) the vectors of these chunks.
pub async fn store_vectors(db: &SqlitePool, rows: &[(i64, Vec<f32>)]) -> Result<()> {
    let mut tx = db.begin().await?;
    for (id, v) in rows {
        let blob: Vec<u8> = v.iter().flat_map(|f| f.to_le_bytes()).collect();
        sqlx::query("DELETE FROM knowledge_vec WHERE rowid = ?").bind(id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO knowledge_vec (rowid, embedding) VALUES (?, ?)").bind(id).bind(blob).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Embeds up to `n` pending chunks; how many were stored.
pub async fn embed_pending(db: &SqlitePool, ai: &Ai, n: i64) -> Result<usize> {
    let rows = pending_chunks(db, n).await?;
    if rows.is_empty() { return Ok(0); }
    let texts: Vec<String> = rows.iter().map(|r| r.1.chars().take(2000).collect()).collect();
    let vecs = ai.embed(&texts).await?;
    anyhow::ensure!(vecs.len() == rows.len(), "the Embedder returned {} vectors for {} chunks", vecs.len(), rows.len());
    let pairs: Vec<(i64, Vec<f32>)> = rows.iter().map(|r| r.0).zip(vecs).collect();
    store_vectors(db, &pairs).await?;
    Ok(pairs.len())
}

/// Links a collection to exactly these projects (only the user's own projects count).
pub async fn set_projects(db: &SqlitePool, user_id: &str, collection_id: &str, projects: &[String]) -> Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM knowledge_project WHERE collection_id = ?").bind(collection_id).execute(&mut *tx).await?;
    for p in projects {
        // only the user's own projects; unknown ids add nothing
        sqlx::query("INSERT OR IGNORE INTO knowledge_project (collection_id, project_id) SELECT ?, id FROM projects WHERE id = ? AND user_id = ?")
            .bind(collection_id).bind(p).bind(user_id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Names of the collections linked to a project, by name.
pub async fn project_collections(db: &SqlitePool, project_id: &str) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT c.name FROM knowledge_project kp JOIN knowledge_collection c ON c.id = kp.collection_id WHERE kp.project_id = ? ORDER BY c.name")
        .bind(project_id)
        .fetch_all(db)
        .await?)
}

/// The user's collections for the app, by name: id, name, source, docs, chunks, vectors,
/// projects (ids) and the 50 newest documents (id, name, chars, addedAt).
pub async fn list(db: &SqlitePool, user_id: &str) -> Result<Value> {
    let cols: Vec<(String, String, String, i64, i64)> = sqlx::query_as("SELECT c.id, c.name, c.source, (SELECT COUNT(*) FROM knowledge_doc d WHERE d.collection_id = c.id), (SELECT COUNT(*) FROM knowledge_chunk k JOIN knowledge_doc d ON d.id = k.doc_id WHERE d.collection_id = c.id) FROM knowledge_collection c WHERE c.user_id = ? ORDER BY c.name")
        .bind(user_id).fetch_all(db).await?;
    let mut out: Vec<Value> = Vec::new();
    for (id, name, source, docs, chunks) in cols {
        let vectors: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_chunk k JOIN knowledge_doc d ON d.id = k.doc_id WHERE d.collection_id = ? AND EXISTS (SELECT 1 FROM knowledge_vec v WHERE v.rowid = k.id)")
            .bind(&id).fetch_one(db).await.unwrap_or(0);
        let projects: Vec<String> = sqlx::query_scalar("SELECT project_id FROM knowledge_project WHERE collection_id = ? ORDER BY project_id").bind(&id).fetch_all(db).await?;
        let documents: Vec<(i64, String, i64, String)> = sqlx::query_as("SELECT id, name, chars, added_at FROM knowledge_doc WHERE collection_id = ? ORDER BY added_at DESC, id DESC LIMIT 50")
            .bind(&id).fetch_all(db).await?;
        let documents: Vec<Value> = documents.into_iter().map(|(id, name, chars, at)| json!({"id": id, "name": name, "chars": chars, "addedAt": at})).collect();
        out.push(json!({"id": id, "name": name, "source": source, "docs": docs, "chunks": chunks, "vectors": vectors, "projects": projects, "documents": documents}));
    }
    Ok(Value::Array(out))
}

/// A document's name and text for reading: its chunks joined by blank lines. None when it is not the user's.
pub async fn doc_text(db: &SqlitePool, user_id: &str, doc_id: i64) -> Result<Option<(String, String)>> {
    let doc: Option<(String,)> = sqlx::query_as("SELECT d.name FROM knowledge_doc d JOIN knowledge_collection c ON c.id = d.collection_id WHERE d.id = ? AND c.user_id = ?")
        .bind(doc_id).bind(user_id).fetch_optional(db).await?;
    let Some((name,)) = doc else { return Ok(None) };
    let parts: Vec<String> = sqlx::query_scalar("SELECT text FROM knowledge_chunk WHERE doc_id = ? ORDER BY n").bind(doc_id).fetch_all(db).await?;
    Ok(Some((name, parts.join("\n\n"))))
}

/// One page (50) of a collection's documents by name, only those whose name contains `q`, and how many
/// match in all. None when the collection is not the user's.
pub async fn docs_page(db: &SqlitePool, user_id: &str, collection_id: &str, q: &str, offset: i64) -> Result<Option<(Vec<Value>, i64)>> {
    let mine: Option<(String,)> = sqlx::query_as("SELECT id FROM knowledge_collection WHERE id = ? AND user_id = ?").bind(collection_id).bind(user_id).fetch_optional(db).await?;
    if mine.is_none() { return Ok(None); }
    let like = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_doc WHERE collection_id = ? AND name LIKE ? ESCAPE '\\'").bind(collection_id).bind(&like).fetch_one(db).await?;
    let rows: Vec<(i64, String, i64, String)> = sqlx::query_as("SELECT id, name, chars, added_at FROM knowledge_doc WHERE collection_id = ? AND name LIKE ? ESCAPE '\\' ORDER BY name LIMIT 50 OFFSET ?")
        .bind(collection_id).bind(&like).bind(offset.max(0)).fetch_all(db).await?;
    Ok(Some((rows.into_iter().map(|(id, name, chars, at)| json!({"id": id, "name": name, "chars": chars, "addedAt": at})).collect(), total)))
}

/// Deletes one of the user's documents with its chunks and vectors; false when it is not theirs.
pub async fn delete_doc(db: &SqlitePool, user_id: &str, doc_id: i64) -> Result<bool> {
    let mine: Option<(i64,)> = sqlx::query_as("SELECT d.id FROM knowledge_doc d JOIN knowledge_collection c ON c.id = d.collection_id WHERE d.id = ? AND c.user_id = ?")
        .bind(doc_id).bind(user_id).fetch_optional(db).await?;
    if mine.is_none() { return Ok(false); }
    sqlx::query("DELETE FROM knowledge_vec WHERE rowid IN (SELECT id FROM knowledge_chunk WHERE doc_id = ?)").bind(doc_id).execute(db).await?;
    sqlx::query("DELETE FROM knowledge_doc WHERE id = ?").bind(doc_id).execute(db).await?;
    Ok(true)
}

/// Deletes one of the user's collections with everything in it; false when it is not theirs.
pub async fn delete_collection(db: &SqlitePool, user_id: &str, collection_id: &str) -> Result<bool> {
    let mine: Option<(String,)> = sqlx::query_as("SELECT id FROM knowledge_collection WHERE id = ? AND user_id = ?")
        .bind(collection_id).bind(user_id).fetch_optional(db).await?;
    if mine.is_none() { return Ok(false); }
    sqlx::query("DELETE FROM knowledge_vec WHERE rowid IN (SELECT k.id FROM knowledge_chunk k JOIN knowledge_doc d ON d.id = k.doc_id WHERE d.collection_id = ?)")
        .bind(collection_id).execute(db).await?;
    sqlx::query("DELETE FROM knowledge_collection WHERE id = ?").bind(collection_id).execute(db).await?;
    Ok(true)
}

/// CHAT-03b: background work: vectors for new chunks (32 at a time), and the Open WebUI
/// re-import (path, user name) when its database changed, checked every 10 minutes.
pub fn spawn(db: SqlitePool, http: reqwest::Client, openwebui: Option<(String, String)>, folders: Vec<(String, String)>, folder_user: String) {
    tokio::spawn(async move {
        let ai = Ai::from_env(http);
        let mut stamp: Option<std::time::SystemTime> = None;
        let mut next_sync = std::time::Instant::now();
        let mut next_folders = std::time::Instant::now();
        let mut last_error = String::new();
        loop {
            if let Some((path, user)) = &openwebui && std::time::Instant::now() >= next_sync {
                next_sync = std::time::Instant::now() + std::time::Duration::from_secs(600);
                // No user named: the first admin.
                let user: String = if user.is_empty() {
                    sqlx::query_scalar("SELECT name FROM users WHERE is_admin = 1 ORDER BY created_at LIMIT 1").fetch_optional(&db).await.ok().flatten().unwrap_or_default()
                } else {
                    user.clone()
                };
                match sync_openwebui(&db, path, &user, &mut stamp).await {
                    Ok(Some(summary)) => tracing::info!("knowledge: Open WebUI re-imported:\n{summary}"),
                    Ok(None) => {}
                    Err(e) => tracing::warn!("knowledge: Open WebUI re-import failed: {e}"),
                }
            }
            if !folders.is_empty() && std::time::Instant::now() >= next_folders {
                next_folders = std::time::Instant::now() + std::time::Duration::from_secs(600);
                let user: String = if folder_user.is_empty() {
                    sqlx::query_scalar("SELECT name FROM users WHERE is_admin = 1 ORDER BY created_at LIMIT 1").fetch_optional(&db).await.ok().flatten().unwrap_or_default()
                } else {
                    folder_user.clone()
                };
                for (name, path) in &folders {
                    match import_folder(&db, &user, name, path).await {
                        Ok(summary) if !summary.contains(": 0 added, 0 updated, 0 removed") => tracing::info!("knowledge: folder re-imported: {summary}"),
                        Ok(_) => {}
                        Err(e) => tracing::warn!("knowledge: folder {name} failed: {e}"),
                    }
                }
            }
            let wait = match embed_pending(&db, &ai, 32).await {
                Ok(0) => 30,
                Ok(_) => { last_error.clear(); 1 }
                Err(e) => {
                    // Say it once, not every five minutes.
                    if e.to_string() != last_error {
                        tracing::warn!("knowledge: no vectors yet: {e}");
                        last_error = e.to_string();
                    }
                    300
                }
            };
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
    });
}

/// Re-imports Open WebUI's collections from a copy of its database when the file (or its -wal)
/// changed since `stamp`; None when nothing changed.
pub async fn sync_openwebui(db: &SqlitePool, path: &str, user: &str, stamp: &mut Option<std::time::SystemTime>) -> Result<Option<String>> {
    // The newest change time of the database and its -wal file (a missing -wal does not count).
    let wal = format!("{path}-wal");
    let mut changed = std::fs::metadata(path)?.modified()?;
    if let Ok(m) = std::fs::metadata(&wal).and_then(|m| m.modified()) {
        changed = changed.max(m);
    }
    if *stamp == Some(changed) {
        return Ok(None);
    }
    // Read a copy: Open WebUI may write while we read, and its folder is mounted read-only.
    let dir = std::env::temp_dir().join("kompanion-openwebui");
    std::fs::create_dir_all(&dir)?;
    let copy = dir.join("webui.db");
    let copy_wal = dir.join("webui.db-wal");
    std::fs::copy(path, &copy)?;
    let _ = std::fs::remove_file(&copy_wal);
    if std::path::Path::new(&wal).exists() {
        std::fs::copy(&wal, &copy_wal)?;
    }
    let result = import_openwebui(db, &copy.to_string_lossy(), user).await;
    let _ = std::fs::remove_dir_all(&dir);
    let summary = result?;
    *stamp = Some(changed);
    Ok(Some(summary))
}

/// Build the MCP tool definition for search_knowledge.
pub fn tools(names: &[String]) -> Vec<Value> {
    let description = format!(
        "Search the user's knowledge collections ({}). Returns the best passages with their source.",
        names.join(", ")
    );

    vec![json!({
        "name": "search_knowledge",
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {"type": "string"},
                "collection": {"type": "string", "description": "optional: one collection name"}
            },
            "required": ["query"]
        }
    })]
}

#[cfg(test)]
#[path = "knowledge_tests.rs"]
mod tests;
