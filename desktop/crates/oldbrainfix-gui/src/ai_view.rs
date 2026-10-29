//! The AI port on the desktop's screen (#112, ADR 0026): the Settings
//! card that opens it for one AI Task, shows where a client reaches it
//! and copies the client's config, and "Done, review", which closes it;
//! and the review of the AI Task, its changes grouped by kind, each with
//! what the log holds and what the task wrote, accepted or rejected as a
//! whole. While an AI Task is open or waits for its review, sync waits;
//! the window says so where a sync, a bundle, an Area offer or a Sent
//! Item was refused for it.

use egui::{Id, RichText, Ui};
use oldbrainfix_core::ai_port::{bridged_client_config, claude_code_command, client_config};
use oldbrainfix_core::ai_task::{AiChangeKind, AiFieldChange, AiReviewItem};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::places::keys as place_keys;
use oldbrainfix_core::routines::keys as routine_keys;
use oldbrainfix_core::{Importance, Notes, Places, Routines, Store, StoredText, parse_time};

use crate::app::{clickable_card, embossed, row_title, when};
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::schedule_text::schedule_text;
use crate::theme::Tones;

/// What the user asked of the AI port's card or of the review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAction {
    /// Opens the port, for the AI Task that waits or a new one.
    Open,
    /// Copies what this client needs to reach the port.
    Copy(Client),
    /// Closes the port and shows the review.
    Done,
    /// Shows the review of the AI Task that waits.
    Review,
    /// Keeps every change of the AI Task.
    Accept,
    /// Asks whether to reject every change of the AI Task, and does.
    AskReject,
    Reject,
}

/// The clients the card copies a config for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    /// Any client that reads `mcpServers` and speaks HTTP.
    Config,
    /// Claude Code's command line.
    ClaudeCode,
    /// Claude Desktop, through `mcp-remote`.
    ClaudeDesktop,
}

/// What `client` needs to reach the port at `url` with `token`.
pub fn client_text(client: Client, url: &str, token: &str) -> String {
    match client {
        Client::Config => client_config(url, token),
        Client::ClaudeCode => claude_code_command(url, token),
        Client::ClaudeDesktop => bridged_client_config(url, token),
    }
}

/// The review of an AI Task as it shows: its groups in order, each item
/// with what it is about and its changes in words.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReviewShown {
    /// The AI Task's id.
    pub task: String,
    pub groups: Vec<(AiChangeKind, Vec<ReviewItem>)>,
}

/// One Note, Section, Routine, Area or Topic the AI Task changes: its
/// title as the log holds it, the Note to open for it, and its changes.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewItem {
    pub entity: String,
    pub title: String,
    pub note: Option<String>,
    pub changes: Vec<ReviewChange>,
}

/// One change in words: what changes, before and after; neither for a
/// deletion. `mine_wins` when the user's own edit came later and stays.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewChange {
    pub label: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub mine_wins: bool,
}

/// What a group of the review is called.
pub fn kind_label(t: &L10n, kind: AiChangeKind) -> &'static str {
    match kind {
        AiChangeKind::NoteAdded => t.ai_kind_note_added(),
        AiChangeKind::NoteFiled => t.ai_kind_note_filed(),
        AiChangeKind::NoteMerged => t.ai_kind_note_merged(),
        AiChangeKind::NoteTrashed => t.ai_kind_note_trashed(),
        AiChangeKind::NoteRestored => t.ai_kind_note_restored(),
        AiChangeKind::NoteText => t.ai_kind_note_text(),
        AiChangeKind::NoteImportance => t.ai_kind_note_importance(),
        AiChangeKind::Routine => t.ai_kind_routine(),
        AiChangeKind::Place => t.ai_kind_place(),
        AiChangeKind::Other => t.ai_kind_other(),
    }
}

/// Reads the review of the AI Task `task` ([`Store::ai_review`]) in
/// words.
pub fn read_review(store: &Store, t: &L10n, task: &str) -> oldbrainfix_core::Result<ReviewShown> {
    let mut groups = Vec::new();
    for (kind, items) in store.ai_review(task)?.groups {
        let mut shown = Vec::new();
        for item in items {
            shown.push(review_item(store, t, item)?);
        }
        groups.push((kind, shown));
    }
    Ok(ReviewShown {
        task: task.to_string(),
        groups,
    })
}

