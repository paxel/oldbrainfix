//! Backups on the desktop's screen (ADR 0010, 0016): the card in the
//! Settings that sets them up with a passphrase, saves the Recovery
//! Sheet, picks the folder, switches the weekly Backup and writes one
//! now; and the first start's view that restores one into the empty log
//! with the passphrase or the recovery key, or starts fresh.

use std::path::PathBuf;

use egui::{Id, RichText, TextEdit, Ui};
use oldbrainfix_core::backup::{self, Header, MIN_PASSPHRASE, Problem};
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
    let error = ui.visuals().error_fg_color;
    ui.label(RichText::new(t.backups_title()).strong().color(text));
    ui.label(RichText::new(t.backups_intro()).small().color(weak));
    if !store.backups_set_up().unwrap_or(false) {
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

/// The Recovery Sheet of `store` in the app's language, made now on the
/// computer `device`; none before Backups are set up.
pub fn sheet(t: &L10n, store: &Store, device: &str, wall: &Wall) -> Option<Vec<u8>> {
    let code = store.recovery_code().ok().flatten()?;
    let made = wall(store.now()).format("%Y-%m-%d %H:%M").to_string();
    let sheet = Sheet {
        title: t.recovery_sheet_pdf_title().to_string(),
        about: vec![
            t.recovery_sheet_made(&made),
            t.recovery_sheet_device(device),
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
    /// Choose the Backup file with the system's dialog.
    Pick,
    /// Restore the file chosen with what was typed.
    Restore,
    /// Start with an empty log.
    Fresh,
}

/// The first start's view: the Backup file chosen, with its header, and
/// what was typed to open it.
#[derive(Debug, Clone, Default)]
pub struct Welcome {
    pub file: Option<(PathBuf, Header)>,
    pub passphrase: String,
    pub code: String,
}

/// The first start's view's content.
pub fn welcome(
    ui: &mut Ui,
    t: &L10n,
    wall: &Wall,
    state: &mut Welcome,
    actions: &mut Vec<WelcomeAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.heading(RichText::new(t.welcome_title()).color(text));
    ui.label(RichText::new(t.restore_offer_detail()).color(text));
    ui.add_space(8.0);
    ui.label(RichText::new(t.restore_intro()).small().color(weak));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.restore_pick()).color(text))
            .clicked()
        {
            actions.push(WelcomeAction::Pick);
        }
        if ui
            .button(RichText::new(t.welcome_fresh()).color(text))
            .clicked()
        {
            actions.push(WelcomeAction::Fresh);
        }
    });
    let Some((_, header)) = &state.file else {
        return;
    };
    ui.add_space(8.0);
    let made = wall(header.created).format("%Y-%m-%d %H:%M").to_string();
    ui.label(RichText::new(t.restore_file(&made)).strong().color(text));
    secret(
        ui,
        "restore passphrase",
        t.backup_passphrase(),
        &mut state.passphrase,
    );
    ui.label(RichText::new(t.restore_or()).color(weak));
    ui.add(
        TextEdit::singleline(&mut state.code)
            .id(Id::new("restore code"))
            .hint_text(t.restore_code_label())
            .text_color(text)
            .desired_width(f32::INFINITY),
    );
    if ui
        .button(RichText::new(t.restore_start()).color(text))
        .clicked()
    {
        actions.push(WelcomeAction::Restore);
    }
}

/// Why a restore took nothing in, in words.
pub fn restore_failure(t: &L10n, error: &Error) -> String {
    match error {
        Error::Backup(Problem::NotABackup) => t.restore_not_a_backup(),
        Error::Backup(Problem::WrongSecret) => t.restore_wrong(),
        Error::Backup(Problem::Damaged) => t.restore_damaged(),
        Error::Backup(Problem::NotEmpty) => t.restore_not_empty(),
        _ => t.restore_failed(),
    }
    .to_string()
}

/// The backup key that opens `welcome`'s file with what was typed: the
/// recovery key when one was, else the passphrase.
pub fn unlock(t: &L10n, welcome: &Welcome) -> Result<[u8; backup::KEY_BYTES], String> {
    let Some((_, header)) = &welcome.file else {
        return Err(t.restore_not_a_backup().to_string());
    };
    let code = welcome.code.trim();
    if !code.is_empty() && recovery::decode_code(code).is_none() {
        return Err(t.restore_invalid_code().to_string());
    }
    let opened = if code.is_empty() {
        backup::unlock(header, Some(&welcome.passphrase), None)
    } else {
        backup::unlock(header, None, Some(code))
    };
    match opened {
        Ok(Some(key)) => Ok(key),
        Ok(None) => Err(t.restore_wrong().to_string()),
        Err(e) => Err(restore_failure(t, &e)),
    }
}
