use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub default: Value,
    pub node: String,
    pub input: String,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Model {
    pub file: String,
    pub licence: String,
    #[serde(default)]
    pub attribution: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Workflow {
    #[serde(default)]
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub graph: String,
    #[serde(default)]
    pub graphs: BTreeMap<String, String>,
    pub outputs: Vec<String>,
    #[serde(default)]
    pub vram_mb: u64,
    /// Machine name -> VRAM in MiB when its graph needs a different amount.
    #[serde(default)]
    pub vram: BTreeMap<String, u64>,
    #[serde(default)]
    pub ram_mb: u64,
    #[serde(default, rename = "param")]
    pub params: Vec<Param>,
    #[serde(default, rename = "model")]
    pub models: Vec<Model>,
    #[serde(default, skip_serializing)]
    pub dir: PathBuf,
}

impl Workflow {
    /// "<file>: <why>" for every model whose licence is refused.
    pub fn licence_problems(&self) -> Vec<String> {
        self.models
            .iter()
            .filter_map(|m| {
                super::licence::check(&m.licence, m.attribution.as_deref())
                    .err()
                    .map(|e| format!("{}: {e}", m.file))
            })
            .collect()
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
                "string" => json!(
                    v.as_str()
                        .ok_or_else(|| format!("{} must be a string", p.name))?
                ),
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
            used.insert(p.name.clone(), value);
        }
        Ok((g, used))
    }
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
    subs.into_iter()
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
            wf().licence_problems(),
            vec!["b.ckpt: CreativeML OpenRAIL-M: OpenRAIL licences are refused".to_string()]
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
}
