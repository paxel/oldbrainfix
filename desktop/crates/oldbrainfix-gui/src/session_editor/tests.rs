use super::*;
use oldbrainfix_core::SigningKey;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_weekday_past_sunday_has_no_own_count() -> TestResult {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?;
    let mut draft = SessionDraft::of(&store, &L10n::new("en"), "note:a", None)?;
    draft.set_own(0, true);
    draft.set_own(7, true);
    assert_eq!(draft.counts, [Some(1), None, None, None, None, None, None]);
    Ok(())
}

/// #267: a tag nested under one picked shows with it, a longer word
/// beginning with it does not.
#[test]
fn a_picked_tag_shows_the_tags_nested_under_it() -> TestResult {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?;
    let mut draft = SessionDraft::of(&store, &L10n::new("en"), "note:a", None)?;
    for (id, text) in [
        ("run", "Run #sport/run"),
        ("swim", "Swim #sport"),
        ("chess", "Chess #sportive"),
    ] {
        draft.tags.insert(id.into(), tags_of(Some(text)));
    }
    let filter = ["#sport".to_string()];
    assert!(draft.shows("run", &filter));
    assert!(draft.shows("swim", &filter));
    assert!(!draft.shows("chess", &filter));
    assert!(!draft.shows("swim", &["#sport/run".to_string()]));
    Ok(())
}
