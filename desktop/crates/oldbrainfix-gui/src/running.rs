//! The page Running (obf-1078): every job that runs beside the window,
//! read from where each runs: a sync with a Device, an Import, the
//! Transcripts, Photo Texts and Attachment Texts being read, the speech
//! model being fetched, an Area being deleted, the trash being emptied,
//! a Pack being opened and the Agenda's first read. Each says what it is doing, how far it is
//! when that is known, since when, and offers Abort, which stops it at
//! its next safe point; the Agenda's first read, which the window waits
//! for, offers none. What Abort stopped stays listed under them while the
//! app runs, with how far it came, and what Abort paused offers Go on.
//! View ▸ Running and a click on the line under the menu bar open it.

use std::time::Instant;

use egui::{Frame, Margin, RichText, Ui};

use crate::enrich::Kind;
use crate::l10n::L10n;
use crate::theme::{self, Tones};

/// The most jobs stopped the page keeps listing, the newest first.
pub const MAX_ENDED: usize = 10;

/// Which job a row is, for its Abort and Go on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobId {
    /// The syncs of this Device: the shared folder's rounds and the
    /// answers to its paired Devices.
    Sync,
    Import,
    /// The Transcripts, Photo Texts or Attachment Texts being read.
    Reading(Kind),
    SpeechModel,
    /// Deleting the Area of this id.
    AreaDeletion(String),
    PackOpening,
    AgendaRead,
    /// The trash being emptied (obf-1095).
    TrashEmptying,
}

/// What Abort does for a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Abort {
    /// It can be pressed.
    Offered,
    /// It was pressed; the job stops at its next safe point.
    Stopping,
    /// The job cannot be stopped: the window waits for it.
    No,
}

/// One job running, as its row shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub id: JobId,
    /// What it is doing, in words.
    pub what: String,
    /// How far it is, in words, when that is known.
    pub far: Option<String>,
    /// Since when it runs, when that is known.
    pub since: Option<Instant>,
    pub abort: Abort,
    /// What Abort does, where it does more or less than stop at once.
    pub abort_does: Option<String>,
}

/// One job Abort stopped or paused, as the page lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ended {
    pub id: JobId,
    /// What it was doing, in words.
    pub what: String,
    /// How it ended: "Stopped: 120 of 340 imported".
    pub said: String,
    /// Whether it was only paused, Go on starting it again.
    pub goes_on: bool,
}

/// What a click on the page asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    Abort(JobId),
    GoOn(JobId),
}

/// The jobs Abort ended for good, the newest first, while the app runs;
/// the paused ones are read from where they pause, never dropped from
/// here.
#[derive(Default)]
pub struct Stopped {
    ended: Vec<Ended>,
}

impl Stopped {
    /// Lists `ended` first; the oldest beyond [`MAX_ENDED`] go.
    pub fn push(&mut self, ended: Ended) {
        self.ended.insert(0, ended);
        self.ended.truncate(MAX_ENDED);
    }

    pub fn list(&self) -> &[Ended] {
        &self.ended
    }
}

/// The page: its title, a row for each job of `jobs`, else a sentence
/// that nothing runs, and under them the jobs `ended`; `since` writes
/// when a job started. What was clicked.
pub fn page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (jobs, ended): (&[Job], &[Ended]),
    since: &dyn Fn(Instant) -> String,
) -> Option<Act> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut asked = None;
    egui::ScrollArea::vertical()
        .id_salt("running")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            // A page's title (obf-1121).
            ui.heading(RichText::new(t.running_title()).color(text));
            if jobs.is_empty() {
                ui.label(RichText::new(t.running_none()).color(weak));
            }
            for job in jobs {
                card(ui, tones, |ui| {
                    ui.label(RichText::new(&job.what).strong().color(text));
                    if let Some(far) = &job.far {
                        ui.label(RichText::new(far).color(text));
                    }
                    if let Some(at) = job.since {
                        ui.label(RichText::new(since(at)).color(weak));
                    }
                    if let Some(does) = &job.abort_does
                        && job.abort == Abort::Offered
                    {
                        ui.label(RichText::new(does).color(weak));
                    }
                    match job.abort {
                        Abort::Offered => {
                            if ui
                                .button(RichText::new(t.running_abort()).color(text))
                                .clicked()
                            {
                                asked = Some(Act::Abort(job.id.clone()));
                            }
                        }
                        Abort::Stopping => {
                            ui.label(RichText::new(t.running_stopping()).color(weak));
                        }
                        Abort::No => {}
                    }
                });
            }
            if ended.is_empty() {
                return;
            }
            ui.add_space(8.0);
            ui.label(RichText::new(t.running_stopped()).strong().color(text));
            for job in ended {
                card(ui, tones, |ui| {
                    ui.label(RichText::new(&job.what).strong().color(text));
                    ui.label(RichText::new(&job.said).color(text));
                    if job.goes_on
                        && ui
                            .button(RichText::new(t.running_go_on()).color(text))
                            .clicked()
                    {
                        asked = Some(Act::GoOn(job.id.clone()));
                    }
                });
            }
        });
    asked
}

/// One row of the page, as a card.
fn card(ui: &mut Ui, tones: Tones, add: impl FnOnce(&mut Ui)) {
    Frame::NONE
        .fill(tones.card)
        .corner_radius(theme::CARD_RADIUS)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 4.0;
            add(ui);
        });
}

#[cfg(test)]
mod tests;
