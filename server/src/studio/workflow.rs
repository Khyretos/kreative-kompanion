use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// STU-01b: another node that gets the same value; ints and seeds get `add` added.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Also {
    pub node: String,
    pub input: String,
    #[serde(default)]
    pub add: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub default: Value,
    #[serde(default)]
    pub node: String,
    #[serde(default)]
    pub input: String,
    /// STU-01c: the allowed values of a "choice" param
    #[serde(default)]
    pub choices: Vec<String>,
    /// STU-01c: choices only users with the adult-content right may use
    #[serde(default)]
    pub adult: Vec<String>,
    /// STU-01c: placeholder name -> (choice -> text); "{placeholder}" in a string template becomes the text for the chosen value, "" when the choice has none
    #[serde(default)]
    pub words: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// STU-01: a string's text goes into the graph through this template ("{value}" = the text).
    #[serde(default)]
    pub template: Option<String>,
    /// STU-01b: more nodes that get this value (TOML: also = [{ node = "9", input = "seed", add = 1 }]).
    #[serde(default)]
    pub also: Vec<Also>,
    /// STU-R2: an empty text uses this other param's text instead (the OC sheet's face prompt falls back to the prompt).
    #[serde(default)]
    pub fallback: Option<String>,
}

/// STU-01: how a workflow shows in the Studio (an image type such as "Character").
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StudioMeta {
    pub label: String,
    /// One line for the type card.
    #[serde(default)]
    pub hint: String,
    /// "square", "wide", "tall"; the first is the default.
    #[serde(default)]
    pub sizes: Vec<String>,
    #[serde(default)]
    pub order: i64,
    /// STU-02: pixels per size when they differ from the image sizes (video: wide = [1280, 704]).
    #[serde(default)]
    pub px: BTreeMap<String, [i64; 2]>,
    /// STU-R2: a refine step works on a picture the user sends ([inputs] source); type pickers leave it out.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub refine: bool,
}

/// STU-01: a preset changes a base workflow's parameter: its template and/or default.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ParamSet {
    pub name: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Model {
    pub file: String,
    pub licence: String,
    #[serde(default)]
    pub attribution: Option<String>,
}

/// STU-01d: an optional face photo (IP-Adapter plus-face, as in Kreative Studio) switches to this
/// graph; `image` is its LoadImage node, `weight` its IPAdapterAdvanced node.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Face {
    pub graph: String,
    pub image: String,
    pub weight: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workflow {
    #[serde(default)]
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// STU-01: a preset names its base workflow and takes its graphs, models and parameters.
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub studio: Option<StudioMeta>,
    #[serde(default, rename = "set")]
    pub sets: Vec<ParamSet>,
    #[serde(default)]
    pub graph: String,
    #[serde(default)]
    pub graphs: BTreeMap<String, String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub vram_mb: u64,
    /// Machine name -> VRAM in MiB when its graph needs a different amount.
    #[serde(default)]
    pub vram: BTreeMap<String, u64>,
    /// Machine name -> host RAM in MiB when it differs from `ram_mb`.
    #[serde(default)]
    pub ram: BTreeMap<String, u64>,
    #[serde(default)]
    pub ram_mb: u64,
    /// STU-02: the machines it may run on (empty: every machine with a ComfyUI).
    #[serde(default)]
    pub machines: Vec<String>,
    #[serde(default, rename = "param")]
    pub params: Vec<Param>,
    #[serde(default, rename = "model")]
    pub models: Vec<Model>,
    #[serde(default)]
    pub face: Option<Face>,
    /// STU-C1: images shipped in the workflow's folder (LoadImage node -> file name), uploaded to
    /// ComfyUI's input before each run, e.g. the OC sheet's pose and depth templates.
    #[serde(default)]
    pub images: BTreeMap<String, String>,
    /// STU-R2: pictures the user sends with a run (form field -> LoadImage node), all required, e.g.
    /// source = "10" (the picture a refine step works on) and mask = "11" (the painted area).
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    #[serde(default, skip_serializing)]
    pub dir: PathBuf,
}

impl Workflow {
    /// "<file>: <why>" for every model whose licence gets a warning (LIC-01: never refused).
    pub fn licence_warnings(&self) -> Vec<String> {
        self.models
            .iter()
            .filter_map(|m| {
                super::licence::check(&m.licence, m.attribution.as_deref())
                    .err()
                    .map(|e| format!("{}: {e}", m.file))
            })
            .collect()
    }

