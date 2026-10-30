//! The Sync view (ADR 0003): the desktop hosts a LAN session while the
//! view shows, with the pair code as a QR and as text and its PIN. A
//! phone that is not paired yet gets the user's question first; a
//! paired one, proven by its key, syncs straight away. Every session's joiner is remembered
//! as a paired Device, with the certificate it hosts with. The other way
//! round, the code a phone or tablet shows is typed here and the desktop
//! joins it, off the window's thread. The paired Devices can be forgotten here, or
//! kept from syncing on their own. The Conflicts are listed here too,
//! each version a button that keeps it, and the shared folder is chosen,
//! synced and forgotten here, with how its last sync went. A bundle file
//! for a paired Device, or of everything, is saved and one imported here
//! too.

use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use egui::{Color32, Rect, RichText, Ui, Vec2};
use oldbrainfix_core::bundle::{BundleResult, import_bundle};
use oldbrainfix_core::checklists::keys as checklist_keys;
use oldbrainfix_core::conflicts::{RADIUS, SCHEDULE_UNIT};
use oldbrainfix_core::join::{JoinResult, SyncFailure, lan_sync};
use oldbrainfix_core::lan::{
    Host, JoinAsk, PairInfo, Request, Response, Session, decode_pair_code, lan_address, new_pin,
    pair_code_lines, pair_code_needs_update,
};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::offers::OfferError;
use oldbrainfix_core::places::keys as place_keys;
use oldbrainfix_core::sync::{FOLDER_SOURCE, SETTINGS_SOURCE, UpdateNeeded};
use oldbrainfix_core::{
    AreaRole, Conflict, Entry, Error, Fields, Invite, Notes, PairedDevice, Places, ProfileChoice,
    Routines, ShoppingLists, Store, StoredText,
};

use crate::app::{Wall, embossed};
use crate::l10n::L10n;
use crate::names::area_name;
use crate::profile_view::{Pictures, display_name, person_picture};
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
    /// Join the Device whose code was typed.
    Join,
}

/// Joins the host a pair code names, as [`lan_sync`] does, as this
/// Device named by the last argument; tests hand in another.
pub type Joiner =
    Arc<dyn Fn(&Store, &PairInfo, &str) -> oldbrainfix_core::Result<JoinResult> + Send + Sync>;

/// The desktop's hosting state.
#[derive(Default)]
pub struct SyncHost {
    host: Option<Host>,
    /// A Device not paired yet that asked to sync, waiting for the
    /// user's answer; its request is held open meanwhile. While this
    /// host invites, every joiner waits so, with the Profile its person
    /// sent, for the user to confirm (ADR 0017).
    asking: Option<(Request, JoinAsk, Option<ProfileChoice>)>,
    /// The pictures of the people asking.
    pictures: Pictures,
    /// Why hosting could not start (`true`), or what a session choked
    /// on, in full, until the window takes it to announce.
    error: Option<(bool, String)>,
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
    /// The code of another Device typed to join it.
    code: String,
    /// Why the typed code joined nothing: not a code, or the sync
    /// failed; red under the field.
    code_error: Option<String>,
    /// Why the last join failed, for the first start to announce once it
    /// is back at its choices.
    join_failure: Option<JoinFailure>,
    /// The join running off the window's thread, until it answers.
    joining: Option<Receiver<oldbrainfix_core::Result<JoinResult>>>,
    /// How a typed code is joined; [`lan_sync`] when none.
    pub joiner: Option<Joiner>,
    /// Whether a session, hosted or joined, went through since hosting
    /// started.
    synced: bool,
}

