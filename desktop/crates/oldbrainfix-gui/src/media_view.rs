//! How photos, voice recordings, their Transcripts and the speech model
//! show in the window and the capture popup: photos as pictures, in the
//! editor with their Photo Texts under them, a play button per recording
//! with its Transcript under it, the texts of those removed while their
//! texts stay (ADR 0020), the buttons under each Transcript and Photo
//! Text that take it over, remove it or make it again (ADR 0025), the
//! microphone's
//! button and the one big stop button (UI laws, "Capture"), and the
//! speech model's card in the Settings.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use egui::{
    ColorImage, RichText, Sense, TextureHandle, TextureOptions, Ui, WidgetInfo, WidgetType,
};
use oldbrainfix_core::Store;
use oldbrainfix_core::settings::PLAYBACK_SPEEDS;
use oldbrainfix_whisper::model::{Outcome, WhisperModel};

use crate::app::icon_button;
use crate::icons;
use crate::l10n::L10n;
use crate::media::{dimensions, pixels, speed_number};
use crate::speech::{SpeechModel, Status};

/// The long edge photos are shown at, at most.
const SHOWN_EDGE: u32 = 1024;

/// How tall a photo on a full card may get, so one of extreme
/// proportions cannot stretch the card (#114), as on the phone.
pub const CARD_PHOTO_HEIGHT: f32 = 480.0;

/// How low the place of a very wide photo on a full card may get.
const CARD_PHOTO_MIN_HEIGHT: f32 = 48.0;

/// Photos decoded for showing, by hash, once each; `None` for one whose
/// bytes are not on this computer (yet) or cannot be read. Their own
/// sizes in pixels too, read from their headers, once each.
#[derive(Default)]
pub struct PhotoCache {
    shown: HashMap<String, Option<TextureHandle>>,
    sizes: HashMap<String, Option<[u32; 2]>>,
}

impl PhotoCache {
    /// The width and height in pixels of the photo `hash`, read without
    /// decoding it; `None` while its bytes are not here, looked for again
    /// then, or when they cannot be read.
    pub fn size(&mut self, store: &Store, hash: &str) -> Option<[u32; 2]> {
        if let Some(size) = self.sizes.get(hash) {
            return *size;
        }
        let size = dimensions(&store.photo(hash)?);
        self.sizes.insert(hash.to_string(), size);
        size
    }

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

/// The photo `hash` on a full card (#114): as wide as the card in its
/// own proportions, no taller than [`CARD_PHOTO_HEIGHT`], decoded once it
/// first comes into sight and then no larger than [`SHOWN_EDGE`] either
/// way, so a photo of 12 × 34,000 pixels cannot break the card or fill
/// the graphics memory; one not here shows as such.
pub fn card_photo(ui: &mut Ui, t: &L10n, (cache, store): (&mut PhotoCache, &Store), hash: &str) {
    let weak = ui.visuals().weak_text_color();
    let Some([width, height]) = cache.size(store, hash) else {
        ui.horizontal(|ui| {
            icons::show(ui, icons::PHOTO, 18.0, weak);
            ui.label(RichText::new(t.photo_missing()).color(weak));
        });
        return;
    };
    let room = ui.available_width();
    let ratio = height as f32 / width as f32;
    let shown = if room * ratio > CARD_PHOTO_HEIGHT {
        egui::vec2(CARD_PHOTO_HEIGHT / ratio, CARD_PHOTO_HEIGHT)
    } else {
        egui::vec2(room, room * ratio)
    };
    let place = egui::vec2(room, shown.y.max(CARD_PHOTO_MIN_HEIGHT));
    let (rect, resp) = ui.allocate_exact_size(place, Sense::hover());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, t.photo()));
    // Out of sight it is not decoded yet; its place is known already.
    if ui.is_rect_visible(rect)
        && let Some(texture) = cache.get(ui, store, hash)
    {
        egui::Image::new(&texture)
            .corner_radius(6.0)
            .paint_at(ui, egui::Rect::from_center_size(rect.center(), shown));
    }
}

/// A Transcript or Photo Text `said` whole on a full card, dimmed under
/// its small `label`, so it never reads as typed text (#114).
fn labelled(ui: &mut Ui, label: &str, said: RichText) {
    let weak = ui.visuals().weak_text_color();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.label(RichText::new(label).small().color(weak));
        ui.label(said.color(weak));
    });
}

