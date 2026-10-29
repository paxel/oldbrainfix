//! Backups on the desktop's screen (ADR 0010, 0016): the card on the
//! Profile page that sets them up with a passphrase, or shows them set
//! up on another of the user's Devices, the backup key belonging to the
//! Profile; saves the Recovery Sheet, picks the folder, switches the
//! weekly Backup and writes one now; and the first start's view that
//! asks whether the user is new, adds a Device of their own or restores
//! a Backup into the empty log with the passphrase or the recovery key.

use std::path::PathBuf;

use egui::{Id, RichText, TextEdit, Ui};
use oldbrainfix_core::backup::{self, Header, MIN_PASSPHRASE, Problem};
use oldbrainfix_core::profile::OwnProfile;
use oldbrainfix_core::recovery::{self, Sheet};
use oldbrainfix_core::{Error, Store};

use crate::app::Wall;
use crate::l10n::L10n;

/// The local setting saying the first start's view was answered.
const RESTORE_OFFERED: &str = "restoreOffered";

/// Whether the first start's view is due: nothing written yet, and the
/// view not answered before.
pub fn restore_due(store: &Store) -> oldbrainfix_core::Result<bool> {
    Ok(store.current_seq()? == 0 && store.local_setting(RESTORE_OFFERED)?.is_none())
}

/// The first start's view was answered; it does not come again.
pub fn restore_answered(store: &Store) -> oldbrainfix_core::Result<()> {
    store.set_local_setting(RESTORE_OFFERED, "true")
}

/// What the user asked for on the Backups card.
#[derive(Debug, Clone, PartialEq)]
pub enum BackupAction {
    /// Set Backups up with the passphrase typed twice.
    SetUp,
    /// Save the Recovery Sheet with the system's dialog.
    SaveSheet,
    /// Choose the folder with the system's dialog.
    PickFolder,
    /// Write a Backup every week, or not.
    Weekly(bool),
    /// Write a Backup now.
    Now,
}

/// The passphrase as typed into the Backups card, and why it was not
/// taken.
#[derive(Debug, Clone, Default)]
pub struct Passphrase {
    pub first: String,
    pub again: String,
    pub problem: Option<String>,
}

impl Passphrase {
    /// The passphrase when both fields hold the same one, long enough;
    /// otherwise why not, kept to show.
    pub fn taken(&mut self, t: &L10n) -> Option<String> {
        self.problem = if self.first.trim().chars().count() < MIN_PASSPHRASE {
            Some(t.backup_passphrase_short().to_string())
        } else if self.first != self.again {
            Some(t.backup_passphrase_mismatch().to_string())
        } else {
            None
        };
        self.problem.is_none().then(|| self.first.clone())
    }
}

/// A password field with its hint, as wide as the card.
fn secret(ui: &mut Ui, id: &str, hint: &str, value: &mut String) {
    let text = ui.visuals().text_color();
    ui.add(
        TextEdit::singleline(value)
            .id(Id::new(id))
            .password(true)
            .hint_text(hint)
            .text_color(text)
            .desired_width(f32::INFINITY),
    );
}

/// How the last Backup went, in words, and whether that is a failure.
pub fn status(t: &L10n, store: &Store, wall: &Wall) -> (String, bool) {
    let when = |at| wall(at).format("%Y-%m-%d %H:%M").to_string();
    if let Some(failed) = store.backup_failed().ok().flatten() {
        return (t.backup_failed_at(&when(failed)), true);
    }
    match store.last_backup().ok().flatten() {
        Some(last) => (t.backup_last(&when(last)), false),
        None => (t.backup_not_yet().to_string(), false),
    }
}

/// The Backups card's content.
pub fn card(
    ui: &mut Ui,
    t: &L10n,
    store: &Store,
    wall: &Wall,
    form: &mut Passphrase,
    actions: &mut Vec<BackupAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.backups_title()).strong().color(text));
    ui.label(RichText::new(t.backups_intro()).small().color(weak));
    own_backups(ui, t, store, wall, form, actions);
}