    /// Host RAM in MiB a run needs on this machine ([ram], else `ram_mb`).
    pub fn ram_for(&self, machine: &str) -> u64 {
        self.ram.get(machine).copied().unwrap_or(self.ram_mb)
    }

    /// VRAM in MiB the graph needs on this machine ([vram], else `vram_mb`).
    pub fn vram_for(&self, machine: &str) -> u64 {
        self.vram.get(machine).copied().unwrap_or(self.vram_mb)
    }

    /// The graph for this machine (its own file in [graphs], else `graph`).
    pub fn graph_for(&self, machine: &str) -> Result<Value, String> {
        let file = self.graphs.get(machine).unwrap_or(&self.graph);
        let content =
            std::fs::read_to_string(self.dir.join(file)).map_err(|e| format!("{file}: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("{file}: {e}"))
    }

    pub fn face_graph(&self, image: &str, weight: f64) -> Result<Value, String> {
        let face = self.face.as_ref().ok_or_else(|| format!("{} takes no face photo", self.title))?;
        if !(0.0..=1.2).contains(&weight) {
            return Err("face weight must be between 0 and 1.2".to_string());
        }
        let content = std::fs::read_to_string(self.dir.join(&face.graph))
            .map_err(|e| format!("{}: {e}", face.graph))?;
        let mut g = serde_json::from_str::<Value>(&content)
            .map_err(|e| format!("{}: {e}", face.graph))?;
        let node_img = g.get_mut(&face.image)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("face: node {} not in the graph", face.image))?;
        let inputs_img = node_img.entry("inputs").or_insert_with(|| json!({}));
        if let Some(inputs_img) = inputs_img.as_object_mut() {
            inputs_img.insert("image".into(), json!(image));
        }
        let node_wt = g.get_mut(&face.weight)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("face: node {} not in the graph", face.weight))?;
        let inputs_wt = node_wt.entry("inputs").or_insert_with(|| json!({}));
        if let Some(inputs_wt) = inputs_wt.as_object_mut() {
            inputs_wt.insert("weight".into(), json!(weight));
        }
        Ok(g)
    }

    /// STU-01c: the first chosen value (given, else the default) of a "choice" param in that param's
    /// `adult` list; None when nothing needs the adult-content right.
    pub fn adult_choice(&self, given: &Map<String, Value>) -> Option<String> {
        self.params.iter().find_map(|p| {
            if p.kind != "choice" {
                return None;
            }
            let v = given.get(&p.name).unwrap_or(&p.default);
            let c = v.as_str()?;
            if p.adult.iter().any(|a| a == c) {
                Some(c.to_string())
            } else {
                None
            }
        })
    }

