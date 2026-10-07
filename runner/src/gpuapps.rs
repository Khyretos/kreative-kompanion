//! GPU-01: the studio apps on this PC (fixed container names from the config), Ollama unload, game detection.
use std::{collections::BTreeMap, fs, path::Path, sync::OnceLock, time::Duration};
use crate::grants::{Grants, Right};
use crate::proc::run_cmd;
use crate::tools::Outcome;

/// App name -> container name, from `[gpu_apps]` in config.toml (set once by main).
pub static APPS: OnceLock<BTreeMap<String, String>> = OnceLock::new();

pub fn apps() -> &'static BTreeMap<String, String> {
    APPS.get_or_init(BTreeMap::new)
}

/// Start, stop or status of one app (or all), or Ollama unload; needs the "gpu" system grant.
pub fn gpu_apps(grants: &Grants, apps: &BTreeMap<String, String>, action: &str, app: Option<&str>, now: &str) -> Outcome {
    if !grants.allows_system(&Right::Gpu, now) {
        return Outcome { ok: false, output: "not granted: gpu".into() };
    }

    let targets = match app {
        Some(name) => {
            if !apps.contains_key(name) {
                return Outcome { ok: false, output: format!("unknown app: {}", name) };
            }
            vec![(name.to_string(), apps[name].clone())]
        }
        None => {
            let mut targets: Vec<(String, String)> = Vec::new();
            for (k, v) in apps.iter() {
                targets.push((k.clone(), v.clone()));
            }
            targets
        }
    };

    match action {
        "status" => {
            let mut lines = Vec::new();
            for (name, container) in targets {
                let out = run_cmd("docker", &docker_args(action, &container), None, &[], 20);
                if out.ok {
                    lines.push(format!("{}: {}", name, out.output.trim()));
                } else {
                    lines.push(format!("{}: missing", name));
                }
            }
            Outcome { ok: true, output: lines.join("\n") }
        }
        "start" | "stop" | "kill" => {
            let mut all_ok = true;
            let mut lines = Vec::new();
            for (name, container) in targets {
                let out = run_cmd("docker", &docker_args(action, &container), None, &[], 60);
                if out.ok {
                    lines.push(format!("{}: {}", name, if action == "start" { "started" } else { "stopped" }));
                } else {
                    all_ok = false;
                    lines.push(format!("{}: failed: {}", name, out.output.trim()));
                }
            }
            Outcome { ok: all_ok, output: lines.join("\n") }
        }
        "unload_ollama" => unload_ollama("http://127.0.0.1:11434"),
        _ => Outcome { ok: false, output: format!("unknown gpu_apps action: {}", action) },
    }
}

/// The docker arguments for "status", "start", "stop" or "kill" of one container.
fn docker_args(action: &str, container: &str) -> Vec<String> {
    let parts: &[&str] = match action {
        "status" => &["inspect", "-f", "{{.State.Status}}"],
        "kill" => &["stop", "-t", "0"],
        _ => &[action],
    };
    parts.iter().map(|s| s.to_string()).chain([container.to_string()]).collect()
}

fn unload_ollama(base: &str) -> Outcome {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).build();
    
    let url = format!("{}/api/ps", base);
    let resp = agent.get(&url).call();
    match resp {
        Ok(response) => {
            let json = response.into_json::<serde_json::Value>().unwrap_or_default();
            let models = json.get("models").and_then(|m| m.as_array()).map(|v| v.to_vec()).unwrap_or_default();
            
            let names: Vec<String> = models.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect();
    
            if names.is_empty() {
                return Outcome { ok: true, output: "nothing loaded".into() };
            }
            
            let gen_url = format!("{}/api/generate", base);
            for name in &names {
                let payload = serde_json::json!({"model": name, "keep_alive": 0});
                let _ = agent.post(&gen_url)
                    .send_json(&payload);
            }
            
            Outcome { ok: true, output: format!("unloaded: {}", names.join(", ")) }
        }
        Err(_) => Outcome { ok: true, output: "Ollama is not running".into() },
    }
}

