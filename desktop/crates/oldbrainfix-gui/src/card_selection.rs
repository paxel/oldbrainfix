//! The cards selected in a Note list (#148), to move, merge, weight,
//! trash or restore several at once (#273): a
//! click selects one card, Ctrl-click (Cmd on macOS) adds or takes away
//! one, Shift-click selects the run from the card clicked last to this
//! one in the order the list shows them. Up and Down move the selection,
//! Shift with them runs it on, Ctrl+A takes every card (#274). Esc and
//! showing another place drop the selection; cards that no longer show
//! leave it.

/// The selected cards by their Notes' ids, in the order they were
/// selected, the card a Shift-click runs from, and the card the keys
/// moved to last, where Up and Down go on from (#274).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardSelection {
    ids: Vec<String>,
    anchor: Option<String>,
    cursor: Option<String>,
}

impl CardSelection {
    /// A click: this card alone, and the runs start at it.
    pub fn click(&mut self, id: &str) {
        self.ids = vec![id.to_string()];
        self.anchor = Some(id.to_string());
        self.cursor = Some(id.to_string());
    }

    /// A Ctrl-click: the card in when it was out, out when it was in;
    /// the runs start at it either way.
    pub fn toggle(&mut self, id: &str) {
        if let Some(at) = self.ids.iter().position(|i| i == id) {
            self.ids.remove(at);
        } else {
            self.ids.push(id.to_string());
        }
        self.anchor = Some(id.to_string());
        self.cursor = Some(id.to_string());
    }

    /// A Shift-click: the cards from the one clicked last to `id`, both
    /// in, in the list's `order`; a click where no card clicked last
    /// shows.
    pub fn extend(&mut self, id: &str, order: &[String]) {
        let from = self
            .anchor
            .as_ref()
            .and_then(|a| order.iter().position(|i| i == a));
        let (Some(from), Some(to)) = (from, order.iter().position(|i| i == id)) else {
            self.click(id);
            return;
        };
        let (start, end) = (from.min(to), from.max(to));
        self.ids = order
            .iter()
            .skip(start)
            .take(end - start + 1)
            .cloned()
            .collect();
        self.cursor = Some(id.to_string());
    }

    /// Down (`down`) or Up over the list in its `order` (#274): the card
    /// after or before the one the keys or a click reached last, alone,
    /// staying at the list's ends; with nothing reached the first card,
    /// or with Up the last. Gives the card selected.
    pub fn step(&mut self, order: &[String], down: bool) -> Option<String> {
        let to = self.next(order, down)?;
        self.click(&to);
        Some(to)
    }

    /// Shift+Down or Shift+Up: the run from the card it started at on to
    /// the next card; with no run started, as [`Self::step`]. Gives the
    /// card reached.
    pub fn step_on(&mut self, order: &[String], down: bool) -> Option<String> {
        let to = self.next(order, down)?;
        self.extend(&to, order);
        Some(to)
    }

    /// Ctrl+A: every card of the list's `order`, a run from the first to
    /// the last.
    pub fn select_all(&mut self, order: &[String]) {
        self.ids = order.to_vec();
        self.anchor = order.first().cloned();
        self.cursor = order.last().cloned();
    }

    /// The card the keys or a click reached last, which Enter opens,
    /// while it is selected.
    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref().filter(|id| self.contains(id))
    }

    /// The card after or before the one reached last, in `order`.
    fn next(&self, order: &[String], down: bool) -> Option<String> {
        let at = self
            .cursor
            .as_ref()
            .and_then(|c| order.iter().position(|i| i == c));
        let to = match at {
            Some(at) if down => order.get(at + 1).or_else(|| order.get(at)),
            Some(at) => order.get(at.saturating_sub(1)),
            None if down => order.first(),
            None => order.last(),
        };
        to.cloned()
    }

    /// An action done on the selection (#273): nothing selected, but Up
    /// and Down go on from the card reached last while it still shows
    /// (#274).
    pub fn end(&mut self) {
        self.ids.clear();
        self.anchor = None;
    }

    /// Esc, or another place shown: nothing selected.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.anchor = None;
        self.cursor = None;
    }

    /// Keeps only the cards still shown in `order`, as after a search,
    /// or once filed out of the list.
    pub fn retain(&mut self, order: &[String]) {
        self.ids.retain(|id| order.contains(id));
        if self.anchor.as_ref().is_some_and(|a| !order.contains(a)) {
            self.anchor = None;
        }
        if self.cursor.as_ref().is_some_and(|c| !order.contains(c)) {
            self.cursor = None;
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids.iter().any(|i| i == id)
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// The selected cards in the list's `order`.
    pub fn in_order(&self, order: &[String]) -> Vec<String> {
        order
            .iter()
            .filter(|id| self.contains(id))
            .cloned()
            .collect()
    }

    /// What moves when the card `id` is dragged or filed: the selection
    /// in the list's `order` when it holds the card, else the card alone.
    pub fn moving(&self, id: &str, order: &[String]) -> Vec<String> {
        if self.contains(id) {
            self.in_order(order)
        } else {
            vec![id.to_string()]
        }
    }
}

#[cfg(test)]
mod tests {
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
}