/// The Backups: set up, the Recovery Sheet, the folder, the weekly
/// switch and "Back up now".
fn own_backups(
    ui: &mut Ui,
    t: &L10n,
    store: &Store,
    wall: &Wall,
    form: &mut Passphrase,
    actions: &mut Vec<BackupAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    if !store.backups_set_up().unwrap_or(false) {
        ui.label(RichText::new(t.backup_set_up_elsewhere()).color(text));
        secret(
            ui,
            "backup passphrase",
            t.backup_passphrase(),
            &mut form.first,
        );
        secret(
            ui,
            "backup passphrase again",
            t.backup_passphrase_again(),
            &mut form.again,
        );
        ui.label(
            RichText::new(t.backup_passphrase_hint())
                .small()
                .color(weak),
        );
        if let Some(problem) = &form.problem {
            ui.label(RichText::new(problem).color(error));
        }
        if ui
            .button(RichText::new(t.backup_set_up()).color(text))
            .clicked()
        {
            actions.push(BackupAction::SetUp);
        }
        return;
    }
    ui.label(RichText::new(t.backup_sheet_title()).strong().color(text));
    ui.label(RichText::new(t.backup_sheet_intro()).small().color(weak));
    if ui
        .button(RichText::new(t.backup_sheet_save()).color(text))
        .clicked()
    {
        actions.push(BackupAction::SaveSheet);
    }
    ui.label(RichText::new(t.backup_folder_title()).strong().color(text));
    let folder = store.backup_folder().ok().flatten();
    let shown = match &folder {
        Some(path) => t.backup_folder_chosen(path),
        None => t.backup_folder_none().to_string(),
    };
    ui.label(RichText::new(shown).color(text));
    if ui
        .button(RichText::new(t.backup_choose_folder()).color(text))
        .clicked()
    {
        actions.push(BackupAction::PickFolder);
    }
    if folder.is_none() {
        return;
    }
    let mut weekly = store.backups_weekly().unwrap_or(true);
    if ui
        .checkbox(&mut weekly, RichText::new(t.backup_weekly()).color(text))
        .changed()
    {
        actions.push(BackupAction::Weekly(weekly));
    }
    ui.label(RichText::new(t.backup_weekly_detail()).small().color(weak));
    let (line, failed) = status(t, store, wall);
    ui.label(RichText::new(line).color(if failed { error } else { text }));
    if ui
        .button(RichText::new(t.backup_now()).color(text))
        .clicked()
    {
        actions.push(BackupAction::Now);
    }
}

/// The Recovery Sheet of `store` in the app's language, made now for the
/// own Profile; none before Backups are set up.
pub fn sheet(t: &L10n, store: &Store, wall: &Wall) -> Option<Vec<u8>> {
    let code = store.recovery_code().ok().flatten()?;
    let made = wall(store.now()).format("%Y-%m-%d %H:%M").to_string();
    let name = OwnProfile::new(store)
        .get()
        .ok()
        .flatten()
        .map(|p| p.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| t.profile_no_name().to_string());
    let sheet = Sheet {
        title: t.recovery_sheet_pdf_title().to_string(),
        about: vec![
            t.recovery_sheet_made(&made),
            t.recovery_sheet_profile(&name),
        ],
        intro: vec![t.recovery_sheet_pdf_intro().to_string()],
        code_label: t.recovery_sheet_key().to_string(),
        steps: vec![
            t.recovery_sheet_step_restore().to_string(),
            t.recovery_sheet_step_settings().to_string(),
        ],
    };
    Some(recovery::sheet_pdf(&sheet, &code))
}

/// What the user asked for on the first start's view.
#[derive(Debug, Clone, PartialEq)]
pub enum WelcomeAction {
    /// New to Old Brain Fix: start with an empty log.
    New,
    /// Add a Device of one's own: pair with it and sync.
    AddDevice,
    /// Choose the Backup file with the system's dialog.
    Pick,
    /// Restore the file chosen with what was typed.
    Restore,
    /// A sync with the other Device went through; go on.
    Synced,
    /// Back to the three choices, nothing kept.
    Back,
}

/// The first start's view: the three choices, or adding a Device of
/// one's own, or the Backup file chosen, with its header, and what was
/// typed to open it.
#[derive(Debug, Clone, Default)]
pub struct Welcome {
    /// Whether a Device of one's own is being added.
    pub adding: bool,
    pub file: Option<(PathBuf, Header)>,
    pub passphrase: String,
    pub code: String,
    /// Whether the key typed is no recovery key; said red under the
    /// field until it changes.
    pub code_invalid: bool,
}

impl Welcome {
    /// Whether the view shows the three choices.
    pub fn choosing(&self) -> bool {
        !self.adding && self.file.is_none()
    }
}

/// The text, then its button below it (UI laws, "Dialogs and pages").
fn choice(ui: &mut Ui, detail: &str, label: &str) -> bool {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.add_space(8.0);
    ui.label(RichText::new(detail).color(weak));
    ui.add_sized(
        [ui.available_width(), 32.0],
        egui::Button::new(RichText::new(label).strong().color(text)),
    )
    .clicked()
}

