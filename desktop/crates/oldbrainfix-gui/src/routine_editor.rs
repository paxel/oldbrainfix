//! Making or changing a Routine or Reminder in the right pane, as the
//! phone's Routine editor does: the Note it comes from (or a text for a
//! new Note), its schedule in a simple editor (once, every N days, weeks,
//! months or years, chosen weekdays, every N months on a day or on the
//! Nth or last weekday, each but once ending never, on a day or after a
//! number of times) or as an RRULE in the advanced mode, its start day
//! and times of day, the next Occurrences, its own Re-nag
//! interval or this Device's default, and Escalation. A Session (#104)
//! has its pool, count, weekdays' counts and rule on top of the schedule
//! ([`SessionDraft`]); Save says under each field what to fix. Save
//! writes it; leaving without saving drops the edit. Opened from a
//! capture for the Note not written yet (#113), Save hands back what was
//! set up instead ([`RoutineEditor::choice`]), as the phone's
//! `chooseSchedule`.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::{Id, RichText, TextEdit, Ui};
use oldbrainfix_core::alarms::DEFAULT_ESCALATION;
use oldbrainfix_core::schedule::WeekdayRule;
use oldbrainfix_core::{
    ClockTime, Day, Note, Occurrence, Recurrence, RepeatEnd, RepeatKind, RepeatUnit, Routine,
    Routines, Schedule, Session, SimpleRepeat, Store,
};

use crate::app::{embossed, icon_button, pill, radio_column};
use crate::capture_choices::ScheduleChoice;
use crate::icons;
use crate::l10n::L10n;
use crate::schedule_text::{clock_text, day_text, occurrence_time, weekday_long, weekday_text};
use crate::session_editor::{SessionDraft, SessionIntent, WEEKDAYS_OPEN};
use crate::theme::Tones;

/// The Re-nag intervals offered, in minutes (the phone's choices).
pub const RENAG_CHOICES: [i64; 5] = [5, 10, 15, 30, 60];

/// The numbers of ignored Re-nags offered before Escalation.
pub const ESCALATION_CHOICES: [i64; 5] = [1, 2, 3, 5, 10];

/// How many coming Occurrences the editor shows.
const PREVIEW: usize = 3;

/// The most times a repeat can end after, as on the phone.
const MAX_COUNT: i64 = 99_999;

/// What the user asked of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Save,
    /// Delete, after asking.
    Delete,
    Close,
    /// Show the Note it comes from.
    OpenNote(String),
    /// Something asked of a Session's part.
    Session(SessionIntent),
}

/// One Routine or Reminder being made or changed.
#[derive(Debug, Clone)]
pub struct RoutineEditor {
    /// The Routine changed; `None` for a new one.
    pub routine: Option<String>,
    /// The Note it comes from: the one a new one spawns from, or the one
    /// an existing one links back to; `None` for a new one made with its
    /// Note behind the scenes, or when the Note is gone.
    pub note: Option<Note>,
    /// Whether it comes from a Note at all (it does unless new from text).
    from_note: bool,
    /// The text of the Note made behind the scenes.
    pub text: String,
    /// The text so far of the Note being captured, not written yet, that
    /// a new one is set up for; `None` otherwise.
    capture: Option<String>,
    pub advanced: bool,
    pub kind: RepeatKind,
    pub every: i64,
    pub unit: RepeatUnit,
    pub weekdays: BTreeSet<u32>,
    /// N in every N months, for the month kinds.
    pub months: i64,
    /// The day of the month; the start's until one is chosen.
    pub month_day: Option<i64>,
    /// 1 to 4 or -1 for the last, and the weekday; the start's position
    /// in its month until they are chosen.
    pub ordinal: Option<i64>,
    pub nth_weekday: Option<u32>,
    pub end: RepeatEnd,
    /// The end day and number, kept while another end is chosen.
    pub until: Option<Day>,
    /// The end day as typed.
    pub until_text: String,
    pub count: i64,
    /// The RRULE of the advanced mode.
    pub rule: String,
    pub start: Day,
    /// The start day as typed.
    pub start_text: String,
    pub times: Vec<ClockTime>,
    /// A time of day being typed in.
    pub time_text: String,
    /// Its own Re-nag interval in minutes; `None` for the Device's.
    pub renag: Option<i64>,
    /// After how many ignored Re-nags it escalates; `None` when off.
    pub escalation: Option<i64>,
    device_renag: i64,
    today: Day,
    /// What is edited now; `None` while it is no schedule.
    schedule: Option<Schedule>,
    rule_error: Option<String>,
    next: Vec<Occurrence>,
    /// A Session's part, with the Session as it was opened; `None` for a
    /// new one.
    session: Option<(SessionDraft, Option<Session>)>,
}

