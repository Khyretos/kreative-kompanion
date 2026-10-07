//! CHAT-03: knowledge collections for chats: documents split into chunks, searched with FTS5 (bm25).

use anyhow::Result;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use crate::util;

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
        "INSERT INTO knowledge_doc (collection_id, name, chars, added_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(collection_id)
    .bind(name)
    .bind(chars as i64)
    .bind(added_at)
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

/// Search knowledge collections using FTS5.
pub async fn search(db: &SqlitePool, user_id: &str, query: &str, collection: Option<&str>, limit: i64) -> Result<Vec<Hit>> {
    let q = fts_query(query);
    if q.is_empty() {
        return Ok(vec![]);
    }

    let hits: Vec<(String, String, i64, String)> = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT c.name, d.name, k.n, k.text
         FROM knowledge_fts
         JOIN knowledge_chunk k ON k.id = knowledge_fts.rowid
         JOIN knowledge_doc d ON d.id = k.doc_id
         JOIN knowledge_collection c ON c.id = d.collection_id
         WHERE knowledge_fts MATCH ? AND c.user_id = ? AND (? IS NULL OR c.name = ?)
         ORDER BY bm25(knowledge_fts) LIMIT ?",
    )
    .bind(&q)
    .bind(user_id)
    .bind(collection)
    .bind(collection)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let result: Vec<Hit> = hits
        .into_iter()
        .map(|(col, doc, n, txt)| Hit {
            collection: col,
            doc,
            n,
            text: txt,
        })
        .collect();

    Ok(result)
}

/// Format search hits as text.
pub fn hits_text(hits: &[Hit]) -> String {
    if hits.is_empty() {
        return "Nothing found in the knowledge collections.".into();
    }
    hits.iter().map(|h| format!("[{} / {}, part {}]\n{}", h.collection, h.doc, h.n, h.text)).collect::<Vec<_>>().join("\n\n")
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

/// CHAT-03: Open WebUI's knowledge collections (their extracted text) into this user's collections;
/// a document already there (same collection and name) or without text is skipped.
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
    // (collection name, added, skipped), in the order the collections come.
    let mut counts: Vec<(String, i64, i64)> = Vec::new();
    for (coll, file, data) in rows {
        if counts.last().map(|c| c.0 != coll).unwrap_or(true) {
            counts.push((coll.clone(), 0, 0));
        }
        let text = data
            .and_then(|d| serde_json::from_str::<Value>(&d).ok())
            .and_then(|v| v["content"].as_str().map(String::from))
            .unwrap_or_default();
        let cid = ensure_collection(db, &user_id, &coll, "openwebui").await?;
        let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM knowledge_doc WHERE collection_id = ? AND name = ?").bind(&cid).bind(&file).fetch_optional(db).await?;
        let last = counts.last_mut().expect("pushed above");
        if text.trim().is_empty() || exists.is_some() {
            last.2 += 1;
            continue;
        }
        add_doc(db, &cid, &file, &text).await?;
        last.1 += 1;
    }
    Ok(counts.iter().map(|(n, a, s)| format!("{n}: {a} documents added, {s} skipped")).collect::<Vec<_>>().join("\n"))
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
