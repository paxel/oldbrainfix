//! Undo, Redo and "Revert all changes" of the Note open in the Note
//! dialog or in a window of its own (obf-1051, C3a (c)): step by step,
//! everything changed since it opened, its text and its tables' cells,
//! its Sections' texts, its Lines and its badges.
//!
//! A step is the whole Note as the editor shows it after a change: what
//! is typed into its fields and what is written at once. Typing becomes
//! a step once it rests for [`PAUSE`], as a new word begins, once its
//! field is left, saved or Ctrl+Enter is pressed, and before anything
//! else is done; a badge, a
//! Line or a format is a step of its own. Taking a step back writes the
//! earlier state as an edit does; nothing is deleted. "Revert all
//! changes" takes every step back at once, and Redo makes them again. Something written
//! elsewhere meanwhile, on another Device or in another window, ends the
//! steps: they start again from the Note as it is then, as an undo does
//! nothing where it was overtaken.
//!
//! A Line checked or unchecked is a step, as on the phone; a box ticked
//! in a text is a check-off with its own history (ADR 0035), no step.

use std::collections::BTreeMap;

use oldbrainfix_core::note_kinds::NoteKind;
use oldbrainfix_core::{
    Checklist, Importance, ListOnAgenda, Reappear, SessionSetup, ShoppingList, TallyPeriod,
    text_boxes,
};

/// How long typing rests before it is a step of its own, in seconds.
pub const PAUSE: f64 = 1.0;

/// The most steps kept back; the oldest go first.
pub const MOST: usize = 200;

/// One Line as a step holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct LineKept {
    pub id: String,
    pub text: String,
    /// Whether it is in the group of the Line above, a Checklist's only.
    pub in_group: bool,
    /// How long it stays struck, a Shopping List's only.
    pub reappear: Option<Reappear>,
    /// Whether it shows checked: a Checklist's Line by its checks, a
    /// Shopping List's struck or gone, as on the phone.
    pub checked: bool,
}

/// The Lines of a Checklist or Shopping List as a step holds them: of
/// its making, a Checklist's in the order they show, a Shopping List's,
/// whose Lines show by their checks, in the order they were placed in,
/// the Lines checked Once and gone among them.
#[derive(Debug, Clone, PartialEq)]
pub struct LinesKept {
    pub making: String,
    pub shopping: bool,
    pub lines: Vec<LineKept>,
}

impl LinesKept {
    pub fn of_checklist(list: &Checklist) -> LinesKept {
        LinesKept {
            making: list.making.clone(),
            shopping: false,
            lines: list
                .lines
                .iter()
                .map(|l| LineKept {
                    id: l.id.clone(),
                    text: l.text.clone(),
                    in_group: l.in_group(),
                    reappear: None,
                    checked: l.checked,
                })
                .collect(),
        }
    }

    pub fn of_shopping(list: &ShoppingList) -> LinesKept {
        let gone = |id: &str| list.gone().iter().any(|l| l.id == id);
        let mut placed: Vec<_> = list.lines.iter().chain(list.gone()).collect();
        placed.sort_by(|a, b| (&a.order, &a.id).cmp(&(&b.order, &b.id)));
        LinesKept {
            making: list.making.clone(),
            shopping: true,
            lines: placed
                .into_iter()
                .map(|l| LineKept {
                    id: l.id.clone(),
                    text: l.text.clone(),
                    in_group: false,
                    reappear: Some(l.reappear),
                    checked: l.struck || gone(&l.id),
                })
                .collect(),
        }
    }

    /// The Line `id`.
    pub fn line(&self, id: &str) -> Option<&LineKept> {
        self.lines.iter().find(|l| l.id == id)
    }
}

