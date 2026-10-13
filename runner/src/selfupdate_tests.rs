use super::*;
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn temp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kk-selfupdate-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn sha_ok_accepts_sha256sum_line() {
    let body = b"runner bytes";
    assert!(sha_ok(body, &format!("{}  kompanion-runner\n", hex(body))));
    assert!(sha_ok(body, &hex(body).to_uppercase()));
}

#[test]
fn sha_ok_rejects_other_bytes_and_junk() {
    let body = b"runner bytes";
    assert!(!sha_ok(b"other bytes", &format!("{}  kompanion-runner\n", hex(body))));
    assert!(!sha_ok(body, ""));
    assert!(!sha_ok(body, "not-a-hash  kompanion-runner"));
}

#[cfg(unix)]
#[test]
fn install_binary_replaces_a_working_binary() {
    let d = temp("ok");
    let target = d.join("kompanion-runner");
    std::fs::write(&target, "#!/bin/sh\necho kompanion-runner 0.1.0\n").unwrap();
    let new = "#!/bin/sh\necho kompanion-runner 9.9.9\n";
    install_binary(new.as_bytes(), &target).unwrap();
    assert_eq!(std::fs::read_to_string(&target).unwrap(), new);
    assert_eq!(std::fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755);
    assert!(!d.join("kompanion-runner.new").exists());
}

#[cfg(unix)]
#[test]
fn install_binary_keeps_the_old_one_when_the_new_one_does_not_run() {
    let d = temp("bad");
    let target = d.join("kompanion-runner");
    std::fs::write(&target, "old").unwrap();
    let err = install_binary(b"this is not a program", &target).unwrap_err();
    assert!(!err.is_empty());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
    assert!(!d.join("kompanion-runner.new").exists());
}

#[cfg(unix)]
#[test]
fn install_binary_rejects_a_program_that_is_not_the_runner() {
    let d = temp("other");
    let target = d.join("kompanion-runner");
    std::fs::write(&target, "old").unwrap();
    assert!(install_binary(b"#!/bin/sh\necho hello\n", &target).is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
}

/// Serves `/download/kompanion-runner` and `/download/kompanion-runner.sha256` on 127.0.0.1 for `requests` requests.
#[cfg(unix)]
fn serve(binary: Vec<u8>, sha_text: String, requests: usize) -> String {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for _ in 0..requests {
            let (mut sock, _) = listener.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = sock.read(&mut buf).unwrap();
            let head = String::from_utf8_lossy(&buf[..n]).to_string();
            let body = if head.starts_with("GET /download/kompanion-runner.sha256 ") {
                sha_text.clone().into_bytes()
            } else if head.starts_with("GET /download/kompanion-runner ") {
                binary.clone()
            } else {
                let _ = sock.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                continue;
            };
            let _ = sock.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes());
            let _ = sock.write_all(&body);
        }
    });
    base
}

#[cfg(unix)]
#[test]
fn update_from_server_installs_a_verified_download() {
    let d = temp("dl-ok");
    let target = d.join("kompanion-runner");
    std::fs::write(&target, "old").unwrap();
    let new = b"#!/bin/sh\necho kompanion-runner 9.9.9\n".to_vec();
    let base = serve(new.clone(), format!("{}  kompanion-runner\n", hex(&new)), 2);
    let agent = ureq::AgentBuilder::new().build();
    let msg = update_from_server(&agent, &base, &target).unwrap();
    assert!(msg.contains("9.9.9"), "{msg}");
    assert_eq!(std::fs::read(&target).unwrap(), new);
}

#[cfg(unix)]
#[test]
fn update_from_server_refuses_a_wrong_digest() {
    let d = temp("dl-bad");
    let target = d.join("kompanion-runner");
    std::fs::write(&target, "old").unwrap();
    let new = b"#!/bin/sh\necho kompanion-runner 9.9.9\n".to_vec();
    let base = serve(new, format!("{}  kompanion-runner\n", hex(b"something else")), 2);
    let agent = ureq::AgentBuilder::new().build();
    let err = update_from_server(&agent, &base, &target).unwrap_err();
    assert!(err.contains("sha256"), "{err}");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
    assert!(!d.join("kompanion-runner.new").exists());
}

#[cfg(unix)]
#[test]
fn update_from_server_reports_a_missing_download() {
    let d = temp("dl-404");
    let target = d.join("kompanion-runner");
    std::fs::write(&target, "old").unwrap();
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(2)).build();
    assert!(update_from_server(&agent, "http://127.0.0.1:9", &target).is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");
}

/// Other threads fork (the runner runs granted tools in threads) while a fresh binary is executed: the child
/// may briefly hold the write handle, and exec then fails with "Text file busy" unless install_binary retries.
#[cfg(unix)]
#[test]
fn install_binary_survives_concurrent_forks() {
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let forkers: Vec<_> = (0..4)
        .map(|_| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let _ = std::process::Command::new("true").output();
                }
            })
        })
        .collect();
    let d = temp("forks");
    for n in 0..60 {
        let target = d.join(format!("kompanion-runner-{n}"));
        std::fs::write(&target, "old").unwrap();
        install_binary(b"#!/bin/sh\necho kompanion-runner 9.9.9\n", &target).unwrap();
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    for f in forkers {
        f.join().unwrap();
    }
}
