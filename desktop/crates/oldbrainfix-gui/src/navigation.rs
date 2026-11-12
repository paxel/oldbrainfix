//! Back and forward through the places the window's list showed (#145),
//! as a browser goes through its pages: the back button, Alt+Left and
//! the mouse's back button go to the place shown before, forward
//! returns; going somewhere new forgets what lay ahead. The path above
//! the list leads up the levels; this leads through the visits. Each
//! visit keeps what the place was left with (#278), such as its search
//! and how far it was scrolled, for back and forward to bring again.

/// How many places back leads through at most; older ones are forgotten.
pub const KEPT: usize = 100;

/// The places before the one shown, the latest last, and those ahead of
/// it after going back, the nearest last; each with what it was left
/// with.
#[derive(Debug, Clone, PartialEq)]
pub struct Navigation<T, S = ()> {
    back: Vec<(T, S)>,
    forward: Vec<(T, S)>,
}

impl<T, S> Default for Navigation<T, S> {
    fn default() -> Navigation<T, S> {
        Navigation {
            back: Vec::new(),
            forward: Vec::new(),
        }
    }
}

impl<T: Clone + PartialEq, S> Navigation<T, S> {
    /// Going from `from`, left with `left`, to `to`: back leads to
    /// `from` again as it was left, and what lay ahead is forgotten.
    /// Staying where it is changes nothing.
    pub fn went(&mut self, from: T, left: S, to: &T) {
        if from == *to {
            return;
        }
        self.back.push((from, left));
        if self.back.len() > KEPT {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    /// The place shown before `current`, with what it was left with;
    /// forward then leads back to `current` as it is left now. None at
    /// the first one.
    pub fn back(&mut self, current: T, left: S) -> Option<(T, S)> {
        let to = self.back.pop()?;
        self.forward.push((current, left));
        Some(to)
    }

    /// The place `back` left, with what it was left with; back then
    /// leads to `current` again as it is left now. None when nothing
    /// lies ahead.
    pub fn forward(&mut self, current: T, left: S) -> Option<(T, S)> {
        let to = self.forward.pop()?;
        self.back.push((current, left));
        Some(to)
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Puts `to` wherever `from` was visited, as an Area shared from afar
    /// gets a new id; what each visit was left with stays.
    pub fn replace(&mut self, from: &T, to: &T) {
        for list in [&mut self.back, &mut self.forward] {
            for (place, _) in list.iter_mut().filter(|(p, _)| p == from) {
                *place = to.clone();
            }
        }
    }

    /// Forgets the places `keep` refuses, such as a deleted Area; of the
    /// visits that then follow each other twice, or would lead to
    /// `current` itself, one stays.
    pub fn retain(&mut self, current: &T, keep: impl Fn(&T) -> bool) {
        for list in [&mut self.back, &mut self.forward] {
            list.retain(|(place, _)| keep(place));
            list.dedup_by(|a, b| a.0 == b.0);
            if list.last().map(|(place, _)| place) == Some(current) {
                list.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
