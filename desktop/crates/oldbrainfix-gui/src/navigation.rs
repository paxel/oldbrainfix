//! Back and forward through the places the window's list showed (#145),
//! as a browser goes through its pages: the back button, Alt+Left and
//! the mouse's back button go to the place shown before, forward
//! returns; going somewhere new forgets what lay ahead. The path above
//! the list leads up the levels; this leads through the visits.

/// How many places back leads through at most; older ones are forgotten.
pub const KEPT: usize = 100;

/// The places before the one shown, the latest last, and those ahead of
/// it after going back, the nearest last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigation<T> {
    back: Vec<T>,
    forward: Vec<T>,
}

impl<T> Default for Navigation<T> {
    fn default() -> Navigation<T> {
        Navigation {
            back: Vec::new(),
            forward: Vec::new(),
        }
    }
}

impl<T: Clone + PartialEq> Navigation<T> {
    /// Going from `from` to `to`: back leads to `from` again, and what
    /// lay ahead is forgotten. Staying where it is changes nothing.
    pub fn went(&mut self, from: T, to: &T) {
        if from == *to {
            return;
        }
        self.back.push(from);
        if self.back.len() > KEPT {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    /// The place shown before `current`, which forward then leads back
    /// to; none at the first one.
    pub fn back(&mut self, current: T) -> Option<T> {
        let to = self.back.pop()?;
        self.forward.push(current);
        Some(to)
    }

    /// The place `back` left, which back then leads to again; none when
    /// nothing lies ahead.
    pub fn forward(&mut self, current: T) -> Option<T> {
        let to = self.forward.pop()?;
        self.back.push(current);
        Some(to)
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Puts `to` wherever `from` was visited, as an Area shared from afar
    /// gets a new id.
    pub fn replace(&mut self, from: &T, to: &T) {
        for list in [&mut self.back, &mut self.forward] {
            for place in list.iter_mut().filter(|p| *p == from) {
                *place = to.clone();
            }
        }
    }

    /// Forgets the places `keep` refuses, such as a deleted Area; of the
    /// visits that then follow each other twice, or would lead to
    /// `current` itself, one stays.
    pub fn retain(&mut self, current: &T, keep: impl Fn(&T) -> bool) {
        for list in [&mut self.back, &mut self.forward] {
            list.retain(&keep);
            list.dedup();
            if list.last() == Some(current) {
                list.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_and_forward_go_through_the_visits() {
        let mut nav = Navigation::default();
        assert!(!nav.can_go_back());
        assert!(!nav.can_go_forward());
        nav.went("agenda", &"all");
        nav.went("all", &"household");
        assert!(nav.can_go_back());
        assert_eq!(nav.back("household"), Some("all"));
        assert_eq!(nav.back("all"), Some("agenda"));
        assert_eq!(nav.back("agenda"), None);
        assert!(nav.can_go_forward());
        assert_eq!(nav.forward("agenda"), Some("all"));
        assert_eq!(nav.forward("all"), Some("household"));
        assert_eq!(nav.forward("household"), None);
        assert_eq!(nav.back("household"), Some("all"));
    }

    #[test]
    fn going_somewhere_new_forgets_what_lay_ahead() {
        let mut nav = Navigation::default();
        nav.went("agenda", &"all");
        nav.went("all", &"household");
        assert_eq!(nav.back("household"), Some("all"));
        nav.went("all", &"frogs");
        assert!(!nav.can_go_forward());
        assert_eq!(nav.back("frogs"), Some("all"));
        assert_eq!(nav.back("all"), Some("agenda"));
    }

    #[test]
    fn staying_in_place_is_no_visit() {
        let mut nav = Navigation::default();
        nav.went("agenda", &"agenda");
        assert!(!nav.can_go_back());
        nav.went("agenda", &"all");
        assert_eq!(nav.back("all"), Some("agenda"));
        nav.went("agenda", &"agenda");
        // Forward is still there: nothing new was shown.
        assert!(nav.can_go_forward());
    }

    #[test]
    fn only_the_latest_visits_are_kept() {
        let mut nav = Navigation::default();
        for i in 0..KEPT + 10 {
            nav.went(i, &(i + 1));
        }
        let mut current = KEPT + 10;
        let mut steps = 0;
        while let Some(to) = nav.back(current) {
            current = to;
            steps += 1;
        }
        assert_eq!(steps, KEPT);
        assert_eq!(current, 10);
    }

    #[test]
    fn a_place_renamed_is_visited_under_its_new_name() {
        let mut nav = Navigation::default();
        nav.went("household", &"all");
        nav.went("all", &"frogs");
        assert_eq!(nav.back("frogs"), Some("all"));
        nav.replace(&"household", &"shared");
        nav.replace(&"frogs", &"done");
        assert_eq!(nav.back("all"), Some("shared"));
        assert_eq!(nav.forward("shared"), Some("all"));
        assert_eq!(nav.forward("all"), Some("done"));
    }

    #[test]
    fn a_place_gone_is_forgotten_and_its_neighbours_meet_once() {
        let mut nav = Navigation::default();
        nav.went("all", &"household");
        nav.went("household", &"all");
        nav.went("all", &"repairs");
        // Repairs is deleted; the window shows All Notes.
        nav.retain(&"all", |p| *p != "household" && *p != "repairs");
        // "all, household, all": household gone, the two meet, and the
        // last one is where the window is.
        assert!(!nav.can_go_back());
        let mut nav = Navigation::default();
        nav.went("agenda", &"household");
        nav.went("household", &"frogs");
        assert_eq!(nav.back("frogs"), Some("household"));
        nav.retain(&"agenda", |p| *p != "household");
        assert!(!nav.can_go_back());
        assert_eq!(nav.forward("agenda"), Some("frogs"));
    }
}
