//! Runner self-update (RUNNER-SYNC): check a downloaded binary and swap it in.

use std::path::Path;

/// True when `sha_text` (the content of a `.sha256` file: a hex digest, optionally followed by a file name) is the SHA-256 of `bytes`.
pub fn sha_ok(bytes: &[u8], sha_text: &str) -> bool {
    use sha2::{Digest, Sha256};
    let expected: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    let Some(word) = sha_text.split_whitespace().next() else { return false };
    word.eq_ignore_ascii_case(&expected)
}

/// Writes `bytes` next to `target` as `<target>.new`, makes it executable (0o755), runs it with `--version`
/// and requires its output to start with `kompanion-runner `; only then renames it over `target`.
/// On any failure the `.new` file is removed, `target` is untouched and the error text is returned.
pub fn install_binary(bytes: &[u8], target: &Path) -> Result<(), String> {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".new");
    let new = target.with_file_name(name);
    let swap = || -> Result<(), String> {
        std::fs::write(&new, bytes).map_err(|e| e.to_string())?;
        make_executable(&new)?;
        let mut tries = 0;
        loop {
            let out = std::process::Command::new(&new).arg("--version").output();
            match out {
                Ok(_) => break,
                Err(e) if e.raw_os_error() == Some(26) && tries < 20 => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    tries += 1;
                    continue;
                }
                Err(_) | Ok(_) if tries >= 20 => return Err("the new runner does not start".into()),
                _ => return Err("the new runner does not start".into()),
            }
        }
        let out = std::process::Command::new(&new).arg("--version").output().map_err(|e| format!("the new runner does not start: {e}"))?;
        if !out.status.success() || !String::from_utf8_lossy(&out.stdout).starts_with("kompanion-runner ") {
            return Err("the new file is not a kompanion-runner".into());
        }
        put_in_place(&new, target)
    };
    let result = swap();
    if result.is_err() {
        let _ = std::fs::remove_file(&new);
    }
    result
}

/// Downloads `{base}/download/kompanion-runner` and `{base}/download/kompanion-runner.sha256` with `agent`, requires
/// the digest to match (error text contains "sha256"), installs the file over `target` with `install_binary` and
/// returns a message naming the new version (the first line of the new runner's `--version` output).
pub fn update_from_server(agent: &ureq::Agent, base: &str, target: &Path) -> Result<String, String> {
    use std::io::Read;
    let url = format!("{}/download/{}", base.trim_end_matches('/'), asset_name());
    let resp = agent.get(&url).call().map_err(|e| format!("download failed: {e}"))?;
    let mut bytes = Vec::new();
    resp.into_reader().take(200_000_000).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    let sha_text = agent
        .get(&format!("{url}.sha256"))
        .call()
        .map_err(|e| format!("download of the sha256 failed: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;
    if !sha_ok(&bytes, &sha_text) {
        return Err("the download does not match its sha256".into());
    }
    install_binary(&bytes, target)?;
    let version = std::process::Command::new(target).arg("--version").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").trim().to_string()).unwrap_or_default();
    Ok(if version.is_empty() { "updated".to_string() } else { format!("updated: {version}") })
}

/// Name of the runner file the server offers for download.
pub fn asset_name() -> &'static str {
    if cfg!(windows) {
        "kompanion-runner.exe"
    } else {
        "kompanion-runner"
    }
}

/// The `.old` file next to `target` (where a running program is moved aside).
fn old_path(target: &Path) -> std::path::PathBuf {
    let mut name = target.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    target.with_file_name(name)
}

/// Windows cannot rename over a running program, but it can rename the running file away: moves `target` to
/// `<target>.old` and `new` to `target`; puts `target` back when the second step fails.
#[cfg_attr(unix, allow(dead_code))]
pub fn replace_with_new(new: &Path, target: &Path) -> Result<(), String> {
    let old = old_path(target);
    let _ = std::fs::remove_file(&old);
    let moved = target.exists();
    if moved {
        std::fs::rename(target, &old).map_err(|e| e.to_string())?;
    }
    if let Err(e) = std::fs::rename(new, target) {
        if moved {
            let _ = std::fs::rename(&old, target);
        }
        return Err(e.to_string());
    }
    Ok(())
}

/// Removes the `.old` file a Windows self-update leaves next to the running program; ignores every error.
pub fn cleanup_old(target: &Path) {
    let _ = std::fs::remove_file(old_path(target));
}

/// Makes the downloaded file executable (0755 on unix; Windows runs any .exe).
fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// Puts the checked file in place: a plain rename on unix, rename-then-replace on Windows.
fn put_in_place(new: &Path, target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::fs::rename(new, target).map_err(|e| e.to_string())
    }
    #[cfg(not(unix))]
    {
        replace_with_new(new, target)
    }
}

#[cfg(test)]
#[path = "selfupdate_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "selfupdate_replace_tests.rs"]
mod replace_tests;