    /// The graph with every parameter set (given, else its default) and the values used.
    pub fn fill(
        &self,
        graph: &Value,
        given: &Map<String, Value>,
    ) -> Result<(Value, Map<String, Value>), String> {
        if let Some(k) = given
            .keys()
            .find(|k| !self.params.iter().any(|p| &p.name == *k))
        {
            return Err(format!("unknown parameter {k}"));
        }
        let mut g = graph.clone();
        let mut used = Map::new();
        // STU-01c: pre-pass over "choice" params to validate and build subs
        let mut subs: Vec<(String, String)> = Vec::new();
        for p in &self.params {
            if p.kind == "choice" {
                let v = given.get(&p.name).unwrap_or(&p.default);
                let c = v.as_str().ok_or_else(|| format!("{} must be a string", p.name))?;
                if !p.choices.contains(&c.to_string()) {
                    return Err(format!("{} must be one of {}", p.name, p.choices.join(", ")));
                }
                subs.push((format!("{{{}}}", p.name), c.to_string()));
                for (slot, by) in &p.words {
                    subs.push((format!("{{{slot}}}"), by.get(c).cloned().unwrap_or_default()));
                }
            }
        }
        for p in &self.params {
            let v = given.get(&p.name).unwrap_or(&p.default);
            let in_range = |x: f64| {
                let (lo, hi) = (p.min.unwrap_or(f64::MIN), p.max.unwrap_or(f64::MAX));
                if x < lo || x > hi {
                    Err(format!("{} must be between {lo} and {hi}", p.name))
                } else {
                    Ok(())
                }
            };
            let value = match p.kind.as_str() {
                "int" => {
                    let n = v
                        .as_i64()
                        .ok_or_else(|| format!("{} must be an integer", p.name))?;
                    in_range(n as f64)?;
                    json!(n)
                }
                "float" => {
                    let n = v
                        .as_f64()
                        .ok_or_else(|| format!("{} must be a number", p.name))?;
                    in_range(n)?;
                    json!(n)
                }
                "string" => {
                    let mut text = v.as_str().ok_or_else(|| format!("{} must be a string", p.name))?;
                    if text.trim().is_empty()
                        && let Some(other) = p.fallback.as_ref().and_then(|f| given.get(f)).and_then(Value::as_str)
                    {
                        text = other;
                    }
                    // STU-01: the graph gets the templated text; `used` keeps what the user typed.
                    if let Some(t) = &p.template {
                        // STU-01c: the choice placeholders first ({rating}, {rating_neg}, ...).
                        let t = subs.iter().fold(t.clone(), |acc, (k, v)| acc.replace(k.as_str(), v));
                        let node = g
                            .get_mut(&p.node)
                            .and_then(Value::as_object_mut)
                            .ok_or_else(|| format!("{}: node {} not in the graph", p.name, p.node))?;
                        let inputs = node.entry("inputs").or_insert_with(|| json!({}));
                        if let Some(inputs) = inputs.as_object_mut() {
                            // STU-01b: an empty text drops its slot (no trailing ", ").
                            let filled = if text.trim().is_empty() {
                                t.replace(", {value}", "").replace("{value}", "")
                            } else {
                                t.replace("{value}", text)
                            };
                            inputs.insert(p.input.clone(), json!(filled));
                        }
                        used.insert(p.name.clone(), v.clone()); // what the user typed (STU-R2: not the fallback)
                        continue;
                    }
                    json!(text)
                }
                "choice" => {
                    let c = v.as_str().unwrap_or_default();
                    if p.node.is_empty() {
                        used.insert(p.name.clone(), json!(c));
                        continue;
                    }
                    json!(c)
                }
                "seed" => {
                    let n = v
                        .as_i64()
                        .ok_or_else(|| format!("{} must be an integer", p.name))?;
                    json!(if n < 0 {
                        rand::random::<u32>() as i64
                    } else {
                        n
                    })
                }
                _ => return Err(format!("{}: unknown type {}", p.name, p.kind)),
            };
            let node = g
                .get_mut(&p.node)
                .and_then(Value::as_object_mut)
                .ok_or_else(|| format!("{}: node {} not in the graph", p.name, p.node))?;
            let inputs = node.entry("inputs").or_insert_with(|| json!({}));
            if let Some(inputs) = inputs.as_object_mut() {
                inputs.insert(p.input.clone(), value.clone());
            }
            for a in &p.also {
                let v = match value.as_i64() {
                    Some(n) => json!(n + a.add),
                    None => value.clone(),
                };
                if let Some(node) = g.get_mut(&a.node).and_then(Value::as_object_mut) {
                    let inputs = node.entry("inputs").or_insert_with(|| json!({}));
                    if let Some(inputs) = inputs.as_object_mut() {
                        inputs.insert(a.input.clone(), v);
                    }
                } else {
                    return Err(format!("{}: node {} not in the graph", p.name, a.node));
                }
            }
            used.insert(p.name.clone(), value);
        }
        Ok((g, used))
    }
}

