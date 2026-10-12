//! M6-11: the game planner. A game described in words becomes code tasks (the KG-02 task format) and asset
//! tasks (a library asset to reuse, or a Studio workflow with parameters). The model's answer is checked against
//! the library and the workflows that exist, and mapped onto a KG-02 `Plan` so the same approve-then-apply path
//! writes it into the task tree.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::planner::{self, Issue, Limits, Milestone, Plan, PlanTask};

/// One asset the game needs: reused from the library (`library_id`) or made by a workflow (`workflow` + `params`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetTask {
    pub key: String,
    pub title: String,
    /// sprite, image, tileset, background, audio, music, sfx, video or model
    pub kind: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub library_id: Option<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub params: Map<String, Value>,
    /// keys of other asset tasks that come first
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// keys of the code tasks that need this asset
    #[serde(default)]
    pub needed_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GamePlan {
    pub title: String,
    pub goal: String,
    pub code_tasks: Vec<PlanTask>,
    pub asset_tasks: Vec<AssetTask>,
}

/// An asset of the project's library, as far as planning needs it.
#[derive(Debug, Clone)]
pub struct LibAsset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
}

/// One parameter of a Studio workflow. `kind` is "number", "text" or "choice".
#[derive(Debug, Clone)]
pub struct WfParam {
    pub name: String,
    pub kind: String,
    pub choices: Vec<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// A Studio workflow as far as planning needs it. `output` is "image", "audio", "video" or "model".
#[derive(Debug, Clone)]
pub struct WfInfo {
    pub name: String,
    pub output: String,
    pub params: Vec<WfParam>,
}

fn issue(kind: &str, key: &str, message: String) -> Issue {
    Issue { kind: kind.into(), key: key.into(), message }
}

/// The JSON schema the model's answer must follow.
pub fn schema() -> Value {
    let task = planner::schema()["properties"]["milestones"]["items"]["properties"]["tasks"]["items"].clone();
    json!({
        "type": "object",
        "title": "Game plan",
        "properties": {
            "goal": { "type": "string" },
            "code_tasks": {
                "type": "array",
                "items": task
            },
            "asset_tasks": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "title": { "type": "string" },
                        "kind": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "library_id": { "type": ["string", "null"] },
                        "workflow": { "type": ["string", "null"] },
                        "params": { "type": "object" },
                        "depends_on": { "type": "array", "items": { "type": "string" } },
                        "needed_by": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["key", "title", "kind"]
                }
            }
        },
        "required": ["title", "goal", "code_tasks", "asset_tasks"]
    })
}

/// A game plan from the model's answer.
pub fn parse(text: &str) -> Result<GamePlan, String> {
    let v = crate::taskrun::parse::json_in(text).ok_or("The answer holds no JSON.")?;
    serde_json::from_value(v).map_err(|e| format!("The answer is not a game plan: {e}"))
}

/// The workflow output an asset kind needs.
pub fn output_of_kind(kind: &str) -> Option<&'static str> {
    let k = kind.to_lowercase();
    match k.as_str() {
        "sprite" | "image" | "tileset" | "background" => Some("image"),
        "audio" | "music" | "sfx" => Some("audio"),
        "video" => Some("video"),
        "model" => Some("model"),
        _ => None,
    }
}

/// What is wrong with one parameter value, or None.
pub fn param_issue(p: &WfParam, v: &Value) -> Option<String> {
    match p.kind.as_str() {
        "number" => {
            if !v.is_number() {
                return Some(format!("Parameter {} must be a number.", p.name));
            }
            let num = v.as_f64().unwrap();
            if let Some(m) = p.min {
                if num < m {
                    return Some(format!("Parameter {} must be between {} and {}.", p.name, p.min.unwrap_or_default(), p.max.unwrap_or_default()));
                }
            }
            if let Some(m) = p.max {
                if num > m {
                    return Some(format!("Parameter {} must be between {} and {}.", p.name, p.min.unwrap_or_default(), p.max.unwrap_or_default()));
                }
            }
            None
        }
        "choice" => {
            let shown = v.as_str().map(String::from).unwrap_or_else(|| v.to_string());
            if v.as_str().is_some_and(|s| p.choices.iter().any(|c| c == s)) {
                return None;
            }
            Some(format!("Parameter {} must be one of {}; \"{}\" is not allowed.", p.name, p.choices.join(", "), shown))
        }
        "text" => {
            if !v.is_string() {
                return Some(format!("Parameter {} must be text.", p.name));
            }
            None
        }
        _ => None,
    }
}

