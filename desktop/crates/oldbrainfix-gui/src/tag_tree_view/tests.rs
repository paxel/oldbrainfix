use super::*;

#[test]
fn a_fold_is_kept_by_the_tag_in_lower_case() {
    let node = TagNode {
        name: "Health".into(),
        tag: "#Life/Health".into(),
        notes: 2,
        children: Vec::new(),
    };
    assert_eq!(key(&node), "#life/health");
}
