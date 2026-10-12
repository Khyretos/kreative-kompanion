//! PROJ-OVERVIEW tests: the pure helpers, and the route on a chat, a game and a programming project.
use super::*;

fn t(id: &str, parent: &str, kind: &str, title: &str, state: &str) -> TaskRow {
    TaskRow { id: id.into(), parent_id: parent.into(), kind: kind.into(), title: title.into(), state: state.into() }
}

#[test]
fn a_grant_covers_its_folder_and_everything_below() {
    assert!(covers("/home/k/app", "/home/k/app"));
    assert!(covers("/home/k", "/home/k/app"));
    assert!(covers("/home/k/", "/home/k/app/"));
    assert!(covers("/", "/home/k/app"));
    assert!(!covers("/home/k/app/src", "/home/k/app"), "a grant inside the repo does not cover the repo");
    assert!(!covers("/home/k/ap", "/home/k/app"), "a name prefix is not a parent folder");
    assert!(!covers("/srv", "/home/k/app"));
    assert!(!covers("system", "/home/k/app"));
    assert!(!covers("/home/k", ""));
}

#[test]
fn about_follows_the_description_and_is_bounded() {
    assert_eq!(with_about("A shop.", "", 1500), "A shop.");
    assert_eq!(with_about("A shop.", "  Sells plants.\n", 1500), "A shop.\n\nSells plants.");
    assert_eq!(with_about("", "Sells plants.", 1500), "Sells plants.");
    let long = "é".repeat(2000);
    let out = with_about("A shop.", &long, 1500);
    assert!(out.starts_with("A shop.\n\néé"));
    assert_eq!(out.chars().filter(|c| *c == 'é').count(), 1500);
    assert!(out.ends_with('…'), "a cut text says so");
}

