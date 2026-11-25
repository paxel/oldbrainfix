//! The trace of the clicks on the app's notifications (obf-1039): for
//! each, when it came, the notification's kind and ids, the button,
//! whether the window showed or waited in the tray, and where it went;
//! never a Note's text. The last [`KEPT`] are held in memory for as long
//! as the app runs, and go along with Send feedback where ticked, as the
//! version and the system do.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::alerts::{CATCH_UP_KEY, CHANGES_KEY, NUDGE_KEY, OPEN, Tap};

/// How many clicks the trace holds, the latest: eight lines of about 110
/// characters leave a mail link room for the words.
pub const KEPT: usize = 8;

/// One click on a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Click {
    pub at: DateTime<Utc>,
    /// The notification's key: [`NUDGE_KEY`], [`CHANGES_KEY`],
    /// [`CATCH_UP_KEY`] or a Routine's id with its Occurrence.
    pub key: String,
    /// The button, [`OPEN`] for a click on the notification itself.
    pub action: String,
    /// Whether the window showed as it came, else it waited in the tray.
    pub shown: bool,
    /// Where it went; `None` until the window has placed it.
    pub went: Option<String>,
}

impl Click {
    /// The click as one line of the feedback: "2026-11-24 09:00:03Z
    /// routine:…@2026-11-24T09:00 click, tray: Agenda today", an
    /// Occurrence's by its key.
    pub fn line(&self) -> String {
        let what = match self.key.as_str() {
            NUDGE_KEY => "Nudge",
            CHANGES_KEY => "Changes Nudge",
            CATCH_UP_KEY => "Catch-up",
            key => key,
        };
        let action = if self.action == OPEN {
            "click"
        } else {
            self.action.as_str()
        };
        let from = if self.shown { "window" } else { "tray" };
        let went = self.went.as_deref().unwrap_or("not placed");
        format!(
            "{} {what} {action}, {from}: {went}",
            self.at.format("%Y-%m-%d %H:%M:%SZ")
        )
    }
}

#[derive(Debug, Default)]
struct Held {
    clicks: VecDeque<Click>,
    shown: bool,
}

/// The trace, shared by the alerts, which hear the clicks, and the
/// window, which places them.
#[derive(Debug, Clone, Default)]
pub struct ClickTrace {
    held: Arc<Mutex<Held>>,
}

impl ClickTrace {
    /// Tells whether the window shows, for the clicks to come.
    pub fn set_shown(&self, shown: bool) {
        if let Ok(mut held) = self.held.lock() {
            held.shown = shown;
        }
    }

    /// Notes `tap` as it came, not yet placed; the oldest goes beyond
    /// [`KEPT`].
    pub fn arrived(&self, tap: &Tap) {
        if let Ok(mut held) = self.held.lock() {
            let shown = held.shown;
            held.clicks.push_back(Click {
                at: tap.at,
                key: tap.key.clone(),
                action: tap.action.clone(),
                shown,
                went: None,
            });
            while held.clicks.len() > KEPT {
                held.clicks.pop_front();
            }
        }
    }

    /// Notes where the latest click on the notification `key` not yet
    /// placed went.
    pub fn went(&self, key: &str, went: &str) {
        if let Ok(mut held) = self.held.lock()
            && let Some(click) = held
                .clicks
                .iter_mut()
                .rev()
                .find(|c| c.key == key && c.went.is_none())
        {
            click.went = Some(went.to_string());
        }
    }

    /// The clicks held, oldest first.
    pub fn clicks(&self) -> Vec<Click> {
        self.held
            .lock()
            .map(|held| held.clicks.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// The clicks held as the feedback's lines, oldest first.
    pub fn lines(&self) -> Vec<String> {
        self.clicks().iter().map(Click::line).collect()
    }
}

#[cfg(test)]
mod tests;
