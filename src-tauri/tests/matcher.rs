use phrasewatch_lib::matcher::{match_phrase, normalize};

#[test]
fn normalizes_contraction() {
    assert_eq!(normalize("I'm sorry"), "i am sorry");
}

#[test]
fn matches_sorry() {
    let phrases = vec!["i'm sorry".into(), "i am sorry".into()];
    assert_eq!(
        match_phrase("yeah I'm sorry about that", &phrases),
        Some("i'm sorry")
    );
    assert!(match_phrase("the weather is nice today", &phrases).is_none());
}