impl SyncHost {
    /// Starts hosting with a fresh PIN and this Device's certificate.
    pub fn start(&mut self, store: &Store) {
        self.halt();
        self.error = None;
        self.last = None;
        self.synced = false;
        self.refused = 0;
        self.found = 0;
        self.reload_conflicts(store);
        self.no_network = self.bind.is_none() && lan_address().is_none();
        match store
            .tls_identity()
            .and_then(|identity| Host::start(&identity, &new_pin(), self.bind))
        {
            Ok(host) => self.host = Some(host),
            Err(e) => self.error = Some((true, format!("host: {e}"))),
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
        if let Some((request, _, _)) = self.asking.take() {
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
            Err(e) => self.error = Some((false, format!("conflicts: {e}"))),
        }
    }

    /// The open Conflicts as last read.
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    /// The Device waiting for the user's answer.
    pub fn asking(&self) -> Option<&JoinAsk> {
        self.asking.as_ref().map(|(_, ask, _)| ask)
    }

    /// The Profile of the person invited, waiting for the user to
    /// confirm it.
    pub fn asking_profile(&self) -> Option<&ProfileChoice> {
        self.asking
            .as_ref()
            .and_then(|(_, _, profile)| profile.as_ref())
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
                // Invited, the joiner shows its person's Profile, and the
                // user confirms it even for a Device trusted before; one
                // without one is refused by the answer.
                let profile = self
                    .inviting
                    .as_ref()
                    .and_then(|_| request.joiner_profile(store));
                let pin = self.host.as_ref().map(|h| h.pin().to_string());
                let trusted =
                    pin.is_some_and(|pin| store.trusts_joiner(&ask, &pin).unwrap_or(false));
                if !trusted || profile.is_some() {
                    if self.asking.is_some() {
                        // One question at a time; the other tries again.
                        request.reply(Response::status(503));
                    } else {
                        self.asking = Some((request, ask, profile));
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
        let Some((request, _, _)) = self.asking.take() else {
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
                // With the certificate it hosts with, so it can be joined
                // back later.
                if let Err(e) = store.remember_joiner(&session) {
                    self.error = Some((false, format!("remember joiner: {e}")));
                }
                self.took_in(store, session)
            }
            Err(e) => {
                request.reply(Response::status(500));
                self.error = Some((false, format!("serve: {e}")));
                false
            }
        }
    }

    /// Notes what a session, hosted or joined, brought. Returns whether
    /// the log changed.
    fn took_in(&mut self, store: &Store, session: Session) -> bool {
        if let Some(area) = &session.shared_area {
            // The person's Profile, as the grant names it.
            let who = store
                .grants_of(area)
                .ok()
                .flatten()
                .and_then(|g| g.person_of(&session.peer).map(str::to_string))
                .and_then(|person| store.person_shown(&person).ok())
                .map(|p| p.name)
                .unwrap_or_default();
            self.shared = Some((who, area.clone()));
        }
        self.synced = true;
        self.refused = session.report.refused_count();
        self.found = session.conflicts.len();
        self.last = Some((session.name, session.applied.len()));
        self.reload_conflicts(store);
        !session.applied.is_empty() || session.profile.changed() || session.people_changed
    }

    /// Joins the Device whose code was typed, as this Device named
    /// `name`, on a thread of its own: the other Device may ask its user
    /// first. A text that is no code says so under the field.
    pub fn join(&mut self, store: Arc<Store>, name: &str, t: &L10n) {
        if self.joining.is_some() {
            return;
        }
        let Some(info) = decode_pair_code(&self.code) else {
            self.code_error = Some(invalid_code_text(t, &self.code).to_string());
            return;
        };
        self.code_error = None;
        self.join_failure = None;
        let joiner = self.joiner.clone();
        let name = name.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let joined = match joiner {
                Some(joiner) => joiner(&store, &info, &name),
                None => lan_sync(&store, &info, &name),
            };
            // A window gone meanwhile is nobody's problem.
            drop(tx.send(joined));
        });
        self.joining = Some(rx);
    }

    /// Whether a typed code is being joined.
    pub fn joining(&self) -> bool {
        self.joining.is_some()
    }

    /// Whether a session, hosted or joined, went through since hosting
    /// last started; a sync that brought nothing counts as well.
    pub fn synced(&self) -> bool {
        self.synced
    }

