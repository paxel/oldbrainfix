//! Leaving an edit dialog (#275, UI laws, "Navigation"), as the phone's
//! `leave_saves.dart`: the Routine editor, a Tally entry, the New Tally
//! page and the Session pages, left by Esc, a click beside them, Alt+Left
//! or the mouse's back button. Left as it opened it just closes; a change
//! that can be saved is saved as its Save does; one that can't asks once,
//! "Discard changes?". Discard closes without saving; Keep editing stays
//! and shows what to fix. Cancel still closes without asking.

use egui::{RichText, Ui};

use crate::l10n::L10n;
use crate::theme;

/// What leaving comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leaving {
    /// Nothing changed: it closes.
    Close,
    /// The change can be saved: Save takes it.
    Save,
    /// The change can't be saved: "Discard changes?" first.
    Ask,
}

impl Leaving {
    /// Leaving a dialog that `changed`, whose edit is `valid` for Save.
    pub fn of(changed: bool, valid: bool) -> Leaving {
        match (changed, valid) {
            (false, _) => Leaving::Close,
            (true, true) => Leaving::Save,
            (true, false) => Leaving::Ask,
        }
    }
}

/// "Discard changes?" with Keep editing and Discard, on the error colour:
/// `Some(true)` for Discard, `Some(false)` for Keep editing.
pub fn question(ui: &mut Ui, t: &L10n) -> Option<bool> {
    let text = ui.visuals().text_color();
    ui.label(
        RichText::new(t.leave_discard_title())
            .size(18.0)
            .strong()
            .color(text),
    );
    ui.label(RichText::new(t.leave_discard_body()).color(text));
    let mut answer = None;
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(RichText::new(t.leave_keep_editing()).color(text))
            .clicked()
        {
            answer = Some(false);
        }
        if theme::destructive(ui, true, t.leave_discard()).clicked() {
            answer = Some(true);
        }
    });
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_closes_valid_saves_and_invalid_asks() {
        assert_eq!(Leaving::of(false, false), Leaving::Close);
        assert_eq!(Leaving::of(false, true), Leaving::Close);
        assert_eq!(Leaving::of(true, true), Leaving::Save);
        assert_eq!(Leaving::of(true, false), Leaving::Ask);
    }
}