impl RoutineEditor {
    /// A new Routine, or a Reminder at the next full hour when
    /// `reminder`, spawned from `note` or, without, with its Note made
    /// from a text.
    pub fn new(
        note: Option<Note>,
        reminder: bool,
        now: DateTime<Utc>,
        wall: &dyn Fn(DateTime<Utc>) -> NaiveDateTime,
        device_renag: Duration,
    ) -> RoutineEditor {
        let shown = wall(now);
        let today = Day::of(shown);
        let (kind, start, times) = if reminder {
            // The next full hour, today or just after midnight.
            let next = Day::of(wall(now + Duration::hours(1)));
            let hour = (chrono::Timelike::hour(&shown) + 1) % 24;
            (
                RepeatKind::Once,
                next,
                ClockTime::new(hour, 0).into_iter().collect(),
            )
        } else {
            (
                RepeatKind::Every,
                today,
                ClockTime::new(8, 0).into_iter().collect(),
            )
        };
        let mut editor = RoutineEditor {
            routine: None,
            from_note: note.is_some(),
            note,
            text: String::new(),
            capture: None,
            advanced: false,
            kind,
            every: 1,
            unit: RepeatUnit::Days,
            weekdays: BTreeSet::new(),
            months: 1,
            month_day: None,
            ordinal: None,
            nth_weekday: None,
            end: RepeatEnd::Never,
            until: None,
            until_text: String::new(),
            count: 10,
            rule: String::new(),
            start,
            start_text: start.to_string(),
            times,
            time_text: String::new(),
            renag: None,
            escalation: None,
            device_renag: device_renag.num_minutes(),
            today,
            schedule: None,
            rule_error: None,
            next: Vec::new(),
            session: None,
        };
        editor.update();
        editor
    }

    /// A new Routine, or a Reminder when `reminder`, set up for the Note
    /// being captured with `text` so far: Save hands it back as a
    /// [`ScheduleChoice`] instead of writing it.
    pub fn choose(
        text: &str,
        reminder: bool,
        now: DateTime<Utc>,
        wall: &dyn Fn(DateTime<Utc>) -> NaiveDateTime,
        device_renag: Duration,
    ) -> RoutineEditor {
        let mut editor = RoutineEditor::new(None, reminder, now, wall, device_renag);
        editor.from_note = true;
        editor.capture = Some(text.trim().to_string());
        editor
    }

    /// Whether it is set up for the Note being captured.
    pub fn capturing(&self) -> bool {
        self.capture.is_some()
    }

    /// Makes it a Session: `draft` for its part, `session` as it was
    /// opened, `None` for a new one.
    pub fn with_session(mut self, draft: SessionDraft, session: Option<Session>) -> RoutineEditor {
        self.session = Some((draft, session));
        self
    }

    /// The Session's part, when it is one.
    pub fn session(&self) -> Option<&SessionDraft> {
        self.session.as_ref().map(|(draft, _)| draft)
    }

    /// The editor of `routine`, which links back to `note`.
    pub fn edit(
        routine: &Routine,
        note: Option<Note>,
        today: Day,
        device_renag: Duration,
    ) -> RoutineEditor {
        let schedule = &routine.schedule;
        let mut editor = RoutineEditor {
            routine: Some(routine.id.clone()),
            note,
            from_note: true,
            text: String::new(),
            capture: None,
            advanced: false,
            kind: RepeatKind::Every,
            every: 1,
            unit: RepeatUnit::Days,
            weekdays: BTreeSet::new(),
            months: 1,
            month_day: None,
            ordinal: None,
            nth_weekday: None,
            end: RepeatEnd::Never,
            until: None,
            until_text: String::new(),
            count: 10,
            rule: schedule
                .rule
                .as_ref()
                .map(Recurrence::format)
                .unwrap_or_default(),
            start: schedule.start,
            start_text: schedule.start.to_string(),
            times: schedule.times.clone(),
            time_text: String::new(),
            renag: routine.renag.map(|r| r.num_minutes()),
            escalation: routine.escalation,
            device_renag: device_renag.num_minutes(),
            today,
            schedule: None,
            rule_error: None,
            next: Vec::new(),
            session: None,
        };
        match SimpleRepeat::of(schedule.rule.as_ref()) {
            Some(simple) => editor.take_simple(&simple),
            None => editor.advanced = true,
        }
        editor.update();
        editor
    }