/// True when a game runs: a gamescope process, or a Steam game (`reaper SteamLaunch ...`).
pub fn game_running(proc_root: &Path) -> bool {
    let entries = match fs::read_dir(proc_root) {
        Ok(d) => d,
        Err(_) => return false,
    };
    
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        
        if !name.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        
        let cmdline_path = entry.path().join("cmdline");
        let bytes = match fs::read(&cmdline_path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        
        let args: Vec<String> = bytes.split(|&b| b == 0)
            .map(|s| String::from_utf8_lossy(s).to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        if args.is_empty() {
            continue;
        }
        
        let first_arg = &args[0];
        let first_file = first_arg.rsplit('/').next().unwrap_or(first_arg);
        
        if first_file == "gamescope" {
            return true;
        }
        
        if args.contains(&"SteamLaunch".to_string()) {
            return true;
        }
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;
    use std::process;

    fn fake_proc(name: &str, procs: &[&[u8]]) -> PathBuf {
        let dir = env::temp_dir().join(format!("kk-gpuapps-{}-{}", name, process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        
        for (i, data) in procs.iter().enumerate() {
            let sub = dir.join(format!("{}", 100 + i));
            fs::create_dir_all(&sub).unwrap();
            fs::write(sub.join("cmdline"), data).unwrap();
        }
        
        let self_dir = dir.join("self");
        fs::create_dir_all(&self_dir).unwrap();
        fs::write(self_dir.join("cmdline"), b"gamescope\0").unwrap();
        
        dir
    }

    #[test]
    fn docker_command_lines() {
        assert_eq!(docker_args("stop", "comfyui-rocm"), ["stop", "comfyui-rocm"]);
        assert_eq!(docker_args("start", "comfyui-rocm"), ["start", "comfyui-rocm"]);
        assert_eq!(docker_args("status", "sfx"), ["inspect", "-f", "{{.State.Status}}", "sfx"]);
        assert_eq!(docker_args("kill", "comfyui-rocm"), ["stop", "-t", "0", "comfyui-rocm"]);
    }

    #[test]
    fn game_detected_for_gamescope() {
        let dir = fake_proc("gs", &[b"/usr/bin/bash\0", b"/usr/bin/gamescope\0-f\0--\0steam\0"]);
        assert!(game_running(&dir));
    }

    #[test]
    fn game_detected_for_steam_launch() {
        let dir = fake_proc("sl", &[b"/home/u/.steam/ubuntu12_32/reaper\0SteamLaunch\0AppId=570\0"]);
        assert!(game_running(&dir));
    }

    #[test]
    fn no_game_for_plain_steam() {
        let dir = fake_proc("ns", &[b"/usr/bin/steam\0-silent\0", b"fish\0"]);
        assert!(!game_running(&dir));
    }

    #[test]
    fn missing_proc_is_no_game() {
        assert!(!game_running(&PathBuf::from("/nonexistent/kk-proc")));
    }

    fn grants(with_gpu: bool) -> Grants {
        // One folder per call: the tests run in parallel.
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = env::temp_dir().join(format!("kk-gpuapps-grants-{}-{}-{}", with_gpu, process::id(), n));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        
        let mut g = Grants::load(&dir.join("grants.json")).unwrap();
        if with_gpu {
            use crate::grants::Grant;
            g.add(Grant {
                target: "system".into(),
                rights: vec![Right::Gpu],
                granted_by: "test".into(),
                granted_at: "2026-01-01T00:00:00Z".into(),
                expires: None,
            }).unwrap();
        }
        g
    }

    #[test]
    fn refused_without_grant() {
        let apps = BTreeMap::new();
        let o = gpu_apps(&grants(false), &apps, "status", None, "2026-10-06T12:00:00Z");
        assert!(!o.ok);
        assert_eq!(o.output, "not granted: gpu");
    }

    #[test]
    fn unknown_app_refused() {
        let mut apps = BTreeMap::new();
        apps.insert("comfyui".to_string(), "comfyui-rocm".to_string());
        let o = gpu_apps(&grants(true), &apps, "stop", Some("steam"), "2026-10-06T12:00:00Z");
        assert!(!o.ok);
        assert_eq!(o.output, "unknown app: steam");
    }

    #[test]
    fn unknown_action_refused() {
        let mut apps = BTreeMap::new();
        apps.insert("comfyui".to_string(), "comfyui-rocm".to_string());
        let o = gpu_apps(&grants(true), &apps, "rm", None, "2026-10-06T12:00:00Z");
        assert!(!o.ok);
        assert_eq!(o.output, "unknown gpu_apps action: rm");
    }

    #[test]
    fn unload_without_ollama() {
        let o = unload_ollama("http://127.0.0.1:9");
        assert_eq!(o.output, "Ollama is not running");
    }
}