fn review_item(
    store: &Store,
    t: &L10n,
    item: AiReviewItem,
) -> oldbrainfix_core::Result<ReviewItem> {
    let ty = item.entity.split_once(':').map_or("", |(ty, _)| ty);
    let title = item_title(store, t, &item)?;
    let mut changes = Vec::new();
    // A Routine's schedule shows as one, as the log holds it and as the
    // task leaves it.
    let schedule = |c: &&AiFieldChange| routine_keys::SCHEDULE_FIELDS.contains(&c.field.as_str());
    if ty == routine_keys::TYPE_ROUTINE && item.changes.iter().any(|c| schedule(&c)) {
        let routines = Routines::new(store);
        let before = store.log_only(|| routines.by_id(&item.entity))?;
        let after = routines.by_id(&item.entity)?;
        let text = |r: Option<oldbrainfix_core::Routine>| {
            r.map_or_else(
                || t.conflict_empty().to_string(),
                |r| schedule_text(t, &r.schedule),
            )
        };
        changes.push(ReviewChange {
            label: t.conflict_schedule().to_string(),
            before: Some(text(before)),
            after: Some(text(after)),
            mine_wins: item.changes.iter().filter(schedule).any(|c| c.mine_wins),
        });
    }
    for change in &item.changes {
        if ty == routine_keys::TYPE_ROUTINE && schedule(&change) {
            continue;
        }
        if let Some(line) = change_line(store, t, ty, change)? {
            changes.push(line);
        }
    }
    Ok(ReviewItem {
        entity: item.entity,
        title,
        note: item.note,
        changes,
    })
}

/// What an item is about: its Note as the log holds it, or as the task
/// made it; the Area or Topic by its name.
fn item_title(store: &Store, t: &L10n, item: &AiReviewItem) -> oldbrainfix_core::Result<String> {
    if let Some(note) = &item.note {
        return note_title(store, t, note);
    }
    Ok(place_name(store, t, &item.entity)?.unwrap_or_else(|| t.conflict_other().to_string()))
}

/// The Note `id` titled as the log holds it, or as the task made it.
fn note_title(store: &Store, t: &L10n, id: &str) -> oldbrainfix_core::Result<String> {
    let notes = Notes::new(store);
    let note = match store.log_only(|| notes.by_id(id))? {
        Some(note) => Some(note),
        None => notes.by_id(id)?,
    };
    Ok(row_title(t, note.as_ref()))
}

/// The name of the Area or Topic `id`, as it shows now or, when the task
/// deleted it, as the log holds it.
fn place_name(store: &Store, t: &L10n, id: &str) -> oldbrainfix_core::Result<Option<String>> {
    let places = Places::new(store);
    let named = |places: &Places| -> oldbrainfix_core::Result<Option<String>> {
        if let Some(area) = places.area_by_id(id)? {
            return Ok(Some(area_name(t, &area)));
        }
        Ok(places.topic_by_id(id)?.map(|topic| topic_name(t, &topic)))
    };
    match named(&places)? {
        Some(name) => Ok(Some(name)),
        None => store.log_only(|| named(&places)),
    }
}

