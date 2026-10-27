use serde_json::{Value, json};

/// The schema of PC agent tools available to the runner.
pub fn schema() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read a text file (up to 256 KiB). A long result comes back cut in the middle, the same way each time: for one part of a big file (a function by name), find its line first (shell: grep -n 'def parse_line' big.py) and read just those lines (shell: sed -n '790,840p' big.py). Or pass offset and length (bytes); the result says where to continue.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "offset": { "type": "integer", "description": "Byte to start at (default 0)." },
                        "length": { "type": "integer", "description": "Bytes to read (at most 61440 per call)." }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_image",
                "description": "Look at a picture (PNG, JPEG, GIF or WebP, up to 3 MiB): it arrives in the next message and you can see it. Only for a model that can see pictures. For text files use read_file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_dir",
                "description": "List the contents of a directory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Create or replace a file with new content. A big file goes in parts of at most about 20,000 characters each: the first part without append, every next part with append: true. If a part fails, send it again with offset set to the size the last good part reported (the file is cut there first, so nothing doubles).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" },
                        "append": { "type": "boolean", "description": "Add to the end of the file (made if missing) instead of replacing it." },
                        "offset": { "type": "integer", "minimum": 0, "description": "Cut the file to this many bytes, then write there. Not with append." }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Replace whole lines in a file; returns a diff. Read the file first. `old` must be one or more COMPLETE lines copied exactly from the file (never part of a line) and unique in it; `new` replaces them. To add code, put the line next to where it goes in `old` and repeat it in `new` with the new lines. Never empty.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "old": { "type": "string" },
                        "new": { "type": "string" }
                    },
                    "required": ["path", "old", "new"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "shell",
                "description": "Run a shell command in a folder (60 s limit).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "cwd": { "type": "string" },
                        "command": { "type": "string" }
                    },
                    "required": ["cwd", "command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "service",
                "description": "Manage the user's systemd services.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["list", "status", "start", "stop", "restart", "enable", "disable", "is-active"] },
                        "unit": { "type": "string" }
                    },
                    "required": ["action"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "package",
                "description": "Install/remove packages using a package manager (asks for password on the PC).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "manager": { "type": "string", "enum": ["pacman", "paru", "apt", "flatpak"] },
                        "action": { "type": "string", "enum": ["install", "remove", "search", "info"] },
                        "names": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["manager", "action", "names"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "reload",
                "description": "Reload a desktop compositor or panel.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "what": { "type": "string", "enum": ["hyprland", "waybar", "mako", "swaync"] }
                    },
                    "required": ["what"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "system_info",
                "description": "Show system information (OS, kernel, package managers, desktop).",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "capabilities",
                "description": "What Kompanion can use right now: models and their status, the user's computers with their grants, the tools and the skills. Answered by Kompanion itself, not a computer.",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        }
    ])
}

