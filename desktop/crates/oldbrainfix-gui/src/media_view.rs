//! How photos, voice recordings, their Transcripts and the speech model
//! show in the window and the capture popup: photos as pictures, a play
//! button per recording with its Transcript under it, the microphone's
//! button and the one big stop button (UI laws, "Capture"), and the
//! speech model's card in the Settings.

use std::collections::HashMap;
use std::time::Duration;

use egui::{ColorImage, RichText, TextEdit, TextureHandle, TextureOptions, Ui};
use oldbrainfix_core::Store;
use oldbrainfix_whisper::model::{MEGABYTES, Outcome};

use crate::app::icon_button;
use crate::icons;
use crate::l10n::L10n;
use crate::media::pixels;
use crate::speech::{SpeechModel, Status};

/// The long edge photos are shown at, at most.
const SHOWN_EDGE: u32 = 1024;

/// Photos decoded for showing, by hash, once each; `None` for one whose
/// bytes are not on this computer (yet) or cannot be read.
#[derive(Default)]
pub struct PhotoCache {
    shown: HashMap<String, Option<TextureHandle>>,
}

impl PhotoCache {
    /// The photo `hash` to show, decoded on first sight.
    pub fn get(&mut self, ui: &Ui, store: &Store, hash: &str) -> Option<TextureHandle> {
        let found = self.shown.entry(hash.to_string()).or_insert_with(|| {
            let image = pixels(&store.photo(hash)?, SHOWN_EDGE)?;
            let size = [image.width() as usize, image.height() as usize];
            let image = ColorImage::from_rgba_unmultiplied(size, image.as_raw());
            Some(
                ui.ctx()
                    .load_texture(format!("photo:{hash}"), image, TextureOptions::LINEAR),
            )
        });
        // One that is not here yet is looked for again once it may be.
        if found.is_none() {
            self.shown.remove(hash);
            return None;
        }
        found.clone()
    }
}

/// The photos `hashes` side by side, each `height` high, the first
/// `max` of them and how many more; one not here shows as such.
pub fn thumbnails(
    ui: &mut Ui,
    t: &L10n,
    cache: &mut PhotoCache,
    store: &Store,
    hashes: &[String],
    height: f32,
    max: usize,
) {
    let weak = ui.visuals().weak_text_color();
    ui.horizontal_wrapped(|ui| {
        for hash in hashes.iter().take(max) {
            match cache.get(ui, store, hash) {
                Some(texture) => {
                    let size = texture.size_vec2();
                    let width = size.x * height / size.y.max(1.0);
                    ui.add(
                        egui::Image::new(&texture)
                            .fit_to_exact_size(egui::vec2(width, height))
                            .corner_radius(6.0)
                            .alt_text(t.photo()),
                    );
                }
                None => {
                    icons::show(ui, icons::PHOTO, height.min(32.0), weak);
                    ui.label(RichText::new(t.photo_missing()).small().color(weak));
                }
            }
        }
        if hashes.len() > max {
            let more = (hashes.len() - max) as i64;
            ui.label(RichText::new(t.more_photos(more)).color(weak));
        }
    });
}

/// The photos `hashes` one under the other, as wide as there is room.
pub fn photos(ui: &mut Ui, t: &L10n, cache: &mut PhotoCache, store: &Store, hashes: &[String]) {
    let weak = ui.visuals().weak_text_color();
    for hash in hashes {
        match cache.get(ui, store, hash) {
            Some(texture) => {
                let size = texture.size_vec2();
                let width = ui.available_width().min(size.x);
                ui.add(
                    egui::Image::new(&texture)
                        .fit_to_exact_size(egui::vec2(width, size.y * width / size.x.max(1.0)))
                        .corner_radius(6.0)
                        .alt_text(t.photo()),
                );
            }
            None => {
                ui.horizontal(|ui| {
                    icons::show(ui, icons::PHOTO, 18.0, weak);
                    ui.label(RichText::new(t.photo_missing()).color(weak));
                });
            }
        }
    }
}

/// What shows of the voice recordings `hashes`: a play or stop button
/// each, with its Transcript, "Transcribing…" while it is being made, or
/// "No words heard." for an empty one. The hash whose button was clicked
/// comes back, `None` without buttons.
pub fn recordings(
    ui: &mut Ui,
    t: &L10n,
    hashes: &[String],
    transcripts: &std::collections::BTreeMap<String, String>,
    playing: Option<&str>,
    transcribing: Option<&str>,
    buttons: bool,
) -> Option<String> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut clicked = None;
    for hash in hashes {
        ui.horizontal(|ui| {
            let on = playing == Some(hash.as_str());
            if buttons {
                let (icon, label) = if on {
                    (icons::STOP, t.stop_playing())
                } else {
                    (icons::PLAY, t.play_recording())
                };
                if icon_button(ui, icon, label, text).clicked() {
                    clicked = Some(hash.clone());
                }
            } else {
                icons::show(ui, icons::MIC, 16.0, weak);
            }
            ui.label(RichText::new(t.voice_recording()).color(text));
        });
        let line = match transcripts.get(hash) {
            Some(said) if said.is_empty() => Some(RichText::new(t.no_words_heard()).italics()),
            Some(said) => Some(RichText::new(said)),
            None if transcribing == Some(hash.as_str()) => {
                Some(RichText::new(t.transcribing()).italics())
            }
            None => None,
        };
        if let Some(line) = line {
            ui.label(line.color(weak));
        }
    }
    clicked
}