#[test]
fn activity_lines_say_what_happened() {
    assert_eq!(activity_what("task", "created", "Login page", "{}"), "Task added: Login page");
    assert_eq!(activity_what("task", "changed", "Login page", "{}"), "Task changed: Login page");
    assert_eq!(activity_what("task", "forge", "Login page", r#"{"pr":"https://f/pulls/3","state":{"to":"done"}}"#), "PR merged: Login page");
    assert_eq!(activity_what("task", "forge", "Login page", r#"{"pr":"https://f/pulls/3"}"#), "PR update: Login page");
    assert_eq!(activity_what("task", "sync_conflict", "Login page", "{}"), "sync conflict: Login page");
    assert_eq!(activity_what("run", "failed", "Login page", ""), "Run failed: Login page");
    assert_eq!(activity_what("chat", "", "Ideas", ""), "Chat: Ideas");
}

#[test]
fn progress_counts_tasks_goals_and_the_next_ready_ones() {
    let rows = vec![
        t("g", "", "goal", "Beta", "queued"),
        t("a", "g", "task", "Login", "done"),
        t("b", "g", "task", "Pay", "running"),
        t("c", "g", "task", "Docs", "queued"),
        t("d", "", "task", "Crash", "needs_input"),
        t("e", "", "task", "Fix", "failed"),
        t("f", "", "task", "Ship", "in_review"),
        t("h", "", "task", "Logo", "queued"),
        t("i", "", "task", "Icons", "queued"),
        t("j", "", "task", "Sound", "queued"),
    ];
    let deps = vec![("c".to_string(), "b".to_string())];
    let p = progress(&rows, &deps);
    assert_eq!((p["done"].as_i64(), p["total"].as_i64()), (Some(1), Some(9)));
    assert_eq!((p["running"].as_i64(), p["needsYou"].as_i64(), p["inReview"].as_i64()), (Some(1), Some(2), Some(1)));
    assert_eq!(p["goals"], json!([{ "title": "Beta", "done": 1, "total": 3 }]));
    // Docs waits on Pay, so the three ready queued tasks come first.
    let next: Vec<&str> = p["next"].as_array().unwrap().iter().map(|n| n["title"].as_str().unwrap()).collect();
    assert_eq!(next, ["Logo", "Icons", "Sound"]);
    assert_eq!(p["next"][0]["ready"], true);
    let only_docs = progress(&rows[..4], &deps);
    assert_eq!(only_docs["next"], json!([{ "id": "c", "title": "Docs", "ready": false }]));
    let empty = progress(&[], &[]);
    assert_eq!((empty["done"].as_i64(), empty["total"].as_i64()), (Some(0), Some(0)));
    assert_eq!(empty["goals"], json!([]));
}

#[test]
fn git_output_gives_remote_and_branch_without_credentials() {
    assert_eq!(parse_git("https://git.example.com/k/app.git\nmain\nexit: 0"), Some(("https://git.example.com/k/app.git".into(), "main".into())));
    assert_eq!(parse_git("https://kees:s3cret@git.example.com/k/app.git\nmain\nexit: 0").unwrap().0, "https://git.example.com/k/app.git");
    assert_eq!(parse_git("git@github.com:k/app.git\n\nfeature/x\nexit: 0").unwrap().1, "feature/x");
    assert_eq!(parse_git("fatal: not a git repository\nexit: 128"), None);
    assert_eq!(parse_git("https://git.example.com/k/app.git\nmain"), None, "no exit code, no answer");
}

async fn db() -> SqlitePool {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    let recent = util::minutes_ago(5);
    let old = util::minutes_ago(60 * 24 * 40);
    let earlier = util::minutes_ago(10);
    for q in [
        "INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1', 'kees', 'x', '2026'), ('u2', 'other', 'x', '2026')".to_string(),
        "INSERT INTO projects (id, name, description, updated_at, user_id) VALUES ('chat', 'Notes', 'Just talk', '2026-10-01T00:00:00Z', 'u1')".into(),
        "INSERT INTO projects (id, name, description, updated_at, user_id, ptype) VALUES ('game', 'Forest', 'A game', '2026-10-02T00:00:00Z', 'u1', 'game')".into(),
        "INSERT INTO projects (id, name, description, updated_at, user_id, ptype, repo_folder, repo_machine_id, dev_settings, scheduler, overseer, about, links)
         VALUES ('prog', 'Shop', 'Sells plants', '2026-10-03T00:00:00Z', 'u1', 'programming', '/home/k/shop', 'm1',
         '{\"build\":\"cargo build\",\"test\":\"cargo test\",\"deploy\":\"sh deploy.sh\",\"last\":{\"test\":{\"ok\":true,\"at\":\"2026-10-10T10:00:00Z\",\"summary\":\"12 passed\"}}}',
         '{\"enabled\":true,\"budget\":{\"minutes\":30}}', 1, 'Why the shop exists.', '[{\"label\":\"Live\",\"url\":\"https://shop.example.com\"}]')".into(),
        "INSERT INTO machines (id, user_id, name, token_hash, created_at, last_seen) VALUES ('m1', 'u1', 'kireserver', 'h1', '2026', NULL), ('m2', 'u1', 'souc', 'h2', '2026', NULL)".into(),
        "INSERT INTO machine_grants (machine_id, target, rights, granted_by, granted_at, expires) VALUES
         ('m1', '/home/k', '[\"read\",\"write\"]', 'kees', '2026', '2026-12-01T00:00:00Z'),
         ('m1', '/home/k/shop', '[\"read\"]', 'kees', '2026', NULL),
         ('m1', '/srv', '[\"read\"]', 'kees', '2026', NULL),
         ('m1', 'system', '[\"packages\"]', 'kees', '2026', NULL),
         ('m2', '/home/k', '[\"read\"]', 'kees', '2026', NULL)".into(),
        "INSERT INTO mcp_grants (id, user_id, server, tool, granted_at, expires) VALUES ('g1', 'u1', 'blender', '*', '2026', '2099-01-01T00:00:00Z'), ('g2', 'u1', 'old', '*', '2026', '2020-01-01T00:00:00Z')".into(),
        "INSERT INTO knowledge_collection (id, user_id, name, created_at) VALUES ('k1', 'u1', 'Plant care', '2026')".into(),
        "INSERT INTO knowledge_project (collection_id, project_id) VALUES ('k1', 'prog')".into(),
        "INSERT INTO tasks (id, project_id, title, state, updated_at, user_id, kind, position) VALUES
         ('t1', 'prog', 'Cart', 'done', '2026', 'u1', 'task', 1), ('t2', 'prog', 'Checkout', 'needs_input', '2026', 'u1', 'task', 2),
         ('t3', 'prog', 'Search', 'queued', '2026', 'u1', 'task', 3)".into(),
        format!("INSERT INTO task_events (task_id, user_id, at, kind, detail) VALUES
         ('t1', 'u1', '2026-10-05T10:00:00Z', 'created', '{{}}'),
         ('t1', 'u1', '{recent}', 'forge', '{{\"pr\":\"https://f/pulls/7\",\"state\":{{\"to\":\"done\"}}}}')"),
        format!("INSERT INTO runs (id, task_id, user_id, chat_id, machine_id, folder, started_at, ended_at, status) VALUES
         ('r1', 't1', 'u1', 'c1', 'm1', '/home/k/shop', '{earlier}', '{earlier}', 'done'),
         ('r2', 't2', 'u1', 'c1', 'm1', '/home/k/shop', '{old}', '{old}', 'failed')"),
        format!("INSERT INTO calls (id, task_id, role, provider_id, model_id, reason, request, response, tokens_in, tokens_out, ms, at, run_id) VALUES
         ('a1', 't1', 'worker', 'ovms', 'qwen3-9b', 'step', '{{}}', '', 1000, 200, 60000, '{earlier}', 'r1'),
         ('a2', 't1', 'worker', 'ovms', 'qwen3-9b', 'step', '{{}}', '', 500, 100, 30000, '{earlier}', 'r1'),
         ('a3', 't2', 'worker', 'ovms', 'old-model', 'step', '{{}}', '', 9, 9, 1000, '{old}', 'r2')"),
        "INSERT INTO chats (id, project_id, title, updated_at, user_id) VALUES ('c1', 'prog', 'Ideas', '2026-10-04T00:00:00Z', 'u1')".into(),
    ] {
        sqlx::query(&q).execute(&db).await.unwrap_or_else(|e| panic!("{e}: {q}"));
    }
    db
}

fn user(id: &str) -> User {
    User { id: id.into(), name: id.into() }
}

fn state(db: SqlitePool) -> AppState {
    AppState::for_tests(toml::from_str("").unwrap(), db)
}

async fn overview(s: &AppState, who: &str, id: &str) -> ApiResult<Value> {
    get(State(s.clone()), Extension(user(who)), Path(id.into())).await.map(|Json(v)| v)
}

async fn row(db: &SqlitePool, id: &str) -> ProjectRow {
    load(db, "u1", id).await.unwrap().unwrap()
}

#[tokio::test]
async fn load_reads_the_project_with_its_owner_and_only_for_the_owner() {
    let db = db().await;
    let p = row(&db, "prog").await;
    assert_eq!((p.name.as_str(), p.ptype.as_str(), p.kind.as_str(), p.owner.as_str()), ("Shop", "programming", "internal", "kees"));
    assert_eq!((p.about.as_str(), p.repo_folder.as_deref(), p.repo_machine_id.as_deref()), ("Why the shop exists.", Some("/home/k/shop"), Some("m1")));
    assert_eq!((p.repo_git.as_str(), p.overseer, p.created_at.as_deref()), ("", true, Some("2026-10-03T00:00:00Z")));
    assert!(p.dev_settings.contains("cargo build") && p.scheduler.contains("enabled") && p.links.contains("Live"));
    assert!(load(&db, "u2", "prog").await.unwrap().is_none());
    assert!(load(&db, "u1", "nope").await.unwrap().is_none());
}

#[tokio::test]
async fn where_an_offline_project_lives() {
    let db = db().await;
    let s = state(db.clone());
    let w = where_part(&s, "u1", &row(&db, "prog").await).await.unwrap();
    assert_eq!(w["machine"]["id"], "m1");
    assert_eq!(w["machine"]["name"], "kireserver");
    assert_eq!(w["machine"]["online"], false);
    assert_eq!(w["folder"], "/home/k/shop");
    assert_eq!(w["gitState"], "unknown", "offline and nothing cached");
    assert!(w["git"].is_null());
    let w = where_part(&s, "u1", &row(&db, "chat").await).await.unwrap();
    assert!(w["machine"].is_null() && w["folder"].is_null() && w["git"].is_null());
    assert_eq!(w["gitState"], "none");
}

#[tokio::test]
async fn an_online_computer_shows_the_cached_git_and_starts_a_fresh_read_when_stale() {
    let db = db().await;
    sqlx::query("UPDATE machines SET last_seen = ? WHERE id = 'm1'").bind(util::now()).execute(&db).await.unwrap();
    let cached = json!({ "remote": "https://git.example.com/k/shop.git", "branch": "main", "at": util::now() }).to_string();
    sqlx::query("UPDATE projects SET repo_git = ? WHERE id = 'prog'").bind(&cached).execute(&db).await.unwrap();
    let s = state(db.clone());
    let w = where_part(&s, "u1", &row(&db, "prog").await).await.unwrap();
    assert_eq!(w["machine"]["online"], true);
    assert_eq!(w["gitState"], "known");
    assert_eq!(w["git"]["branch"], "main");
    // A fresh cache queues no job.
    let (jobs,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM machine_jobs").fetch_one(&db).await.unwrap();
    assert_eq!(jobs, 0);
    // A stale one is read again in the background, as a shell job in the repo folder.
    let stale = json!({ "remote": "x", "branch": "main", "at": "2026-01-01T00:00:00Z" }).to_string();
    sqlx::query("UPDATE projects SET repo_git = ? WHERE id = 'prog'").bind(&stale).execute(&db).await.unwrap();
    let w = where_part(&s, "u1", &row(&db, "prog").await).await.unwrap();
    assert_eq!(w["gitState"], "reading");
    assert_eq!(w["git"]["remote"], "x", "the old value shows until the new one is in");
    let mut tool = None;
    for _ in 0..50 {
        tool = sqlx::query_scalar::<_, String>("SELECT tool FROM machine_jobs").fetch_optional(&db).await.unwrap();
        if tool.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let tool: Value = serde_json::from_str(&tool.expect("a git job was queued")).unwrap();
    assert_eq!((tool["tool"].as_str(), tool["cwd"].as_str()), (Some("shell"), Some("/home/k/shop")));
    assert_eq!(tool["command"], "git remote get-url origin; git branch --show-current");
}

#[tokio::test]
async fn a_finished_git_read_is_cached() {
    let db = db().await;
    let s = state(db.clone());
    let job = tokio::spawn(refresh_git(s.clone(), "u1".into(), "prog".into(), "m1".into(), "/home/k/shop".into()));
    let mut id = None;
    for _ in 0..50 {
        id = sqlx::query_scalar::<_, String>("SELECT id FROM machine_jobs").fetch_optional(&db).await.unwrap();
        if id.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    sqlx::query("UPDATE machine_jobs SET state = 'done', result = ? WHERE id = ?")
        .bind("https://kees:tok@git.example.com/k/shop.git\nmain\nexit: 0")
        .bind(id.expect("a git job was queued"))
        .execute(&db)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), job).await.expect("the read ends once the job is done").unwrap();
    let git: String = sqlx::query_scalar("SELECT repo_git FROM projects WHERE id = 'prog'").fetch_one(&db).await.unwrap();
    let git: Value = serde_json::from_str(&git).unwrap();
    assert_eq!((git["remote"].as_str(), git["branch"].as_str()), (Some("https://git.example.com/k/shop.git"), Some("main")));
    assert!(git["at"].as_str().is_some_and(|a| a.starts_with("20")));
}

#[tokio::test]
async fn access_lists_covering_grants_mcp_knowledge_assets_and_commands() {
    let db = db().await;
    let s = state(db.clone());
    let a = access_part(&s, "u1", &row(&db, "prog").await).await.unwrap();
    // Only grants on the repo computer that cover the folder.
    let targets: Vec<&str> = a["grants"].as_array().unwrap().iter().map(|g| g["target"].as_str().unwrap()).collect();
    assert_eq!(targets, ["/home/k", "/home/k/shop"]);
    assert_eq!(a["grants"][0]["rights"], json!(["read", "write"]));
    assert_eq!(a["grants"][0]["expires"], "2026-12-01T00:00:00Z");
    assert!(a["grants"][1]["expires"].is_null());
    assert_eq!(a["mcp"], json!([{ "server": "blender", "tool": "*", "expires": "2099-01-01T00:00:00Z" }]));
    assert_eq!(a["knowledge"], json!([{ "id": "k1", "name": "Plant care" }]));
    assert_eq!(a["assets"], 0);
    let cmds = a["commands"].as_array().unwrap();
    let kinds: Vec<&str> = cmds.iter().map(|c| c["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["build", "test", "deploy"]);
    assert_eq!(cmds[0]["command"], "cargo build");
    assert_eq!(cmds[1]["last"], json!({ "ok": true, "at": "2026-10-10T10:00:00Z", "summary": "12 passed" }));
    assert!(cmds[0]["last"].is_null());
    assert_eq!((cmds[0]["fromHere"].as_bool(), cmds[2]["fromHere"].as_bool()), (Some(true), Some(false)), "deploy is never run from here");
    // A project without a repo computer has no computer grants and no commands.
    let a = access_part(&s, "u1", &row(&db, "chat").await).await.unwrap();
    assert_eq!((a["grants"].clone(), a["commands"].clone()), (json!([]), json!([])));
    assert_eq!(a["mcp"].as_array().unwrap().len(), 1, "MCP grants are the user's, shown on every project");
}

#[tokio::test]
async fn a_game_project_counts_its_assets() {
    let db = db().await;
    for q in [
        "INSERT INTO asset_pack (id, key, name, kind) VALUES (1, 'forest.zip', 'Forest pack', 'zip')",
        "INSERT INTO asset (id, pack_id, path, name, ext, size, category, rule) VALUES (7, 1, 'tree.png', 'tree.png', 'png', 10, 'sprite', 'ext'), (8, 1, 'a.ogg', 'a.ogg', 'ogg', 1, 'sound', 'ext')",
        "INSERT INTO project_asset (project_id, asset_id, added_at) VALUES ('game', 7, '2026'), ('game', 8, '2026')",
    ] {
        sqlx::query(q).execute(&db).await.unwrap();
    }
    let s = state(db.clone());
    let a = access_part(&s, "u1", &row(&db, "game").await).await.unwrap();
    assert_eq!(a["assets"], 2);
}

#[tokio::test]
async fn progress_part_adds_the_last_merged_pr() {
    let db = db().await;
    let p = progress_part(&db, "u1", "prog").await.unwrap();
    assert_eq!((p["done"].as_i64(), p["total"].as_i64(), p["needsYou"].as_i64()), (Some(1), Some(3), Some(1)));
    assert_eq!(p["lastMerged"], json!({ "title": "Cart", "pr": "https://f/pulls/7" }));
    assert_eq!(p["next"], json!([{ "id": "t3", "title": "Search", "ready": true }]));
    let p = progress_part(&db, "u1", "chat").await.unwrap();
    assert_eq!(p["total"], 0);
    assert!(p["lastMerged"].is_null());
    // Someone else's project counts nothing.
    assert_eq!(progress_part(&db, "u2", "prog").await.unwrap()["total"], 0);
}

#[tokio::test]
async fn activity_is_newest_first_across_events_runs_and_chats() {
    let db = db().await;
    let a = activity_part(&db, "u1", "prog").await.unwrap();
    let what: Vec<&str> = a.as_array().unwrap().iter().map(|a| a["what"].as_str().unwrap()).collect();
    assert_eq!(what, ["PR merged: Cart", "Run done: Cart", "Task added: Cart", "Chat: Ideas", "Run failed: Checkout"]);
    assert_eq!(a[1]["who"], "qwen3-9b", "a run names the model it used most");
    assert_eq!(a[0]["who"], "");
    assert!(a[0]["at"].as_str().is_some_and(|t| t.starts_with("20")));
    // At most 10.
    for i in 0..12 {
        sqlx::query("INSERT INTO task_events (task_id, user_id, at, kind, detail) VALUES ('t3', 'u1', ?, 'changed', '{}')").bind(format!("2026-10-06T00:00:{i:02}Z")).execute(&db).await.unwrap();
    }
    assert_eq!(activity_part(&db, "u1", "prog").await.unwrap().as_array().unwrap().len(), 10);
    assert_eq!(activity_part(&db, "u2", "prog").await.unwrap(), json!([]));
}

#[tokio::test]
async fn ai_use_counts_the_last_30_days_per_model() {
    let db = db().await;
    let ai = ai_part(&db, "u1", "prog").await.unwrap();
    assert_eq!(ai, json!([{ "model": "qwen3-9b", "runs": 1, "tokens": 1800, "minutes": 1.5 }]));
    assert_eq!(ai_part(&db, "u1", "chat").await.unwrap(), json!([]));
}

#[tokio::test]
async fn the_route_puts_the_header_and_every_part_together() {
    let s = state(db().await);
    let o = overview(&s, "u1", "prog").await.unwrap();
    assert_eq!(o["errors"], json!({}));
    assert_eq!((o["id"].as_str(), o["name"].as_str(), o["description"].as_str(), o["type"].as_str()), (Some("prog"), Some("Shop"), Some("Sells plants"), Some("programming")));
    assert_eq!((o["about"].as_str(), o["owner"].as_str(), o["kind"].as_str()), (Some("Why the shop exists."), Some("kees"), Some("internal")));
    assert!(o["source"].is_null());
    assert_eq!(o["links"], json!([{ "label": "Live", "url": "https://shop.example.com" }]));
    assert_eq!((o["createdAt"].as_str(), o["updatedAt"].as_str()), (Some("2026-10-03T00:00:00Z"), Some("2026-10-03T00:00:00Z")));
    assert!(o["archivedAt"].is_null());
    assert_eq!(o["where"]["machine"]["name"], "kireserver");
    assert_eq!(o["access"]["knowledge"][0]["name"], "Plant care");
    assert_eq!(o["progress"]["total"], 3);
    assert_eq!(o["activity"][0]["what"], "PR merged: Cart");
    assert_eq!(o["ai"][0]["model"], "qwen3-9b");
    assert_eq!(o["automation"], json!({ "scheduler": { "enabled": true, "budget": { "minutes": 30, "rounds": null, "tokens": null } }, "overseer": true }));
    // A chat project: empty parts, never an error.
    let o = overview(&s, "u1", "chat").await.unwrap();
    assert_eq!(o["errors"], json!({}));
    assert_eq!((o["links"].clone(), o["activity"].clone(), o["ai"].clone()), (json!([]), json!([]), json!([])));
    assert_eq!(o["automation"], json!({ "scheduler": { "enabled": false, "budget": { "minutes": null, "rounds": null, "tokens": null } }, "overseer": false }));
    // Broken links JSON shows as no links.
    sqlx::query("UPDATE projects SET links = 'not json' WHERE id = 'chat'").execute(&s.db).await.unwrap();
    assert_eq!(overview(&s, "u1", "chat").await.unwrap()["links"], json!([]));
}

#[tokio::test]
async fn a_failing_part_is_null_with_its_reason() {
    let db = db().await;
    sqlx::query("DROP TABLE runs").execute(&db).await.unwrap();
    let s = state(db);
    let o = overview(&s, "u1", "prog").await.unwrap();
    assert!(o["ai"].is_null() && o["activity"].is_null());
    assert!(o["errors"]["ai"].as_str().is_some_and(|e| !e.is_empty()));
    assert!(o["errors"]["activity"].is_string());
    assert_eq!(o["progress"]["total"], 3, "the other parts still come");
}

#[tokio::test]
async fn another_user_gets_not_found() {
    let s = state(db().await);
    assert!(matches!(overview(&s, "u2", "prog").await, Err(ApiError::NotFound)));
    assert!(matches!(overview(&s, "u1", "nope").await, Err(ApiError::NotFound)));
}

#[tokio::test]
async fn a_new_project_gets_its_creation_date() {
    let db = db().await;
    sqlx::query("INSERT INTO projects (id, name, updated_at, user_id) VALUES ('new', 'New', '2026-10-13T08:00:00Z', 'u1')").execute(&db).await.unwrap();
    let created: Option<String> = sqlx::query_scalar("SELECT created_at FROM projects WHERE id = 'new'").fetch_one(&db).await.unwrap();
    assert_eq!(created.as_deref(), Some("2026-10-13T08:00:00Z"));
}

#[tokio::test]
async fn prompts_read_about_after_the_description() {
    let db = db().await;
    let ctx = crate::project_ctx::project_context(&db, "prog", "u1", 8_000).await.unwrap();
    assert!(ctx.contains("Sells plants\n\nWhy the shop exists."), "{ctx}");
}

#[tokio::test]
async fn the_overseer_reads_about_after_the_description() {
    let db = db().await;
    let scan = crate::overseer::scan(&db, "u1", 20_000).await;
    assert!(scan.text.contains("Sells plants\nWhy the shop exists."), "{}", scan.text);
    // The Overseer reads at most ABOUT_IN_SCAN chars of it per project.
    sqlx::query("UPDATE projects SET about = ? WHERE id = 'prog'").bind("w".repeat(ABOUT_IN_SCAN + 50)).execute(&db).await.unwrap();
    let scan = crate::overseer::scan(&db, "u1", 20_000).await;
    assert!(scan.text.contains(&"w".repeat(ABOUT_IN_SCAN)) && !scan.text.contains(&"w".repeat(ABOUT_IN_SCAN + 1)), "{}", scan.text);
}
