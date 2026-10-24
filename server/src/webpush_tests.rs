use super::*;
use p256::ecdsa::{VerifyingKey, signature::Verifier};

fn b64(s: &str) -> Vec<u8> {
    B64.decode(s).unwrap()
}

/// RFC 8291, Appendix A: the same keys, salt and text give exactly the published message.
#[test]
fn encryption_matches_the_rfc_8291_example() {
    let as_secret = SecretKey::from_slice(&b64("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")).unwrap();
    assert_eq!(B64.encode(as_secret.public_key().to_encoded_point(false).as_bytes()), "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8");
    let salt: [u8; 16] = b64("DGv6ra1nlYgDCS1FRnbzlw").try_into().unwrap();
    let ua_public = b64("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4");
    let auth = b64("BTBZMqHH6r4Tts7J_aSIgg");
    let body = encrypt_with(&as_secret, salt, &ua_public, &auth, b"When I grow up, I want to be a watermelon").unwrap();
    assert_eq!(
        B64.encode(body),
        "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
    );
    assert!(encrypt_with(&as_secret, salt, b"not a key", &auth, b"x").is_err());
    assert!(encrypt_with(&as_secret, salt, &ua_public, &auth, &vec![b'x'; 5000]).is_err());
}

#[test]
fn the_vapid_header_is_a_signed_es256_token_for_the_push_service() {
    let key = SigningKey::from_slice(&[7u8; 32]).unwrap();
    let h = vapid_header(&key, "https://fcm.googleapis.com/fcm/send/abc", "https://kompanion.example.com", 1_700_000_000).unwrap();
    let (t, k) = h.strip_prefix("vapid t=").unwrap().split_once(", k=").unwrap();
    assert_eq!(k, public_b64(&key));
    let parts: Vec<&str> = t.split('.').collect();
    assert_eq!(parts.len(), 3);
    let claims: serde_json::Value = serde_json::from_slice(&b64(parts[1])).unwrap();
    assert_eq!(claims, json!({ "aud": "https://fcm.googleapis.com", "exp": 1_700_043_200, "sub": "https://kompanion.example.com" }));
    let sig = Signature::from_slice(&b64(parts[2])).unwrap();
    let vk = VerifyingKey::from_sec1_bytes(&b64(k)).unwrap();
    assert!(vk.verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &sig).is_ok());
}

#[test]
fn only_the_browsers_push_services_are_allowed() {
    for ok in [
        "https://fcm.googleapis.com/fcm/send/dX9",
        "https://updates.push.services.mozilla.com/wpush/v2/gAAA",
        "https://wns2-db5p.notify.windows.com/w/?token=BQYAAA",
        "https://web.push.apple.com/QGuQyavXutnMH",
    ] {
        assert!(endpoint_allowed(ok), "{ok}");
    }
    for bad in [
        "http://fcm.googleapis.com/fcm/send/x",
        "https://fcm.googleapis.com:8443/x",
        "https://127.0.0.1/x",
        "https://evil.example.com/fcm.googleapis.com",
        "https://notify.windows.com.evil.example/x",
        "https://.push.apple.com/x",
        "https://user@fcm.googleapis.com/x",
        "not a url",
    ] {
        assert!(!endpoint_allowed(bad), "{bad}");
    }
}

#[tokio::test]
async fn subscriptions_are_checked_and_the_key_is_made_once() {
    let opts = sqlx::sqlite::SqliteConnectOptions::new().in_memory(true).foreign_keys(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    sqlx::query("INSERT INTO users (id, name, password_hash, created_at) VALUES ('u1','kees','x','2026')").execute(&db).await.unwrap();
    let s = AppState::for_tests(toml::from_str("").unwrap(), db);
    let user = || Extension(User { id: "u1".into(), name: "kees".into() });
    let Json(k1) = key(State(s.clone()), user()).await.unwrap();
    let Json(k2) = key(State(s.clone()), user()).await.unwrap();
    assert_eq!(k1, k2);
    assert_eq!(b64(k1["key"].as_str().unwrap()).len(), 65);
    // A browser's real-looking keys (the RFC example's) are taken; junk and other hosts are not.
    let sub = |endpoint: &str, p256dh: &str, auth: &str| Json(serde_json::from_value::<Subscription>(json!({ "endpoint": endpoint, "keys": { "p256dh": p256dh, "auth": auth }, "device": "Firefox" })).unwrap());
    let good_key = "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    let good_auth = "BTBZMqHH6r4Tts7J_aSIgg";
    assert_eq!(subscribe(State(s.clone()), user(), sub("https://updates.push.services.mozilla.com/wpush/v2/abc", good_key, good_auth)).await.unwrap(), StatusCode::NO_CONTENT);
    assert!(subscribe(State(s.clone()), user(), sub("https://evil.example.com/x", good_key, good_auth)).await.is_err());
    assert!(subscribe(State(s.clone()), user(), sub("https://fcm.googleapis.com/fcm/send/x", "AAAA", good_auth)).await.is_err());
    assert!(subscribe(State(s.clone()), user(), sub("https://fcm.googleapis.com/fcm/send/x", good_key, "AAAA")).await.is_err());
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM web_push_subs").fetch_one(&s.db).await.unwrap();
    assert_eq!(n, 1);
    unsubscribe(State(s.clone()), user(), Json(Unsubscribe { endpoint: "https://updates.push.services.mozilla.com/wpush/v2/abc".into() })).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM web_push_subs").fetch_one(&s.db).await.unwrap();
    assert_eq!(n, 0);
}
