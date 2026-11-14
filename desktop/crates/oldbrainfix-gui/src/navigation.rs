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
mod tests;