/// What the microphone's button asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    Record,
    /// The big stop button: end the recording and keep the Note.
    StopAndSave,
}

/// The microphone's button, or while `recording` runs, how long it has
/// run and the one big stop button.
pub fn voice_button(ui: &mut Ui, t: &L10n, recording: Option<Duration>) -> Option<Voice> {
    let text = ui.visuals().text_color();
    let Some(elapsed) = recording else {
        let mic = ui.horizontal(|ui| {
            icons::show(ui, icons::MIC, 20.0, text);
            ui.button(RichText::new(t.record_voice()).color(text))
                .clicked()
        });
        return mic.inner.then_some(Voice::Record);
    };
    // The elapsed time moves on every second.
    ui.ctx().request_repaint_after(Duration::from_millis(500));
    let seconds = elapsed.as_secs();
    let mut stop = false;
    ui.horizontal(|ui| {
        icons::show(ui, icons::MIC, 22.0, ui.visuals().error_fg_color);
        ui.label(
            RichText::new(format!(
                "{} {}:{:02}",
                t.recording(),
                seconds / 60,
                seconds % 60
            ))
            .color(text),
        );
    });
    let big = egui::Button::new(RichText::new(t.stop_and_save()).size(18.0).color(text))
        .min_size(egui::vec2(ui.available_width(), 44.0));
    if ui.add(big).clicked() {
        stop = true;
    }
    stop.then_some(Voice::StopAndSave)
}

/// What the speech model's card asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Speech {
    Download,
    TakeFile(String),
    Cancel,
}

/// The speech model's card in the Settings: what it is, where it comes
/// from, how it stands, and the buttons to fetch it.
pub fn speech_card(
    ui: &mut Ui,
    t: &L10n,
    model: &SpeechModel,
    path: &mut String,
) -> Option<Speech> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    let mut asked = None;
    ui.label(RichText::new(t.speech_model_title()).strong().color(text));
    ui.label(RichText::new(t.speech_model_intro()).color(weak));
    match model.outcome() {
        Some(Outcome::Failed) => {
            ui.label(RichText::new(t.speech_model_failed()).color(error));
        }
        Some(Outcome::Mismatch) => {
            ui.label(RichText::new(t.speech_model_mismatch()).color(error));
        }
        _ => {}
    }
    match model.status() {
        Status::Checking => {
            ui.label(RichText::new(t.speech_model_checking()).color(text));
        }
        Status::Ready => {
            ui.label(RichText::new(t.speech_model_ready()).color(text));
        }
        Status::Fetching {
            downloading,
            received,
            total,
        } => {
            if downloading {
                let mb = |bytes: u64| format!("{:.0}", bytes as f64 / 1_000_000.0);
                let total = total.map_or_else(|| MEGABYTES.to_string(), mb);
                ui.label(
                    RichText::new(t.speech_model_downloading(&mb(received), &total)).color(text),
                );
            }
            ui.ctx().request_repaint_after(Duration::from_millis(250));
            if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
                asked = Some(Speech::Cancel);
            }
        }
        Status::Missing => {
            ui.label(RichText::new(t.speech_model_missing()).color(text));
            ui.label(
                RichText::new(t.speech_model_source(MEGABYTES))
                    .small()
                    .color(weak),
            );
            if ui
                .button(RichText::new(t.speech_model_download(MEGABYTES)).color(text))
                .clicked()
            {
                asked = Some(Speech::Download);
            }
            ui.label(RichText::new(t.speech_model_pick()).color(text));
            ui.label(
                RichText::new(t.speech_model_pick_detail())
                    .small()
                    .color(weak),
            );
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(path)
                        .id(egui::Id::new("speech model path"))
                        .hint_text(t.speech_model_path_hint())
                        .text_color(text)
                        .desired_width(260.0),
                );
                let typed = !path.trim().is_empty();
                let use_it =
                    egui::Button::new(RichText::new(t.speech_model_use_file()).color(text));
                if ui.add_enabled(typed, use_it).clicked() {
                    asked = Some(Speech::TakeFile(path.trim().to_string()));
                }
            });
        }
    }
    asked
}
