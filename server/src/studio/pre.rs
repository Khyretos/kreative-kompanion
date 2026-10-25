//! STU-SHEET3D: a workflow's `[pre]` stage. A first graph runs on another studio app's ComfyUI on the same computer
//! (MV-Adapter: four views of a character from one picture), its pictures are copied into the main ComfyUI's input
//! and fed to the main graph. Two ComfyUIs on the RX 9070 XT at once filled the card and killed the desktop
//! (2026-10-19), so the main ComfyUI is stopped for the stage and started again after it, through the runner.
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{comfy, workflow};
use crate::AppState;

/// The app's ComfyUI address: the main ComfyUI's host with the app's port ("http://h:8188" -> "http://h:8192").
pub fn app_url(main: &str, port: u16) -> Result<String> {
    let (scheme, rest) = main.split_once("://").unwrap_or(("http", main));
    let host = rest.split('/').next().unwrap_or(rest);
    let host = match host.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => h,
        _ => host,
    };
    if host.is_empty() {
        bail!("no host in the ComfyUI address {main}");
    }
    Ok(format!("{scheme}://{host}:{port}"))
}

/// The pre graph's LoadImage nodes the run's pictures go to: those of `pics` whose node is in the pre graph.
pub fn split_pics(pre_graph: &Value, pics: &[(String, PathBuf)]) -> (Vec<(String, PathBuf)>, Vec<(String, PathBuf)>) {
    pics.iter().cloned().partition(|(node, _)| pre_graph.get(node).is_some())
}

/// Run the stage and return its pictures, already uploaded to the main ComfyUI and set on the main graph's feeds.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    s: &AppState,
    p: &workflow::Pre,
    mut pre_graph: Value,
    machine: &str,
    main_url: &str,
    graph: &mut Value,
    pics: &[(String, PathBuf)],
    out_dir: &Path,
    id: &str,
) -> Result<Vec<PathBuf>> {
    let url = app_url(main_url, p.port)?;
    if !crate::gpus::gaming::app_action(s, machine, "stop", "comfyui").await {
        bail!("can't stop the studio's ComfyUI on {machine} for the {} stage (no runner with the gpu grant)", p.app);
    }
    let _ = crate::gpus::gaming::app_action(s, machine, "start", &p.app).await;
    let result = async {
        for (node, path) in pics {
            let bytes = tokio::fs::read(path).await.map_err(|e| anyhow::anyhow!("picture: {e}"))?;
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
            let name = comfy::upload(&s.http, &url, bytes, &format!("{id}-pre-{node}.{ext}")).await?;
            pre_graph[node.as_str()]["inputs"]["image"] = json!(name);
        }
        // comfy::run waits up to 5 minutes for the app's ComfyUI to answer after its start.
        comfy::run(&s.http, &url, &pre_graph, &p.outputs, &out_dir.join("pre"), &format!("{id}-pre"), Duration::from_secs(p.timeout_s)).await
    }
    .await;
    // Always hand the card back to the main ComfyUI, also when the stage failed.
    let _ = crate::gpus::gaming::app_action(s, machine, "stop", &p.app).await;
    let _ = crate::gpus::gaming::app_action(s, machine, "start", "comfyui").await;
    let files = result?;
    if files.len() < p.feeds.len() {
        bail!("the {} stage made {} pictures, the workflow needs {}", p.app, files.len(), p.feeds.len());
    }
    for (node, file) in p.feeds.iter().zip(&files) {
        let bytes = tokio::fs::read(file).await?;
        let name = comfy::upload(&s.http, main_url, bytes, &format!("{id}-{node}.png")).await?;
        graph[node.as_str()]["inputs"]["image"] = json!(name);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_lives_on_the_main_host() {
        assert_eq!(app_url("http://192.168.178.80:8188", 8192).unwrap(), "http://192.168.178.80:8192");
        assert_eq!(app_url("http://comfyui:8188/", 8192).unwrap(), "http://comfyui:8192");
        assert_eq!(app_url("https://studio.example", 8192).unwrap(), "https://studio.example:8192");
        assert_eq!(app_url("http://h:8188/sub/path", 9000).unwrap(), "http://h:9000");
        assert!(app_url("http://", 1).is_err());
    }

    #[test]
    fn pictures_go_to_the_graph_that_has_their_node() {
        let pre = json!({"4": {"class_type": "LoadImage", "inputs": {}}});
        let pics = vec![("4".to_string(), PathBuf::from("a.png")), ("11".to_string(), PathBuf::from("m.png"))];
        let (to_pre, to_main) = split_pics(&pre, &pics);
        assert_eq!(to_pre, vec![("4".to_string(), PathBuf::from("a.png"))]);
        assert_eq!(to_main, vec![("11".to_string(), PathBuf::from("m.png"))]);
    }

    #[test]
    fn a_pre_stage_reads_from_workflow_toml() {
        let w: workflow::Workflow = toml::from_str(
            "title = \"S\"\noutputs = [\"20\"]\n[pre]\ngraph = \"mv.json\"\napp = \"mv\"\nport = 8192\noutputs = [\"6\"]\nfeeds = [\"101\", \"102\", \"103\", \"104\"]\n",
        )
        .unwrap();
        let p = w.pre.unwrap();
        assert_eq!((p.app.as_str(), p.port, p.timeout_s), ("mv", 8192, 600));
        assert_eq!(p.feeds.len(), 4);
    }
}