/// Everything wrong with the asset tasks: source, library, workflow, parameters, needed_by.
pub fn asset_issues(plan: &GamePlan, library: &[LibAsset], workflows: &[WfInfo]) -> Vec<Issue> {
    let mut out = Vec::new();
    let task_keys: HashSet<&str> = plan.code_tasks.iter().map(|t| t.key.as_str()).collect();

    for task in &plan.asset_tasks {
        // 0. unknown_need
        for key in &task.needed_by {
            if !task_keys.contains(key.as_str()) {
                out.push(issue("unknown_need", &task.key, format!("Asset \"{}\" is needed by \"{key}\", which is not a code task.", task.key)));
            }
        }

        // 1. Source: exactly one of library_id or workflow
        if task.library_id.is_none() && task.workflow.is_none() {
            out.push(issue("no_source", &task.key, format!("Asset \"{}\" has no source: give a library_id or a workflow.", task.key)));
            continue;
        }
        if task.library_id.is_some() && task.workflow.is_some() {
            out.push(issue("two_sources", &task.key, format!("Asset \"{}\" has both a library_id and a workflow.", task.key)));
            continue;
        }

        // 2. If library_id
        if let Some(lib_id) = &task.library_id {
            let found = library.iter().find(|l| l.id == *lib_id);
            match found {
                None => out.push(issue("unknown_asset", &task.key, format!("Library asset \"{}\" not found.", lib_id))),
                Some(l) => {
                    if l.kind.to_lowercase() != task.kind.to_lowercase() {
                        out.push(issue("wrong_kind", &task.key, format!("Library asset \"{}\" is kind \"{}\", but task expects \"{}\".", lib_id, l.kind, task.kind)));
                    } else if !task.tags.is_empty() && !l.tags.iter().any(|lt| task.tags.iter().any(|tk| lt == tk)) {
                        out.push(issue("missing_tags", &task.key, format!("Library asset \"{}\" does not have the required tags: {:?}", lib_id, task.tags)));
                    }
                }
            }
            continue;
        }

        // 3. If workflow
        if let Some(wf_name) = &task.workflow {
            let wf = workflows.iter().find(|w| w.name == *wf_name);
            if wf.is_none() {
                out.push(issue("unknown_workflow", &task.key, format!("Workflow \"{}\" not found.", wf_name)));
                continue;
            }

            if output_of_kind(&task.kind).is_none() {
                out.push(issue("unknown_kind", &task.key, format!("Kind \"{}\" has no known output type.", task.kind)));
                continue;
            }

            if let Some(wf) = wf {
                if wf.output != output_of_kind(&task.kind).unwrap() {
                    out.push(issue("wrong_output", &task.key, format!("Workflow \"{}\" outputs \"{}\", but task kind \"{}\" expects \"{}\".", wf_name, wf.output, task.kind, output_of_kind(&task.kind).unwrap())));
                    continue;
                }
            }

            // Check params
            for (name, value) in &task.params {
                if wf.as_ref().unwrap().params.iter().all(|p| p.name != *name) {
                    out.push(issue("unknown_param", &task.key, format!("Workflow \"{}\" has no parameter \"{}\".", wf_name, name)));
                    continue;
                }
                if let Some(msg) = param_issue(&wf.as_ref().unwrap().params.iter().find(|p| p.name == *name).unwrap(), value) {
                    out.push(issue("bad_param", &task.key, format!("Parameter {} in workflow \"{}\": {}", name, wf_name, msg)));
                }
            }
        }
    }
    out
}

/// Swaps a generated asset for a matching library asset; returns the keys that were swapped.
pub fn prefer_library(plan: &mut GamePlan, library: &[LibAsset]) -> Vec<String> {
    let mut result = Vec::new();
    for task in &mut plan.asset_tasks {
        if task.workflow.is_none() || task.library_id.is_some() {
            continue;
        }
        let kind_lower = task.kind.to_ascii_lowercase();
        let tags_lower: HashSet<String> = task.tags.iter().map(|s| s.to_lowercase()).collect();
        let mut best_asset: Option<(usize, &LibAsset)> = None;
        for lib in library {
            if lib.kind.to_lowercase() != kind_lower {
                continue;
            }
            let asset_tags_lower: HashSet<String> = lib.tags.iter().map(|s| s.to_lowercase()).collect();
            let shared: usize = tags_lower.intersection(&asset_tags_lower).count();
            if shared == 0 {
                continue;
            }
            match best_asset {
                Some((best_shared, best)) if shared < best_shared || (shared == best_shared && lib.id >= best.id) => {}
                _ => best_asset = Some((shared, lib)),
            }
        }
        if let Some((_, asset)) = best_asset {
            task.library_id = Some(asset.id.clone());
            task.workflow = None;
            task.params.clear();
            result.push(task.key.clone());
        }
    }
    result
}

