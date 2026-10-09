use super::*;
use serde_json::json;

fn obj(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn no_description_means_no_pair() {
    let mut p = obj(json!({"background": "white"}));
    assert_eq!(split("a knight", &mut p, || "x".into()), Ok(None));
    let mut p = obj(json!({"helmet_desc": "  "}));
    assert_eq!(split("a knight", &mut p, || "x".into()), Ok(None));
    assert!(p.is_empty(), "the empty helmet_desc is taken out");
}

#[test]
fn a_description_makes_both_sides() {
    let mut p = obj(json!({"helmet_desc": " white helmet with a teal visor ", "face_prompt": "short black hair", "background": "white"}));
    let pair = split("a knight in white and gold armour", &mut p, || "p1".into()).unwrap().unwrap();
    assert_eq!(pair.id, "p1");
    assert_eq!(pair.desc, "white helmet with a teal visor");
    assert_eq!(
        pair.sides,
        vec![
            Side { state: "on", prompt: "a knight in white and gold armour, white helmet with a teal visor".into(), face_prompt: Some("white helmet with a teal visor".into()), face_photo: false },
            Side { state: "off", prompt: "a knight in white and gold armour".into(), face_prompt: None, face_photo: true },
        ]
    );
    // helmet_desc, helmet and pair leave params; the type's own settings stay.
    assert_eq!(p, obj(json!({"face_prompt": "short black hair", "background": "white"})));
}

#[test]
fn one_side_and_a_given_pair() {
    let mut p = obj(json!({"helmet_desc": "black hood", "helmet": "off", "pair": "p9"}));
    let pair = split("a rogue", &mut p, || unreachable!()).unwrap().unwrap();
    assert_eq!(pair.id, "p9");
    assert_eq!(pair.sides, vec![Side { state: "off", prompt: "a rogue".into(), face_prompt: None, face_photo: true }]);
    assert!(p.is_empty());
}

#[test]
fn bad_requests_say_why() {
    let mut p = obj(json!({"helmet_desc": "hood", "helmet": "half"}));
    assert_eq!(split("x", &mut p, || "x".into()), Err("helmet must be on or off".to_string()));
    let mut p = obj(json!({"helmet": "on"}));
    assert_eq!(split("x", &mut p, || "x".into()), Err("Describe the helmet.".to_string()));
    let mut p = obj(json!({"helmet_desc": "h".repeat(301)}));
    assert_eq!(split("x", &mut p, || "x".into()), Err("Keep the helmet description under 300 characters.".to_string()));
}
