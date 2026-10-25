//! The Sync view (ADR 0003): the desktop hosts a LAN session while the
//! view shows, with the pair code as a QR and as text and its PIN. A
//! phone that is not paired yet gets the user's question first; a
//! paired one, proven by its key, syncs straight away. Every session's joiner is remembered
//! as a paired Device, and the paired Devices can be forgotten here, or
//! kept from syncing on their own. The Conflicts are listed here too,
//! each version a button that keeps it, and the shared folder is chosen,
//! synced and forgotten here, with how its last sync went. A bundle file
//! for a paired Device, or of everything, is saved and one imported here
//! too.

use std::net::IpAddr;
use std::path::Path;

use egui::{Color32, Rect, RichText, Ui, Vec2};
use oldbrainfix_core::bundle::{BundleResult, import_bundle};
use oldbrainfix_core::conflicts::{RADIUS, SCHEDULE_UNIT};
use oldbrainfix_core::lan::{
    Host, JoinAsk, Request, Response, lan_address, new_pin, pair_code_lines,
};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::places::keys as place_keys;
use oldbrainfix_core::{
    AreaRole, Conflict, Entry, Error, Fields, Invite, PairedDevice, Places, Routines, Store,
};

use crate::app::{Wall, embossed};
use crate::l10n::L10n;
use crate::names::area_name;
use crate::schedule_text::schedule_text;
use crate::theme::Tones;

/// What the user asked for in the view this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncAction {
    /// Trust the Device asking, or refuse it.
    Trust(bool),
    /// Forget the paired Device with this id.
    Forget(String),
    /// Let the paired Device with this id sync on its own, or not.
    AutoSync(String, bool),
    /// Keep the version at `index` of the Conflict on `unit` of `entity`.
    Keep {
        entity: String,
        unit: String,
        index: usize,
    },
    /// Choose the shared folder with the system's dialog.
    PickFolder,
    /// Stop syncing through the shared folder.
    ForgetFolder,
    /// Sync through the shared folder now.
    FolderNow,
    /// Save a bundle for the paired Device with this id, or of
    /// everything, with the system's dialog.
    SaveBundle(Option<String>),
    /// Import a bundle chosen with the system's dialog.
    OpenBundle,
    /// End the invite to a Shared Area; hosting goes on with a fresh
    /// code.
    StopInviting,
}

/// The desktop's hosting state.
#[derive(Default)]
pub struct SyncHost {
    host: Option<Host>,
    /// A Device not paired yet that asked to sync, waiting for the
    /// user's answer; its request is held open meanwhile.
    asking: Option<(Request, JoinAsk)>,
    /// Why hosting could not start, or what a session choked on.
    error: Option<String>,
    no_network: bool,
    /// The last session: with whom and how many changes came in.
    last: Option<(String, usize)>,
    /// Entries refused for their signatures in the last session.
    refused: usize,
    /// Conflicts the last session's entries are part of.
    found: usize,
    /// The open Conflicts, read again whenever the log may have changed.
    conflicts: Vec<Conflict>,
    /// The address to bind instead of the LAN one; tests use the
    /// loopback.
    pub bind: Option<IpAddr>,
    /// The paired Device the next bundle is for; everything when none.
    pub bundle_for: Option<String>,
    /// The invite to a Shared Area this host hands out while it shows
    /// (ADR 0013): whoever joins with its code gets that Area only.
    pub inviting: Option<Invite>,
    /// The last session that shared an Area: with whom, and which.
    shared: Option<(String, String)>,
}