/// The game plan as a KG-02 plan: an "Assets" milestone, then a "Code" milestone.
pub fn to_plan(plan: &GamePlan) -> Plan {
    let mut milestones = Vec::new();

    // Assets milestone
    if !plan.asset_tasks.is_empty() {
        let mut tasks = Vec::new();
        for asset in &plan.asset_tasks {
            let key = asset.key.clone();
            let title = asset.title.clone();
            let goal = if let Some(lib_id) = &asset.library_id {
                format!("Use library asset {} ({}) in the project.", lib_id, asset.kind)
            } else {
                let workflow = asset.workflow.as_deref().unwrap_or("none");
                let params = serde_json::to_string(&asset.params).unwrap_or_default();
                format!("Make a {} asset with the {} workflow and these parameters: {}", asset.kind, workflow, params)
            };
            let steps = vec![
                "Get the asset (reuse it or run the workflow).".into(),
                "Add it to the project's library.".into(),
            ];
            let done_when = "The asset is in the project's library and passes the asset check.".into();
            let files = Vec::new();
            let lines = 0;
            let depends_on = asset.depends_on.clone();
            tasks.push(PlanTask { key, title, goal, steps, done_when, files, lines, depends_on });
        }
        milestones.push(Milestone {
            title: "Assets".into(),
            goal: "Get or make the assets the game needs.".into(),
            tasks,
        });
    }

    // Code milestone
    if !plan.code_tasks.is_empty() {
        let mut code_tasks = plan.code_tasks.clone();
        // Add asset dependencies
        for code in &mut code_tasks {
            let mut deps = code.depends_on.clone();
            for asset in &plan.asset_tasks {
                if asset.needed_by.contains(&code.key) {
                    deps.push(asset.key.clone());
                }
            }
            deps.sort();
            deps.dedup();
            code.depends_on = deps;
        }
        milestones.push(Milestone {
            title: "Code".into(),
            goal: "Build the game on top of the assets.".into(),
            tasks: code_tasks,
        });
    }

    Plan {
        title: plan.title.clone(),
        goal: plan.goal.clone(),
        milestones,
    }
}

/// All rules: the KG-02 task rules on the mapped plan plus the asset rules.
pub fn all_issues(plan: &GamePlan, lim: &Limits, library: &[LibAsset], workflows: &[WfInfo]) -> Vec<Issue> {
    let mut out = planner::issues(&to_plan(plan), lim);
    out.extend(asset_issues(plan, library, workflows));
    out
}

/// Asks the model for a game plan, up to 3 rounds while the answer breaks a rule.
pub async fn generate<F, Fut>(mut ask: F, system: &str, user: &str, lim: &Limits, library: &[LibAsset], workflows: &[WfInfo]) -> Result<(GamePlan, Vec<Issue>), String>
where
    F: FnMut(Vec<Value>) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let mut messages = vec![json!({ "role": "system", "content": system }), json!({ "role": "user", "content": user })];
    let mut last: Option<(GamePlan, Vec<Issue>)> = None;
    let mut last_err = String::from("The model made no plan.");
    for _ in 0..3 {
        let text = ask(messages.clone()).await?;
        messages.push(json!({ "role": "assistant", "content": text }));
        match parse(&text) {
            Ok(mut plan) => {
                prefer_library(&mut plan, library);
                let found = all_issues(&plan, lim, library, workflows);
                if found.is_empty() {
                    return Ok((plan, found));
                }
                messages.push(json!({ "role": "user", "content": planner::fix_request(&found, lim) }));
                last = Some((plan, found));
            }
            Err(e) => {
                messages.push(json!({ "role": "user", "content": format!("{e} Answer only with the JSON plan.") }));
                last_err = e;
            }
        }
    }
    last.ok_or(last_err)
}

#[cfg(test)]
#[path = "gameplan_tests.rs"]
mod tests;