/// What a step holds of the Note that is written at once: its kind, the
/// format and language of its text, its badges' values and its Lines.
#[derive(Debug, Clone, PartialEq)]
pub struct Kept {
    pub kind: NoteKind,
    pub format: String,
    pub language: Option<String>,
    pub importance: Importance,
    /// The Area or Topic it is filed into, none in Unsorted.
    pub place: Option<String>,
    pub lines: Option<LinesKept>,
    pub on_agenda: Option<ListOnAgenda>,
    /// Whether the Note carries its own mark Hidden from AI.
    pub hidden: bool,
    /// Whether Show ticks is on for it.
    pub ticks: bool,
    /// A Tally's bar and whether it counts in sets.
    pub tally: Option<(TallyPeriod, bool)>,
    pub session: Option<SessionSetup>,
}

/// What is typed into the Note's fields: its text, a Note of a kind its
/// name and its description joined (#188), and each Section's, by id.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Typed {
    pub text: String,
    pub sections: BTreeMap<String, String>,
}

impl Typed {
    /// Whether it reads as `other` does: the same words, trimmed as a
    /// save trims them, the ids of the boxes aside, which a save gives
    /// new boxes (ADR 0035), in the Sections both hold: one added by
    /// Amend, or gone, is no typing.
    pub fn reads_as(&self, other: &Typed) -> bool {
        same_text(&self.text, &other.text)
            && self
                .sections
                .iter()
                .all(|(id, text)| other.sections.get(id).is_none_or(|o| same_text(text, o)))
    }
}

/// Whether `after` is `before` with a new word begun in one of its
/// fields: a space or a line break typed after a word, as the phone cuts
/// its typing into steps.
fn new_word(before: &Typed, after: &Typed) -> bool {
    begins_word(&before.text, &after.text)
        || after.sections.iter().any(|(id, text)| {
            before
                .sections
                .get(id)
                .is_some_and(|was| begins_word(was, text))
        })
}

/// Whether `after` is `before` with only something put in that starts
/// with a space or a line break, right after a letter or other mark.
fn begins_word(before: &str, after: &str) -> bool {
    if before == after {
        return false;
    }
    let mut start = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(start) || !after.is_char_boundary(start) {
        start -= 1;
    }
    let (was, is) = (&before[start..], &after[start..]);
    let mut end = was
        .bytes()
        .rev()
        .zip(is.bytes().rev())
        .take_while(|(a, b)| a == b)
        .count()
        .min(was.len());
    while !was.is_char_boundary(was.len() - end) || !is.is_char_boundary(is.len() - end) {
        end -= 1;
    }
    let removed = &was[..was.len() - end];
    let put = &is[..is.len() - end];
    removed.is_empty()
        && put.starts_with(char::is_whitespace)
        && before[..start]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_whitespace())
}

impl Typed {
    /// Whether the Note's own text holds nothing but spaces.
    pub fn empty(&self) -> bool {
        self.text.trim().is_empty()
    }
}

/// Whether the texts `a` and `b` read the same, as [`Typed::reads_as`].
pub fn same_text(a: &str, b: &str) -> bool {
    a == b || text_boxes::strip(a.trim()) == text_boxes::strip(b.trim())
}

/// The Note as one step holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub typed: Typed,
    pub kept: Kept,
}

impl State {
    /// Whether it is `other` for the person: the same words typed and the
    /// same written.
    pub fn reads_as(&self, other: &State) -> bool {
        self.kept == other.kept && self.typed.reads_as(&other.typed)
    }
}

/// When the Note is looked at for a step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum See {
    /// As it is drawn, at this time in seconds: typing that rested for
    /// [`PAUSE`] is a step.
    Drawn(f64),
    /// Before something asked of the window is done: what was typed is a
    /// step of its own, and what was written meanwhile came from
    /// elsewhere.
    Before,
    /// After something asked was done: what it changed is a step.
    After,
}

/// The steps of the Note open, from what it was as it opened.
#[derive(Debug, Clone)]
pub struct Steps {
    note: String,
    opened: State,
    at: State,
    back: Vec<State>,
    ahead: Vec<State>,
    /// What was typed last and when, while it is no step yet.
    typing: Option<(Typed, f64)>,
}

impl Steps {
    /// The steps of the Note `note`, opened as `state`.
    pub fn new(note: &str, state: State) -> Steps {
        Steps {
            note: note.to_string(),
            opened: state.clone(),
            at: state,
            back: Vec::new(),
            ahead: Vec::new(),
            typing: None,
        }
    }