impl SyncHost {
    /// Starts hosting with a fresh PIN and this Device's certificate.
    pub fn start(&mut self, store: &Store) {
        self.halt();
        self.error = None;
        self.last = None;
        self.refused = 0;
        self.found = 0;
        self.reload_conflicts(store);
        self.no_network = self.bind.is_none() && lan_address().is_none();
        match store
            .tls_identity()
            .and_then(|identity| Host::start(&identity, &new_pin(), self.bind))
        {
            Ok(host) => self.host = Some(host),
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Starts hosting an invite to a Shared Area.
    pub fn start_inviting(&mut self, store: &Store, invite: Invite) {
        self.inviting = Some(invite);
        self.start(store);
        self.shared = None;
    }

    /// Ends the invite and hosts on with a fresh code, so the one shown
    /// to the other person pairs nobody else.
    pub fn stop_inviting(&mut self, store: &Store) {
        self.inviting = None;
        let shared = self.shared.take();
        self.start(store);
        self.shared = shared;
    }

    /// Stops hosting; a Device still waiting for an answer is refused.
    /// An invite ends with it.
    pub fn stop(&mut self) {
        self.inviting = None;
        self.halt();
    }

    /// Stops the host, the invite left as it is.
    fn halt(&mut self) {
        if let Some((request, _)) = self.asking.take() {
            request.reply(Response::declined());
        }
        if let Some(mut host) = self.host.take() {
            host.stop();
        }
    }

    pub fn hosting(&self) -> bool {
        self.host.is_some()
    }

    pub fn host(&self) -> Option<&Host> {
        self.host.as_ref()
    }

    /// Reads the open Conflicts again.
    pub fn reload_conflicts(&mut self, store: &Store) {
        match store.conflicts() {
            Ok(conflicts) => self.conflicts = conflicts,
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// The open Conflicts as last read.
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    /// The Device waiting for the user's answer.
    pub fn asking(&self) -> Option<&JoinAsk> {
        self.asking.as_ref().map(|(_, ask)| ask)
    }

    /// Answers what the joiners sent since last frame, as this Device
    /// named `name`. Returns whether the log changed.
    pub fn poll(&mut self, store: &Store, name: &str) -> bool {
        let mut changed = false;
        loop {
            let Some(request) = self.host.as_ref().and_then(Host::next_request) else {
                break;
            };
            if let Some(ask) = request.join_ask() {
                let pin = self.host.as_ref().map(|h| h.pin().to_string());
                let trusted =
                    pin.is_some_and(|pin| store.trusts_joiner(&ask, &pin).unwrap_or(false));
                if !trusted {
                    if self.asking.is_some() {
                        // One question at a time; the other tries again.
                        request.reply(Response::status(503));
                    } else {
                        self.asking = Some((request, ask));
                    }
                    continue;
                }
            }
            changed |= self.serve(store, request, name);
        }
        changed
    }

    /// The user's answer to the Device asking. Returns whether the log
    /// changed.
    pub fn answer(&mut self, store: &Store, trust: bool, name: &str) -> bool {
        let Some((request, _)) = self.asking.take() else {
            return false;
        };
        if trust {
            self.serve(store, request, name)
        } else {
            request.reply(Response::declined());
            false
        }
    }

    fn serve(&mut self, store: &Store, request: Request, name: &str) -> bool {
        match store.serve_inviting(&request, name, self.inviting.as_ref()) {
            Ok((response, session)) => {
                request.reply(response);
                let Some(session) = session else {
                    return false;
                };
                if !session.peer.is_empty() {
                    let remembered = store.remember_device(&PairedDevice {
                        id: session.peer.clone(),
                        name: session.name.clone(),
                        fingerprint: String::new(),
                        host: None,
                        port: None,
                        last_sync: store.now(),
                        shared: session.shared_area.is_some(),
                    });
                    if let Err(e) = remembered {
                        self.error = Some(e.to_string());
                    }
                }
                if let Some(area) = &session.shared_area {
                    let who = match &self.inviting {
                        Some(invite) => invite.name.clone(),
                        None => session.name.clone(),
                    };
                    self.shared = Some((who, area.clone()));
                }
                self.refused = session.report.refused_count();
                self.found = session.conflicts.len();
                self.last = Some((session.name, session.applied.len()));
                self.reload_conflicts(store);
                !session.applied.is_empty() || session.profile.changed()
            }
            Err(e) => {
                request.reply(Response::status(500));
                self.error = Some(e.to_string());
                false
            }
        }
    }

    /// Draws the view.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        t: &L10n,
        tones: Tones,
        store: &Store,
        wall: &Wall,
    ) -> Option<SyncAction> {
        let mut action = None;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let error = ui.visuals().error_fg_color;
        ui.heading(RichText::new(t.sync_title()).color(text));
        ui.label(RichText::new(t.sync_host_intro()).color(weak));
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("sync")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                if let Some(message) = &self.error {
                    ui.label(RichText::new(message).color(error));
                }
                if let Some(ask) = self.asking() {
                    let who = if ask.name.is_empty() {
                        t.unnamed_device().to_string()
                    } else {
                        ask.name.clone()
                    };
                    embossed(ui, tones, true, None, |ui| {
                        ui.label(
                            RichText::new(t.sync_trust_question(&who))
                                .strong()
                                .color(text),
                        );
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new(t.sync_trust()).color(text))
                                .clicked()
                            {
                                action = Some(SyncAction::Trust(true));
                            }
                            if ui
                                .button(RichText::new(t.sync_refuse()).color(text))
                                .clicked()
                            {
                                action = Some(SyncAction::Trust(false));
                            }
                        });
                    });
                }
                if let Some(invite) = &self.inviting {
                    let area = Places::new(store)
                        .area_by_id(&invite.area)
                        .ok()
                        .flatten()
                        .map(|a| area_name(t, &a))
                        .unwrap_or_default();
                    let role = match invite.role {
                        AreaRole::Manager => t.role_manager(),
                        AreaRole::Member => t.role_member(),
                    };
                    embossed(ui, tones, true, None, |ui| {
                        ui.label(
                            RichText::new(t.inviting_banner(&invite.name, &area, role))
                                .strong()
                                .color(text),
                        );
                        if ui
                            .button(RichText::new(t.stop_inviting()).color(text))
                            .clicked()
                        {
                            action = Some(SyncAction::StopInviting);
                        }
                    });
                }
                if let Some(host) = &self.host {
                    embossed(ui, tones, false, None, |ui| {
                        if self.no_network {
                            ui.label(RichText::new(t.sync_no_network()).color(error));
                        }
                        if host.locked_out() {
                            ui.label(RichText::new(t.sync_locked_out()).color(error));
                        }
                        ui.vertical_centered(|ui| {
                            qr(ui, &host.pair_code(false), 240.0);
                            let typed = host.pair_code(true);
                            for line in pair_code_lines(&typed) {
                                ui.label(
                                    RichText::new(line)
                                        .monospace()
                                        .strong()
                                        .size(20.0)
                                        .color(text),
                                );
                            }
                            ui.label(RichText::new(t.sync_pin(host.pin())).strong().color(text));
                            ui.label(
                                RichText::new(format!("{}:{}", host.address(), host.port()))
                                    .small()
                                    .color(weak),
                            );
                            if ui
                                .button(RichText::new(t.sync_copy_code()).color(text))
                                .clicked()
                            {
                                ui.ctx().copy_text(typed);
                            }
                        });
                    });
                }
                if let Some((name, count)) = &self.last {
                    ui.label(RichText::new(t.sync_came_in(name, *count as i64)).color(text));
                    if let Some((who, area)) = &self.shared
                        && let Ok(Some(area)) = Places::new(store).area_by_id(area)
                    {
                        let area = area_name(t, &area);
                        let said = if self.inviting.is_some() {
                            t.share_done(&area, who)
                        } else {
                            t.shared_with_you(who, &area)
                        };
                        ui.label(RichText::new(said).strong().color(text));
                    }
                    if self.refused > 0 {
                        ui.label(RichText::new(t.sync_refused(self.refused as i64)).color(error));
                    }
                    if self.found > 0 {
                        ui.label(
                            RichText::new(t.sync_conflicts_below(self.found as i64))
                                .strong()
                                .color(text),
                        );
                    }
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.conflicts_title()).strong().color(text));
                    if self.conflicts.is_empty() {
                        ui.label(RichText::new(t.conflicts_none()).color(weak));
                    } else {
                        ui.label(RichText::new(t.conflicts_intro()).color(weak));
                    }
                    for conflict in &self.conflicts {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(conflict_label(t, conflict))
                                .strong()
                                .color(text),
                        );
                        if let Some(note) = routine_note(store, conflict) {
                            ui.label(RichText::new(note).color(text));
                        }
                        // In a Shared Area only who may change it picks.
                        let picks = store
                            .may_resolve(&conflict.entity, &conflict.entity_type)
                            .unwrap_or(false);
                        if !picks {
                            ui.label(RichText::new(t.only_managers()).color(weak));
                        }
                        for (index, version) in conflict.versions.iter().enumerate() {
                            let value = conflict_value(t, conflict, &version.values);
                            let when = wall(version.entry.recorded_at)
                                .format("%Y-%m-%d %H:%M")
                                .to_string();
                            let origin = conflict_origin(t, store, &version.entry, &when);
                            let button = egui::Button::new(
                                RichText::new(format!("{value}\n{origin}")).color(text),
                            );
                            if ui
                                .add_enabled(picks, button)
                                .on_hover_text(t.conflict_keep())
                                .clicked()
                            {
                                action = Some(SyncAction::Keep {
                                    entity: conflict.entity.clone(),
                                    unit: conflict.unit.clone(),
                                    index,
                                });
                            }
                        }
                    }
                });
                if let Some(asked) = folder_section(ui, t, tones, store, wall) {
                    action = Some(asked);
                }
                if let Some(asked) = bundle_section(ui, t, tones, store, &mut self.bundle_for) {
                    action = Some(asked);
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.paired_devices_title()).strong().color(text));
                    let devices = store.paired_devices().unwrap_or_default();
                    if devices.is_empty() {
                        ui.label(RichText::new(t.paired_none()).color(weak));
                    } else {
                        ui.label(RichText::new(t.sync_auto_intro()).small().color(weak));
                    }
                    for device in devices {
                        ui.horizontal(|ui| {
                            let name = if device.name.is_empty() {
                                t.unnamed_device().to_string()
                            } else {
                                device.name.clone()
                            };
                            ui.label(RichText::new(name).color(text));
                            if device.shared {
                                ui.label(RichText::new(t.paired_person()).small().color(weak));
                            }
                            let when = wall(device.last_sync).format("%Y-%m-%d %H:%M").to_string();
                            ui.label(RichText::new(t.paired_last_sync(&when)).small().color(weak));
                            let mut on = store.auto_sync_on(&device.id).unwrap_or(true);
                            if ui
                                .checkbox(
                                    &mut on,
                                    RichText::new(t.sync_automatically()).color(text),
                                )
                                .changed()
                            {
                                action = Some(SyncAction::AutoSync(device.id.clone(), on));
                            }
                            if ui
                                .button(RichText::new(t.forget_device()).color(text))
                                .clicked()
                            {
                                action = Some(SyncAction::Forget(device.id.clone()));
                            }
                        });
                    }
                    ui.label(RichText::new(t.sync_settings_stay()).small().color(weak));
                });
            });
        action
    }
}

