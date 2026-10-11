use super::*;

#[test]
fn blank_or_missing_sends_nothing() {
    assert_eq!(negative_param(None, "VN scene", Some("")), Ok(None));
    assert_eq!(negative_param(Some("  , "), "VN scene", Some("text")), Ok(None));
    assert_eq!(negative_param(Some(""), "Music", None), Ok(None));
}

#[test]
fn user_words_are_trimmed() {
    assert_eq!(negative_param(Some(" blurry, hats, "), "VN scene", Some("")), Ok(Some("blurry, hats".to_string())));
}

#[test]
fn user_words_follow_the_type_default() {
    assert_eq!(
        negative_param(Some("hats"), "Icon", Some("text, letters, watermark")),
        Ok(Some("text, letters, watermark, hats".to_string()))
    );
}

#[test]
fn types_without_a_negative_refuse_it() {
    assert_eq!(negative_param(Some("noise"), "Music", None), Err("Music takes no negative prompt.".to_string()));
}

#[test]
fn length_is_capped_at_500() {
    let ok = "a".repeat(500);
    assert_eq!(negative_param(Some(&ok), "Icon", Some("")), Ok(Some(ok.clone())));
    assert_eq!(
        negative_param(Some(&"a".repeat(501)), "Icon", Some("")),
        Err("Keep the negative prompt under 500 characters.".to_string())
    );
}

#[test]
fn the_adult_guard_cannot_be_negated() {
    let err = Err("Adult and mature stay in every picture; take them out of the negative prompt.".to_string());
    assert_eq!(negative_param(Some("blurry, Adult"), "VN scene", Some("")), err);
    assert_eq!(negative_param(Some("mature woman"), "VN scene", Some("")), err);
    assert_eq!(negative_param(Some("ADULTS"), "VN scene", Some("")), err);
    // Words that only contain the letters are fine.
    assert_eq!(negative_param(Some("immature style, adulterated"), "VN scene", Some("")), Ok(Some("immature style, adulterated".to_string())));
}
