use super::*;

/// Back without what places were left with, the place only.
fn back<T: Clone + PartialEq>(nav: &mut Navigation<T>, current: T) -> Option<T> {
    nav.back(current, ()).map(|(place, ())| place)
}

/// Forward without what places were left with, the place only.
fn forward<T: Clone + PartialEq>(nav: &mut Navigation<T>, current: T) -> Option<T> {
    nav.forward(current, ()).map(|(place, ())| place)
}

#[test]
fn back_and_forward_go_through_the_visits() {
    let mut nav = Navigation::default();
    assert!(!nav.can_go_back());
    assert!(!nav.can_go_forward());
    nav.went("agenda", (), &"all");
    nav.went("all", (), &"household");
    assert!(nav.can_go_back());
    assert_eq!(back(&mut nav, "household"), Some("all"));
    assert_eq!(back(&mut nav, "all"), Some("agenda"));
    assert_eq!(back(&mut nav, "agenda"), None);
    assert!(nav.can_go_forward());
    assert_eq!(forward(&mut nav, "agenda"), Some("all"));
    assert_eq!(forward(&mut nav, "all"), Some("household"));
    assert_eq!(forward(&mut nav, "household"), None);
    assert_eq!(back(&mut nav, "household"), Some("all"));
}

#[test]
fn going_somewhere_new_forgets_what_lay_ahead() {
    let mut nav = Navigation::default();
    nav.went("agenda", (), &"all");
    nav.went("all", (), &"household");
    assert_eq!(back(&mut nav, "household"), Some("all"));
    nav.went("all", (), &"frogs");
    assert!(!nav.can_go_forward());
    assert_eq!(back(&mut nav, "frogs"), Some("all"));
    assert_eq!(back(&mut nav, "all"), Some("agenda"));
}

#[test]
fn staying_in_place_is_no_visit() {
    let mut nav = Navigation::default();
    nav.went("agenda", (), &"agenda");
    assert!(!nav.can_go_back());
    nav.went("agenda", (), &"all");
    assert_eq!(back(&mut nav, "all"), Some("agenda"));
    nav.went("agenda", (), &"agenda");
    // Forward is still there: nothing new was shown.
    assert!(nav.can_go_forward());
}

#[test]
fn only_the_latest_visits_are_kept() {
    let mut nav = Navigation::default();
    for i in 0..KEPT + 10 {
        nav.went(i, (), &(i + 1));
    }
    let mut current = KEPT + 10;
    let mut steps = 0;
    while let Some(to) = back(&mut nav, current) {
        current = to;
        steps += 1;
    }
    assert_eq!(steps, KEPT);
    assert_eq!(current, 10);
}

#[test]
fn a_place_renamed_is_visited_under_its_new_name() {
    let mut nav = Navigation::default();
    nav.went("household", (), &"all");
    nav.went("all", (), &"frogs");
    assert_eq!(back(&mut nav, "frogs"), Some("all"));
    nav.replace(&"household", &"shared");
    nav.replace(&"frogs", &"done");
    assert_eq!(back(&mut nav, "all"), Some("shared"));
    assert_eq!(forward(&mut nav, "shared"), Some("all"));
    assert_eq!(forward(&mut nav, "all"), Some("done"));
}

#[test]
fn a_place_gone_is_forgotten_and_its_neighbours_meet_once() {
    let mut nav = Navigation::default();
    nav.went("all", (), &"household");
    nav.went("household", (), &"all");
    nav.went("all", (), &"repairs");
    // Repairs is deleted; the window shows All Notes.
    nav.retain(&"all", |p| *p != "household" && *p != "repairs");
    // "all, household, all": household gone, the two meet, and the
    // last one is where the window is.
    assert!(!nav.can_go_back());
    let mut nav = Navigation::default();
    nav.went("agenda", (), &"household");
    nav.went("household", (), &"frogs");
    assert_eq!(back(&mut nav, "frogs"), Some("household"));
    nav.retain(&"agenda", |p| *p != "household");
    assert!(!nav.can_go_back());
    assert_eq!(forward(&mut nav, "agenda"), Some("frogs"));
}

#[test]
fn each_visit_comes_back_as_it_was_left() {
    let mut nav: Navigation<&str, &str> = Navigation::default();
    nav.went("all", "searched milk", &"household");
    nav.went("household", "scrolled down", &"all");
    // The same place twice, each visit as it was left.
    assert_eq!(
        nav.back("all", "searched eggs"),
        Some(("household", "scrolled down"))
    );
    assert_eq!(
        nav.back("household", "scrolled up"),
        Some(("all", "searched milk"))
    );
    assert_eq!(
        nav.forward("all", "searched bread"),
        Some(("household", "scrolled up"))
    );
    assert_eq!(
        nav.forward("household", "at the top"),
        Some(("all", "searched eggs"))
    );
    nav.replace(&"household", &"shared");
    assert_eq!(nav.back("all", ""), Some(("shared", "at the top")));
}
