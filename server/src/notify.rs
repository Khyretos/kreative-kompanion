//! Task notifications: per-user switches per event (needs you, failed, done, a step waiting for
//! approval, a daily summary) and per channel (mail; push to the phone app and to browsers),
//! with optional quiet hours. They carry the task title and its new state only, never
//! descriptions, prompts or secrets. At most one mail per task per 10 minutes.

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    AppState,
    auth::User,
    error::{ApiError, ApiResult},
    mail, util,
};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    pub email: String,
    pub on_needs_input: bool,
    pub on_failed: bool,
    pub on_done: bool,
    pub daily_summary: bool,
    /// NOTIFY: a step waits for the user's approval.
    pub on_approval: bool,
    /// NOTIFY: channels: mail, and push (the phone app and browsers that subscribed).
    pub mail_on: bool,
    pub push_on: bool,
    /// NOTIFY: "HH:MM" in the user's time; both empty = none. tz_offset: minutes east of UTC.
    pub quiet_from: String,
    pub quiet_to: String,
    pub tz_offset: i64,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            email: String::new(), on_needs_input: true, on_failed: true, on_done: false, daily_summary: false,
            on_approval: true, mail_on: true, push_on: true, quiet_from: String::new(), quiet_to: String::new(), tz_offset: 0,
        }
    }
}

