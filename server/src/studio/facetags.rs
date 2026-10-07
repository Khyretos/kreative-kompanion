//! KS-02: face tags for a face photo (Coder describes it; port of Kreative Studio's mergeTags).
use crate::assets::ai::Ai;
use serde_json::json;

pub const PROMPT: &str = "Describe this person's appearance as comma-separated Danbooru-style tags, at most 12, lowercase, nothing else. Cover: 1man or 1woman, skin tone (for example dark skin, brown skin, tan skin, pale skin), hair colour, hair length and style, facial hair, glasses, eye colour, build. Do not describe clothing, pose, background or age.";

const YOUNG: [&str; 9] = ["child", "kid", "teen", "young", "boy", "girl", "loli", "shota", "minor"];

pub fn clean(raw: &str) -> Vec<String> {
    raw.split(|c| c == ',' || c == '\n')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .filter(|s| !s.starts_with("no ") && !s.starts_with("without "))
        .filter(|s| !YOUNG.iter().any(|w| s.contains(w)))
        .collect()
}

pub fn skin_tags(tags: &[String]) -> Vec<String> {
    let mut out = tags.to_vec();
    let is_man = tags.iter().any(|t| t == "1man" || t == "1boy");
    let is_dark_skin = tags.iter().any(|t| matches!(t.as_str(), "brown skin" | "dark skin" | "tan skin" | "dark-skinned"));
    if is_dark_skin {
        out.push(if is_man { "dark-skinned male".to_string() } else { "dark-skinned female".to_string() });
    }
    out
}

pub fn merge_tags(tags: &[String], prompt: &str) -> String {
    const PARTS: [&[&str]; 5] = [&["hair"], &["beard", "stubble", "moustache", "mustache", "goatee"], &["eyes"], &["glasses"], &["skin"]];
    let low = prompt.to_lowercase();
    let said: Vec<&[&str]> = PARTS.iter().copied().filter(|words| words.iter().any(|w| low.contains(w))).collect();
    let out: Vec<String> = skin_tags(tags)
        .into_iter()
        .filter(|t| !said.iter().any(|words| words.iter().any(|w| t.contains(w))))
        .collect();
    let joined = out.join(", ");
    joined.chars().take(300).collect()
}

pub async fn describe(ai: &Ai, png: &[u8]) -> anyhow::Result<Vec<String>> {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    let answer = ai.chat(json!([{ "type": "text", "text": PROMPT }, { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{b64}") } }]), 120).await?;
    Ok(clean(&answer))
}

pub async fn to_png(image: &[u8]) -> anyhow::Result<Vec<u8>> {
    use tokio::io::AsyncWriteExt;
    use std::process::Stdio;
    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.arg("-v").arg("error")
       .arg("-nostdin")
       .arg("-i").arg("pipe:0")
       .arg("-vf").arg("scale=w='min(512,iw)':h='min(512,ih)':force_original_aspect_ratio=decrease")
       .arg("-frames:v").arg("1")
       .arg("-f").arg("image2pipe")
       .arg("-c:v").arg("png")
       .arg("pipe:1");
    let mut child = cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(image).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    if !output.status.success() || output.stdout.is_empty() {
        anyhow::bail!("couldn't read the face photo");
    }
    Ok(output.stdout)
}

#[cfg(test)]
#[path = "facetags_tests.rs"]
mod tests;