    fn take_simple(&mut self, simple: &SimpleRepeat) {
        self.kind = simple.kind;
        match simple.kind {
            RepeatKind::Once => {}
            RepeatKind::Every => {
                self.every = simple.every;
                self.unit = simple.unit;
            }
            RepeatKind::Weekdays => self.weekdays = simple.weekdays.clone(),
            RepeatKind::MonthDay => {
                self.months = simple.every;
                self.month_day = simple.month_day;
            }
            RepeatKind::MonthWeekday => {
                if let Some(nth) = simple.nth {
                    self.months = simple.every;
                    self.ordinal = nth.ordinal;
                    self.nth_weekday = Some(nth.weekday);
                }
            }
        }
        self.end = simple.end();
        if let Some(until) = simple.until {
            self.until = Some(until);
            self.until_text = until.to_string();
        }
        if let Some(count) = simple.count {
            self.count = count;
        }
    }

    /// Where `day` lies in its month: the 1st to 4th of its weekday, or
    /// -1 for a 5th, which is the last.
    fn ordinal_of(day: Day) -> i64 {
        let nth = i64::from((day.day - 1) / 7 + 1);
        if nth > 4 { -1 } else { nth }
    }

    /// The end day, or a month after the start while none is chosen.
    fn until_day(&self) -> Day {
        self.until.unwrap_or_else(|| {
            Day::new(
                self.start.year,
                i64::from(self.start.month) + 1,
                i64::from(self.start.day),
            )
        })
    }

    /// The rule the simple editor shows.
    fn simple_rule(&self) -> Option<Recurrence> {
        let count = (self.end == RepeatEnd::Count).then_some(self.count);
        let until = (self.end == RepeatEnd::Until).then(|| self.until_day());
        let simple = match self.kind {
            RepeatKind::Once => return None,
            RepeatKind::Every => SimpleRepeat::every(self.every, self.unit),
            RepeatKind::Weekdays => SimpleRepeat::weekdays(self.weekdays.clone()),
            RepeatKind::MonthDay => SimpleRepeat::month_day(
                self.month_day.unwrap_or(i64::from(self.start.day)),
                self.months,
            ),
            RepeatKind::MonthWeekday => SimpleRepeat::month_weekday(
                WeekdayRule {
                    weekday: self.nth_weekday.unwrap_or(self.start.weekday()),
                    ordinal: Some(self.ordinal.unwrap_or(Self::ordinal_of(self.start))),
                },
                self.months,
            ),
        };
        simple.ending(count, until).to_rule()
    }

    /// Whether what is edited happens once.
    pub fn once(&self) -> bool {
        match &self.schedule {
            Some(schedule) => schedule.once(),
            None if self.advanced => self.rule.trim().is_empty(),
            None => self.kind == RepeatKind::Once,
        }
    }

    /// Reads the edit into the schedule and its coming Occurrences.
    fn update(&mut self) {
        self.rule_error = None;
        let rule = if self.advanced {
            let text = self.rule.trim();
            if text.is_empty() {
                None
            } else {
                match Recurrence::parse(text) {
                    Ok(rule) => Some(rule),
                    Err(e) => {
                        // The offending part; the core's message is not
                        // translated.
                        self.rule_error = Some(e.source);
                        None
                    }
                }
            }
        } else {
            self.simple_rule()
        };
        let once = if self.advanced {
            self.rule.trim().is_empty()
        } else {
            self.kind == RepeatKind::Once
        };
        if once && self.times.len() > 1 {
            self.times.truncate(1);
        }
        let weekdays_missing =
            !self.advanced && self.kind == RepeatKind::Weekdays && self.weekdays.is_empty();
        self.schedule = if self.rule_error.is_some() || weekdays_missing {
            None
        } else {
            Schedule::new(self.start, &self.times, rule)
        };
        self.next = self
            .schedule
            .as_ref()
            .map(|s| s.next(self.today, PREVIEW))
            .unwrap_or_default();
    }

    /// The schedule as edited; `None` while it is none.
    pub fn schedule(&self) -> Option<&Schedule> {
        self.schedule.as_ref()
    }

    /// A Routine saves whatever became of its Note; a new one needs a Note
    /// in the lists to spawn from, or a text for one.
    pub fn can_save(&self) -> bool {
        if self.schedule.is_none() {
            return false;
        }
        if self.routine.is_some() || self.capture.is_some() {
            return true;
        }
        match &self.note {
            Some(note) => !note.is_trashed() && !note.is_merged(),
            None => !self.from_note && !self.text.trim().is_empty(),
        }
    }

    /// Whether a Session's part needs fixing, which then says so under
    /// its fields.
    fn session_to_fix(&mut self, store: &Store) -> oldbrainfix_core::Result<bool> {
        let Some((draft, _)) = &mut self.session else {
            return Ok(false);
        };
        if draft.valid() {
            return Ok(false);
        }
        let was_open = draft.weekdays_open;
        draft.check();
        // The weekday to fix comes into view, and stays so here.
        if draft.weekdays_open && !was_open {
            store.set_local_setting(WEEKDAYS_OPEN, "true")?;
        }
        Ok(true)
    }