impl Drop for SyncHost {
    fn drop(&mut self) {
        self.halt();
    }
}

/// The shared folder: chosen or not, how its last sync went, and the
/// ways to choose, forget and sync it now.
fn folder_section(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    store: &Store,
    wall: &Wall,
) -> Option<SyncAction> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    let mut action = None;
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.folder_title()).strong().color(text));
        ui.label(RichText::new(t.folder_intro()).small().color(weak));
        match store.sync_folder().ok().flatten() {
            None => {
                if ui
                    .button(RichText::new(t.folder_pick()).color(text))
                    .clicked()
                {
                    action = Some(SyncAction::PickFolder);
                }
            }
            Some(path) => {
                ui.label(RichText::new(path).monospace().color(text));
                for (line, failed) in folder_status_lines(t, store, wall) {
                    ui.label(RichText::new(line).color(if failed { error } else { weak }));
                }
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t.folder_sync_now()).color(text))
                        .clicked()
                    {
                        action = Some(SyncAction::FolderNow);
                    }
                    if ui
                        .button(RichText::new(t.folder_pick()).color(text))
                        .clicked()
                    {
                        action = Some(SyncAction::PickFolder);
                    }
                    if ui
                        .button(RichText::new(t.folder_forget()).color(text))
                        .clicked()
                    {
                        action = Some(SyncAction::ForgetFolder);
                    }
                });
            }
        }
    });
    action
}

