//! A Session's part of its Agenda row (#104, ADR 0029): what it shows,
//! today its picks with their targets, drawn under the row's heading as
//! entries with boxes (#208), on later days how many it picks. A Session
//! Note's detailed card (#191, ADR 0039) shows the picks for today
//! under each of its Routines' times, after its pool, how often it was
//! Done and skipped in each of the last four weeks, and its latest past
//! runs (#264), which its page in the Note dialog lists all of.

use egui::{Id, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::{
    Day, Notes, Occurrence, Routines, Session, SessionOccurrence, SessionPick, SessionRule,
    SessionSetup, Sessions, Store, UnitSystem, WallClock,
};

use crate::app::row_title;
use crate::history_view;
use crate::icons;
use crate::l10n::L10n;
use crate::schedule_text::{day_text, occurrence_time};
use crate::tally_view::{decimal_sign, shown_value};

/// How many weeks of its history a Session Note's card shows (#191).
pub const CARD_WEEKS: i64 = 4;

/// How many past runs a Session Note's card shows (#264).
pub const CARD_RUNS: usize = 5;

/// One past run of a Session as it shows (#264): its day and time, and
/// each picked Tally's name with whether it was done.
#[derive(Debug, Clone, PartialEq)]
pub struct RunShown {
    pub when: String,
    pub picks: Vec<(String, bool)>,
}

/// The past runs of the Session `note_id`, newest first, as the phone's
/// `SessionRuns` shows them; read with the card or the Note, never while
/// drawn.
pub fn runs(
    store: &Store,
    t: &L10n,
    note_id: &str,
    wall: &WallClock,
) -> oldbrainfix_core::Result<Vec<RunShown>> {
    let notes = Notes::new(store);
    let mut out = Vec::new();
    for run in Sessions::new(store).runs(note_id, wall)? {
        let mut picks = Vec::new();
        for (note, done) in &run.picks {
            picks.push((row_title(t, notes.by_id(note)?.as_ref()), *done));
        }
        out.push(RunShown {
            when: format!(
                "{} · {}",
                day_text(t, run.occurrence.day),
                occurrence_time(t, &run.occurrence)
            ),
            picks,
        });
    }
    Ok(out)
}

/// Draws the past runs `shown`, at most `limit` when given with how many
/// older ones the Session's page shows; "No past runs yet." for none.
pub fn draw_runs(ui: &mut Ui, t: &L10n, shown: &[RunShown], limit: Option<usize>) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    if shown.is_empty() {
        ui.label(RichText::new(t.session_runs_none()).color(weak));
        return;
    }
    let count = limit.map_or(shown.len(), |l| l.min(shown.len()));
    for run in shown.iter().take(count) {
        ui.label(RichText::new(&run.when).strong().color(text));
        for (name, done) in &run.picks {
            ui.horizontal(|ui| {
                let icon = if *done {
                    icons::CHECKED
                } else {
                    icons::UNCHECKED
                };
                icons::show(ui, icon, 18.0, if *done { text } else { weak });
                let words = if *done {
                    t.session_run_done(name)
                } else {
                    t.session_run_not_done(name)
                };
                ui.add(egui::Label::new(RichText::new(words).color(text)).wrap());
            });
        }
    }
    let older = shown.len() - count;
    if older > 0 {
        let words = t.session_older_runs(older as i64);
        ui.label(RichText::new(words).small().color(weak));
    }
}

/// Today's Occurrence of a Session as its row shows it: the text of each
/// pick, whether each opens its entry form here, and whether they swap.
#[derive(Debug, Clone)]
pub struct PicksShown {
    pub today: SessionOccurrence,
    pub texts: Vec<String>,
    pub opens: Vec<bool>,
    pub swaps: bool,
}

impl PicksShown {
    /// `today` as this Device shows it, in the unit system `system`.
    pub fn read(
        store: &Store,
        t: &L10n,
        today: SessionOccurrence,
        system: UnitSystem,
    ) -> oldbrainfix_core::Result<PicksShown> {
        let id = today.session.id().to_string();
        let mut opens = Vec::new();
        for pick in &today.picks {
            opens.push(opens_pick(store, &id, pick)?);
        }
        Ok(PicksShown {
            texts: today
                .picks
                .iter()
                .map(|p| pick_text(t, p, system))
                .collect(),
            swaps: today.can_swap && store.may_check_off(&id)?,
            opens,
            today,
        })
    }
}

/// A Session Note's detailed card as it shows (#191): its pool, how many
/// it picks and how, today's picks of each of its Routines under their
/// time, and how often its Routines were Done and Missed in each of the
/// last weeks, this week first; read with the card, never while drawn.
#[derive(Debug, Clone)]
pub struct SessionCardShown {
    pub pool: Vec<String>,
    pub setup: String,
    pub today: Vec<(String, PicksShown)>,
    pub weeks: Vec<String>,
    /// Its past runs, newest first (#264).
    pub runs: Vec<RunShown>,
}

