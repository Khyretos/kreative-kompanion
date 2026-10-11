//! STU-C3: helmet on and off. A make request with `helmet_desc` (what the helmet looks like) for a type
//! with a `helmet` setting becomes two runs with the same seed: "on" adds the description to the prompt
//! and keeps the face fix on the helmet (no face photo); "off" leaves the description out and shows the
//! face (face photo and face prompt as asked). Both carry the same `pair` id, so the cards can toggle.
//! `helmet` = "on" / "off" asks for one side only (a shot made from one sheet of a pair).
use serde_json::{Map, Value};

#[derive(Debug, PartialEq)]
pub struct Side {
    /// "on" or "off".
    pub state: &'static str,
    pub prompt: String,
    /// Replaces the face prompt (the helmet side: the description); None keeps what was asked.
    pub face_prompt: Option<String>,
    /// Whether this side takes the face photo.
    pub face_photo: bool,
}

#[derive(Debug, PartialEq)]
pub struct Pair {
    pub id: String,
    pub desc: String,
    pub sides: Vec<Side>,
}

/// Takes "helmet_desc", "helmet" and "pair" out of `params`. Ok(None): no helmet asked.
pub fn split(prompt: &str, params: &mut Map<String, Value>, new_id: impl FnOnce() -> String) -> Result<Option<Pair>, String> {
    let desc = params.remove("helmet_desc");
    let state = params.remove("helmet");
    let pair = params.remove("pair");

    let text = desc.as_ref().and_then(Value::as_str).unwrap_or("").trim().to_string();
    if text.is_empty() {
        if state.is_some() {
            return Err("Describe the helmet.".to_string());
        }
        return Ok(None);
    }
    if text.chars().count() > 300 {
        return Err("Keep the helmet description under 300 characters.".to_string());
    }

    let state_val = state.as_ref().and_then(Value::as_str);
    let states = match state_val {
        Some("on") => vec!["on"],
        Some("off") => vec!["off"],
        None if state.is_none() => vec!["on", "off"],
        _ => return Err("helmet must be on or off".to_string()),
    };

    let id = pair.as_ref().and_then(Value::as_str).filter(|s| !s.trim().is_empty()).map(str::to_string).unwrap_or_else(new_id);

    let sides = states.iter().map(|st| {
        let prompt = if *st == "on" {
            format!("{prompt}, {text}")
        } else {
            prompt.to_string()
        };
        Side {
            state: *st,
            prompt,
            face_prompt: if *st == "on" { Some(text.clone()) } else { None },
            face_photo: *st == "off",
        }
    }).collect();

    Ok(Some(Pair { id, desc: text, sides }))
}

#[cfg(test)]
#[path = "helmet_tests.rs"]
mod tests;