/// The bundle file: for which paired Device, or everything, and the
/// ways to save one and to import one.
fn bundle_section(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    store: &Store,
    bundle_for: &mut Option<String>,
) -> Option<SyncAction> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut action = None;
    let devices = store.paired_devices().unwrap_or_default();
    // A Device forgotten meanwhile is no choice any more.
    if bundle_for
        .as_ref()
        .is_some_and(|id| !devices.iter().any(|d| &d.id == id))
    {
        *bundle_for = None;
    }
    let named = |device: &PairedDevice| {
        if device.name.is_empty() {
            t.unnamed_device().to_string()
        } else {
            device.name.clone()
        }
    };
    embossed(ui, tones, false, None, |ui| {
        ui.label(RichText::new(t.bundle_title()).strong().color(text));
        ui.label(RichText::new(t.bundle_intro()).small().color(weak));
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.bundle_for()).color(text));
            let shown = bundle_for
                .as_ref()
                .and_then(|id| devices.iter().find(|d| &d.id == id))
                .map(named)
                .unwrap_or_else(|| t.bundle_everything().to_string());
            egui::ComboBox::from_id_salt("bundle-for")
                .selected_text(RichText::new(shown).color(text))
                .show_ui(ui, |ui| {
                    for device in &devices {
                        ui.selectable_value(
                            bundle_for,
                            Some(device.id.clone()),
                            RichText::new(named(device)).color(text),
                        );
                    }
                    ui.selectable_value(
                        bundle_for,
                        None,
                        RichText::new(t.bundle_everything()).color(text),
                    );
                });
        });
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(t.bundle_save()).color(text))
                .clicked()
            {
                action = Some(SyncAction::SaveBundle(bundle_for.clone()));
            }
            if ui
                .button(RichText::new(t.bundle_open()).color(text))
                .clicked()
            {
                action = Some(SyncAction::OpenBundle);
            }
        });
        ui.label(RichText::new(t.bundle_drop()).small().color(weak));
    });
    action
}