/// The photos and voice recordings of a Note or Section in full on a
/// card (#114), each `(shown, removed, texts)`: each photo with its Photo
/// Text under it, each recording with its Transcript, "Transcribing…"
/// while the one `transcribing` is being made or "No words heard." for
/// an empty one, then the texts of those removed while their texts stay
/// (ADR 0020); every text whole, dimmed and labelled.
pub fn whole_media(
    ui: &mut Ui,
    t: &L10n,
    (cache, store): (&mut PhotoCache, &Store),
    (photos, removed_photos, photo_texts): (&[String], &[String], &BTreeMap<String, String>),
    (recordings, removed_recordings, transcripts): (
        &[String],
        &[String],
        &BTreeMap<String, String>,
    ),
    transcribing: Option<&str>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let said = |texts: &BTreeMap<String, String>, hash: &String| {
        texts
            .get(hash)
            .filter(|s| !s.trim().is_empty())
            .map(RichText::new)
    };
    for hash in photos {
        card_photo(ui, t, (&mut *cache, store), hash);
        if let Some(said) = said(photo_texts, hash) {
            labelled(ui, t.photo_text_label(), said);
        }
    }
    for hash in removed_photos {
        if let Some(said) = said(photo_texts, hash) {
            labelled(ui, t.photo_text_label(), said);
        }
    }
    for hash in recordings {
        ui.horizontal(|ui| {
            icons::show(ui, icons::MIC, 16.0, weak);
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
            labelled(ui, t.transcript_label(), line);
        }
    }
    for hash in removed_recordings {
        if let Some(said) = said(transcripts, hash) {
            labelled(ui, t.transcript_label(), said);
        }
    }
}

/// What the editor offers under a photo or voice recording and its
/// text, by its hash; nothing on a card.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Offers {
    /// "Replace Note text" and "Append to Note text": the text of its
    /// Note or Section may change (ADR 0025).
    pub take_over: bool,
    /// "Remove generated text" (ADR 0025).
    pub remove_text: bool,
    /// "Transcribe again" or "Read the text again" while the file is
    /// here, an engine runs and the Note may change: `Some(true)` while
    /// it is being made, when the button rests.
    pub again: Option<bool>,
    /// The button that removes the file and keeps its text (ADR 0020).
    pub remove_file: bool,
}

/// What was asked of a Transcript or Photo Text, by its file's hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Generated {
    /// Make it the Note's text (`append` false) or add it on a line of
    /// its own after the Note's text.
    TakeOver { hash: String, append: bool },
    /// Remove it; the file stays.
    Remove(String),
    /// Make it again from the file.
    Again(String),
}

/// The buttons under the Photo Text (`photo`) or Transcript `said` of
/// `hash`, one per line, in the phone's order: Replace and Append for a
/// text that says something, Remove, and "again". Only "again" shows for
/// a removed text, which has none.
fn generated_buttons(
    ui: &mut Ui,
    t: &L10n,
    hash: &str,
    said: Option<&String>,
    offers: Offers,
    photo: bool,
) -> Option<Generated> {
    let text = ui.visuals().text_color();
    let mut asked = None;
    if offers.take_over && said.is_some_and(|s| !s.trim().is_empty()) {
        for (append, label) in [
            (false, t.replace_note_text()),
            (true, t.append_to_note_text()),
        ] {
            if ui.button(RichText::new(label).color(text)).clicked() {
                asked = Some(Generated::TakeOver {
                    hash: hash.to_string(),
                    append,
                });
            }
        }
    }
    if offers.remove_text
        && ui
            .button(RichText::new(t.remove_generated_text()).color(text))
            .clicked()
    {
        asked = Some(Generated::Remove(hash.to_string()));
    }
    if let Some(busy) = offers.again {
        let label = if photo {
            t.read_text_again()
        } else {
            t.transcribe_again()
        };
        let again = egui::Button::new(RichText::new(label).color(text));
        if ui.add_enabled(!busy, again).clicked() {
            asked = Some(Generated::Again(hash.to_string()));
        }
    }
    asked
}

/// What was asked of a photo.
#[derive(Debug, Clone, PartialEq)]
pub enum Photo {
    /// Removing it and keeping its Photo Text (ADR 0020).
    Remove(String),
    /// Something of its Photo Text.
    Text(Generated),
}

