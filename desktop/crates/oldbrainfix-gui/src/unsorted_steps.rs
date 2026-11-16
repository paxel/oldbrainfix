//! Sorting Unsorted one Note after another (#313, Q718): once a step
//! sorts a Note Unsorted shows (files or merges it, adds it to a Note,
//! makes it a list or a Tally, or gives it a Routine, Reminder, Frog or
//! Session), the Note dialog opened there shows the next Unsorted Note,
//! the list selects its first card, and once Unsorted is empty the
//! Agenda shows. Ctrl+Z, bringing a sorted Note back, shows it where it
//! was sorted.

/// What Unsorted showed as a step began: its cards Unsorted then, in the
/// list's order, and the one of them open in the Note dialog, if one was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sorting {
    pub order: Vec<String>,
    pub open: Option<String>,
}

/// The Notes a step sorted before Unsorted moved on, and whether that
/// was in the Note dialog: Ctrl+Z, bringing one back, shows it there
/// again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedOn {
    pub notes: Vec<String>,
    pub dialog: bool,
}

/// The Note the dialog shows once `sorted` left Unsorted: the next one
/// after it in `order`, the list's order as the step began, that the list
/// still `shows`, else the first it shows; none when it shows none.
pub fn next_after(order: &[String], sorted: &str, shows: &[String]) -> Option<String> {
    let after = order
        .iter()
        .position(|id| id == sorted)
        .map_or(0, |at| at + 1);
    order
        .iter()
        .skip(after)
        .find(|id| shows.contains(id))
        .or_else(|| shows.first())
        .cloned()
}
