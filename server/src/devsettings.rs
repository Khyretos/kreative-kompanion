//! KG-10: project settings (preset, engine, build, test, run, deploy, screenshot folder) shared by every project type.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::scheduler;

const PRESETS: &[&str] = &["software", "game"];
const MAX_COMMAND: usize = 500;
const MAX_SUMMARY: usize = 600;

/// The result of the last build, test or run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LastRun {
    pub ok: bool,
    pub at: String,
    pub summary: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub preset: String,
    pub engine: String,
    pub build: String,
    pub test: String,
    pub run: String,
    pub deploy: String,
    pub screenshots: String,
    pub last: BTreeMap<String, LastRun>,
}

/// Both presets, software first.
pub fn presets() -> Vec<(String, Settings)> {
    PRESETS.iter().filter_map(|n| Settings::preset(n).map(|s| (n.to_string(), s))).collect()
}

/// Puts the project's repo and its test command into the scheduler config where the scheduler has no value of its own.
pub fn apply_to_scheduler(cfg: &mut scheduler::Config, dev: &Settings, machine: Option<&str>, folder: Option<&str>) {
    if let (true, Some(m), Some(f)) = (cfg.machines.is_empty(), machine, folder) {
        cfg.machines.insert(m.to_string(), f.to_string());
    }
    if cfg.check.trim().is_empty()
        && let Some(test) = dev.command("test")
    {
        cfg.check = test.to_string();
    }
}

impl Settings {
    pub fn parse(text: &str) -> Settings {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn preset(name: &str) -> Option<Settings> {
        let base = Settings { preset: name.to_string(), ..Default::default() };
        match name {
            "software" => Some(Settings { build: "make".into(), test: "make test".into(), ..base }),
            "game" => Some(Settings {
                engine: "kk-engine".into(),
                build: "cmake --build build".into(),
                test: "ctest --test-dir build --output-on-failure".into(),
                screenshots: "screenshots".into(),
                ..base
            }),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.preset.is_empty() && !PRESETS.contains(&self.preset.as_str()) {
            return Err("The preset is software or game.".into());
        }
        for (name, v) in [("engine", &self.engine), ("build", &self.build), ("test", &self.test), ("run", &self.run), ("deploy", &self.deploy)] {
            if v.len() > MAX_COMMAND || v.contains(['\n', '\r', '\0']) {
                return Err(format!("The {name} setting is one line of at most {MAX_COMMAND} characters."));
            }
        }
        let shots = &self.screenshots;
        if shots.len() > 200 || shots.starts_with('/') || shots.split('/').any(|c| c == "..") || shots.contains('\0') {
            return Err("The screenshot folder is relative to the repo, without \"..\".".into());
        }
        Ok(())
    }

    /// The trimmed command of `kind` (build, test, run or deploy), None when blank or unknown.
    pub fn command(&self, kind: &str) -> Option<&str> {
        let v = match kind {
            "build" => &self.build,
            "test" => &self.test,
            "run" => &self.run,
            "deploy" => &self.deploy,
            _ => return None,
        };
        Some(v.trim()).filter(|c| !c.is_empty())
    }

    /// What the planner is told about the repo and its commands ("" when nothing is set).
    pub fn brief(&self, repo_folder: Option<&str>) -> String {
        let items = [
            ("Folder", repo_folder.unwrap_or("")),
            ("Engine", self.engine.trim()),
            ("Build", self.command("build").unwrap_or("")),
            ("Test", self.command("test").unwrap_or("")),
            ("Run", self.command("run").unwrap_or("")),
            ("Deploy", self.command("deploy").unwrap_or("")),
            ("Screenshots", self.screenshots.trim()),
        ];
        let lines: Vec<String> = items.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| format!("{k}: {v}")).collect();
        if lines.is_empty() {
            return String::new();
        }
        format!("Repository and commands:\n{}\n\n", lines.join("\n"))
    }

    /// Stores the newest result of a command, keeping only the end of its output.
    pub fn record(&mut self, kind: &str, ok: bool, at: &str, summary: &str) {
        let mut start = summary.len().saturating_sub(MAX_SUMMARY);
        while !summary.is_char_boundary(start) {
            start += 1;
        }
        self.last.insert(kind.to_string(), LastRun { ok, at: at.to_string(), summary: summary[start..].trim().to_string() });
    }
}

#[cfg(test)]
#[path = "devsettings_tests.rs"]
mod tests;