/// One changed field in words; `None` for one that says nothing the
/// group does not, as the type of a thing the task made.
fn change_line(
    store: &Store,
    t: &L10n,
    ty: &str,
    change: &AiFieldChange,
) -> oldbrainfix_core::Result<Option<ReviewChange>> {
    let field = change.field.as_str();
    if field == note_keys::TYPE {
        // Deleted: an Area or Topic the task deleted.
        return Ok(
            (change.before.is_some() && change.after.is_none()).then(|| ReviewChange {
                label: t.ai_deleted().to_string(),
                before: None,
                after: None,
                mine_wins: change.mine_wins,
            }),
        );
    }
    let label = match (ty, field) {
        (note_keys::TYPE_NOTE, note_keys::TEXT) => t.conflict_note_text(),
        (note_keys::TYPE_SECTION, note_keys::TEXT) => t.conflict_section_text(),
        (note_keys::TYPE_NOTE, note_keys::PLACE) => t.ai_field_place(),
        (note_keys::TYPE_NOTE, note_keys::IMPORTANCE) => t.importance(),
        (note_keys::TYPE_NOTE, note_keys::MERGED) => t.ai_field_merged(),
        (note_keys::TYPE_NOTE, note_keys::NUDGE) => t.ai_field_nudge(),
        (_, note_keys::CAPTURED) => t.captured_heading(),
        (_, note_keys::TRASHED) => t.trash_title(),
        (_, routine_keys::NOTE) => t.note_title(),
        (routine_keys::TYPE_ROUTINE, routine_keys::RENAG) => t.renag_heading(),
        (routine_keys::TYPE_ROUTINE, routine_keys::ESCALATION) => t.escalation_heading(),
        (place_keys::TYPE_AREA, place_keys::NAME) => t.conflict_area_name(),
        (place_keys::TYPE_TOPIC, place_keys::NAME) => t.conflict_topic_name(),
        (place_keys::TYPE_AREA, place_keys::COLOUR) => t.colour_heading(),
        (place_keys::TYPE_AREA, place_keys::ICON) => t.icon_heading(),
        (place_keys::TYPE_TOPIC, place_keys::AREA) => t.ai_field_area(),
        _ => t.conflict_other(),
    };
    Ok(Some(ReviewChange {
        label: label.to_string(),
        before: Some(value_text(store, t, ty, field, change.before.as_deref())?),
        after: Some(value_text(store, t, ty, field, change.after.as_deref())?),
        mine_wins: change.mine_wins,
    }))
}

/// A stored value of `field` in words: a text as its source text, a
/// place or a Note by its name, an Importance translated, a time in the
/// app's format; anything else as stored.
fn value_text(
    store: &Store,
    t: &L10n,
    ty: &str,
    field: &str,
    value: Option<&str>,
) -> oldbrainfix_core::Result<String> {
    let Some(value) = value else {
        return Ok(match field {
            note_keys::TRASHED => t.ai_not_in_trash(),
            note_keys::PLACE if ty == note_keys::TYPE_NOTE => t.unsorted(),
            note_keys::IMPORTANCE => t.importance_normal(),
            _ => t.conflict_empty(),
        }
        .to_string());
    };
    Ok(match (ty, field) {
        (_, note_keys::TEXT) => {
            let text = StoredText::read(Some(value)).text;
            if text.is_empty() {
                t.conflict_empty().to_string()
            } else {
                text
            }
        }
        (_, note_keys::TRASHED) => t.ai_in_trash().to_string(),
        (note_keys::TYPE_NOTE, note_keys::IMPORTANCE) => match Importance::from_name(Some(value)) {
            Importance::Unimportant => t.importance_unimportant(),
            Importance::Normal => t.importance_normal(),
            Importance::Important => t.importance_important(),
        }
        .to_string(),
        (note_keys::TYPE_NOTE, note_keys::PLACE) | (place_keys::TYPE_TOPIC, place_keys::AREA) => {
            place_name(store, t, value)?.unwrap_or_else(|| t.conflict_empty().to_string())
        }
        (note_keys::TYPE_NOTE, note_keys::MERGED) | (_, routine_keys::NOTE) => {
            note_title(store, t, value)?
        }
        (routine_keys::TYPE_ROUTINE, routine_keys::RENAG) => match value.parse::<i64>() {
            Ok(minutes) => t.renag_minutes(minutes),
            Err(_) => value.to_string(),
        },
        _ => match parse_time(value) {
            Ok(at) => when(t, at),
            Err(_) => value.to_string(),
        },
    })
}