pub fn inherit(child: Workflow, base: &Workflow) -> Result<Workflow, String> {
    // Rule 1: Check if base is a preset itself
    if base.base.is_some() {
        return Err(format!("the base {} is a preset itself", base.name));
    }

    // Rule 2: Check if child has its own graph (presets must inherit graph)
    if !child.graph.is_empty() {
        return Err("a preset takes its graph from its base".to_string());
    }

    // Rule 3: Start from base and apply child overrides
    let mut w = base.clone();
    w.name = child.name;
    w.title = child.title;
    w.base = child.base;
    w.studio = child.studio;
    w.sets = child.sets;
    
    // description from child when not empty
    if !child.description.is_empty() {
        w.description = child.description;
    }
    
    // models from child when not empty
    if !child.models.is_empty() {
        w.models = child.models;
    }
    
    // vram_mb and ram_mb from child when not 0
    if child.vram_mb != 0 {
        w.vram_mb = child.vram_mb;
    }
    if child.ram_mb != 0 {
        w.ram_mb = child.ram_mb;
    }

    // Rule 4: Apply parameter sets from child
    for s in &w.sets {
        // Find the parameter in w.params with matching name
        let param_idx = w.params.iter().position(|p| p.name == s.name);
        
        match param_idx {
            Some(idx) => {
                let mut p = w.params[idx].clone();
                
                // If template is set, update it
                if let Some(template) = &s.template {
                    p.template = Some(template.clone());
                }
                
                // If default is set, update it
                if let Some(default_val) = &s.default {
                    p.default = default_val.clone();
                }
                
                w.params[idx] = p;
            }
            None => {
                return Err(format!("unknown parameter {}", s.name));
            }
        }
    }

    Ok(w)
}