/// The photos `hashes` one under the other, as wide as there is room,
/// each with its Photo Text from `photo_texts` under it, as the phone's
/// editor shows it, the buttons it `offers` for that text, and the
/// button that removes the photo and keeps its Photo Text (ADR 0020).
/// What was clicked comes back.
pub fn photos(
    ui: &mut Ui,
    t: &L10n,
    (cache, store): (&mut PhotoCache, &Store),
    hashes: &[String],
    photo_texts: &BTreeMap<String, String>,
    offers: &dyn Fn(&str) -> Offers,
) -> Option<Photo> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut clicked = None;
    for hash in hashes {
        let offered = offers(hash);
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
        // A photo without text, or not read yet, shows none.
        let said = photo_texts.get(hash);
        if let Some(said) = said.filter(|s| !s.trim().is_empty()) {
            ui.label(RichText::new(said).color(weak));
        }
        if let Some(asked) = generated_buttons(ui, t, hash, said, offered, true) {
            clicked = Some(Photo::Text(asked));
        }
        if offered.remove_file
            && ui
                .button(RichText::new(t.remove_photo_keep_text()).color(text))
                .clicked()
        {
            clicked = Some(Photo::Remove(hash.clone()));
        }
    }
    clicked
}

/// The texts of the photos and voice recordings removed while their
/// texts stay (ADR 0020), where the files were: each Photo Text of
/// `photos` and Transcript of `recordings` that is not empty, with the
/// buttons it `offers` by hash and whether it is a photo's. What was
/// asked of a Photo Text (`true`) or Transcript comes back.
pub fn removed_texts(
    ui: &mut Ui,
    t: &L10n,
    (photos, photo_texts): (&[String], &BTreeMap<String, String>),
    (recordings, transcripts): (&[String], &BTreeMap<String, String>),
    offers: &dyn Fn(&str, bool) -> Offers,
) -> Option<(bool, Generated)> {
    let weak = ui.visuals().weak_text_color();
    let texts = photos
        .iter()
        .map(|h| (h, photo_texts.get(h), true))
        .chain(recordings.iter().map(|h| (h, transcripts.get(h), false)));
    let mut clicked = None;
    for (hash, said, photo) in texts {
        let Some(said) = said.filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        ui.label(RichText::new(said).color(weak));
        if let Some(asked) = generated_buttons(ui, t, hash, Some(said), offers(hash, photo), photo)
        {
            clicked = Some((photo, asked));
        }
    }
    clicked
}

/// What was asked of a voice recording.
#[derive(Debug, Clone, PartialEq)]
pub enum Recording {
    /// Its play or stop button.
    Play(String),
    /// Removing it and keeping its Transcript (ADR 0020).
    Remove(String),
    /// Something of its Transcript.
    Text(Generated),
    /// Playing every recording at this speed from now on, 1 being normal
    /// speed.
    Speed(f64),
}

/// What shows of the voice recordings `hashes`: a play or stop button
/// each with the playback `speed` beside it, which opens the speeds to
/// choose from, and its Transcript, "Transcribing…" while it is being
/// made, or "No words heard." for an empty one, none for a removed one;
/// under it the buttons it `offers` for its Transcript and the one that
/// removes it and keeps its Transcript. What was clicked comes back,
/// `None` without buttons.
pub fn recordings(
    ui: &mut Ui,
    t: &L10n,
    hashes: &[String],
    transcripts: &BTreeMap<String, String>,
    (playing, transcribing, speed): (Option<&str>, Option<&str>, f64),
    buttons: bool,
    offers: &dyn Fn(&str) -> Offers,
) -> Option<Recording> {
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
                    clicked = Some(Recording::Play(hash.clone()));
                }
                if let Some(chosen) = speed_button(ui, t, speed) {
                    clicked = Some(Recording::Speed(chosen));
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
        let offered = offers(hash);
        let said = transcripts.get(hash);
        if let Some(asked) = generated_buttons(ui, t, hash, said, offered, false) {
            clicked = Some(Recording::Text(asked));
        }
        if offered.remove_file
            && ui
                .button(RichText::new(t.remove_recording_keep_text()).color(text))
                .clicked()
        {
            clicked = Some(Recording::Remove(hash.clone()));
        }
    }
    clicked
}

