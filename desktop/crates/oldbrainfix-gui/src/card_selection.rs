//! The cards selected in a Note list (#148), to move, merge, weight,
//! trash or restore several at once (#273): a
//! click selects one card, Ctrl-click (Cmd on macOS) adds or takes away
//! one, Shift-click selects the run from the card clicked last to this
//! one in the order the list shows them. Esc and showing another place
//! drop the selection; cards that no longer show leave it.

/// The selected cards by their Notes' ids, in the order they were
/// selected, and the card a Shift-click runs from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardSelection {
    ids: Vec<String>,
    anchor: Option<String>,
}

impl CardSelection {
    /// A click: this card alone, and the runs start at it.
    pub fn click(&mut self, id: &str) {
        self.ids = vec![id.to_string()];
        self.anchor = Some(id.to_string());
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
    }

    /// Esc, or another place shown: nothing selected.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.anchor = None;
    }

    /// Keeps only the cards still shown in `order`, as after a search,
    /// or once filed out of the list.
    pub fn retain(&mut self, order: &[String]) {
        self.ids.retain(|id| order.contains(id));
        if self.anchor.as_ref().is_some_and(|a| !order.contains(a)) {
            self.anchor = None;
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
}
