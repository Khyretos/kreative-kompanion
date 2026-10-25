//! NOTIFY: Web Push to the installed web app (the PWA) and browsers, also when Kompanion is closed.
//! Payloads are encrypted for the browser (RFC 8291, aes128gcm) and signed for the push service
//! (RFC 8292, VAPID); the server's VAPID key is made once and kept in `settings`. Subscriptions only
//! point at the browsers' own push services (no user-chosen hosts: no SSRF), and a push carries the
//! same as a mail's subject: the title, its new state and a link, never descriptions or prompts.

use aes_gcm::{Aes128Gcm, KeyInit, aead::Aead};
use axum::{Extension, Json, extract::State, http::StatusCode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use hkdf::Hkdf;
use p256::{PublicKey, SecretKey, ecdsa::{Signature, SigningKey, signature::Signer}, elliptic_curve::sec1::ToEncodedPoint};
use serde::Deserialize;
use serde_json::json;
use sha2::Sha256;
use sqlx::SqlitePool;

use crate::{AppState, auth::User, error::{ApiError, ApiResult}, util};

/// Hosts of the browsers' push services (Chrome/Edge/Android, Firefox, Windows, Safari).
const PUSH_HOSTS: &[&str] = &["fcm.googleapis.com", "updates.push.services.mozilla.com", "web.push.apple.com"];
const PUSH_HOST_SUFFIXES: &[&str] = &[".push.services.mozilla.com", ".notify.windows.com", ".push.apple.com"];
/// Bytes of one record (one record is all a notification needs).
const RECORD_SIZE: u32 = 4096;

/// An https URL on one of the browsers' push services, on the default port.
pub fn endpoint_allowed(endpoint: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(endpoint) else { return false };
    let host = u.host_str().unwrap_or("");
    endpoint.len() <= 1000
        && u.scheme() == "https"
        && u.port().is_none()
        && u.username().is_empty()
        && (PUSH_HOSTS.contains(&host) || PUSH_HOST_SUFFIXES.iter().any(|s| host.ends_with(s) && host.len() > s.len()))
}

fn random<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    rand::fill(&mut b);
    b
}

/// The server's VAPID key, made on first use and kept in settings.
async fn vapid_key(db: &SqlitePool) -> anyhow::Result<SigningKey> {
    let stored: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'vapidKey'").fetch_optional(db).await?;
    if let Some(v) = stored.and_then(|v| serde_json::from_str::<String>(&v).ok()) {
        return Ok(SigningKey::from_slice(&B64.decode(v)?)?);
    }
    let key = loop {
        if let Ok(k) = SecretKey::from_slice(&random::<32>()) {
            break k;
        }
    };
    let value = serde_json::to_string(&B64.encode(key.to_bytes()))?;
    // Two first uses at once: the first one stored wins, and both read it back.
    sqlx::query("INSERT INTO settings (key, value) VALUES ('vapidKey', ?) ON CONFLICT(key) DO NOTHING").bind(value).execute(db).await?;
    let v: String = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'vapidKey'").fetch_one(db).await?;
    Ok(SigningKey::from_slice(&B64.decode(serde_json::from_str::<String>(&v)?)?)?)
}

fn public_b64(key: &SigningKey) -> String {
    B64.encode(key.verifying_key().to_encoded_point(false).as_bytes())
}

/// RFC 8291: `plaintext` encrypted for a browser's key (`ua_public`, 65 bytes) and auth secret,
/// with this side's one-time key `as_secret` and `salt` (random for every push).
pub fn encrypt_with(as_secret: &SecretKey, salt: [u8; 16], ua_public: &[u8], auth: &[u8], plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let ua = PublicKey::from_sec1_bytes(ua_public).map_err(|_| anyhow::anyhow!("the browser's key is not a P-256 point"))?;
    let as_public = as_secret.public_key().to_encoded_point(false);
    let shared = p256::ecdh::diffie_hellman(as_secret.to_nonzero_scalar(), ua.as_affine());
    let mut key_info = b"WebPush: info\0".to_vec();
    key_info.extend_from_slice(ua_public);
    key_info.extend_from_slice(as_public.as_bytes());
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth), shared.raw_secret_bytes()).expand(&key_info, &mut ikm).map_err(|_| anyhow::anyhow!("hkdf"))?;
    let prk = Hkdf::<Sha256>::new(Some(&salt), &ikm);
    let (mut cek, mut nonce) = ([0u8; 16], [0u8; 12]);
    prk.expand(b"Content-Encoding: aes128gcm\0", &mut cek).map_err(|_| anyhow::anyhow!("hkdf"))?;
    prk.expand(b"Content-Encoding: nonce\0", &mut nonce).map_err(|_| anyhow::anyhow!("hkdf"))?;
    let mut record = plaintext.to_vec();
    record.push(2); // the last (and only) record
    anyhow::ensure!(record.len() + 16 <= RECORD_SIZE as usize - 86, "the push text is too long");
    let sealed = Aes128Gcm::new(&cek.into()).encrypt(&nonce.into(), record.as_slice()).map_err(|_| anyhow::anyhow!("encryption failed"))?;
    let mut body = salt.to_vec();
    body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    body.push(as_public.as_bytes().len() as u8);
    body.extend_from_slice(as_public.as_bytes());
    body.extend_from_slice(&sealed);
    Ok(body)
}