    /// The Note whose steps they are.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// The Note as it opened, or as the steps started again.
    pub fn opened(&self) -> &State {
        &self.opened
    }

    /// The Note as the last step left it.
    pub fn at(&self) -> &State {
        &self.at
    }

    /// How many steps Undo can take back.
    pub fn depth(&self) -> usize {
        self.back.len()
    }

    /// What Undo goes back to.
    pub fn undo_to(&self) -> Option<&State> {
        self.back.last()
    }

    /// What Redo goes on to.
    pub fn redo_to(&self) -> Option<&State> {
        self.ahead.last()
    }

    /// Whether "Revert all changes" has something to go back to.
    pub fn changed(&self) -> bool {
        !self.at.reads_as(&self.opened)
    }

    /// Whether typing waits to become a step once it rests.
    pub fn typing(&self) -> bool {
        self.typing.is_some()
    }

    /// Takes the Note as it is `now` in, seen as `how` says; whether it
    /// made a step.
    pub fn see(&mut self, now: State, how: See) -> bool {
        let written = now.kept != self.at.kept;
        let typed = !now.typed.reads_as(&self.at.typed);
        if written && how != See::After {
            // Written elsewhere: the steps start again from it.
            *self = Steps::new(&self.note, now);
            return false;
        }
        if written || (typed && !now.typed.empty() && !matches!(how, See::Drawn(_))) {
            self.step(now);
            return true;
        }
        if !typed {
            // The same, the boxes perhaps with their new ids.
            self.at = now;
            self.typing = None;
            return false;
        }
        // A text emptied is kept as it was by a save: no step until
        // words are typed into it again.
        if now.typed.empty() {
            return false;
        }
        let See::Drawn(time) = how else {
            return false;
        };
        match &self.typing {
            Some((last, since)) if *last == now.typed => {
                if time - since >= PAUSE {
                    self.step(now);
                    return true;
                }
            }
            // A new word begun: the words before it are a step.
            Some((last, _)) if new_word(last, &now.typed) => {
                let words = State {
                    typed: last.clone(),
                    kept: now.kept.clone(),
                };
                self.step(words);
                self.typing = Some((now.typed, time));
                return true;
            }
            _ => self.typing = Some((now.typed, time)),
        }
        false
    }

    fn step(&mut self, now: State) {
        let before = std::mem::replace(&mut self.at, now);
        self.back.push(before);
        if self.back.len() > MOST {
            self.back.remove(0);
        }
        self.ahead.clear();
        self.typing = None;
    }

    /// Undo was done: the Note is `now`.
    pub fn undone(&mut self, now: State) {
        if self.back.pop().is_some() {
            let after = std::mem::replace(&mut self.at, now);
            self.ahead.push(after);
        }
        self.typing = None;
    }

    /// Redo was done: the Note is `now`.
    pub fn redone(&mut self, now: State) {
        if self.ahead.pop().is_some() {
            let before = std::mem::replace(&mut self.at, now);
            self.back.push(before);
        }
        self.typing = None;
    }

    /// "Revert all changes" was done: the Note is `now`, as it opened.
    /// Every step was taken back at once, and Redo makes them again one
    /// by one, as on the phone; nothing is lost.
    pub fn reverted(&mut self, now: State) {
        let after = std::mem::replace(&mut self.at, now);
        self.ahead.push(after);
        while let Some(state) = self.back.pop() {
            self.ahead.push(state);
        }
        // The Note as it opened is where it is now, no step ahead.
        if self.ahead.last().is_some_and(|s| s.reads_as(&self.at)) {
            self.ahead.pop();
        }
        self.typing = None;
    }

    /// Ends the steps where one could not be taken back as it was made
    /// (a Tally or a Session made or unmade): they start again from `now`.
    pub fn restart(&mut self, now: State) {
        *self = Steps::new(&self.note, now);
    }
}

#[cfg(test)]
mod tests;