/// The AI port's card in the Settings: closed, "Open the AI port", and
/// the review when an AI Task waits for one; open, the address, the
/// three ways to copy what a client needs, the copied one marked, and
/// "Done, review".
pub fn port_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    url: Option<&str>,
    waiting: bool,
    copied: Option<Client>,
) -> Option<AiAction> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut asked = None;
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.ai_port_title()).strong().color(text));
        ui.label(RichText::new(t.ai_port_intro()).small().color(weak));
        let Some(url) = url else {
            if waiting {
                ui.label(RichText::new(t.ai_task_waiting()).color(text));
                if ui
                    .button(RichText::new(t.ai_review_open()).color(text))
                    .clicked()
                {
                    asked = Some(AiAction::Review);
                }
            }
            if ui
                .button(RichText::new(t.ai_port_open()).color(text))
                .clicked()
            {
                asked = Some(AiAction::Open);
            }
            return;
        };
        ui.label(RichText::new(t.ai_port_address(url)).color(text));
        for (client, label, detail) in [
            (
                Client::Config,
                t.ai_port_copy_config(),
                t.ai_port_config_detail(),
            ),
            (Client::ClaudeCode, t.ai_port_copy_claude_code(), ""),
            (
                Client::ClaudeDesktop,
                t.ai_port_copy_claude_desktop(),
                t.ai_port_bridged_detail(),
            ),
        ] {
            if ui.button(RichText::new(label).color(text)).clicked() {
                asked = Some(AiAction::Copy(client));
            }
            if !detail.is_empty() {
                ui.label(RichText::new(detail).small().color(weak));
            }
            if copied == Some(client) {
                ui.label(RichText::new(t.ai_port_copied()).small().color(weak));
            }
        }
        if ui
            .button(RichText::new(t.ai_port_done()).strong().color(text))
            .clicked()
        {
            asked = Some(AiAction::Done);
        }
    });
    asked
}

/// The review page: what it is, Accept all and Reject all, then a section
/// per kind of change, each item a card naming what it is about with its
/// changes, before and after, the user's own later edits marked as
/// staying. A click on an item with a Note opens that Note on the right,
/// as the task leaves it. Whether a Note was asked for, by its id.
pub fn review_page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    shown: &ReviewShown,
    asked: &mut Option<AiAction>,
) -> Option<String> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    ui.heading(RichText::new(t.ai_review_title()).color(text));
    ui.label(RichText::new(t.ai_review_intro()).color(weak));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.ai_accept_all()).strong().color(text))
            .clicked()
        {
            *asked = Some(AiAction::Accept);
        }
        if ui
            .button(RichText::new(t.ai_reject_all()).color(error))
            .clicked()
        {
            *asked = Some(AiAction::AskReject);
        }
    });
    ui.add_space(8.0);
    let mut open = None;
    egui::ScrollArea::vertical()
        .id_salt("ai review")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            for (kind, items) in &shown.groups {
                ui.add_space(4.0);
                ui.label(RichText::new(kind_label(t, *kind)).strong().color(text));
                for item in items {
                    let content = |ui: &mut Ui| {
                        ui.label(RichText::new(&item.title).strong().color(text));
                        for change in &item.changes {
                            change_rows(ui, t, tones, change);
                        }
                    };
                    match &item.note {
                        Some(note) => {
                            let id = Id::new(("ai review", *kind, &item.entity));
                            if clickable_card(ui, tones, (id, &item.title), content) {
                                open = Some(note.clone());
                            }
                        }
                        None => {
                            embossed(ui, tones, false, None, content);
                        }
                    }
                }
            }
            ui.add_space(8.0);
        });
    open
}

/// One change of an item: what changes, before and after, and "Your
/// edit stays" where the user's own later edit is kept.
fn change_rows(ui: &mut Ui, t: &L10n, tones: Tones, change: &ReviewChange) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(&change.label).small().color(weak));
    if let Some(before) = &change.before {
        ui.label(RichText::new(t.ai_before(before)).color(weak));
    }
    if let Some(after) = &change.after {
        ui.label(RichText::new(t.ai_after(after)).color(text));
    }
    if change.mine_wins {
        ui.label(
            RichText::new(t.ai_your_edit_stays())
                .small()
                .color(tones.accent),
        );
    }
}