impl SessionCardShown {
    /// The card of the Session `setup` of its Note, with `today`, the
    /// Occurrences of its Routines today as the Agenda shows them, by time
    /// and then by Routine, written nowhere; days as `wall` shows them.
    pub fn read(
        store: &Store,
        t: &L10n,
        setup: &SessionSetup,
        today: Vec<SessionOccurrence>,
        (system, wall): (UnitSystem, &WallClock),
    ) -> oldbrainfix_core::Result<SessionCardShown> {
        let sessions = Sessions::new(store);
        let notes = Notes::new(store);
        let mut pool = Vec::new();
        for id in sessions.pool(setup)? {
            pool.push(row_title(t, notes.by_id(&id)?.as_ref()));
        }
        let setup_line = format!(
            "{} · {}",
            t.session_picks(setup.count),
            rule_name(t, setup.rule)
        );
        let mut shown = Vec::new();
        for occurrence in today {
            let time = occurrence_time(t, &occurrence.occurrence);
            shown.push((time, PicksShown::read(store, t, occurrence, system)?));
        }
        let runs = sessions.of_note(&setup.note_id)?;
        let weeks = weeks(store, t, &runs, wall)?;
        Ok(SessionCardShown {
            pool,
            setup: setup_line,
            today: shown,
            weeks,
            runs: self::runs(store, t, &setup.note_id, wall)?,
        })
    }
}

/// How often the Routines `runs` of a Session Note were Done and
/// skipped (#269) in each of the last [`CARD_WEEKS`] weeks, this week first, up to
/// today, weeks starting as this Device's language has them; none
/// without a Routine.
fn weeks(
    store: &Store,
    t: &L10n,
    runs: &[Session],
    wall: &WallClock,
) -> oldbrainfix_core::Result<Vec<String>> {
    if runs.is_empty() {
        return Ok(Vec::new());
    }
    let routines = Routines::new(store);
    let week_start = history_view::week_start(t);
    let mut histories = Vec::new();
    for run in runs {
        histories.push(routines.history(&run.routine, wall, week_start)?);
    }
    let today = Day::today(store.now(), wall);
    let back = (i64::from(today.weekday()) - i64::from(week_start)).rem_euclid(7);
    let start = today.add_days(-back);
    let mut lines = Vec::new();
    for i in 0..CARD_WEEKS {
        let from = start.add_days(-7 * i);
        let to = if i == 0 { today } else { from.add_days(6) };
        let (mut done, mut skipped) = (0, 0);
        for history in &histories {
            let week = history.adherence(from, to);
            done += week.done as i64;
            skipped += week.skipped as i64;
        }
        lines.push(t.session_history_week(&day_text(t, from), done, skipped));
    }
    Ok(lines)
}

/// The rule `rule` in words, the value of its badge.
pub fn rule_name(t: &L10n, rule: SessionRule) -> &'static str {
    match rule {
        SessionRule::Random => t.session_rule_random(),
        SessionRule::Rolling => t.session_rule_rolling(),
        SessionRule::Least => t.session_rule_least(),
    }
}

/// The rule `rule` as the value icon of its badge (#192), the phone's
/// `sessionRuleIcon`.
pub fn rule_icon(rule: SessionRule) -> char {
    match rule {
        SessionRule::Random => icons::SESSION_RULE,
        SessionRule::Rolling => icons::FORMAT_NUMBERS,
        SessionRule::Least => icons::SESSION_LEAST,
    }
}

/// Draws a Session Note's detailed card `shown`: its pool, its setup,
/// today's picks of each Routine under its time, each opening its entry
/// form and offering "Another one" as on the Agenda, then its last weeks.
pub fn card(ui: &mut Ui, t: &L10n, shown: &SessionCardShown, actions: &mut Vec<SessionAction>) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.style_mut().interaction.selectable_labels = false;
    ui.label(RichText::new(t.session_pool()).strong().color(text));
    for name in &shown.pool {
        ui.horizontal(|ui| {
            icons::show(ui, icons::TALLY, 18.0, text);
            ui.add(egui::Label::new(RichText::new(name).color(text)).wrap());
        });
    }
    ui.label(RichText::new(&shown.setup).size(13.0).color(weak));
    if !shown.today.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(t.agenda_today()).strong().color(text));
        for (time, picks_shown) in &shown.today {
            ui.label(RichText::new(time).color(text));
            ui.scope(|ui| picks(ui, t, picks_shown, actions));
        }
    }
    if !shown.weeks.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(t.session_history()).strong().color(text));
        for week in &shown.weeks {
            ui.label(RichText::new(week).color(text));
        }
    }
    ui.add_space(4.0);
    ui.label(RichText::new(t.session_runs()).strong().color(text));
    draw_runs(ui, t, &shown.runs, Some(CARD_RUNS));
}

