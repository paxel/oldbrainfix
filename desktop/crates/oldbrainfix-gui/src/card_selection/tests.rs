use super::*;

fn order(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_click_selects_one_card_alone() {
    let mut s = CardSelection::default();
    s.click("a");
    s.click("b");
    assert_eq!(s.in_order(&order(&["a", "b", "c"])), ["b"]);
    assert_eq!(s.len(), 1);
}

#[test]
fn ctrl_click_adds_and_takes_away() {
    let mut s = CardSelection::default();
    s.click("a");
    s.toggle("c");
    assert_eq!(s.in_order(&order(&["a", "b", "c"])), ["a", "c"]);
    s.toggle("a");
    assert_eq!(s.in_order(&order(&["a", "b", "c"])), ["c"]);
    s.toggle("c");
    assert!(s.is_empty());
}

#[test]
fn shift_click_selects_the_run_in_the_list_order() {
    let list = order(&["a", "b", "c", "d", "e"]);
    let mut s = CardSelection::default();
    s.click("b");
    s.extend("d", &list);
    assert_eq!(s.in_order(&list), ["b", "c", "d"]);
    // Upwards from the same card, the run replaces the one before.
    s.extend("a", &list);
    assert_eq!(s.in_order(&list), ["a", "b"]);
    // From the card Ctrl-clicked last.
    s.toggle("e");
    s.extend("c", &list);
    assert_eq!(s.in_order(&list), ["c", "d", "e"]);
}

#[test]
fn shift_click_without_a_card_clicked_before_is_a_click() {
    let list = order(&["a", "b", "c"]);
    let mut s = CardSelection::default();
    s.extend("b", &list);
    assert_eq!(s.in_order(&list), ["b"]);
    s.extend("c", &list);
    assert_eq!(s.in_order(&list), ["b", "c"]);
}

#[test]
fn esc_clears_and_cards_gone_leave() {
    let list = order(&["a", "b", "c"]);
    let mut s = CardSelection::default();
    s.click("a");
    s.extend("c", &list);
    s.retain(&order(&["b", "c"]));
    assert_eq!(s.in_order(&list), ["b", "c"]);
    // The card the runs started at is gone: a Shift-click is a click.
    s.extend("b", &order(&["b", "c"]));
    assert_eq!(s.in_order(&list), ["b"]);
    s.clear();
    assert!(s.is_empty());
    assert!(!s.contains("b"));
}

#[test]
fn a_selected_card_moves_the_selection_another_one_itself() {
    let list = order(&["a", "b", "c"]);
    let mut s = CardSelection::default();
    s.click("c");
    s.toggle("a");
    assert_eq!(s.moving("c", &list), ["a", "c"]);
    assert_eq!(s.moving("b", &list), ["b"]);
}

#[test]
fn up_and_down_move_the_selection_and_stop_at_the_ends() {
    let list = order(&["a", "b", "c"]);
    let mut s = CardSelection::default();
    // Nothing reached yet: Down takes the first card, Up the last.
    assert_eq!(s.step(&list, true).as_deref(), Some("a"));
    s.clear();
    assert_eq!(s.step(&list, false).as_deref(), Some("c"));
    assert_eq!(s.step(&list, false).as_deref(), Some("b"));
    assert_eq!(s.in_order(&list), ["b"]);
    assert_eq!(s.step(&list, false).as_deref(), Some("a"));
    assert_eq!(s.step(&list, false).as_deref(), Some("a"));
    s.click("c");
    assert_eq!(s.step(&list, true).as_deref(), Some("c"));
    assert_eq!(s.in_order(&list), ["c"]);
    assert_eq!(s.cursor(), Some("c"));
    // An empty list has nothing to reach.
    assert_eq!(CardSelection::default().step(&[], true), None);
}

#[test]
fn shift_with_up_and_down_runs_the_selection_on_and_back() {
    let list = order(&["a", "b", "c", "d"]);
    let mut s = CardSelection::default();
    s.click("b");
    s.step_on(&list, true);
    s.step_on(&list, true);
    assert_eq!(s.in_order(&list), ["b", "c", "d"]);
    assert_eq!(s.cursor(), Some("d"));
    s.step_on(&list, false);
    assert_eq!(s.in_order(&list), ["b", "c"]);
    s.step_on(&list, false);
    s.step_on(&list, false);
    assert_eq!(s.in_order(&list), ["a", "b"]);
    // Down alone goes on from the card reached, alone.
    assert_eq!(s.step(&list, true).as_deref(), Some("b"));
    assert_eq!(s.in_order(&list), ["b"]);
}

#[test]
fn after_an_action_the_keys_go_on_from_the_card_reached() {
    let list = order(&["a", "b", "c"]);
    let mut s = CardSelection::default();
    s.step(&list, true);
    s.end();
    assert!(s.is_empty());
    assert_eq!(s.cursor(), None);
    assert_eq!(s.step(&list, true).as_deref(), Some("b"));
    // Gone from the list, as once filed: Down starts at the top.
    s.end();
    s.retain(&order(&["a", "c"]));
    assert_eq!(s.step(&order(&["a", "c"]), true).as_deref(), Some("a"));
}

#[test]
fn ctrl_a_takes_every_card() {
    let list = order(&["a", "b", "c"]);
    let mut s = CardSelection::default();
    s.select_all(&list);
    assert_eq!(s.in_order(&list), ["a", "b", "c"]);
    s.step_on(&list, false);
    assert_eq!(s.in_order(&list), ["a", "b"]);
    s.retain(&order(&["a"]));
    assert_eq!(s.cursor(), None);
}