    /// What was set up for the Note being captured, written nowhere;
    /// `None` while there is no schedule or a Session's part needs
    /// fixing, which then says so under its fields.
    pub fn choice(&mut self, store: &Store) -> oldbrainfix_core::Result<Option<ScheduleChoice>> {
        let Some(schedule) = self.schedule.clone() else {
            return Ok(None);
        };
        if self.session_to_fix(store)? {
            return Ok(None);
        }
        Ok(Some(ScheduleChoice {
            schedule,
            renag: self.renag.map(Duration::minutes),
            escalation: self.escalation,
            session: self.session.as_ref().map(|(draft, _)| draft.choice()),
        }))
    }

    /// Writes the Routine; its id, or `None` when a Session's part needs
    /// fixing first, which then says so under its fields and writes
    /// nothing.
    pub fn save(&mut self, store: &Store) -> oldbrainfix_core::Result<Option<String>> {
        if self.session_to_fix(store)? {
            return Ok(None);
        }
        let routines = Routines::new(store);
        let Some(schedule) = &self.schedule else {
            return Err(oldbrainfix_core::Error::Invalid("no schedule".into()));
        };
        let id = match (&self.routine, &self.note, &self.session) {
            (Some(id), _, session) => {
                routines.reschedule(id, schedule)?;
                if let Some((draft, Some(session))) = session {
                    draft.save(store, session)?;
                }
                id.clone()
            }
            (None, Some(note), Some((draft, _))) => draft.spawn(store, &note.id, schedule)?,
            (None, Some(note), None) => routines.spawn(&note.id, schedule)?.id,
            (None, None, _) => routines.create(&self.text, schedule)?.id,
        };
        routines.set_renag(&id, self.renag.map(Duration::minutes))?;
        routines.set_escalation(&id, self.escalation)?;
        Ok(Some(id))
    }