/// What shows under a Session's title on the Agenda.
#[derive(Debug, Clone)]
pub enum SessionPart {
    /// Today: its picks.
    Today(Box<PicksShown>),
    /// A later day: how many it picks.
    Later(i64),
}

/// What the user asked of a Session's row.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionAction {
    /// Opens the entry form of a pick: a new entry recorded for the
    /// Occurrence, or the pick's latest entry to change.
    Open {
        session: String,
        occurrence: Occurrence,
        note_id: String,
        entry: Option<String>,
    },
    /// "Another one": swaps the Tally of this Note for the next.
    Another {
        session: String,
        occurrence: Occurrence,
        note_id: String,
    },
}

/// A picked Tally as its line shows: its Note's text and the targets of
/// its fields, each with the field's name when it has more than one
/// ("Push-ups 20 times").
pub fn pick_text(t: &L10n, pick: &SessionPick, system: UnitSystem) -> String {
    let name = row_title(t, pick.note.as_ref());
    let decimal = decimal_sign(t);
    let aimed: Vec<_> = pick
        .tally
        .as_ref()
        .map(|tally| tally.shown())
        .unwrap_or_default()
        .into_iter()
        .filter(|field| field.target.is_some())
        .collect();
    let targets: Vec<String> = aimed
        .iter()
        .filter_map(|field| {
            let shown = shown_value(field.target?, &field.unit, system, decimal);
            Some(if aimed.len() > 1 {
                format!("{} {shown}", field.name)
            } else {
                shown
            })
        })
        .collect();
    if targets.is_empty() {
        name
    } else {
        format!("{name} {}", targets.join(" · "))
    }
}

/// Whether this Device may open the entry form of `pick` of the Session
/// `session`: record for it, or change the entry it has.
pub fn opens_pick(
    store: &Store,
    session: &str,
    pick: &SessionPick,
) -> oldbrainfix_core::Result<bool> {
    let gone = pick.note.as_ref().is_none_or(|n| n.is_trashed()) || pick.tally.is_none();
    if gone {
        return Ok(false);
    }
    match pick.entries.last() {
        Some(entry) => store.may_change(&entry.id),
        None => Ok(store.may_check_off(session)? && store.may_write_in(&pick.note_id)?),
    }
}

/// Today's picks, each a line opening its entry form, and "Another one"
/// under each without an entry (the control goes below its text).
fn picks(ui: &mut Ui, t: &L10n, shown: &PicksShown, actions: &mut Vec<SessionAction>) {
    let text = ui.visuals().text_color();
    let today = &shown.today;
    if today.picks.is_empty() {
        ui.label(RichText::new(t.session_nothing_to_pick()).color(text));
        return;
    }
    let session = today.session.id();
    let occurrence = today.occurrence;
    ui.spacing_mut().item_spacing.y = 4.0;
    for (i, pick) in today.picks.iter().enumerate() {
        let words = shown.texts.get(i).map(String::as_str).unwrap_or_default();
        let opens = shown.opens.get(i).copied().unwrap_or(false);
        let line = ui.horizontal(|ui| {
            ui.style_mut().interaction.selectable_labels = false;
            let icon = if pick.done() {
                icons::PICK_DONE
            } else {
                icons::PICK_OPEN
            };
            icons::show(ui, icon, 18.0, text);
            let mut shown = RichText::new(words).color(text);
            if pick.done() {
                shown = shown.strikethrough();
            }
            ui.add(egui::Label::new(shown).wrap());
        });
        let id = Id::new(("session pick", session, occurrence.key(), &pick.note_id));
        let sense = if opens {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut open = ui.interact(line.response.rect, id, sense);
        open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, opens, words));
        if opens {
            open = open.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        if open.clicked() {
            actions.push(SessionAction::Open {
                session: session.to_string(),
                occurrence,
                note_id: pick.note_id.clone(),
                entry: pick.entries.last().map(|e| e.id.clone()),
            });
        }
        if shown.swaps && !pick.done() {
            let another = ui.horizontal(|ui| {
                ui.add_space(26.0);
                icons::show(ui, icons::SWAP, 18.0, text);
                ui.button(RichText::new(t.session_another()).color(text))
            });
            if another.inner.clicked() {
                actions.push(SessionAction::Another {
                    session: session.to_string(),
                    occurrence,
                    note_id: pick.note_id.clone(),
                });
            }
        }
    }
}