/// An edit's `old` text must be complete lines of the file and found exactly once, else the
/// edit is refused with a reason the model can act on. (2026-10-04: `old` = "def full_name"
/// replaced half a line and left "(first: str, last: str) -> str:" behind, a broken file.)
pub fn whole_lines(content: &str, old: &str) -> Result<(), String> {
    if old.trim().is_empty() {
        return Err("`old` is empty. Copy the complete line(s) you want to change; to add code, put the line next to it in `old` and repeat it in `new`.".into());
    }
    let hits: Vec<usize> = content.match_indices(old).map(|(i, _)| i).collect();
    match hits.len() {
        0 => return Err("`old` was not found in the file. Read the file again and copy the complete lines exactly.".into()),
        1 => {}
        n => return Err(format!("`old` is in the file {n} times. Include more lines so it is unique.")),
    }
    let start = hits[0];
    let end = start + old.len();
    let starts_line = start == 0 || content[..start].ends_with('\n');
    let ends_line = end == content.len() || old.ends_with('\n') || content[end..].starts_with('\n') || content[end..].starts_with("\r\n");
    if !starts_line || !ends_line {
        let line_start = content[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = content[end..].find('\n').map_or(content.len(), |i| end + i);
        return Err(format!(
            "`old` is only part of a line. Replace whole lines: use the full line(s), for example:\n{}",
            &content[line_start..line_end]
        ));
    }
    Ok(())
}

/// Convert a tool call JSON into a runner job JSON.
/// Returns None if the name is not recognized or arguments are invalid.
pub fn to_job(name: &str, args: &Value) -> Option<Value> {
    let known = match name {
        "read_file" => vec!["path", "offset", "length"],
        "read_image" => vec!["path"],
        "list_dir" => vec!["path"],
        "write_file" => vec!["path", "content", "append", "offset"],
        "edit_file" => vec!["path", "old", "new"],
        "shell" => vec!["cwd", "command"],
        "service" => vec!["action", "unit"],
        "package" => vec!["manager", "action", "names"],
        "reload" => vec!["what"],
        "system_info" => vec![],
        _ => return None,
    };

    // Ensure all string values in args are actually Strings (not numbers or booleans masquerading)
    // and that arrays contain strings.
    let clean_args = |v: &Value| -> Option<Value> {
        match v {
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, val) in map {
                    if !known.contains(&k.as_str()) {
                        continue;
                    }
                    match k.as_str() {
                        "names" => {
                            // Must be an array of strings
                            if let Value::Array(arr) = val {
                                if arr.iter().all(|item| item.is_string()) {
                                    out.insert(k.clone(), val.clone());
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        }
                        // WRITE-CHUNKS: a flag, sent as true/false or as a word.
                        "append" => match val.as_bool().or_else(|| val.as_str().and_then(|t| t.trim().parse::<bool>().ok())) {
                            Some(b) => { out.insert(k.clone(), json!(b)); }
                            None => return None,
                        },
                        "offset" | "length" => {
                            // Byte counts: a whole number >= 0, sent as a number or as digits.
                            let n = val.as_u64().or_else(|| val.as_str().and_then(|t| t.trim().parse::<u64>().ok()));
                            match n {
                                Some(n) => { out.insert(k.clone(), json!(n)); }
                                None => return None,
                            }
                        }
                        _ => {
                            // All other params must be strings
                            if val.is_string() {
                                out.insert(k.clone(), val.clone());
                            } else {
                                return None;
                            }
                        }
                    }
                }
                Some(Value::Object(out))
            }
            _ => None,
        }
    };

    let clean = clean_args(args)?;
    let Value::Object(mut map) = clean else { return None };
    map.insert("tool".into(), json!(name));
    Some(Value::Object(map))
}

/// One readable line for an approval card. The job is `{"tool": name, ...args}`.
pub fn summary(job: &Value) -> String {
    let s = |k: &str| job[k].as_str().unwrap_or("").to_string();
    let line = match job["tool"].as_str().unwrap_or("") {
        "read_file" => format!("Read {}", s("path")),
        "read_image" => format!("View picture {}", s("path")),
        "list_dir" => format!("List {}", s("path")),
        "write_file" if job["append"] == true => format!("Append to {}", s("path")),
        "write_file" if job["offset"].is_u64() => format!("Write {} from byte {}", s("path"), job["offset"]),
        "write_file" => format!("Write {}", s("path")),
        "edit_file" => format!("Edit {}", s("path")),
        "shell" => format!("Run `{}` in {}", s("command"), s("cwd")),
        "service" => match job["unit"].as_str() {
            Some(unit) => format!("{} the user service {unit}", s("action")),
            None => format!("{} user services", s("action")),
        },
        "package" => {
            let names: Vec<&str> = job["names"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
            format!("{} {} with {}", s("action"), names.join(", "), s("manager"))
        }
        "reload" => format!("reload {}", s("what")),
        "system_info" => "Show system info".to_string(),
        super::spawn::NAME => super::spawn::summary(job),
        other => format!("Run {other}"),
    };
    line.chars().take(200).collect()
}

/// WRITE-CHUNKS: a write_file that adds to a file (append) or writes at an offset.
pub fn writes_in_parts(job: &Value) -> bool {
    job["tool"] == "write_file" && (job["append"] == true || job["offset"].is_u64())
}

/// Runners from 0.6.4 on write in parts; an older one (or one that has not said) would replace the file.
pub fn runner_writes_in_parts(version: Option<&str>) -> bool {
    runner_at_least(version, (0, 6, 4))
}

/// IMAGE-01: runners from 0.6.5 on can read a picture (read_image); an older one does not know the tool.
pub fn runner_reads_images(version: Option<&str>) -> bool {
    runner_at_least(version, (0, 6, 5))
}

fn runner_at_least(version: Option<&str>, min: (u64, u64, u64)) -> bool {
    let Some(v) = version else { return false };
    let n: Vec<u64> = v.trim_start_matches('v').split(['.', '-', '+']).take(3).map(|x| x.parse().unwrap_or(0)).collect();
    (n.first().copied().unwrap_or(0), n.get(1).copied().unwrap_or(0), n.get(2).copied().unwrap_or(0)) >= min
}

/// The grant "Always allow" creates for this job: (target, rights).
pub fn grant_for(job: &Value) -> Option<(String, Vec<&'static str>)> {
    let path = || job["path"].as_str().filter(|p| safe(p)).map(str::to_string);
    match job["tool"].as_str()? {
        "read_file" | "read_image" => Some((parent(&path()?), vec!["read"])),
        "list_dir" => Some((path()?, vec!["read"])),
        "write_file" | "edit_file" => Some((parent(&path()?), vec!["write"])),
        "shell" => Some((job["cwd"].as_str().filter(|p| safe(p))?.to_string(), vec!["shell"])),
        "service" => Some(("system".into(), vec!["services"])),
        "package" => {
            let changes = matches!(job["action"].as_str(), Some("install" | "remove"));
            let root = changes && job["manager"].as_str() != Some("flatpak");
            Some(("system".into(), if root { vec!["packages", "root"] } else { vec!["packages"] }))
        }
        "reload" => Some(("system".into(), vec!["desktop"])),
        _ => None,
    }
}

/// The folder a path is in ("/" for top-level files).
fn parent(path: &str) -> String {
    std::path::Path::new(path).parent().map(|p| p.to_string_lossy().into_owned()).filter(|p| !p.is_empty()).unwrap_or_else(|| "/".into())
}

/// Absolute and without `..` components.
fn safe(path: &str) -> bool {
    path.starts_with('/') && !path.split('/').any(|c| c == "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: &str = "\"\"\"Helpers for people's names.\"\"\"\n\n\ndef full_name(first: str, last: str) -> str:\n    \"\"\"First and last name with one space between them.\"\"\"\n    return f\"{first.strip()} {last.strip()}\"\n";

    #[test]
    fn edits_must_replace_whole_lines() {
        // The case that broke names.py on 2026-10-04: half a line.
        let err = whole_lines(NAMES, "def full_name").unwrap_err();
        assert!(err.contains("part of a line") && err.contains("def full_name(first: str, last: str) -> str:"), "{err}");
        // An empty old text, as the worker first tried.
        assert!(whole_lines(NAMES, "").unwrap_err().contains("empty"));
        // Whole lines, with or without the newline, at the end of the file too.
        assert!(whole_lines(NAMES, "def full_name(first: str, last: str) -> str:").is_ok());
        assert!(whole_lines(NAMES, "def full_name(first: str, last: str) -> str:\n").is_ok());
        assert!(whole_lines(NAMES, "    return f\"{first.strip()} {last.strip()}\"\n").is_ok());
        assert!(whole_lines(NAMES, "def initials").unwrap_err().contains("not found"));
        assert!(whole_lines("a\nb\na\n", "a").unwrap_err().contains("2 times"));
    }

    #[test]
    fn test_schema_has_11_tools() {
        let schema = schema();
        let tools = schema.as_array().unwrap();
        assert_eq!(tools.len(), 11);
    }

    #[test]
    fn test_to_job_edit_file_keeps_only_keys() {
        let input = json!({
            "path": "/home/k/x.conf",
            "old": "foo",
            "new": "bar",
            "extra": "ignored"
        });
        let job = to_job("edit_file", &input).unwrap();
        let obj = job.as_object().unwrap();
        assert_eq!(obj.len(), 4); // tool + path + old + new
        assert!(obj.contains_key("tool"));
        assert!(obj.contains_key("path"));
        assert!(obj.contains_key("old"));
        assert!(obj.contains_key("new"));
        assert!(!obj.contains_key("extra"));
    }

    #[test]
    fn read_image_is_a_read_of_the_folder_and_takes_only_a_path() {
        let job = to_job("read_image", &json!({"path": "/media/pics/a.png", "length": "5"})).unwrap();
        assert_eq!(job, json!({"tool": "read_image", "path": "/media/pics/a.png"}));
        assert!(to_job("read_image", &json!({"path": 5})).is_none());
        assert_eq!(grant_for(&job), Some(("/media/pics".to_string(), vec!["read"])));
        assert_eq!(summary(&job), "View picture /media/pics/a.png");
        assert!(schema().as_array().unwrap().iter().any(|t| t["function"]["name"] == "read_image"));
        assert!(!runner_reads_images(None) && !runner_reads_images(Some("0.6.4")));
        assert!(runner_reads_images(Some("0.6.5")) && runner_reads_images(Some("v0.7.0")));
        assert!(runner_writes_in_parts(Some("0.6.4")) && !runner_writes_in_parts(Some("0.6.3")));
    }

    #[test]
    fn read_file_takes_an_optional_offset_and_length() {
        let job = to_job("read_file", &json!({"path": "/a", "offset": 10, "length": "20"})).unwrap();
        assert_eq!(job, json!({"tool": "read_file", "path": "/a", "offset": 10, "length": 20}));
        // Without them the job is what it was before.
        assert_eq!(to_job("read_file", &json!({"path": "/a"})).unwrap(), json!({"tool": "read_file", "path": "/a"}));
        // Junk is refused rather than guessed at.
        assert!(to_job("read_file", &json!({"path": "/a", "offset": -1})).is_none());
        assert!(to_job("read_file", &json!({"path": "/a", "length": "lots"})).is_none());
        assert!(to_job("read_file", &json!({"path": "/a", "offset": 1.5})).is_none());
        // Other tools still take strings only.
        assert!(to_job("list_dir", &json!({"path": 5})).is_none());
    }

    #[test]
    fn write_file_takes_append_or_an_offset_for_parts() {
        let job = to_job("write_file", &json!({"path": "/a/b.txt", "content": "x", "append": true})).unwrap();
        assert_eq!(job, json!({"tool": "write_file", "path": "/a/b.txt", "content": "x", "append": true}));
        assert_eq!(summary(&job), "Append to /a/b.txt");
        let job = to_job("write_file", &json!({"path": "/a/b.txt", "content": "x", "append": "false", "offset": "12"})).unwrap();
        assert_eq!(job, json!({"tool": "write_file", "path": "/a/b.txt", "content": "x", "append": false, "offset": 12}));
        assert_eq!(summary(&job), "Write /a/b.txt from byte 12");
        assert_eq!(summary(&to_job("write_file", &json!({"path": "/a/b.txt", "content": "x"})).unwrap()), "Write /a/b.txt");
        assert!(to_job("write_file", &json!({"path": "/a", "content": "x", "append": "sometimes"})).is_none());
        // The grant is the same as for a whole file: write on its folder.
        assert_eq!(grant_for(&job), Some(("/a".to_string(), vec!["write"])));
        // Only a runner that knows parts gets them.
        assert!(writes_in_parts(&job) && !writes_in_parts(&json!({"tool": "write_file", "append": false})));
        for (v, ok) in [(None, false), (Some("0.6.3"), false), (Some("0.6.4"), true), (Some("0.7.0"), true), (Some("1.0.0-rc1"), true), (Some("0.5.9"), false)] {
            assert_eq!(runner_writes_in_parts(v), ok, "{v:?}");
        }
    }

    #[test]
    fn test_to_job_unknown_tool() {
        assert!(to_job("rm", &json!({})) .is_none());
    }

    #[test]
    fn test_grant_for_edit_file() {
        let job = json!({
            "tool": "edit_file",
            "path": "/home/k/x.conf",
                "old": "foo",
                "new": "bar"
        });
        let grant = grant_for(&job).unwrap();
        assert_eq!(grant.0, "/home/k");
        assert_eq!(grant.1, vec!["write"]);
    }

    #[test]
    fn test_grant_for_package_install() {
        let job = json!({
            "tool": "package",
            "manager": "paru",
                "action": "install",
                "names": ["htop"]
        });
        let grant = grant_for(&job).unwrap();
        assert_eq!(grant.0, "system");
        assert_eq!(grant.1, vec!["packages", "root"]);
    }

    #[test]
    fn test_grant_for_read_file_relative() {
        let job = json!({
            "tool": "read_file",
            "path": "relative/path.txt"
        });
        assert!(grant_for(&job).is_none());
    }
}