/// Imports the bundle at `path` into `store` and says what came in, or
/// what went wrong; Conflicts it brought are announced as an automatic
/// sync's are.
pub fn import_bundle_text(t: &L10n, store: &Store, path: &Path) -> Result<String, String> {
    match import_bundle(store, path) {
        Ok(result) => {
            if !result.conflicts.is_empty() {
                drop(store.set_conflicts_arrived(true));
            }
            Ok(bundle_result_text(t, &result))
        }
        Err(Error::NotABundle) => Err(t.bundle_not_a_bundle().to_string()),
        Err(e) => Err(format!("{} ({e})", t.bundle_failed())),
    }
}

/// What a bundle import did, in words.
pub fn bundle_result_text(t: &L10n, result: &BundleResult) -> String {
    if result.needs_pairing() {
        return t.bundle_pair_first().to_string();
    }
    let mut lines = vec![t.bundle_came_in(result.applied.len() as i64)];
    let refused = result.report.refused_count();
    if refused > 0 {
        lines.push(t.sync_refused(refused as i64));
    }
    lines.join("\n")
}

/// How the shared folder's last sync went, in words, each line with
/// whether it tells of a failure.
pub fn folder_status_lines(t: &L10n, store: &Store, wall: &Wall) -> Vec<(String, bool)> {
    let Some(status) = store.folder_status().ok().flatten() else {
        return vec![(t.folder_not_yet().to_string(), false)];
    };
    let when = wall(status.at).format("%Y-%m-%d %H:%M").to_string();
    if status.failed {
        return vec![(t.folder_unreachable(&when), true)];
    }
    let mut lines = vec![(t.folder_synced(&when, status.entries_in as i64), false)];
    if status.unreadable > 0 {
        lines.push((t.folder_unreadable(status.unreadable as i64), true));
    }
    lines
}

