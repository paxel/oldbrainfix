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
mod tests;