/// Every subfolder of `dir` with its parsed workflow.toml, sorted by name.
pub fn load_all(dir: &Path) -> Vec<(String, Result<Workflow, String>)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut subs: Vec<PathBuf> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subs.sort();
    let all: Vec<(String, Result<Workflow, String>)> = subs.into_iter()
        .map(|sub| {
            let name = sub
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let wf = std::fs::read_to_string(sub.join("workflow.toml"))
                .map_err(|e| format!("workflow.toml: {e}"))
                .and_then(|c| {
                    toml::from_str::<Workflow>(&c).map_err(|e| format!("workflow.toml: {e}"))
                })
                .map(|mut w| {
                    w.name = name.clone();
                    w.dir = sub.clone();
                    w
                });
            (name, wf)
        })
        .collect();
    // STU-01: presets take the rest from their base.
    all.iter()
        .map(|(name, wf)| {
            let resolved = match wf {
                Ok(w) if w.base.is_some() => {
                    let b = w.base.clone().unwrap_or_default();
                    match all.iter().find(|(n, _)| *n == b) {
                        Some((_, Ok(base))) => inherit(w.clone(), base),
                        Some((_, Err(e))) => Err(format!("base {b}: {e}")),
                        None => Err(format!("base {b} not found")),
                    }
                }
                other => other.clone(),
            };
            (name.clone(), resolved)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML: &str = r#"
title = "T"
graph = "graph.json"
outputs = ["9"]
[[param]]
name = "steps"
type = "int"
default = 8
min = 1
max = 50
node = "3"
input = "steps"
[[param]]
name = "seed"
type = "seed"
default = -1
node = "3"
input = "seed"
[[param]]
name = "prompt"
type = "string"
default = "fox"
node = "6"
input = "text"
[[model]]
file = "a.safetensors"
licence = "Apache-2.0"
[[model]]
file = "b.ckpt"
licence = "CreativeML OpenRAIL-M"
"#;

    fn wf() -> Workflow {
        toml::from_str(TOML).unwrap()
    }

    fn graph() -> Value {
        json!({"3": {"class_type": "KSampler", "inputs": {"steps": 4, "seed": 1}}, "6": {"class_type": "CLIPTextEncode", "inputs": {"text": ""}}})
    }

    fn m(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn defaults() {
        let (g, p) = wf().fill(&graph(), &Map::new()).unwrap();
        assert_eq!(g["3"]["inputs"]["steps"], json!(8));
        assert_eq!(g["6"]["inputs"]["text"], json!("fox"));
        assert!(p["seed"].as_i64().unwrap() >= 0);
        assert_eq!(g["3"]["inputs"]["seed"], p["seed"]);
    }

    #[test]
    fn given() {
        let (g, p) = wf()
            .fill(
                &graph(),
                &m(json!({"steps": 20, "seed": 7, "prompt": "cat"})),
            )
            .unwrap();
        assert_eq!(g["3"]["inputs"]["steps"], json!(20));
        assert_eq!(g["3"]["inputs"]["seed"], json!(7));
        assert_eq!(g["6"]["inputs"]["text"], json!("cat"));
        assert_eq!(p["seed"], json!(7));
    }

    #[test]
    fn errors() {
        assert_eq!(
            wf().fill(&graph(), &m(json!({"steps": 0}))).unwrap_err(),
            "steps must be between 1 and 50"
        );
        assert_eq!(
            wf().fill(&graph(), &m(json!({"steps": "x"}))).unwrap_err(),
            "steps must be an integer"
        );
        assert_eq!(
            wf().fill(&graph(), &m(json!({"foo": 1}))).unwrap_err(),
            "unknown parameter foo"
        );
        assert_eq!(
            wf().fill(&json!({"3": {"inputs": {}}}), &Map::new())
                .unwrap_err(),
            "prompt: node 6 not in the graph"
        );
    }

    #[test]
    fn licences() {
        assert_eq!(
            wf().licence_warnings(),
            vec!["b.ckpt: CreativeML OpenRAIL-M: OpenRAIL licence, check its use restrictions".to_string()]
        );
    }

    #[test]
    fn load() {
        let d = std::env::temp_dir().join(format!("m605-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(d.join("z")).unwrap();
        std::fs::write(
            d.join("z/workflow.toml"),
            format!("{TOML}\n[graphs]\nsouc = \"g2.json\"\n"),
        )
        .unwrap();
        std::fs::write(d.join("z/graph.json"), "{\"k\": 1}").unwrap();
        std::fs::write(d.join("z/g2.json"), "{\"k\": 2}").unwrap();
        let all = load_all(&d);
        assert_eq!(all.len(), 1);
        let w = all[0].1.as_ref().unwrap();
        assert_eq!(w.name, "z");
        assert_eq!(w.graph_for("other").unwrap(), json!({"k": 1}));
        assert_eq!(w.graph_for("souc").unwrap(), json!({"k": 2}));
        assert!(load_all(&d.join("missing")).is_empty());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn per_machine_vram_and_ram() {
        let w: Workflow = toml::from_str(&format!(
            "vram_mb = 11000\nram_mb = 8000\n{TOML}\n[vram]\nsouc = 7000\n[ram]\nsouc = 6000\n"
        ))
        .unwrap();
        assert_eq!((w.vram_for("souc"), w.ram_for("souc")), (7000, 6000));
        assert_eq!((w.vram_for("other"), w.ram_for("other")), (11000, 8000));
    }

    fn base() -> Workflow {
        let mut b: Workflow = toml::from_str(TOML).unwrap();
        b.name = "z".into();
        b.dir = PathBuf::from("/wf/z");
        b
    }

    #[test]
    fn preset_takes_the_base() {
        let child: Workflow = toml::from_str(
            "title = \"Character\"\nbase = \"z\"\n[studio]\nlabel = \"Character\"\nsizes = [\"tall\", \"square\"]\n[[set]]\nname = \"prompt\"\ntemplate = \"{value}, full body\"\n",
        )
        .unwrap();
        let w = inherit(Workflow { name: "character".into(), ..child }, &base()).unwrap();
        assert_eq!(w.name, "character");
        assert_eq!(w.title, "Character");
        assert_eq!(w.graph, base().graph);
        assert_eq!(w.outputs, base().outputs);
        assert_eq!(w.dir, PathBuf::from("/wf/z"));
        assert_eq!(w.models.len(), base().models.len());
        assert_eq!(w.studio.as_ref().unwrap().sizes, ["tall", "square"]);
        let p = w.params.iter().find(|p| p.name == "prompt").unwrap();
        assert_eq!(p.template.as_deref(), Some("{value}, full body"));
        assert_eq!(p.node, base().params.iter().find(|p| p.name == "prompt").unwrap().node);
    }

    #[test]
    fn preset_errors() {
        let mut child: Workflow = toml::from_str("title = \"X\"\nbase = \"z\"\n[[set]]\nname = \"nope\"\ndefault = 1\n").unwrap();
        assert_eq!(inherit(child.clone(), &base()).unwrap_err(), "unknown parameter nope");
        child.sets.clear();
        child.graph = "own.json".into();
        assert_eq!(inherit(child, &base()).unwrap_err(), "a preset takes its graph from its base");
        let mut b2 = base();
        b2.base = Some("other".into());
        let c2: Workflow = toml::from_str("title = \"Y\"\nbase = \"z\"\n").unwrap();
        assert_eq!(inherit(c2, &b2).unwrap_err(), "the base z is a preset itself");
    }

    #[test]
    fn template_fills_the_graph() {
        let mut w = wf();
        w.params.iter_mut().find(|p| p.kind == "string").unwrap().template = Some("{value}, wide landscape".into());
        let name = w.params.iter().find(|p| p.kind == "string").unwrap().name.clone();
        let mut given = Map::new();
        given.insert(name.clone(), json!("mountains"));
        let (g, used) = w.fill(&graph(), &given).unwrap();
        assert_eq!(used[&name], json!("mountains"));
        assert!(g.to_string().contains("mountains, wide landscape"));
    }


    #[test]
    fn also_gets_the_value_plus_add() {
        let mut w = wf();
        let seed = w.params.iter_mut().find(|p| p.kind == "seed").unwrap();
        seed.also = vec![Also { node: "6".into(), input: "seed".into(), add: 2 }];
        let (g, used) = w.fill(&graph(), &m(json!({"seed": 7}))).unwrap();
        assert_eq!(g["3"]["inputs"]["seed"], json!(7));
        assert_eq!(g["6"]["inputs"]["seed"], json!(9));
        assert_eq!(used["seed"], json!(7));
        seed_missing(&mut w);
    }

    fn seed_missing(w: &mut Workflow) {
        w.params.iter_mut().find(|p| p.kind == "seed").unwrap().also[0].node = "99".into();
        assert_eq!(w.fill(&graph(), &Map::new()).unwrap_err(), "seed: node 99 not in the graph");
    }

    #[test]
    fn empty_value_drops_the_template_slot() {
        let mut w = wf();
        w.params.iter_mut().find(|p| p.kind == "string").unwrap().template = Some("safe, {value}".into());
        let name = w.params.iter().find(|p| p.kind == "string").unwrap().name.clone();
        let (g, _) = w.fill(&graph(), &m(json!({ name.clone(): "" }))).unwrap();
        assert_eq!(g["6"]["inputs"]["text"], json!("safe"));
        let (g, _) = w.fill(&graph(), &m(json!({ name: "cat" }))).unwrap();
        assert_eq!(g["6"]["inputs"]["text"], json!("safe, cat"));
    }

    #[test]
    fn repo_workflows_all_load() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../studio/workflows");
        let all = load_all(&dir);
        for (name, w) in &all {
            let _w = w.as_ref().unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        let types: Vec<&str> = all.iter().filter_map(|(_, w)| w.as_ref().ok()?.studio.as_ref().map(|s| s.label.as_str())).collect();
        for t in ["Character", "Scene", "Landscape", "Sprite", "Icon"] {
            assert!(types.contains(&t), "missing image type {t}");
        }
    }

    #[test]
    fn face_graph_sets_the_photo_and_weight() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../studio/workflows");
        let all = load_all(&dir);
        let get = |n: &str| all.iter().find(|(name, _)| name == n).unwrap().1.as_ref().unwrap();

        for n in ["vn-portrait", "oc-sheet"] {
            let g = get(n).face_graph("kompanion/abc.png", 0.7).unwrap();
            assert_eq!(g["30"]["inputs"]["image"], json!("kompanion/abc.png"), "{n}");
            assert_eq!(g["34"]["inputs"]["weight"], json!(0.7), "{n}");
            assert_eq!(g["7"]["inputs"]["model"], json!(["34", 0]), "{n}");
            assert!(get(n).face_graph("x.png", 1.5).unwrap_err().contains("between 0 and 1.2"), "{n}");
        }

        assert_eq!(get("vn-portrait").face_graph("x.png", 0.85).unwrap()["34"]["inputs"]["model"], json!(["1", 0]));
        assert!(get("character").face_graph("x.png", 0.8).unwrap_err().contains("takes no face photo"));
    }

    #[test]
    fn oc_sheet_ships_its_pose_images() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../studio/workflows");
        let all = load_all(&dir);
        let get = |n: &str| all.iter().find(|(name, _)| name == n).unwrap().1.as_ref().unwrap();
        let w = get("oc-sheet");
        assert_eq!(w.images.keys().cloned().collect::<Vec<_>>(), vec!["60".to_string(), "64".to_string()]);
        for (node, file) in &w.images {
            assert!(w.dir.join(file).is_file(), "{file} missing");
            for g in [w.graph_for("rx9070").unwrap(), w.face_graph("x.png", 0.7).unwrap()] {
                assert_eq!(g[node.as_str()]["class_type"], json!("LoadImage"), "node {node}");
            }
        }
        assert!(get("oc-sheet-classic").images.is_empty());
        assert_eq!(get("oc-sheet-classic").title, "OC sheet (classic)");
    }

    fn rated() -> Workflow {
        let mut w = wf();
        let s = w.params.iter_mut().find(|p| p.kind == "string").unwrap();
        s.template = Some("best, {rating}, {rating_neg}{value}".into());
        let r = toml::from_str::<Param>(r#"name = "rating"
    type = "choice"
    default = "general"
    choices = ["general", "sensitive", "questionable", "explicit"]
    adult = ["questionable", "explicit"]
    words = { rating_neg = { general = "nsfw, " } }"#).unwrap();
        w.params.push(r);
        w
    }

    #[test]
    fn choice_fills_the_placeholders() {
        let w = rated();
        let (g, used) = w.fill(&graph(), &Map::new()).unwrap();
        assert_eq!(g["6"]["inputs"]["text"], json!("best, general, nsfw, fox"));
        assert_eq!(used["rating"], json!("general"));
        let (g, _) = w.fill(&graph(), &m(json!({"rating": "explicit", "prompt": "cat"}))).unwrap();
        assert_eq!(g["6"]["inputs"]["text"], json!("best, explicit, cat"));
        assert_eq!(w.fill(&graph(), &m(json!({"rating": "bad"}))).unwrap_err(), "rating must be one of general, sensitive, questionable, explicit");
    }

    #[test]
    fn adult_choice_names_the_adult_rating() {
        let w = rated();
        assert_eq!(w.adult_choice(&m(json!({"rating": "explicit"}))), Some("explicit".to_string()));
        assert_eq!(w.adult_choice(&m(json!({"rating": "sensitive"}))), None);
        assert_eq!(w.adult_choice(&Map::new()), None);
    }

    #[test]
    fn oc_sheet_face_prompt_and_background() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../studio/workflows");
        let all = load_all(&dir);
        let w = all.iter().find(|(n, _)| n == "oc-sheet").unwrap().1.as_ref().unwrap();
        let g = w.graph_for("rx9070").unwrap();
        assert_eq!(g["13"]["inputs"]["positive"], json!(["16", 0]), "face fix uses the face prompt");
        assert_eq!(w.face_graph("x.png", 0.7).unwrap()["13"]["inputs"]["positive"], json!(["16", 0]));
        let given = |v: Value| v.as_object().unwrap().clone();
        // empty face prompt: the main prompt; white background by default
        let (f, used) = w.fill(&g, &given(json!({"prompt": "android gentleman, moustache", "face_prompt": ""}))).unwrap();
        let text = |n: &str| f[n]["inputs"]["text"].as_str().unwrap().to_string();
        assert!(text("16").ends_with("detailed face, android gentleman, moustache"), "{}", text("16"));
        assert!(text("4").contains("standing, simple background, white background, android"), "{}", text("4"));
        assert!(text("5").contains("gradient background, starry sky, frame"), "{}", text("5"));
        assert_eq!(used["face_prompt"], json!(""));
        // own face prompt, starry sky
        let (f, _) = w.fill(&g, &given(json!({"prompt": "android gentleman", "face_prompt": "1boy, moustache", "background": "starry"}))).unwrap();
        let text = |n: &str| f[n]["inputs"]["text"].as_str().unwrap().to_string();
        assert!(text("16").ends_with("detailed face, 1boy, moustache") && !text("16").contains("android"), "{}", text("16"));
        assert!(text("4").contains("purple starry night sky background") && !text("4").contains("white background"), "{}", text("4"));
        assert!(text("5").contains("white background, simple background, frame") && !text("5").contains("starry"), "{}", text("5"));
        assert!(w.fill(&g, &given(json!({"prompt": "x", "background": "beach"}))).unwrap_err().contains("background must be one of white, starry"));
    }

    #[test]
    fn refine_workflows_take_their_pictures() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../studio/workflows");
        let all = load_all(&dir);
        let names = ["refine-4k", "refine-change", "refine-repaint", "refine-chibi", "refine-parts", "oc-shots"];
        for n in names {
            let w = all.iter().find(|(name, _)| name == n).unwrap().1.as_ref().unwrap_or_else(|e| panic!("{n}: {e}"));
            assert!(w.studio.as_ref().unwrap().refine, "{n} is a refine step");
            assert_eq!(w.inputs.get("source").map(String::as_str), Some("10"), "{n}");
            let g = w.graph_for("rx9070").unwrap();
            assert_eq!(g["10"]["class_type"], json!("LoadImage"), "{n}");
            let (f, _) = w.fill(&g, &Map::new()).unwrap_or_else(|e| panic!("{n}: {e}"));
            for o in &w.outputs {
                assert_eq!(f[o.as_str()]["class_type"], json!("SaveImage"), "{n} output {o}");
            }
        }
        let get = |n: &str| all.iter().find(|(name, _)| name == n).unwrap().1.as_ref().unwrap();
        assert_eq!(get("refine-repaint").inputs.get("mask").map(String::as_str), Some("11"));
        assert_eq!(get("refine-repaint").graph_for("x").unwrap()["11"]["class_type"], json!("LoadImageMask"));
        assert_eq!(get("refine-repaint").face_graph("f.png", 0.8).unwrap()["14"]["inputs"]["model"], json!(["34", 0]));
        assert_eq!(get("refine-parts").outputs.len(), 3);
        let chibi = get("refine-chibi").params.iter().find(|p| p.name == "rating").unwrap();
        assert!(chibi.adult.is_empty() && !chibi.choices.contains(&"explicit".to_string()), "no adult ratings for chibi");
        // other types are not refine steps
        assert!(!get("oc-sheet").studio.as_ref().unwrap().refine && get("oc-sheet").inputs.is_empty());
    }

    #[test]
    fn oc_shots_change_only_the_shot() {
        // STU-C2: a sheet's prompt and seed, the shot tags and size from the shot, the face prompt for the face fix.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../studio/workflows");
        let all = load_all(&dir);
        let w = all.iter().find(|(name, _)| name == "oc-shots").unwrap().1.as_ref().unwrap();
        let st = w.studio.as_ref().unwrap();
        assert_eq!(st.sizes, vec!["tall".to_string(), "square".to_string()]);
        assert_eq!(st.px.get("tall"), Some(&[1024, 1280]));
        let g = w.graph_for("rx9070").unwrap();
        let given = |v: Value| v.as_object().unwrap().clone();
        let (f, used) = w.fill(&g, &given(json!({"prompt": "android gentleman", "face_prompt": "1boy, moustache", "seed": 300}))).unwrap();
        let text = |f: &Value, n: &str| f[n]["inputs"]["text"].as_str().unwrap().to_string();
        assert!(text(&f, "4").contains("portrait, close-up, head and shoulders") && text(&f, "4").ends_with("android gentleman"), "{}", text(&f, "4"));
        assert!(text(&f, "5").contains("full body, feet, legs, "), "{}", text(&f, "5"));
        assert!(text(&f, "16").ends_with("detailed face, 1boy, moustache"), "{}", text(&f, "16"));
        assert_eq!(f["42"]["inputs"]["positive"], json!(["16", 0]), "the face fix uses the face prompt");
        assert_eq!((f["7"]["inputs"]["seed"].clone(), f["42"]["inputs"]["seed"].clone()), (json!(300), json!(302)));
        assert_eq!(used["shot"], json!("headshot"));
        let (c, _) = w.fill(&g, &given(json!({"prompt": "knight", "shot": "cowboy", "width": 1216, "height": 1216}))).unwrap();
        assert!(text(&c, "4").contains("cowboy shot, from the thighs up") && !text(&c, "4").contains("close-up"), "{}", text(&c, "4"));
        assert!(text(&c, "16").ends_with("detailed face, knight"), "empty face prompt falls back: {}", text(&c, "16"));
        assert_eq!((c["6"]["inputs"]["width"].clone(), c["6"]["inputs"]["height"].clone()), (json!(1216), json!(1216)));
        assert!(w.fill(&g, &given(json!({"shot": "full"}))).unwrap_err().contains("shot must be one of headshot, cowboy"));
    }
}
