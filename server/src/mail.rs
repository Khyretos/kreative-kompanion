//! Sends plain-text mail through the admin's SMTP server, or the one the server's environment names
//! (KOMPANION_SMTP_HOST, _PORT, _TLS, _USER, _FROM, _REPLY_TO). The password comes from the
//! environment (KOMPANION_SMTP_PASSWORD, or a file in KOMPANION_SMTP_PASSWORD_FILE, or the older
//! SMTP_PASSWORD) and is never stored, shown or logged.

use anyhow::{Context, Result};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, header::ContentType},
    transport::smtp::authentication::Credentials,
};

pub struct SmtpSettings {
    pub host: String,
    pub port: u16,
    /// "starttls", "tls" (implicit TLS) or "none".
    pub tls: String,
    pub user: String,
    pub from: String,
    /// Optional Reply-To address.
    pub reply_to: String,
}

/// NOTIFY: the mail server set by the environment, when KOMPANION_SMTP_HOST and _FROM are there.
/// `var` reads a variable (std::env::var in the app, a map in tests).
pub fn from_env(var: &dyn Fn(&str) -> Option<String>) -> Option<SmtpSettings> {
    let get = |k: &str| var(&format!("KOMPANION_SMTP_{k}")).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    let (host, from) = (get("HOST")?, get("FROM")?);
    let tls = get("TLS").unwrap_or_else(|| "starttls".into());
    let default_port = if tls == "tls" { 465 } else { 587 };
    Some(SmtpSettings {
        host,
        port: get("PORT").and_then(|p| p.parse().ok()).unwrap_or(default_port),
        tls,
        user: get("USER").unwrap_or_default(),
        from,
        reply_to: get("REPLY_TO").unwrap_or_default(),
    })
}

/// Whether the environment sets the mail server (Settings then shows its fields read-only).
pub fn env_set() -> bool {
    from_env(&|k| std::env::var(k).ok()).is_some()
}

/// The SMTP password: KOMPANION_SMTP_PASSWORD, else the file KOMPANION_SMTP_PASSWORD_FILE names,
/// else SMTP_PASSWORD. Trimmed; never logged.
pub fn password(var: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    var("KOMPANION_SMTP_PASSWORD")
        .or_else(|| var("KOMPANION_SMTP_PASSWORD_FILE").and_then(|f| std::fs::read_to_string(f.trim()).ok()))
        .or_else(|| var("SMTP_PASSWORD"))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
}

pub async fn send(s: &SmtpSettings, password: Option<&str>, to: &str, subject: &str, body: &str, html: Option<&str>) -> Result<()> {
    let builder = match s.tls.as_str() {
        "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&s.host),
        "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&s.host),
        _ => Ok(AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&s.host)),
    }
    .with_context(|| format!("mail server {}", s.host))?
    .port(s.port);
    let builder = match password {
        Some(p) if !s.user.is_empty() => builder.credentials(Credentials::new(s.user.clone(), p.to_string())),
        _ => builder,
    };
    let mut message = Message::builder().from(s.from.parse::<Mailbox>().context("sender address")?);
    if !s.reply_to.trim().is_empty() {
        message = message.reply_to(s.reply_to.parse::<Mailbox>().context("reply-to address")?);
    }
    let message = message.to(to.parse::<Mailbox>().context("recipient address")?).subject(subject);
    // Branded HTML with the plain text next to it; plain text alone when there is no HTML.
    let message = match html {
        Some(h) => message.multipart(MultiPart::alternative_plain_html(body.to_string(), h.to_string())),
        None => message.header(ContentType::TEXT_PLAIN).body(body.to_string()),
    }
    .context("building the mail")?;
    builder.build().send(message).await.context("sending")?;
    Ok(())
}