fn minutes(hhmm: &str) -> Option<i64> {
    let (h, m) = hhmm.split_once(':')?;
    let (h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
    ((0..24).contains(&h) && (0..60).contains(&m) && hhmm.len() == 5).then_some(h * 60 + m)
}

/// Whether `now` (UTC) falls in the user's quiet hours; a span may run past midnight (22:00-07:00).
pub fn quiet(p: &Prefs, now: time::OffsetDateTime) -> bool {
    let (Some(from), Some(to)) = (minutes(&p.quiet_from), minutes(&p.quiet_to)) else { return false };
    let local = (i64::from(now.hour()) * 60 + i64::from(now.minute()) + p.tz_offset).rem_euclid(1440);
    if from <= to { from <= local && local < to } else { local >= from || local < to }
}

/// The user's switches; the address defaults to the account's email (from
/// single sign-on), so notifications work without any setup.
async fn prefs(db: &SqlitePool, user_id: &str) -> sqlx::Result<Prefs> {
    let mut p: Prefs = sqlx::query_as(
        "SELECT email, on_needs_input, on_failed, on_done, daily_summary, on_approval, mail_on, push_on, quiet_from, quiet_to, tz_offset
         FROM notification_prefs WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .unwrap_or_default();
    if p.email.is_empty() {
        let account: Option<(Option<String>,)> = sqlx::query_as("SELECT email FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(db)
            .await?;
        p.email = account.and_then(|a| a.0).unwrap_or_default();
    }
    Ok(p)
}

pub async fn get_prefs(State(s): State<AppState>, Extension(u): Extension<User>) -> ApiResult<Json<Prefs>> {
    Ok(Json(prefs(&s.db, &u.id).await?))
}

pub async fn put_prefs(
    State(s): State<AppState>,
    Extension(u): Extension<User>,
    Json(mut p): Json<Prefs>,
) -> ApiResult<StatusCode> {
    p.email = p.email.trim().to_string();
    if !p.email.is_empty()
        && (p.email.len() > 200 || p.email.matches('@').count() != 1 || p.email.contains(char::is_whitespace))
    {
        return Err(ApiError::BadRequest("That doesn't look like a mail address.".into()));
    }
    p.quiet_from = p.quiet_from.trim().to_string();
    p.quiet_to = p.quiet_to.trim().to_string();
    let set = |t: &str| !t.is_empty();
    if set(&p.quiet_from) != set(&p.quiet_to) || (set(&p.quiet_from) && (minutes(&p.quiet_from).is_none() || minutes(&p.quiet_to).is_none())) {
        return Err(ApiError::BadRequest("Quiet hours need a start and an end, like 22:00 and 07:00.".into()));
    }
    if !(-14 * 60..=14 * 60).contains(&p.tz_offset) {
        return Err(ApiError::BadRequest("That time zone is out of range.".into()));
    }
    sqlx::query(
        "INSERT INTO notification_prefs (user_id, email, on_needs_input, on_failed, on_done, daily_summary, on_approval, mail_on, push_on, quiet_from, quiet_to, tz_offset)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET email = excluded.email, on_needs_input = excluded.on_needs_input,
           on_failed = excluded.on_failed, on_done = excluded.on_done, daily_summary = excluded.daily_summary,
           on_approval = excluded.on_approval, mail_on = excluded.mail_on, push_on = excluded.push_on,
           quiet_from = excluded.quiet_from, quiet_to = excluded.quiet_to, tz_offset = excluded.tz_offset",
    )
    .bind(&u.id)
    .bind(&p.email)
    .bind(p.on_needs_input)
    .bind(p.on_failed)
    .bind(p.on_done)
    .bind(p.daily_summary)
    .bind(p.on_approval)
    .bind(p.mail_on)
    .bind(p.push_on)
    .bind(&p.quiet_from)
    .bind(&p.quiet_to)
    .bind(p.tz_offset)
    .execute(&s.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Which label a move to `to` gets, if this user wants a mail for it.
fn wanted(p: &Prefs, to: &str) -> Option<&'static str> {
    match to {
        "needs_input" | "waiting_resources" if p.on_needs_input => Some("needs you"),
        "failed" if p.on_failed => Some("failed"),
        "done" if p.on_done => Some("done"),
        _ => None,
    }
}

/// The app's public address for links in mails ("https://kompanion.example.com"), set at start.
pub static PUBLIC_URL: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// A Reply-To with a display name: a bare address shows whatever name the reader's mail app
/// learned for it (info@ showed as "Watchtower", whose updates come from that address).
fn reply_to_with_name(reply_to: &str) -> String {
    let r = reply_to.trim();
    if r.is_empty() || r.contains('<') { r.to_string() } else { format!("Kreative Kompas <{r}>") }
}

static SENT: LazyLock<Mutex<HashMap<String, Instant>>> = LazyLock::new(Default::default);

/// The parts of a branded mail besides the app's own name, link, logo and colour.
struct Card<'a> {
    status: crate::mailhtml::Status,
    title: &'a str,
    intro: &'a str,
    rows: &'a [(&'a str, &'a str)],
    button: Option<(&'a str, &'a str)>,
}

/// The branded HTML of a mail, with this server's name, link, logo and colour.
async fn card_html(db: &SqlitePool, c: Card<'_>) -> anyhow::Result<String> {
    let st = crate::admin::load(db).await?;
    let url = PUBLIC_URL.get().cloned().unwrap_or_default();
    let logo = if url.is_empty() { String::new() } else { format!("{url}/api/mail-logo.png") };
    Ok(crate::mailhtml::render(&crate::mailhtml::Mail {
        app_name: &st.app_name, app_url: &url, logo_url: &logo, brand: &st.color_brand,
        status: c.status, title: c.title, intro: c.intro, rows: c.rows, button: c.button,
    }))
}

async fn send(db: &SqlitePool, to: &str, subject_tail: &str, body: &str, card: Option<Card<'_>>) -> anyhow::Result<()> {
    let st = crate::admin::load(db).await?;
    // NOTIFY: the server's own mail settings (KOMPANION_SMTP_*) win over the ones typed in Settings.
    let smtp = match mail::from_env(&|k| std::env::var(k).ok()) {
        Some(mut env) => {
            env.reply_to = reply_to_with_name(&env.reply_to);
            env
        }
        None => {
            anyhow::ensure!(!st.smtp_host.is_empty() && !st.smtp_from.is_empty(), "mail is not set up");
            mail::SmtpSettings {
                host: st.smtp_host,
                port: st.smtp_port,
                tls: st.smtp_tls,
                user: st.smtp_user,
                from: st.smtp_from,
                reply_to: reply_to_with_name(&st.smtp_reply_to),
            }
        }
    };
    let password = mail::password(&|k| std::env::var(k).ok());
    let url = PUBLIC_URL.get().cloned().unwrap_or_default();
    let logo = if url.is_empty() { String::new() } else { format!("{url}/api/mail-logo.png") };
    let html = card.map(|c| {
        crate::mailhtml::render(&crate::mailhtml::Mail {
            app_name: &st.app_name, app_url: &url, logo_url: &logo, brand: &st.color_brand,
            status: c.status, title: c.title, intro: c.intro, rows: c.rows, button: c.button,
        })
    });
    mail::send(&smtp, password.as_deref(), to, &format!("{}: {subject_tail}", st.app_name), body, html.as_deref()).await
}

/// The text of a task mail: what happened, what is needed, and where to open it.
fn mail_body(title: &str, label: &str, step: &str, link: &str) -> String {
    let mut b = format!("{title}\nNow: {label}.\n");
    let need = match label {
        "needs you" if !step.is_empty() => format!("What's needed: {step}.\n"),
        "needs you" => "What's needed: an answer or a decision from you in Kompanion.\n".to_string(),
        "failed" if !step.is_empty() => format!("Where it stopped: {step}.\n"),
        _ => String::new(),
    };
    b.push_str(&need);
    b.push_str(&if link.is_empty() { "Open Kompanion to see the details.\n".to_string() } else { format!("Open it: {link}\n") });
    b
}

/// Call after a task's state changed; mails in the background if wanted.
pub fn task_changed(db: SqlitePool, user_id: String, task_id: String, title: String, from: String, to: String) {
    let hash = format!("task={task_id}");
    task_changed_at(db, user_id, task_id, title, from, to, hash);
}

/// Like task_changed, with the app link's hash given: "task=<id>", or "chat=<id>&msg=<id>" for a
/// message in the project thread.
pub fn task_changed_at(db: SqlitePool, user_id: String, task_id: String, title: String, from: String, to: String, hash: String) {
    if from == to {
        return;
    }
    tokio::spawn(async move {
        let p = match prefs(&db, &user_id).await {
            Ok(p) => p,
            Err(e) => return tracing::warn!("notification prefs: {e}"),
        };
        let Some(label) = wanted(&p, &to) else { return };
        let push_link = PUBLIC_URL.get().map(|u| format!("{u}/#{hash}")).unwrap_or_default();
        if !deliver_push(&db, &user_id, &p, &title, label, &push_link) || !p.mail_on || p.email.is_empty() {
            return;
        }
        {
            let mut sent = SENT.lock().unwrap();
            let key = format!("{user_id}/{title}");
            if sent.get(&key).is_some_and(|t| t.elapsed() < Duration::from_secs(600)) {
                return;
            }
            sent.insert(key, Instant::now());
        }
        let step: String = sqlx::query_as::<_, (Option<String>,)>("SELECT step FROM tasks WHERE id = ?")
            .bind(&task_id)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten()
            .and_then(|(s,)| s)
            .unwrap_or_default();
        let link = PUBLIC_URL.get().map(|u| format!("{u}/#{hash}")).unwrap_or_default();
        let status = match label { "needs you" => crate::mailhtml::Status::NeedsYou, "failed" => crate::mailhtml::Status::Failed, _ => crate::mailhtml::Status::Done };
        let intro = match label { "needs you" => "This task is waiting for you.", "failed" => "This task stopped with an error.", _ => "This task is finished." };
        let need_label = if label == "failed" { "Where it stopped" } else { "What's needed" };
        let rows_owned: Vec<(&str, &str)> = if step.is_empty() || label == "done" { vec![] } else { vec![(need_label, step.as_str())] };
        let card = Card { status, title: &title, intro, rows: &rows_owned, button: if link.is_empty() { None } else { Some(("Open the task", link.as_str())) } };
        let body = mail_body(&title, label, &step, &link);
        if let Err(e) = send(&db, &p.email, &format!("{title} — {label}"), &body, Some(card)).await {
            tracing::warn!("task mail failed: {e:#}");
        }
    });
}

/// The live event (open apps show it), then, outside quiet hours, push to the phone app and to the
/// user's browsers. False in quiet hours: the caller sends no mail then either.
fn deliver_push(db: &SqlitePool, user_id: &str, p: &Prefs, title: &str, label: &'static str, link: &str) -> bool {
    if let Some(bus) = crate::events::BUS.get() {
        bus.send(user_id, crate::events::Event::Notify { title: title.to_string(), state: label, url: link.to_string() });
    }
    if quiet(p, time::OffsetDateTime::now_utc()) {
        return false;
    }
    if p.push_on {
        crate::push::notify(db.clone(), user_id.to_string(), title.to_string(), label, link.to_string());
        crate::webpush::notify(db.clone(), user_id.to_string(), crate::push::payload(title, label, link));
    }
    true
}

/// At most one "waits for your approval" per chat in this time.
const APPROVAL_GAP: Duration = Duration::from_secs(300);

/// NOTIFY: a step in `chat_id` waits for the user's approval: push (and mail) unless the user
/// turned it off, is in quiet hours, or was told about this chat in the last five minutes.
pub fn approval_needed(db: SqlitePool, user_id: String, chat_id: String, summary: String) {
    {
        let mut sent = SENT.lock().unwrap();
        let key = format!("{user_id}/approval/{chat_id}");
        if sent.get(&key).is_some_and(|t| t.elapsed() < APPROVAL_GAP) {
            return;
        }
        sent.insert(key, Instant::now());
    }
    tokio::spawn(async move {
        let Ok(p) = prefs(&db, &user_id).await else { return };
        if !p.on_approval {
            return;
        }
        let link = PUBLIC_URL.get().map(|u| format!("{u}/#chat={chat_id}")).unwrap_or_default();
        let title: String = summary.chars().take(120).collect();
        if !deliver_push(&db, &user_id, &p, &title, "waits for your approval", &link) || !p.mail_on || p.email.is_empty() {
            return;
        }
        let rows: [(&str, &str); 1] = [("Step", title.as_str())];
        let card = Card { status: crate::mailhtml::Status::NeedsYou, title: "A step waits for your approval", intro: "Kompanion wants to run a step and asks you first.", rows: &rows,
            button: if link.is_empty() { None } else { Some(("Open the chat", link.as_str())) } };
        let body = format!("A step waits for your approval: {title}\n{}", if link.is_empty() { "Open Kompanion to approve or deny it.\n".to_string() } else { format!("Open it: {link}\n") });
        if let Err(e) = send(&db, &p.email, &format!("approval needed — {title}"), &body, Some(card)).await {
            tracing::warn!("approval mail failed: {e:#}");
        }
    });
}

/// Daily summary at or after 08:00 UTC, once per day per user.
pub fn spawn_daily(db: SqlitePool) {
    crate::util::supervise("daily-summary", move || { let db = db.clone(); async move {
        loop {
            tokio::time::sleep(Duration::from_secs(900)).await;
            let now = util::now();
            if now.get(11..13).and_then(|h| h.parse::<u32>().ok()).unwrap_or(0) < 8 {
                continue;
            }
            let today = &now[..10];
            let users: Vec<(String, String)> = match sqlx::query_as(
                "SELECT user_id, email FROM notification_prefs
                 WHERE daily_summary = 1 AND mail_on = 1 AND email <> '' AND (last_daily IS NULL OR substr(last_daily, 1, 10) <> ?)",
            )
            .bind(today)
            .fetch_all(&db)
            .await
            {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!("daily summary: {e}");
                    continue;
                }
            };
            for (user_id, email) in users {
                let counts: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT state, COUNT(*) FROM tasks WHERE user_id = ? GROUP BY state ORDER BY state",
                )
                .bind(&user_id)
                .fetch_all(&db)
                .await
                .unwrap_or_default();
                let rows: Vec<(String, String)> = counts.iter().map(|(s, n)| (s.replace('_', " "), n.to_string())).collect();
                let row_refs: Vec<(&str, &str)> = rows.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
                let url = PUBLIC_URL.get().cloned().unwrap_or_default();
                let card = Card { status: crate::mailhtml::Status::Info, title: "Your tasks today", intro: "How your tasks stand this morning.", rows: &row_refs,
                    button: if url.is_empty() { None } else { Some(("Open Kompanion", url.as_str())) } };
                let body: String = counts.iter().map(|(s, n)| format!("{}: {n}\n", s.replace('_', " "))).collect();
                if let Err(e) = send(&db, &email, "today", &body, Some(card)).await {
                    tracing::warn!("daily summary mail failed: {e:#}");
                    continue;
                }
                let _ = sqlx::query("UPDATE notification_prefs SET last_daily = ? WHERE user_id = ?")
                    .bind(util::now())
                    .bind(&user_id)
                    .execute(&db)
                    .await;
            }
        }
    } });
}

/// `kompanion-server test-mail <address>`: a sample "needs you" mail through the configured
/// mail server, to check the branded mail renders.
pub async fn send_test(db: &SqlitePool, to: &str, print: bool) -> anyhow::Result<()> {
    let url = PUBLIC_URL.get().cloned().unwrap_or_default();
    let link = format!("{url}/");
    let rows: [(&str, &str); 2] = [("What's needed", "folder not found on soucouyant"), ("Project", "Kreative Kompanion")];
    let card = Card { status: crate::mailhtml::Status::NeedsYou, title: "Add initials() to names.py (test mail)", intro: "This task is waiting for you.", rows: &rows,
        button: Some(("Open the task", link.as_str())) };
    if print {
        println!("{}", card_html(db, card).await?);
        return Ok(());
    }
    send(db, to, "test mail", "Add initials() to names.py (test mail)\nNow: needs you.\nWhat's needed: folder not found on soucouyant.\n", Some(card)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_wanted_moves_mail() {
        let p = Prefs::default();
        assert_eq!(wanted(&p, "needs_input"), Some("needs you"));
        assert_eq!(wanted(&p, "failed"), Some("failed"));
        assert_eq!(wanted(&p, "done"), None);
        assert_eq!(wanted(&Prefs { on_done: true, ..p }, "done"), Some("done"));
    }

    #[test]
    fn task_mails_say_what_is_needed() {
        let b = mail_body("Add initials()", "needs you", "folder not found", "https://k.example/#task=t1");
        assert!(b.contains("What's needed: folder not found.") && b.contains("Open it: https://k.example/#task=t1"));
        assert!(mail_body("T", "needs you", "", "").contains("an answer or a decision"));
        assert!(!mail_body("T", "done", "x", "").contains("needed"));
        assert_eq!(reply_to_with_name("info@kreative-kompas.com"), "Kreative Kompas <info@kreative-kompas.com>");
        assert_eq!(reply_to_with_name("Team <a@b.c>"), "Team <a@b.c>");
        assert_eq!(reply_to_with_name(" "), "");
    }

    #[test]
    fn quiet_hours_follow_the_users_clock_and_may_cross_midnight() {
        let at = |h: u8, m: u8| time::OffsetDateTime::UNIX_EPOCH.replace_hour(h).unwrap().replace_minute(m).unwrap();
        let night = Prefs { quiet_from: "22:00".into(), quiet_to: "07:00".into(), ..Prefs::default() };
        assert!(quiet(&night, at(23, 30)) && quiet(&night, at(6, 59)) && !quiet(&night, at(7, 0)) && !quiet(&night, at(12, 0)));
        // Amsterdam in summer (UTC+2): 05:30 UTC is 07:30 there, past the quiet hours.
        let ams = Prefs { tz_offset: 120, ..night.clone() };
        assert!(!quiet(&ams, at(5, 30)) && quiet(&ams, at(4, 30)) && quiet(&ams, at(20, 0)));
        let lunch = Prefs { quiet_from: "12:00".into(), quiet_to: "13:00".into(), ..Prefs::default() };
        assert!(quiet(&lunch, at(12, 30)) && !quiet(&lunch, at(13, 0)));
        assert!(!quiet(&Prefs::default(), at(3, 0)), "no quiet hours set");
        assert_eq!(minutes("7:00"), None);
        assert_eq!(minutes("24:00"), None);
    }

    #[test]
    fn the_mail_server_and_password_can_come_from_the_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(a, _)| *a == k).map(|(_, v)| v.to_string());
        assert!(mail::from_env(&env(&[("KOMPANION_SMTP_HOST", "mailserver")])).is_none(), "a sender is needed too");
        let m = mail::from_env(&env(&[("KOMPANION_SMTP_HOST", "mailserver"), ("KOMPANION_SMTP_FROM", "kompanion@kreative-kompas.com"), ("KOMPANION_SMTP_USER", "info@kreative-kompas.com")])).unwrap();
        assert_eq!((m.host.as_str(), m.port, m.tls.as_str(), m.user.as_str()), ("mailserver", 587, "starttls", "info@kreative-kompas.com"));
        let tls = mail::from_env(&env(&[("KOMPANION_SMTP_HOST", "h"), ("KOMPANION_SMTP_FROM", "f@x"), ("KOMPANION_SMTP_TLS", "tls")])).unwrap();
        assert_eq!(tls.port, 465);
        let dir = std::env::temp_dir().join(format!("kk-smtp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("pw");
        std::fs::write(&file, "test-only-not-a-password\n").unwrap();
        let path: &'static str = Box::leak(file.display().to_string().into_boxed_str());
        let pairs: &'static [(&'static str, &'static str)] = Box::leak(vec![("KOMPANION_SMTP_PASSWORD_FILE", path), ("SMTP_PASSWORD", "older")].into_boxed_slice());
        assert_eq!(mail::password(&env(pairs)).as_deref(), Some("test-only-not-a-password"));
        assert_eq!(mail::password(&env(&[("SMTP_PASSWORD", "older")])).as_deref(), Some("older"));
        assert_eq!(mail::password(&env(&[("KOMPANION_SMTP_PASSWORD", " ")])), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn prefs_keep_channels_and_quiet_hours_and_refuse_half_a_span() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026')").execute(&db).await.unwrap();
        let s = AppState::for_tests(toml::from_str("").unwrap(), db);
        let u = || Extension(User { id: "u1".into(), name: "kees".into() });
        let put = |v: serde_json::Value| Json(serde_json::from_value::<Prefs>(v).unwrap());
        // An older app sends only the first five fields: the rest keep their defaults.
        put_prefs(State(s.clone()), u(), put(serde_json::json!({ "email": "k@x.nl", "onNeedsInput": true, "onFailed": true, "onDone": true, "dailySummary": false }))).await.unwrap();
        let Json(p) = get_prefs(State(s.clone()), u()).await.unwrap();
        assert!(p.on_approval && p.mail_on && p.push_on && p.on_done);
        put_prefs(State(s.clone()), u(), put(serde_json::json!({ "email": "k@x.nl", "pushOn": false, "onApproval": false, "quietFrom": "22:00", "quietTo": "07:30", "tzOffset": 60 }))).await.unwrap();
        let Json(p) = get_prefs(State(s.clone()), u()).await.unwrap();
        assert_eq!((p.push_on, p.on_approval, p.quiet_from.as_str(), p.quiet_to.as_str(), p.tz_offset), (false, false, "22:00", "07:30", 60));
        assert!(put_prefs(State(s.clone()), u(), put(serde_json::json!({ "quietFrom": "22:00" }))).await.is_err());
        assert!(put_prefs(State(s.clone()), u(), put(serde_json::json!({ "quietFrom": "25:00", "quietTo": "07:00" }))).await.is_err());
    }

    #[tokio::test]
    async fn task_change_reaches_the_live_stream() {
        let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&db).await.unwrap();
        let bus = crate::events::BUS.get_or_init(crate::events::Bus::new);
        let mut rx = bus.subscribe();
        task_changed(db.clone(), "u-live".into(), "t1".into(), "Backup".into(), "running".into(), "done".into());
        task_changed(db.clone(), "u-live".into(), "t1".into(), "Backup".into(), "running".into(), "needs_input".into());
        let event = loop {
            let (user, e) = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await.unwrap().unwrap();
            if user == "u-live" {
                break e;
            }
        };
        let v = serde_json::to_value(&event).unwrap();
        assert_eq!(v["type"], "notify");
        assert_eq!(v["title"], "Backup");
        assert_eq!(v["state"], "needs you");
    }
}