/// The first start's view's content, but for the adding of a Device of
/// one's own, which the Sync view's pieces draw.
pub fn welcome(
    ui: &mut Ui,
    t: &L10n,
    wall: &Wall,
    state: &mut Welcome,
    actions: &mut Vec<WelcomeAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let Some((_, header)) = &state.file else {
        ui.heading(RichText::new(t.first_start_title()).color(text));
        ui.label(RichText::new(t.first_start_intro()).color(text));
        // Actions, each with what it means; not a choice of value.
        if choice(ui, t.welcome_new_detail(), t.first_start_new()) {
            actions.push(WelcomeAction::New);
        }
        if choice(ui, t.first_start_device_detail(), t.first_start_device()) {
            actions.push(WelcomeAction::AddDevice);
        }
        if choice(ui, t.restore_offer_detail(), t.restore_offer()) {
            actions.push(WelcomeAction::Pick);
        }
        return;
    };
    ui.heading(RichText::new(t.restore_offer()).color(text));
    ui.label(RichText::new(t.restore_intro()).small().color(weak));
    ui.add_space(8.0);
    let made = wall(header.created).format("%Y-%m-%d %H:%M").to_string();
    ui.label(RichText::new(t.restore_file(&made)).strong().color(text));
    if ui
        .button(RichText::new(t.restore_pick()).color(text))
        .clicked()
    {
        actions.push(WelcomeAction::Pick);
    }
    secret(
        ui,
        "restore passphrase",
        t.backup_passphrase(),
        &mut state.passphrase,
    );
    ui.label(RichText::new(t.restore_or()).color(weak));
    let code = ui.add(
        TextEdit::singleline(&mut state.code)
            .id(Id::new("restore code"))
            .hint_text(t.restore_code_label())
            .text_color(text)
            .desired_width(f32::INFINITY),
    );
    if code.changed() {
        state.code_invalid = false;
    }
    if state.code_invalid {
        let error = ui.visuals().error_fg_color;
        ui.label(RichText::new(t.restore_invalid_code()).color(error));
    }
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.restore_start()).color(text))
            .clicked()
        {
            actions.push(WelcomeAction::Restore);
        }
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            actions.push(WelcomeAction::Back);
        }
    });
}

/// Why a restore took nothing in, and what to do, one sentence each, as
/// the phone says them.
pub fn restore_failure(t: &L10n, error: &Error) -> (&'static str, &'static str) {
    match error {
        Error::Backup(Problem::NotABackup) => (t.restore_not_a_backup(), t.restore_pick_fix()),
        Error::Backup(Problem::WrongSecret) => (t.restore_wrong(), t.restore_wrong_fix()),
        Error::Backup(Problem::Damaged) => (t.restore_damaged_cause(), t.restore_damaged_fix()),
        Error::Backup(Problem::NotEmpty) => (t.restore_not_empty(), t.restore_not_empty_fix()),
        _ => (t.restore_failed_cause(), t.restore_failed_fix()),
    }
}

/// Why what was typed opens no Backup.
#[derive(Debug, Clone, PartialEq)]
pub enum Unlocked {
    /// The key typed is no recovery key: said under its field.
    NotAKey,
    /// Nothing opens it: why, what to do, and the full text.
    Failed(&'static str, &'static str, String),
}

/// The backup key that opens `welcome`'s file with what was typed: the
/// recovery key when one was, else the passphrase.
pub fn unlock(t: &L10n, welcome: &Welcome) -> Result<[u8; backup::KEY_BYTES], Unlocked> {
    let Some((_, header)) = &welcome.file else {
        return Err(Unlocked::Failed(
            t.restore_not_a_backup(),
            t.restore_pick_fix(),
            "backup: no file picked".into(),
        ));
    };
    let code = welcome.code.trim();
    if !code.is_empty() && recovery::decode_code(code).is_none() {
        return Err(Unlocked::NotAKey);
    }
    let opened = if code.is_empty() {
        backup::unlock(header, Some(&welcome.passphrase), None)
    } else {
        backup::unlock(header, None, Some(code))
    };
    match opened {
        Ok(Some(key)) => Ok(key),
        Ok(None) => Err(Unlocked::Failed(
            t.restore_wrong(),
            t.restore_wrong_fix(),
            "backup: the passphrase or recovery key does not open it".into(),
        )),
        Err(e) => {
            let (cause, fix) = restore_failure(t, &e);
            Err(Unlocked::Failed(cause, fix, format!("restore: {e}")))
        }
    }
}
