use super::*;

#[test]
fn unchanged_closes_valid_saves_and_invalid_asks() {
    assert_eq!(Leaving::of(false, false), Leaving::Close);
    assert_eq!(Leaving::of(false, true), Leaving::Close);
    assert_eq!(Leaving::of(true, true), Leaving::Save);
    assert_eq!(Leaving::of(true, false), Leaving::Ask);
}