    /// Takes why the last join failed, out of the field as well.
    pub fn take_join_failure(&mut self) -> Option<JoinFailure> {
        self.code_error = None;
        self.join_failure.take()
    }

    /// Takes why hosting could not start (`true`) or what a session
    /// choked on, in full, to announce once.
    pub fn take_error(&mut self) -> Option<(bool, String)> {
        self.error.take()
    }

    /// Takes in the join's answer once there: `None` while it runs, else
    /// whether the log changed. A failure says why under the field; the
    /// code stays there to be fixed.
    pub fn joined(&mut self, store: &Store, t: &L10n) -> Option<bool> {
        let answer = match self.joining.as_ref()?.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return None,
            // The thread broke off without an answer.
            Err(TryRecvError::Disconnected) => Err(Error::Sync(SyncFailure::Failed)),
        };
        self.joining = None;
        match answer {
            Ok(joined) => {
                self.code.clear();
                self.code_error = None;
                if !joined.session.conflicts.is_empty() {
                    // Announced at the top when this view is not showing.
                    drop(store.set_conflicts_arrived(true));
                }
                Some(self.took_in(store, joined.session))
            }
            Err(e) => {
                let failure = join_failure(t, &e);
                self.code_error = Some(match &failure {
                    JoinFailure::Paused => t.sync_paused_error().to_string(),
                    JoinFailure::Failed { cause, fix, .. } => format!("{cause} {fix}"),
                });
                self.join_failure = Some(failure);
                Some(false)
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
                if let (Some(choice), Some(invite)) =
                    (self.asking_profile().cloned(), self.inviting.clone())
                {
                    let area = Places::new(store)
                        .area_by_id(&invite.area)
                        .ok()
                        .flatten()
                        .map(|a| area_name(t, &a))
                        .unwrap_or_default();
                    let name = display_name(t, &choice.profile.name).to_string();
                    embossed(ui, tones, true, None, |ui| {
                        ui.label(
                            RichText::new(t.invite_confirm_title(&area))
                                .strong()
                                .color(text),
                        );
                        ui.horizontal(|ui| {
                            person_picture(ui, &mut self.pictures, &choice.profile, 56.0, &name);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&name).strong().color(text));
                                ui.label(
                                    RichText::new(format!(
                                        "{}: {}",
                                        t.profile_their_code(),
                                        choice.code
                                    ))
                                    .monospace()
                                    .color(text),
                                );
                            });
                        });
                        ui.label(RichText::new(t.invite_confirm_text()).color(weak));
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new(t.invite_confirm_share()).color(text))
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
                } else if let Some(asked) = self.trust_card(ui, t, tones) {
                    action = Some(asked);
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
                            RichText::new(t.inviting_banner(&area, role))
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
                self.host_card(ui, t, tones);
                // Joining another Device by its code; not while inviting,
                // which hands out this Device's own code.
                if self.inviting.is_none() && self.join_section(ui, t, tones) {
                    action = Some(SyncAction::Join);
                }
                if let Some((name, count)) = &self.last {
                    ui.label(RichText::new(t.sync_came_in(name, *count as i64)).color(text));
                    if let Some((who, area)) = &self.shared
                        && let Ok(Some(area)) = Places::new(store).area_by_id(area)
                    {
                        let area = area_name(t, &area);
                        let who = display_name(t, who);
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
                            RichText::new(conflict_heading(t, store, conflict))
                                .strong()
                                .color(text),
                        );
                        if let Some(note) = conflict_note(store, conflict) {
                            ui.label(RichText::new(note).color(text));
                        }
                        // In a Shared Area only who may change it picks; a
                        // text in a format not known here waits for a newer
                        // version (ADR 0018).
                        let resolvable = conflict.resolvable();
                        let picks = resolvable
                            && store
                                .may_resolve(&conflict.entity, &conflict.entity_type)
                                .unwrap_or(false);
                        if !resolvable {
                            ui.label(RichText::new(t.conflict_newer_format()).color(weak));
                        } else if !picks {
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

impl SyncHost {
    /// The ways to pair either way round, as the first start's adding of
    /// a Device of one's own shows them: the question about a Device
    /// asking, this Device's code, and the field for the other's.
    pub fn pairing(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<SyncAction> {
        let mut action = None;
        ui.spacing_mut().item_spacing.y = 12.0;
        if let Some(asked) = self.trust_card(ui, t, tones) {
            action = Some(asked);
        }
        self.host_card(ui, t, tones);
        if self.join_section(ui, t, tones) {
            action = Some(SyncAction::Join);
        }
        action
    }

    /// The question about a Device not paired yet that asks to sync,
    /// with Trust and Refuse.
    fn trust_card(&self, ui: &mut Ui, t: &L10n, tones: Tones) -> Option<SyncAction> {
        let text = ui.visuals().text_color();
        let ask = self.asking()?;
        let who = if ask.name.is_empty() {
            t.unnamed_device().to_string()
        } else {
            ask.name.clone()
        };
        let mut action = None;
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
        action
    }

    /// This Device's pair code as a QR and as text, with its PIN and
    /// address, while it hosts.
    fn host_card(&self, ui: &mut Ui, t: &L10n, tones: Tones) {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let error = ui.visuals().error_fg_color;
        let Some(host) = &self.host else {
            return;
        };
        embossed(ui, tones, false, None, |ui| {
            if self.no_network {
                ui.label(RichText::new(t.sync_no_network()).color(error));
            }
            if host.locked_out() {
                ui.label(RichText::new(t.sync_locked_out()).color(error));
            }
            ui.vertical_centered(|ui| {
                qr(ui, &host.pair_code(false), qrcode::EcLevel::M, 240.0, None);
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

    /// The field for another Device's code, its error under it, and the
    /// button, or the waiting while it joins. Returns whether the user
    /// asked to join.
    fn join_section(&mut self, ui: &mut Ui, t: &L10n, tones: Tones) -> bool {
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let error = ui.visuals().error_fg_color;
        let joining = self.joining.is_some();
        let mut asked = false;
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.sync_join_title()).strong().color(text));
            ui.label(RichText::new(t.sync_type_code_intro()).small().color(weak));
            let label = ui.label(RichText::new(t.sync_code_label()).color(text));
            let field = ui
                .add_enabled(
                    !joining,
                    egui::TextEdit::singleline(&mut self.code)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label.id);
            if let Some(message) = &self.code_error {
                ui.label(RichText::new(message).color(error));
            }
            if joining {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(t.sync_join_waiting()).color(text));
                });
            } else {
                let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let clicked = ui
                    .button(RichText::new(t.sync_join_button()).color(text))
                    .clicked();
                asked = entered || clicked;
            }
        });
        asked
    }
}

/// Why joining a typed code failed.
#[derive(Debug, Clone, PartialEq)]
pub enum JoinFailure {
    /// Refused while an AI Task waits: said as everywhere.
    Paused,
    /// Failed: why and what to do, one sentence each, and the full text.
    Failed {
        cause: &'static str,
        fix: &'static str,
        details: String,
    },
}

/// Why joining a typed code failed as `error` says: each
/// [`SyncFailure`] as the phone says it, anything else as broken off.
pub fn join_failure(t: &L10n, error: &Error) -> JoinFailure {
    let (failure, details) = match error {
        Error::SyncPaused => return JoinFailure::Paused,
        Error::Sync(failure) => (*failure, format!("sync: {failure:?}")),
        e => (SyncFailure::Failed, format!("sync: {e}")),
    };
    let (cause, fix) = sync_failure_why(t, failure);
    JoinFailure::Failed {
        cause,
        fix,
        details,
    }
}

/// Why a sync did not happen and what to do, one sentence each, as the
/// phone's `syncFailureWhy`.
pub fn sync_failure_why(t: &L10n, failure: SyncFailure) -> (&'static str, &'static str) {
    match failure {
        SyncFailure::NotPrivate => (t.sync_not_private_cause(), t.sync_not_private_fix()),
        SyncFailure::Unreachable => (t.sync_unreachable_cause(), t.sync_unreachable_fix()),
        SyncFailure::WrongHost => (t.sync_fail_wrong_host(), t.sync_wrong_host_fix()),
        SyncFailure::WrongPin => (t.sync_wrong_pin_cause(), t.sync_wrong_pin_fix()),
        SyncFailure::Locked => (t.sync_locked_cause(), t.sync_locked_fix()),
        SyncFailure::Declined => (t.sync_fail_declined(), t.sync_declined_fix()),
        SyncFailure::UpdateHere => (t.sync_fail_update_here(), t.update_fix()),
        SyncFailure::UpdateThere => (t.sync_fail_update_there(), t.update_fix()),
        SyncFailure::Failed => (t.sync_broke_off_cause(), t.nothing_lost_fix()),
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
/// sync's are. A bundle offering an Area (#108) is held as the offer and
/// said to wait on the Agenda.
pub fn import_bundle_text(t: &L10n, store: &Store, path: &Path) -> Result<String, String> {
    import_bundle_result(store, path)
        .map(|result| bundle_result_text(t, &result))
        .map_err(|e| bundle_error_text(t, e))
}

/// Imports the bundle at `path` into `store`, Conflicts it brought
/// announced as an automatic sync's are.
pub fn import_bundle_result(store: &Store, path: &Path) -> Result<BundleResult, Error> {
    let result = import_bundle(store, path)?;
    if !result.conflicts.is_empty() {
        drop(store.set_conflicts_arrived(true));
    }
    Ok(result)
}

/// What went wrong importing a bundle, in words; an Area offer refused
/// says why and what to do.
pub fn bundle_error_text(t: &L10n, error: Error) -> String {
    match error {
        Error::NotABundle => t.bundle_not_a_bundle().to_string(),
        Error::BundleNeedsUpdate => t.bundle_needs_update().to_string(),
        Error::Offer(e) => {
            let (cause, fix) = offer_refusal(t, e);
            format!("{cause} {fix}")
        }
        Error::SyncPaused => t.sync_paused_error().to_string(),
        e => format!("{} ({e})", t.bundle_failed()),
    }
}

/// Why a bundle brought nothing in as `error` says, and what to do, one
/// sentence each, as the phone says them.
pub fn bundle_import_why(t: &L10n, error: &Error) -> (&'static str, &'static str) {
    match error {
        Error::NotABundle => (t.bundle_not_a_bundle(), t.bundle_ask_again_fix()),
        Error::BundleNeedsUpdate => (t.bundle_needs_update_cause(), t.bundle_update_fix()),
        _ => (t.bundle_not_read_cause(), t.bundle_import_again_fix()),
    }
}

/// Why an Area offer was refused as `error` says (#108), and what to
/// do, as the phone's area offer dialog.
pub fn offer_refusal(t: &L10n, error: OfferError) -> (&'static str, &'static str) {
    match error {
        OfferError::NotAnOffer => (t.not_an_area_offer(), t.area_offer_ask_again_fix()),
        OfferError::NotForUs => (t.area_offer_not_for_us(), t.area_offer_ask_again_fix()),
        OfferError::FromBlocked => (t.area_offer_from_blocked(), t.blocked_fix()),
        OfferError::Refused => (t.area_offer_refused(), t.area_offer_own_fix()),
    }
}

/// What the code field says under a `code` that does not join, as the
/// phone's `invalidCodeText`: that it is none, or that it comes from a
/// later version (ADR 0034).
pub fn invalid_code_text(t: &L10n, code: &str) -> &'static str {
    if pair_code_needs_update(code) {
        t.pair_code_needs_update()
    } else {
        t.sync_invalid_code()
    }
}

/// What a sync that needs an update (ADR 0019) says at the top, as the
/// phone's `updateNoticeText`: which Device needs it, by the name it is
/// paired under where known.
pub fn update_notice_text(t: &L10n, store: &Store, source: &str, update: UpdateNeeded) -> String {
    if source == FOLDER_SOURCE {
        return t.folder_needs_update().to_string();
    }
    if source == SETTINGS_SOURCE {
        return t.settings_need_update().to_string();
    }
    let name = store
        .paired_device(source)
        .ok()
        .flatten()
        .map(|d| d.name)
        .unwrap_or_default();
    match (update, name.is_empty()) {
        (UpdateNeeded::Here, false) => t.sync_update_here_with(&name).to_string(),
        (UpdateNeeded::There, false) => t.sync_update_there_with(&name).to_string(),
        (UpdateNeeded::Here, true) => t.sync_fail_update_here().to_string(),
        (UpdateNeeded::There, true) => t.sync_fail_update_there().to_string(),
    }
}

/// What a bundle import did, in words.
pub fn bundle_result_text(t: &L10n, result: &BundleResult) -> String {
    if result.offer.is_some() {
        return t.area_offer_waits().to_string();
    }
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
        (checklist_keys::TYPE_LINE, checklist_keys::TEXT) => t.conflict_line_text(),
        (place_keys::TYPE_AREA, place_keys::NAME) => t.conflict_area_name(),
        (place_keys::TYPE_TOPIC, place_keys::NAME) => t.conflict_topic_name(),
        (NAMED_PLACE, place_keys::NAME) => t.conflict_place_name(),
        (NAMED_PLACE, RADIUS) => t.conflict_place_radius(),
        ("routine", SCHEDULE_UNIT) => t.conflict_schedule(),
        _ => t.conflict_other(),
    }
}

/// What `conflict` is about as its card heads it: as [`conflict_label`]
/// says, a Line of a Shopping List named as one (#122).
fn conflict_heading(t: &L10n, store: &Store, conflict: &Conflict) -> &'static str {
    if conflict.entity_type == checklist_keys::TYPE_LINE
        && let Ok(Some(note)) = store.current(&conflict.entity, checklist_keys::NOTE)
        && ShoppingLists::new(store)
            .is_shopping_list(&note)
            .unwrap_or(false)
    {
        return t.conflict_shopping_line_text();
    }
    conflict_label(t, conflict)
}

/// A Named Place's `$type`, the Dart core's `NamedPlaceKeys.typeNamedPlace`.
const NAMED_PLACE: &str = "namedPlace";

/// The text of the Note a Conflict belongs to: the Note a schedule
/// Conflict's Routine comes from, or the Note of a Checklist's Line
/// (#120).
fn conflict_note(store: &Store, conflict: &Conflict) -> Option<String> {
    if conflict.entity_type == checklist_keys::TYPE_LINE {
        let note = store
            .current(&conflict.entity, checklist_keys::NOTE)
            .ok()
            .flatten()?;
        return Notes::new(store)
            .by_id(&note)
            .ok()
            .flatten()
            .map(|n| n.text);
    }
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
    // A text shows as its source text, whatever its format.
    let value = if conflict.unit == note_keys::TEXT {
        StoredText::read(Some(value)).text
    } else {
        value.to_string()
    };
    if value.is_empty() {
        return t.conflict_empty().to_string();
    }
    if conflict.unit == RADIUS
        && let Ok(metres) = value.parse::<f64>()
    {
        return t.radius_metres(metres.round() as i64);
    }
    value
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

/// `value` as a QR code with error correction `level`, `size` points
/// wide, black on white, read out as `label` when there is one; false,
/// and nothing drawn, when `value` does not fit a QR code.
pub(crate) fn qr(
    ui: &mut Ui,
    value: &str,
    level: qrcode::EcLevel,
    size: f32,
    label: Option<&str>,
) -> bool {
    let Ok(code) = qrcode::QrCode::with_error_correction_level(value.as_bytes(), level) else {
        return false;
    };
    let width = code.width();
    let colors = code.to_colors();
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if let Some(label) = label {
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, label));
    }
    if !ui.is_rect_visible(rect) {
        return true;
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
    true
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