/// What `conflict` is about: which field of which kind of thing.
pub fn conflict_label(t: &L10n, conflict: &Conflict) -> &'static str {
    match (conflict.entity_type.as_str(), conflict.unit.as_str()) {
        (note_keys::TYPE_NOTE, note_keys::TEXT) => t.conflict_note_text(),
        (note_keys::TYPE_SECTION, note_keys::TEXT) => t.conflict_section_text(),
        (place_keys::TYPE_AREA, place_keys::NAME) => t.conflict_area_name(),
        (place_keys::TYPE_TOPIC, place_keys::NAME) => t.conflict_topic_name(),
        (NAMED_PLACE, place_keys::NAME) => t.conflict_place_name(),
        (NAMED_PLACE, RADIUS) => t.conflict_place_radius(),
        ("routine", SCHEDULE_UNIT) => t.conflict_schedule(),
        _ => t.conflict_other(),
    }
}

/// A Named Place's `$type`, the Dart core's `NamedPlaceKeys.typeNamedPlace`.
const NAMED_PLACE: &str = "namedPlace";

/// The text of the Note a schedule Conflict's Routine comes from.
fn routine_note(store: &Store, conflict: &Conflict) -> Option<String> {
    if conflict.unit != SCHEDULE_UNIT {
        return None;
    }
    let routines = Routines::new(store);
    let routine = routines.by_id(&conflict.entity).ok().flatten()?;
    routines.note(&routine).ok().flatten().map(|n| n.text)
}

/// What a version holding `values` says, in words.
pub fn conflict_value(
    t: &L10n,
    conflict: &Conflict,
    values: &[(String, Option<String>)],
) -> String {
    if conflict.unit == SCHEDULE_UNIT {
        return match Routines::schedule_of(&Fields::new(values.to_vec())) {
            Some(schedule) => schedule_text(t, &schedule),
            None => t.conflict_empty().to_string(),
        };
    }
    let value = values
        .first()
        .and_then(|(_, v)| v.as_deref())
        .unwrap_or_default();
    if value.is_empty() {
        return t.conflict_empty().to_string();
    }
    if conflict.unit == RADIUS
        && let Ok(metres) = value.parse::<f64>()
    {
        return t.radius_metres(metres.round() as i64);
    }
    value.to_string()
}

/// Where and when `entry` was written: this Device, a paired Device by
/// the name it goes by, or the author name it carries.
pub fn conflict_origin(t: &L10n, store: &Store, entry: &Entry, when: &str) -> String {
    if store.device_id().ok().as_deref() == Some(entry.device.as_str()) {
        return t.conflict_on_this_device(when);
    }
    let paired = store
        .paired_device(&entry.device)
        .ok()
        .flatten()
        .map(|d| d.name)
        .unwrap_or_default();
    if !paired.is_empty() {
        return t.conflict_on_device(&paired, when);
    }
    if !entry.author.is_empty() {
        return t.conflict_on_device(&entry.author, when);
    }
    t.conflict_on_unnamed(when)
}

/// `value` as a QR code, `size` points wide, black on white.
fn qr(ui: &mut Ui, value: &str, size: f32) {
    let Ok(code) = qrcode::QrCode::new(value.as_bytes()) else {
        return;
    };
    let width = code.width();
    let colors = code.to_colors();
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, Color32::WHITE);
    let quiet = 2.0;
    let cell = size / (width as f32 + 2.0 * quiet);
    for y in 0..width {
        for x in 0..width {
            if colors[y * width + x] == qrcode::Color::Dark {
                let min =
                    rect.min + Vec2::new((x as f32 + quiet) * cell, (y as f32 + quiet) * cell);
                painter.rect_filled(
                    Rect::from_min_size(min, Vec2::splat(cell + 0.2)),
                    0.0,
                    Color32::BLACK,
                );
            }
        }
    }
}

/// The name this computer goes by in a session: its host name.
pub fn device_name() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| oldbrainfix_core::APP_NAME.to_string())
}
