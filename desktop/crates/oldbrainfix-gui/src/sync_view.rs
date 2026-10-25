//! The Sync view (ADR 0003): the desktop hosts a LAN session while the
//! view shows, with the pair code as a QR and as text and its PIN. A
//! phone that is not paired yet gets the user's question first; a
//! paired one, proven by its key, syncs straight away. Every session's joiner is remembered
//! as a paired Device, and the paired Devices can be forgotten here.

use std::net::IpAddr;

use egui::{Color32, Rect, RichText, Ui, Vec2};
use oldbrainfix_core::lan::{
    Host, JoinAsk, Request, Response, lan_address, new_pin, pair_code_lines,
};
use oldbrainfix_core::{PairedDevice, Store};

use crate::app::{Wall, embossed};
use crate::l10n::L10n;
use crate::theme::Tones;

/// What the user asked for in the view this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncAction {
    /// Trust the Device asking, or refuse it.
    Trust(bool),
    /// Forget the paired Device with this id.
    Forget(String),
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
    /// The address to bind instead of the LAN one; tests use the
    /// loopback.
    pub bind: Option<IpAddr>,
}

impl SyncHost {
    /// Starts hosting with a fresh PIN and this Device's certificate.
    pub fn start(&mut self, store: &Store) {
        self.stop();
        self.error = None;
        self.last = None;
        self.refused = 0;
        self.no_network = self.bind.is_none() && lan_address().is_none();
        match store
            .tls_identity()
            .and_then(|identity| Host::start(&identity, &new_pin(), self.bind))
        {
            Ok(host) => self.host = Some(host),
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    /// Stops hosting; a Device still waiting for an answer is refused.
    pub fn stop(&mut self) {
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
        match store.serve(&request, name) {
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
                    });
                    if let Err(e) = remembered {
                        self.error = Some(e.to_string());
                    }
                }
                self.refused = session.report.refused_count();
                self.last = Some((session.name, session.applied.len()));
                !session.applied.is_empty()
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
                    if self.refused > 0 {
                        ui.label(RichText::new(t.sync_refused(self.refused as i64)).color(error));
                    }
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.paired_devices_title()).strong().color(text));
                    let devices = store.paired_devices().unwrap_or_default();
                    if devices.is_empty() {
                        ui.label(RichText::new(t.paired_none()).color(weak));
                    }
                    for device in devices {
                        ui.horizontal(|ui| {
                            let name = if device.name.is_empty() {
                                t.unnamed_device().to_string()
                            } else {
                                device.name.clone()
                            };
                            ui.label(RichText::new(name).color(text));
                            let when = wall(device.last_sync).format("%Y-%m-%d %H:%M").to_string();
                            ui.label(RichText::new(t.paired_last_sync(&when)).small().color(weak));
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
        self.stop();
    }
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