/// RFC 8292: the Authorization header for a push to `endpoint`, valid 12 hours.
pub fn vapid_header(key: &SigningKey, endpoint: &str, subject: &str, now_unix: i64) -> anyhow::Result<String> {
    let u = reqwest::Url::parse(endpoint)?;
    let aud = format!("{}://{}", u.scheme(), u.host_str().unwrap_or(""));
    let head = B64.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
    let claims = B64.encode(json!({ "aud": aud, "exp": now_unix + 12 * 3600, "sub": subject }).to_string());
    let signed = format!("{head}.{claims}");
    let sig: Signature = key.sign(signed.as_bytes());
    Ok(format!("vapid t={signed}.{}, k={}", B64.encode(sig.to_bytes()), public_b64(key)))
}

#[derive(Deserialize)]
pub struct Keys {
    p256dh: String,
    auth: String,
}

#[derive(Deserialize)]
pub struct Subscription {
    endpoint: String,
    keys: Keys,
    #[serde(default)]
    device: String,
}

/// GET /api/push/web/key: the server's public VAPID key for PushManager.subscribe.
pub async fn key(State(s): State<AppState>, Extension(_u): Extension<User>) -> ApiResult<Json<serde_json::Value>> {
    let k = vapid_key(&s.db).await.map_err(|e| ApiError::BadRequest(format!("push keys: {e}")))?;
    Ok(Json(json!({ "key": public_b64(&k) })))
}

/// POST /api/push/web: this browser's subscription.
pub async fn subscribe(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<Subscription>) -> ApiResult<StatusCode> {
    let endpoint = b.endpoint.trim();
    if !endpoint_allowed(endpoint) {
        return Err(ApiError::BadRequest("That push address is not a browser push service.".into()));
    }
    let (Ok(p256dh), Ok(auth)) = (B64.decode(b.keys.p256dh.trim_end_matches('=')), B64.decode(b.keys.auth.trim_end_matches('='))) else {
        return Err(ApiError::BadRequest("The browser's push keys are not readable.".into()));
    };
    if PublicKey::from_sec1_bytes(&p256dh).is_err() || auth.len() != 16 {
        return Err(ApiError::BadRequest("The browser's push keys are not readable.".into()));
    }
    let device: String = b.device.trim().chars().take(80).collect();
    sqlx::query(
        "INSERT INTO web_push_subs (id, user_id, endpoint, p256dh, auth, device, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (user_id, endpoint) DO UPDATE SET p256dh = excluded.p256dh, auth = excluded.auth, device = excluded.device",
    )
    .bind(util::new_id())
    .bind(&u.id)
    .bind(endpoint)
    .bind(B64.encode(&p256dh))
    .bind(B64.encode(&auth))
    .bind(device)
    .bind(util::now())
    .execute(&s.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Unsubscribe {
    endpoint: String,
}

/// DELETE /api/push/web: this browser no longer gets pushes.
pub async fn unsubscribe(State(s): State<AppState>, Extension(u): Extension<User>, Json(b): Json<Unsubscribe>) -> ApiResult<StatusCode> {
    sqlx::query("DELETE FROM web_push_subs WHERE user_id = ? AND endpoint = ?").bind(&u.id).bind(b.endpoint.trim()).execute(&s.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Sends `payload` to every browser of the user in the background; a subscription the push
/// service says is gone (404, 410) is removed.
pub fn notify(db: SqlitePool, user_id: String, payload: String) {
    tokio::spawn(async move {
        let subs: Vec<(String, String, String)> = sqlx::query_as("SELECT endpoint, p256dh, auth FROM web_push_subs WHERE user_id = ?")
            .bind(&user_id)
            .fetch_all(&db)
            .await
            .unwrap_or_default();
        if subs.is_empty() {
            return;
        }
        let key = match vapid_key(&db).await {
            Ok(k) => k,
            Err(e) => return tracing::warn!("web push key: {e:#}"),
        };
        let subject = crate::notify::PUBLIC_URL.get().cloned().filter(|u| u.starts_with("https://")).unwrap_or_else(|| "mailto:kompanion@kreative-kompas.com".into());
        let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().unwrap_or_default();
        for (endpoint, p256dh, auth) in subs {
            if !endpoint_allowed(&endpoint) {
                continue;
            }
            let sent = async {
                let as_secret = loop {
                    if let Ok(k) = SecretKey::from_slice(&random::<32>()) {
                        break k;
                    }
                };
                let body = encrypt_with(&as_secret, random::<16>(), &B64.decode(&p256dh)?, &B64.decode(&auth)?, payload.as_bytes())?;
                let auth_header = vapid_header(&key, &endpoint, &subject, time::OffsetDateTime::now_utc().unix_timestamp())?;
                let r = client
                    .post(&endpoint)
                    .header("TTL", "86400")
                    .header("Urgency", "high")
                    .header("Content-Encoding", "aes128gcm")
                    .header("Content-Type", "application/octet-stream")
                    .header("Authorization", auth_header)
                    .body(body)
                    .send()
                    .await?;
                anyhow::Ok(r.status())
            }
            .await;
            match sent {
                Ok(st) if st.is_success() => {
                    let _ = sqlx::query("UPDATE web_push_subs SET last_ok_at = ? WHERE user_id = ? AND endpoint = ?").bind(util::now()).bind(&user_id).bind(&endpoint).execute(&db).await;
                }
                Ok(st) if st.as_u16() == 404 || st.as_u16() == 410 => {
                    let _ = sqlx::query("DELETE FROM web_push_subs WHERE user_id = ? AND endpoint = ?").bind(&user_id).bind(&endpoint).execute(&db).await;
                }
                Ok(st) => tracing::warn!("web push answered {st}"),
                Err(e) => tracing::warn!("web push failed: {e:#}"),
            }
        }
    });
}

#[cfg(test)]
#[path = "webpush_tests.rs"]
mod tests;