    /// Draws the editor; what was asked of it.
    pub fn show(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<Intent> {
        let mut intent = None;
        let mut changed = false;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let once = self.once();
        let heading = match (&self.routine, once) {
            _ if self.session.is_some() && self.routine.is_none() => t.new_session(),
            _ if self.session.is_some() => t.session(),
            (None, true) => t.new_reminder(),
            (None, false) => t.new_routine(),
            (Some(_), true) => t.reminder(),
            (Some(_), false) => t.routine(),
        };
        egui::ScrollArea::vertical()
            .id_salt("routine")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                ui.label(RichText::new(heading).size(18.0).strong().color(text));
                embossed(ui, tones, false, None, |ui| {
                    if !self.from_note {
                        ui.add(
                            TextEdit::multiline(&mut self.text)
                                .id(Id::new("routine text"))
                                .hint_text(t.routine_what_hint())
                                .text_color(text)
                                .desired_rows(1)
                                .desired_width(f32::INFINITY)
                                .frame(egui::Frame::NONE),
                        );
                        return;
                    }
                    ui.label(RichText::new(t.from_note()).strong().color(text));
                    if let Some(captured) = &self.capture {
                        let shown = if captured.is_empty() {
                            t.note_being_captured()
                        } else {
                            captured.as_str()
                        };
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::NOTE, 18.0, text);
                            ui.label(RichText::new(shown).color(text));
                        });
                        return;
                    }
                    match &self.note {
                        None => drop(ui.label(RichText::new(t.note_deleted()).color(weak))),
                        Some(note) if note.is_trashed() => {
                            ui.label(RichText::new(note.visible_text()).color(text));
                            ui.label(RichText::new(t.note_in_trash()).color(weak));
                        }
                        Some(note) => {
                            let link = ui.horizontal(|ui| {
                                icons::show(ui, icons::NOTE, 18.0, text);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(note.visible_text()).color(text),
                                    )
                                    .sense(egui::Sense::click()),
                                )
                            });
                            if link
                                .inner
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                intent = Some(Intent::OpenNote(note.id.clone()));
                            }
                        }
                    }
                });
                if let Some((draft, _)) = &mut self.session
                    && let Some(asked) = draft.show(ui, t, tones)
                {
                    intent = Some(Intent::Session(asked));
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.horizontal(|ui| {
                        for (advanced, label) in
                            [(false, t.schedule_simple()), (true, t.schedule_advanced())]
                        {
                            if pill(ui, tones, self.advanced == advanced, label).clicked()
                                && self.advanced != advanced
                            {
                                self.switch_mode(advanced);
                                changed = true;
                            }
                        }
                    });
                    if self.advanced {
                        changed |= self.advanced_editor(ui, t);
                    } else {
                        changed |= self.simple_editor(ui, t, tones);
                    }
                    changed |= self.start_editor(ui, t, once);
                    if !self.advanced && self.kind != RepeatKind::Once {
                        changed |= self.end_editor(ui, t);
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    changed |= self.times_editor(ui, t, tones, once);
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.next_occurrences()).strong().color(text));
                    if self.schedule.is_some() && self.next.is_empty() {
                        ui.label(RichText::new(t.never_due()).color(text));
                    }
                    for o in &self.next {
                        let line = format!("{}  {}", day_text(t, o.day), occurrence_time(t, o));
                        ui.label(RichText::new(line).color(text));
                    }
                });
                embossed(ui, tones, false, None, |ui| self.renag_editor(ui, t));
                embossed(ui, tones, false, None, |ui| self.escalation_editor(ui, t));
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
                        intent = Some(Intent::Close);
                    }
                    if self.routine.is_some()
                        && ui
                            .button(RichText::new(t.delete()).color(ui.visuals().error_fg_color))
                            .clicked()
                    {
                        intent = Some(Intent::Delete);
                    }
                    let save = egui::Button::new(RichText::new(t.save()).color(text));
                    if ui.add_enabled(self.can_save(), save).clicked() {
                        intent = Some(Intent::Save);
                    }
                });
            });
        if changed {
            self.update();
        }
        intent
    }

    fn switch_mode(&mut self, advanced: bool) {
        if advanced {
            self.rule = self.simple_rule().map(|r| r.format()).unwrap_or_default();
        } else {
            let text = self.rule.trim();
            let simple = if text.is_empty() {
                Some(SimpleRepeat::once())
            } else {
                Recurrence::parse(text)
                    .ok()
                    .and_then(|rule| SimpleRepeat::of(Some(&rule)))
            };
            if let Some(simple) = simple {
                self.take_simple(&simple);
            }
        }
        self.advanced = advanced;
    }

    fn simple_editor(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> bool {
        let text = ui.visuals().text_color();
        let mut kind = self.kind;
        let kinds = [
            (RepeatKind::Once, t.repeat_once()),
            (RepeatKind::Every, t.repeat_every()),
            (RepeatKind::Weekdays, t.repeat_weekdays()),
            (RepeatKind::MonthDay, t.repeat_month_day()),
            (RepeatKind::MonthWeekday, t.repeat_month_weekday()),
        ];
        radio_column(
            ui,
            &mut kind,
            kinds.map(|(k, label)| (k, label.to_string())),
        );
        let mut changed = kind != self.kind;
        if changed {
            self.kind = kind;
            match kind {
                RepeatKind::Weekdays if self.weekdays.is_empty() => {
                    self.weekdays.insert(self.start.weekday());
                }
                RepeatKind::MonthDay => {
                    self.month_day.get_or_insert(i64::from(self.start.day));
                }
                RepeatKind::MonthWeekday => {
                    self.ordinal.get_or_insert(Self::ordinal_of(self.start));
                    self.nth_weekday.get_or_insert(self.start.weekday());
                }
                _ => {}
            }
        }
        if self.kind == RepeatKind::Every {
            ui.label(RichText::new(t.every_label()).color(text));
            changed |= stepper(ui, t, &mut self.every, (1, None), None);
            let unit_text = |unit: RepeatUnit| match unit {
                RepeatUnit::Days => t.unit_days(self.every),
                RepeatUnit::Weeks => t.unit_weeks(self.every),
                RepeatUnit::Months => t.unit_months(self.every),
                RepeatUnit::Years => t.unit_years(self.every),
            };
            let mut unit = self.unit;
            let units = [
                RepeatUnit::Days,
                RepeatUnit::Weeks,
                RepeatUnit::Months,
                RepeatUnit::Years,
            ];
            radio_column(ui, &mut unit, units.map(|u| (u, unit_text(u))));
            if unit != self.unit {
                self.unit = unit;
                changed = true;
            }
        }
        if self.kind == RepeatKind::Weekdays {
            ui.horizontal_wrapped(|ui| {
                for day in 1..=7 {
                    let on = self.weekdays.contains(&day);
                    if pill(ui, tones, on, weekday_text(t, day)).clicked() {
                        if on {
                            self.weekdays.remove(&day);
                        } else {
                            self.weekdays.insert(day);
                        }
                        changed = true;
                    }
                }
            });
        }
        if matches!(self.kind, RepeatKind::MonthDay | RepeatKind::MonthWeekday) {
            changed |= self.month_editor(ui, t);
        }
        changed
    }

    /// The controls of the month kinds: every N months, and the day or
    /// the Nth weekday.
    fn month_editor(&mut self, ui: &mut Ui, t: &L10n) -> bool {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.every_label()).color(text));
        let unit = t.unit_months(self.months);
        let mut changed = stepper(ui, t, &mut self.months, (1, None), Some(&unit));
        if self.kind == RepeatKind::MonthDay {
            let mut day = self.month_day.unwrap_or(i64::from(self.start.day));
            ui.label(RichText::new(t.day_of_month_label()).color(text));
            if day > 28 {
                ui.label(RichText::new(t.month_day_skipped()).small().color(weak));
            }
            if stepper(ui, t, &mut day, (1, Some(31)), None) {
                self.month_day = Some(day);
                changed = true;
            }
            return changed;
        }
        let mut ordinal = self.ordinal.unwrap_or(Self::ordinal_of(self.start));
        ui.label(RichText::new(t.which_in_month_label()).color(text));
        let ordinals = [1, 2, 3, 4, -1].map(|n| (n, t.nth_option(&n.to_string())));
        if radio_column(ui, &mut ordinal, ordinals) {
            self.ordinal = Some(ordinal);
            changed = true;
        }
        let mut weekday = self.nth_weekday.unwrap_or(self.start.weekday());
        ui.label(RichText::new(t.weekday_label()).color(text));
        let weekdays = (1..=7).map(|day| (day, weekday_long(t, day).to_string()));
        if radio_column(ui, &mut weekday, weekdays) {
            self.nth_weekday = Some(weekday);
            changed = true;
        }
        changed
    }

    /// How a repeat ends: never, on a day, or after a number of times.
    fn end_editor(&mut self, ui: &mut Ui, t: &L10n) -> bool {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.ends_label()).color(text));
        let mut end = self.end;
        let ends = [
            (RepeatEnd::Never, t.end_never()),
            (RepeatEnd::Until, t.end_on_day()),
            (RepeatEnd::Count, t.end_after_count()),
        ];
        radio_column(ui, &mut end, ends.map(|(e, label)| (e, label.to_string())));
        let mut changed = end != self.end;
        if changed {
            self.end = end;
            // The day offered first is kept from here on.
            if end == RepeatEnd::Until && self.until.is_none() {
                let day = self.until_day();
                self.until = Some(day);
                self.until_text = day.to_string();
            }
        }
        match self.end {
            RepeatEnd::Never => {}
            RepeatEnd::Until => {
                ui.label(RichText::new(t.end_last_day()).color(text));
                let mut day = self.until_day();
                let stepped = ui
                    .horizontal_wrapped(|ui| {
                        day_stepper(ui, t, &mut day, &mut self.until_text, "until day")
                    })
                    .inner;
                if stepped {
                    self.until = Some(day);
                    changed = true;
                }
                if Day::parse(self.until_text.trim()) != Some(day) {
                    ui.label(RichText::new(t.date_invalid()).color(ui.visuals().error_fg_color));
                }
            }
            RepeatEnd::Count => {
                ui.label(RichText::new(t.end_count_label()).color(text));
                ui.label(RichText::new(t.end_count_explain()).small().color(weak));
                let unit = t.unit_times(self.count);
                changed |= stepper(ui, t, &mut self.count, (1, Some(MAX_COUNT)), Some(&unit));
            }
        }
        changed
    }

    fn advanced_editor(&mut self, ui: &mut Ui, t: &L10n) -> bool {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.rrule_label()).color(text));
        let field = ui.add(
            TextEdit::singleline(&mut self.rule)
                .id(Id::new("rrule"))
                .hint_text(t.rrule_label())
                .text_color(text)
                .desired_width(f32::INFINITY),
        );
        ui.label(RichText::new(t.rrule_helper()).small().color(weak));
        if let Some(detail) = &self.rule_error {
            ui.label(RichText::new(t.rrule_invalid(detail)).color(ui.visuals().error_fg_color));
        }
        field.changed()
    }

    fn start_editor(&mut self, ui: &mut Ui, t: &L10n, once: bool) -> bool {
        let text = ui.visuals().text_color();
        let changed = ui
            .horizontal_wrapped(|ui| {
                ui.label(RichText::new(if once { t.on_day() } else { t.starts_on() }).color(text));
                day_stepper(ui, t, &mut self.start, &mut self.start_text, "start day")
            })
            .inner;
        if Day::parse(self.start_text.trim()) != Some(self.start) {
            ui.label(RichText::new(t.date_invalid()).color(ui.visuals().error_fg_color));
        }
        changed
    }

    fn times_editor(&mut self, ui: &mut Ui, t: &L10n, tones: Tones, once: bool) -> bool {
        let mut changed = false;
        let text = ui.visuals().text_color();
        ui.label(RichText::new(t.times_heading()).strong().color(text));
        ui.horizontal_wrapped(|ui| {
            if self.times.is_empty() {
                pill(ui, tones, false, t.all_day());
            }
            let mut removed = None;
            for time in &self.times {
                ui.label(RichText::new(clock_text(*time)).strong().color(text));
                let label = t.remove_time(&clock_text(*time));
                if icon_button(ui, icons::CLOSE, &label, text).clicked() {
                    removed = Some(*time);
                }
            }
            if let Some(gone) = removed {
                self.times.retain(|kept| *kept != gone);
                changed = true;
            }
        });
        if !once || self.times.is_empty() {
            ui.horizontal(|ui| {
                let field = ui.add(
                    TextEdit::singleline(&mut self.time_text)
                        .id(Id::new("new time"))
                        .hint_text(t.time_hint())
                        .text_color(text)
                        .desired_width(70.0),
                );
                let typed = ClockTime::parse(self.time_text.trim());
                let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let add = ui.add_enabled(
                    typed.is_some(),
                    egui::Button::new(RichText::new(t.add_time()).color(text)),
                );
                if let Some(time) = typed
                    && (add.clicked() || entered)
                {
                    if !self.times.contains(&time) {
                        self.times.push(time);
                        self.times.sort();
                    }
                    self.time_text.clear();
                    changed = true;
                }
            });
            if !self.time_text.trim().is_empty()
                && ClockTime::parse(self.time_text.trim()).is_none()
            {
                ui.label(RichText::new(t.time_invalid()).color(ui.visuals().error_fg_color));
            }
        }
        changed
    }

    fn renag_editor(&mut self, ui: &mut Ui, t: &L10n) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let own = self.renag;
        let device = self.device_renag;
        ui.label(RichText::new(t.renag_heading()).strong().color(text));
        ui.label(RichText::new(t.renag_summary(own.unwrap_or(device))).color(text));
        ui.label(RichText::new(t.renag_explain()).small().color(weak));
        // A value set elsewhere stays pickable.
        let mut choices: BTreeSet<i64> = RENAG_CHOICES.into_iter().collect();
        choices.extend(own);
        let label = |m: Option<i64>| match m {
            None => t.renag_device_default(device),
            Some(m) => t.renag_minutes(m),
        };
        ui.label(RichText::new(t.renag_this()).color(text));
        let mut picked = own;
        let options = std::iter::once(None)
            .chain(choices.into_iter().map(Some))
            .map(|m| (m, label(m)));
        radio_column(ui, &mut picked, options);
        self.renag = picked;
    }

    fn escalation_editor(&mut self, ui: &mut Ui, t: &L10n) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.escalation_heading()).strong().color(text));
        let summary = match self.escalation {
            None => t.escalation_off().to_string(),
            Some(after) => t.escalation_summary(after),
        };
        ui.label(RichText::new(summary).color(text));
        ui.label(RichText::new(t.escalation_explain()).small().color(weak));
        let mut on = self.escalation.is_some();
        if ui
            .checkbox(&mut on, RichText::new(t.escalation_switch()).color(text))
            .changed()
        {
            self.escalation = on.then_some(DEFAULT_ESCALATION);
        }
        if let Some(after) = self.escalation {
            ui.label(RichText::new(t.escalation_after()).color(text));
            let mut choices: BTreeSet<i64> = ESCALATION_CHOICES.into_iter().collect();
            choices.insert(after);
            let mut picked = after;
            radio_column(
                ui,
                &mut picked,
                choices.into_iter().map(|n| (n, n.to_string())),
            );
            self.escalation = Some(picked);
        }
    }
}

