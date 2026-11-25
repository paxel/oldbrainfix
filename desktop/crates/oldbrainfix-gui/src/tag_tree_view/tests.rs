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

fn node(tag: &str, notes: usize, children: Vec<TagNode>) -> TagNode {
    TagNode {
        name: tag.rsplit('/').next().unwrap_or(tag).into(),
        tag: tag.into(),
        notes,
        children,
    }
}

#[test]
fn the_most_used_come_first_of_every_level_ties_by_the_tree() {
    let tree = vec![
        node("#garden", 1, Vec::new()),
        node(
            "#life",
            3,
            vec![node(
                "#life/health",
                2,
                vec![node("#life/health/back", 1, Vec::new())],
            )],
        ),
        node("#work", 2, Vec::new()),
    ];
    assert_eq!(
        most_used(&tree, 12),
        [
            "#life",
            "#life/health",
            "#work",
            "#garden",
            "#life/health/back"
        ]
    );
    assert_eq!(most_used(&tree, 2), ["#life", "#life/health"]);
    assert!(most_used(&[], 12).is_empty());
}