/// The playback `speed` beside a play button ("1×"); a click opens the
/// speeds under it, the current one marked. The speed chosen comes back.
fn speed_button(ui: &mut Ui, t: &L10n, speed: f64) -> Option<f64> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let number = |speed| speed_number(t.locale(), speed);
    let mut chosen = None;
    let menu = ui.menu_button(
        RichText::new(t.playback_speed_label(&number(speed))).color(text),
        |ui| {
            ui.label(RichText::new(t.playback_speed()).color(weak));
            for option in PLAYBACK_SPEEDS {
                let label = RichText::new(t.playback_speed_option(&number(option))).color(text);
                if ui.radio(option == speed, label).clicked() {
                    chosen = Some(option);
                    ui.close();
                }
            }
        },
    );
    menu.response
        .on_hover_text(t.playback_speed_semantics(&number(speed)));
    chosen.filter(|&chosen| chosen != speed)
}

/// What the microphone's button asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voice {
    Record,
    /// The big stop button: end the recording and hold it on the
    /// capture, with the rest, until the Note is saved.
    Stop,
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
    let big = egui::Button::new(RichText::new(t.stop_recording()).size(18.0).color(text))
        .min_size(egui::vec2(ui.available_width(), 44.0));
    if ui.add(big).clicked() {
        stop = true;
    }
    stop.then_some(Voice::Stop)
}

/// What the speech model's card asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Speech {
    /// The model chosen, to download in place of the one in use.
    Download(WhisperModel),
    /// A file of the user's own, chosen in the system's dialog.
    PickFile,
    Cancel,
}

/// The speech model's card in the Settings: what it is and where it
/// comes from, how it stands, then the choice of model, each with its
/// size and trade-off: choosing one downloads it, and only the chosen one
/// is kept. A file of the user's own works too. Nothing to choose while a
/// model is being fetched.
pub fn speech_card(ui: &mut Ui, t: &L10n, model: &SpeechModel) -> Option<Speech> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    let mut asked = None;
    ui.label(RichText::new(t.speech_model_title()).strong().color(text));
    ui.label(RichText::new(t.speech_model_intro()).color(weak));
    ui.label(
        RichText::new(t.speech_model_source_models())
            .small()
            .color(weak),
    );
    ui.label(
        RichText::new(t.speech_model_credit_models())
            .small()
            .color(weak),
    );
    match model.outcome() {
        Some(Outcome::Failed) => {
            ui.label(RichText::new(t.speech_model_failed()).color(error));
        }
        Some(Outcome::Mismatch) => {
            ui.label(RichText::new(t.speech_model_mismatch()).color(error));
        }
        _ => {}
    }
    let status = model.status();
    match status {
        Status::Checking => {
            ui.label(RichText::new(t.speech_model_checking()).color(text));
            return None;
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
                let size = model.chosen().unwrap_or(WhisperModel::Base).megabytes();
                let total = total.map_or_else(|| size.to_string(), mb);
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
        }
    }
    let idle = !matches!(status, Status::Fetching { .. });
    let chosen = model.chosen();
    ui.label(RichText::new(t.speech_model_choice()).strong().color(text));
    for (option, title, detail) in [
        (
            WhisperModel::Base,
            t.speech_model_base(WhisperModel::Base.megabytes()),
            t.speech_model_base_detail(),
        ),
        (
            WhisperModel::Small,
            t.speech_model_small(WhisperModel::Small.megabytes()),
            t.speech_model_small_detail(),
        ),
    ] {
        let radio =
            egui::RadioButton::new(chosen == Some(option), RichText::new(title).color(text));
        if ui.add_enabled(idle, radio).clicked() && chosen != Some(option) {
            asked = Some(Speech::Download(option));
        }
        ui.label(RichText::new(detail).small().color(weak));
    }
    ui.label(
        RichText::new(t.speech_model_keeps_one())
            .small()
            .color(weak),
    );
    let pick = egui::Button::new(RichText::new(t.speech_model_pick()).color(text));
    if ui.add_enabled(idle, pick).clicked() {
        asked = Some(Speech::PickFile);
    }
    ui.label(
        RichText::new(t.speech_model_pick_either())
            .small()
            .color(weak),
    );
    asked
}