/// A number from `min` to `max` with a minus and a plus button, and its
/// `unit` after it; whether it changed.
fn stepper(
    ui: &mut Ui,
    t: &L10n,
    value: &mut i64,
    (min, max): (i64, Option<i64>),
    unit: Option<&str>,
) -> bool {
    let text = ui.visuals().text_color();
    let mut changed = false;
    ui.horizontal(|ui| {
        if *value > min && icon_button(ui, icons::REMOVE, t.fewer(), text).clicked() {
            *value -= 1;
            changed = true;
        }
        ui.label(RichText::new(value.to_string()).strong().color(text));
        if max.is_none_or(|max| *value < max)
            && icon_button(ui, icons::ADD, t.more(), text).clicked()
        {
            *value += 1;
            changed = true;
        }
        if let Some(unit) = unit {
            ui.label(RichText::new(unit).color(text));
        }
    });
    changed
}

/// A day with the day before and after as buttons and a field to type it
/// in, `text` as typed, in a row; whether `day` changed.
fn day_stepper(ui: &mut Ui, t: &L10n, day: &mut Day, text: &mut String, id: &str) -> bool {
    let colour = ui.visuals().text_color();
    let mut changed = false;
    if icon_button(ui, icons::PREVIOUS, t.day_before(), colour).clicked() {
        *day = day.add_days(-1);
        changed = true;
    }
    ui.label(RichText::new(day_text(t, *day)).strong().color(colour));
    if icon_button(ui, icons::NEXT, t.day_after(), colour).clicked() {
        *day = day.add_days(1);
        changed = true;
    }
    if changed {
        *text = day.to_string();
    }
    let field = ui.add(
        TextEdit::singleline(text)
            .id(Id::new(id))
            .hint_text(t.date_hint())
            .text_color(colour)
            .desired_width(110.0),
    );
    if field.changed()
        && let Some(typed) = Day::parse(text.trim())
    {
        *day = typed;
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
        utc.naive_utc()
    }

    fn now() -> DateTime<Utc> {
        oldbrainfix_core::parse_time("2026-10-24T07:30:00.000Z").unwrap_or_default()
    }

    #[test]
    fn a_new_reminder_starts_at_the_next_full_hour() {
        let editor = RoutineEditor::new(None, true, now(), &utc_wall, Duration::minutes(10));
        assert!(editor.once());
        assert_eq!(
            editor.times,
            ClockTime::new(8, 0).into_iter().collect::<Vec<_>>()
        );
        assert_eq!(editor.start, Day::new(2026, 10, 24));
        // No Note and no text: nothing to save yet.
        assert!(!editor.can_save());
    }

    #[test]
    fn modes_carry_the_rule_across() {
        let mut editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
        editor.kind = RepeatKind::Weekdays;
        editor.weekdays = [1, 6].into_iter().collect();
        editor.switch_mode(true);
        editor.update();
        assert_eq!(editor.rule, "FREQ=WEEKLY;BYDAY=MO,SA");
        editor.rule = "FREQ=MONTHLY;BYDAY=1MO".into();
        editor.update();
        assert!(editor.schedule().is_some());
        editor.rule = "FREQ=HOURLY".into();
        editor.update();
        assert_eq!(editor.rule_error.as_deref(), Some("HOURLY"));
        assert!(editor.schedule().is_none());
        editor.rule = "FREQ=DAILY;INTERVAL=3".into();
        editor.switch_mode(false);
        editor.update();
        assert_eq!(
            (editor.kind, editor.every, editor.unit),
            (RepeatKind::Every, 3, RepeatUnit::Days)
        );
    }

    #[test]
    fn the_monthly_kinds_and_ends_stay_in_the_simple_mode() {
        let mut editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
        for rule in [
            "FREQ=MONTHLY;INTERVAL=2;COUNT=5;BYDAY=-1FR",
            "FREQ=MONTHLY;UNTIL=20270131;BYMONTHDAY=31",
            "FREQ=WEEKLY;COUNT=8;BYDAY=MO,TH",
        ] {
            editor.advanced = true;
            editor.rule = rule.into();
            editor.update();
            editor.switch_mode(false);
            editor.update();
            assert!(!editor.advanced, "{rule}");
            let Some(rule_back) = editor.simple_rule() else {
                panic!("a rule for {rule}")
            };
            assert_eq!(rule_back.format(), rule);
        }
        assert_eq!(editor.end, RepeatEnd::Count);
        assert_eq!(editor.count, 8);
        // The last-Friday kind kept its choices while another was shown.
        editor.kind = RepeatKind::MonthWeekday;
        assert_eq!((editor.ordinal, editor.nth_weekday), (Some(-1), Some(5)));
    }

    #[test]
    fn a_new_monthly_repeat_starts_from_the_start_day() {
        // Saturday, 24 October 2026: the fourth Saturday of its month.
        let mut editor = RoutineEditor::new(None, false, now(), &utc_wall, Duration::minutes(10));
        editor.kind = RepeatKind::MonthWeekday;
        editor.update();
        assert_eq!(
            editor.simple_rule().map(|r| r.format()).as_deref(),
            Some("FREQ=MONTHLY;BYDAY=4SA")
        );
        editor.kind = RepeatKind::MonthDay;
        editor.end = RepeatEnd::Until;
        editor.update();
        // A month after the start while no end day is chosen.
        assert_eq!(
            editor.simple_rule().map(|r| r.format()).as_deref(),
            Some("FREQ=MONTHLY;UNTIL=20261124;BYMONTHDAY=24")
        );
        // The 31st of October is a fifth weekday: the last.
        assert_eq!(RoutineEditor::ordinal_of(Day::new(2026, 10, 31)), -1);
        assert_eq!(RoutineEditor::ordinal_of(Day::new(2026, 10, 28)), 4);
    }
}
