//! Voice recordings and photos on the desktop: the microphone, the
//! speakers, the clipboard, and photos made the phone's way.
//!
//! A recording is WAV, 16-bit, mono, at 16 kHz: no AAC encoder is
//! available in pure Rust, and WAV at Whisper's own rate is what the
//! phone and the desktop both play and transcribe without converting
//! (the phone reads it with its own MediaExtractor). The phone records
//! AAC in `.m4a`; the desktop plays and transcribes that too.
//!
//! A picture or sound file chosen with "From file…" or dropped in
//! becomes a photo or a voice recording ([`from_file`]). A sound file is
//! kept as it is, MP3, Ogg Opus as messengers send voice messages, m4a
//! or WAV; one that cannot be decoded here is kept all the same, with
//! why it cannot be transcribed.
//!
//! A photo pasted or dropped in is kept as the phone keeps one
//! (Photos.kt): upright, at most 2048 pixels on the long side, a JPEG
//! at quality 85 on white where it was see-through, without metadata.
//!
//! The microphone, the speakers and the clipboard sit behind traits, so
//! the window is tested with fakes; the real ones are device-only.

use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use image::{DynamicImage, ImageDecoder, ImageReader, RgbImage, RgbaImage};
use parking_lot::Mutex;

/// The rate a recording is kept at: Whisper's own.
pub const SAMPLE_RATE: u32 = oldbrainfix_whisper::WHISPER_RATE;

/// The long edge of a kept photo, the phone's.
pub const MAX_EDGE: u32 = 2048;

/// The JPEG quality of a kept photo, the phone's.
pub const QUALITY: u8 = 85;

/// The microphone.
pub trait Recorder {
    /// Starts recording; whether it did.
    fn start(&mut self) -> bool;

    /// Ends the recording: a WAV file, or `None` when none ran or
    /// nothing was heard.
    fn stop(&mut self) -> Option<Vec<u8>>;

    /// Whether a recording runs.
    fn recording(&self) -> bool;

    /// How long the running recording has run.
    fn elapsed(&self) -> Duration;
}

/// The speakers.
pub trait Player {
    /// Plays the recording `bytes`, stopping whatever played before;
    /// whether it plays.
    fn play(&mut self, bytes: Vec<u8>) -> bool;

    /// Stops playing.
    fn stop(&mut self);

    /// Whether something still plays.
    fn playing(&self) -> bool;

    /// Plays recordings at `speed`, 1 being normal speed: the one playing
    /// at once, and every one after it.
    fn set_speed(&mut self, speed: f64);
}

/// `speed` as the locale `locale` writes it: "1", "1.5", or "1,5" in
/// German.
pub fn speed_number(locale: &str, speed: f64) -> String {
    let number = speed.to_string();
    match locale {
        "de" => number.replace('.', ","),
        _ => number,
    }
}

/// The clipboard.
pub trait Clipboard {
    /// The picture on the clipboard, as a photo to keep; `None` when
    /// there is none.
    fn photo(&mut self) -> Option<Vec<u8>>;

    /// The text on the clipboard; `None` when there is none, or only
    /// blanks (#130).
    fn text(&mut self) -> Option<String> {
        None
    }

    /// Puts `text` on the clipboard, with `html` beside it for apps that
    /// take formatted text (#129); whether it is there. A clipboard that
    /// cannot leaves it to the window's own, which takes plain text.
    fn put_text(&mut self, _text: &str, _html: Option<&str>) -> bool {
        false
    }

    /// Puts the photo `bytes` on the clipboard as a picture (#149);
    /// whether it is there. The window's own clipboard takes no
    /// pictures, so one that cannot says so.
    fn put_photo(&mut self, _bytes: &[u8]) -> bool {
        false
    }
}

/// `samples` of one channel at [`SAMPLE_RATE`], from -1 to 1, as a WAV
/// file of 16-bit samples.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes a second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes a frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits a sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        let pcm = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        out.extend_from_slice(&pcm.to_le_bytes());
    }
    out
}

/// What the recording thread hears.
struct Heard {
    samples: Vec<f32>,
    channels: usize,
    rate: u32,
}

/// The system's default microphone, through rodio. Device-only.
#[derive(Default)]
pub struct MicRecorder {
    running: Option<Running>,
}

struct Running {
    stop: Arc<AtomicBool>,
    heard: Arc<Mutex<Heard>>,
    thread: JoinHandle<()>,
    since: Instant,
}

impl Recorder for MicRecorder {
    fn start(&mut self) -> bool {
        if self.running.is_some() {
            return true;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let heard = Arc::new(Mutex::new(Heard {
            samples: Vec::new(),
            channels: 1,
            rate: SAMPLE_RATE,
        }));
        let (opened, open) = std::sync::mpsc::channel();
        let thread = {
            let stop = stop.clone();
            let heard = heard.clone();
            // The stream belongs to the thread that opens it.
            std::thread::spawn(move || {
                use rodio::Source;
                let mic = rodio::microphone::MicrophoneBuilder::new()
                    .default_device()
                    .and_then(|b| b.default_config())
                    .ok()
                    .and_then(|b| b.open_stream().ok());
                let Some(mut mic) = mic else {
                    opened.send(false).ok();
                    return;
                };
                {
                    let mut heard = heard.lock();
                    heard.channels = usize::from(mic.channels().get());
                    heard.rate = mic.sample_rate().get();
                }
                opened.send(true).ok();
                let mut chunk = Vec::with_capacity(4096);
                while !stop.load(Ordering::Relaxed) {
                    match mic.next() {
                        Some(sample) => chunk.push(sample),
                        None => break,
                    }
                    if chunk.len() == chunk.capacity() {
                        heard.lock().samples.append(&mut chunk);
                    }
                }
                heard.lock().samples.append(&mut chunk);
            })
        };
        if open.recv().unwrap_or(false) {
            self.running = Some(Running {
                stop,
                heard,
                thread,
                since: Instant::now(),
            });
            true
        } else {
            drop(thread.join());
            false
        }
    }

    fn stop(&mut self) -> Option<Vec<u8>> {
        let running = self.running.take()?;
        running.stop.store(true, Ordering::Relaxed);
        drop(running.thread.join());
        let heard = std::mem::replace(
            &mut *running.heard.lock(),
            Heard {
                samples: Vec::new(),
                channels: 1,
                rate: SAMPLE_RATE,
            },
        );
        let samples = oldbrainfix_whisper::to_whisper(&heard.samples, heard.channels, heard.rate);
        (!samples.is_empty()).then(|| wav(&samples))
    }

    fn recording(&self) -> bool {
        self.running.is_some()
    }

    fn elapsed(&self) -> Duration {
        self.running
            .as_ref()
            .map_or(Duration::ZERO, |r| r.since.elapsed())
    }
}

/// The system's default speakers, through rodio, as Catlog plays its
/// sounds. Device-only.
pub struct SpeakerPlayer {
    playing: Option<(rodio::MixerDeviceSink, rodio::Player)>,
    /// How fast recordings play, 1 being normal speed, as an `f32`'s
    /// bits, shared with the [`Stretch`] playing: it keeps the pitch, so
    /// rodio's own speed, which plays the samples faster and raises the
    /// voice with it, stays at 1.
    speed: Arc<AtomicU32>,
}

impl Default for SpeakerPlayer {
    fn default() -> Self {
        SpeakerPlayer {
            playing: None,
            speed: Arc::new(AtomicU32::new(1.0f32.to_bits())),
        }
    }
}

impl Player for SpeakerPlayer {
    fn play(&mut self, bytes: Vec<u8>) -> bool {
        self.stop();
        let Some(sound) = playable(bytes) else {
            return false;
        };
        let Ok(sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return false;
        };
        let player = rodio::Player::connect_new(sink.mixer());
        player.append(Stretch::new(sound, self.speed.clone()));
        self.playing = Some((sink, player));
        true
    }

    fn stop(&mut self) {
        if let Some((_, player)) = self.playing.take() {
            player.stop();
        }
    }

    fn playing(&self) -> bool {
        self.playing.as_ref().is_some_and(|(_, p)| !p.empty())
    }

    fn set_speed(&mut self, speed: f64) {
        self.speed
            .store((speed as f32).to_bits(), Ordering::Relaxed);
    }
}

/// The recording `bytes` as the speakers play it: through rodio's own
/// decoders, or, for what they cannot decode, such as Ogg Opus, decoded
/// whole by oldbrainfix-whisper; `None` when neither can.
pub(crate) fn playable(bytes: Vec<u8>) -> Option<Box<dyn rodio::Source + Send>> {
    match rodio::Decoder::new(Cursor::new(bytes.clone())) {
        Ok(decoder) => Some(Box::new(decoder)),
        Err(_) => {
            let pcm = oldbrainfix_whisper::pcm(bytes)?;
            let channels = rodio::ChannelCount::new(u16::try_from(pcm.channels).ok()?)?;
            let rate = rodio::SampleRate::new(pcm.rate)?;
            Some(Box::new(rodio::buffer::SamplesBuffer::new(
                channels,
                rate,
                pcm.samples,
            )))
        }
    }
}

/// A recording played faster or slower at the same pitch, by WSOLA
/// (waveform-similarity overlap-add): it plays the recording in pieces
/// of [`STRETCH_PIECE`], each overlapping the one before by half, and
/// moves through the recording faster or slower than it plays; each
/// piece is taken where it best continues the one before, within
/// [`STRETCH_SEARCH`], so the waves join without a crackle.
///
/// The speed is read before every piece, so a new one applies at once.
/// At normal speed the recording plays unchanged.
struct Stretch<S: rodio::Source> {
    source: S,
    channels: usize,
    channel_count: rodio::ChannelCount,
    rate: rodio::SampleRate,
    speed: Arc<AtomicU32>,
    /// Frames heard from `source` and still needed, interleaved; the
    /// first is frame `base` of the recording.
    heard: Vec<rodio::Sample>,
    base: usize,
    /// Whether `source` ended.
    ended: bool,
    /// The fade of a piece, a Hann window: two halves overlapping add
    /// up to 1.
    fade: Vec<rodio::Sample>,
    /// Half a piece, in frames: how much each piece plays.
    half: usize,
    /// How far a piece may move to continue the one before, in frames.
    search: usize,
    /// How many frames apart the waves are compared, to save time at
    /// high sample rates.
    stride: usize,
    /// Where the last piece started in the recording, in frames.
    last: isize,
    /// Where the next piece would start at the speed alone.
    nominal: f64,
    /// The faded second half of the last piece, to add to the next.
    tail: Vec<rodio::Sample>,
    /// What plays next, and how much of it played.
    out: Vec<rodio::Sample>,
    at: usize,
    done: bool,
}

/// How long a piece of a recording played faster or slower is.
const STRETCH_PIECE: Duration = Duration::from_millis(30);

/// How far a piece may move to continue the one before: longer than
/// the wave of the lowest voice.
const STRETCH_SEARCH: Duration = Duration::from_millis(12);

impl<S: rodio::Source> Stretch<S> {
    fn new(source: S, speed: Arc<AtomicU32>) -> Self {
        let channel_count = source.channels();
        let rate = source.sample_rate();
        let channels = usize::from(channel_count.get());
        let per_second = rate.get() as f64;
        let half = ((STRETCH_PIECE.as_secs_f64() * per_second / 2.0).round() as usize).max(1);
        let search = (STRETCH_SEARCH.as_secs_f64() * per_second).round() as usize;
        let piece = 2 * half;
        let fade = (0..piece)
            .map(|i| {
                let phase = 2.0 * std::f64::consts::PI * i as f64 / piece as f64;
                (0.5 - 0.5 * phase.cos()) as rodio::Sample
            })
            .collect();
        let mut stretch = Stretch {
            source,
            channels,
            channel_count,
            rate,
            speed,
            heard: Vec::new(),
            base: 0,
            ended: false,
            fade,
            half,
            search,
            stride: (rate.get() / 16_000).max(1) as usize,
            // A piece before the recording, silent but for its second
            // half, so that the recording starts at full strength.
            last: -(half as isize),
            nominal: -(half as f64),
            tail: vec![0.0; half * channels],
            out: Vec::new(),
            at: 0,
            done: false,
        };
        // The silent piece's second half is the recording's start.
        stretch.hear(half as isize);
        // The tail holds `half` frames of `channels` samples; the fade's
        // second half fades them out.
        let mut tail = std::mem::take(&mut stretch.tail);
        let fade_out = stretch.fade.iter().skip(half);
        for (i, (frame, fade)) in tail.chunks_mut(channels).zip(fade_out).enumerate() {
            for (c, slot) in frame.iter_mut().enumerate() {
                *slot = fade * stretch.sample(i as isize, c);
            }
        }
        stretch.tail = tail;
        stretch
    }

    /// The frames heard so far.
    fn frames(&self) -> usize {
        self.base + self.heard.len() / self.channels
    }

    /// Hears `source` up to frame `until`, or to its end.
    fn hear(&mut self, until: isize) {
        while !self.ended && (self.frames() as isize) < until {
            for channel in 0..self.channels {
                match self.source.next() {
                    Some(sample) => self.heard.push(sample),
                    None => {
                        self.ended = true;
                        // A cut-off last frame is filled with silence.
                        if channel > 0 {
                            self.heard
                                .resize(self.heard.len() + self.channels - channel, 0.0);
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Channel `channel` of frame `frame`; silence outside the recording.
    fn sample(&self, frame: isize, channel: usize) -> rodio::Sample {
        if frame < self.base as isize {
            return 0.0;
        }
        let index = (frame as usize - self.base) * self.channels + channel;
        self.heard.get(index).copied().unwrap_or(0.0)
    }

    /// Frame `frame` with its channels mixed into one.
    fn mono(&self, frame: isize) -> rodio::Sample {
        (0..self.channels).map(|c| self.sample(frame, c)).sum()
    }

    /// Where the piece that best continues the last one starts, near
    /// `nominal`.
    fn best_start(&self, nominal: isize) -> isize {
        let follow = self.last + self.half as isize;
        let lowest = (nominal - self.search as isize).max(0);
        let highest = (nominal + self.search as isize).max(lowest);
        let mut best = (follow.clamp(lowest, highest), rodio::Sample::MIN);
        for start in lowest..=highest {
            let (mut similarity, mut energy) = (0.0, 0.0);
            for i in (0..self.half as isize).step_by(self.stride) {
                let candidate = self.mono(start + i);
                similarity += candidate * self.mono(follow + i);
                energy += candidate * candidate;
            }
            let score = similarity / (energy + 1e-9).sqrt();
            if score > best.1 {
                best = (start, score);
            }
        }
        best.0
    }

    /// Plays the next half piece into `out`; false at the end.
    fn next_piece(&mut self) -> bool {
        let speed = f32::from_bits(self.speed.load(Ordering::Relaxed)).clamp(0.25, 4.0);
        let half = self.half as isize;
        let start = if (speed - 1.0).abs() < 1e-3 {
            // At normal speed every piece follows the last unchanged.
            let start = self.last + half;
            self.nominal = start as f64;
            start
        } else {
            self.nominal += f64::from(speed) * self.half as f64;
            let nominal = self.nominal.round() as isize;
            self.hear(nominal.max(self.last + half) + self.search as isize + 2 * half);
            self.best_start(nominal)
        };
        self.hear(start + 2 * half);
        self.out.clear();
        self.at = 0;
        if self.ended && start >= self.frames() as isize {
            // The last piece's fade-out.
            self.out.append(&mut self.tail);
            return !self.out.is_empty();
        }
        // The tail's `half` frames, each of `channels` samples, against
        // the fade's first half rising and its second half falling.
        let mut tail = std::mem::take(&mut self.tail);
        for (i, (frame, fade)) in tail.chunks(self.channels).zip(&self.fade).enumerate() {
            for (c, held) in frame.iter().enumerate() {
                let rising = fade * self.sample(start + i as isize, c);
                self.out.push(held + rising);
            }
        }
        let fade_out = self.fade.iter().skip(self.half);
        for (i, (frame, fade)) in tail.chunks_mut(self.channels).zip(fade_out).enumerate() {
            for (c, slot) in frame.iter_mut().enumerate() {
                *slot = fade * self.sample(start + half + i as isize, c);
            }
        }
        self.tail = tail;
        self.last = start;
        // Forgets what no later piece reaches back to.
        let needed = (self.last + half).min(self.nominal.floor() as isize - self.search as isize);
        let forget = needed.max(0) as usize;
        if forget > self.base + 4 * self.half {
            let frames = (forget - self.base).min(self.heard.len() / self.channels);
            self.heard.drain(..frames * self.channels);
            self.base += frames;
        }
        true
    }
}

impl<S: rodio::Source> Iterator for Stretch<S> {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<rodio::Sample> {
        while self.at >= self.out.len() {
            if self.done || !self.next_piece() {
                self.done = true;
                return None;
            }
        }
        // The loop above leaves `at` inside `out`.
        let sample = self.out.get(self.at).copied()?;
        self.at += 1;
        Some(sample)
    }
}

impl<S: rodio::Source> rodio::Source for Stretch<S> {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.channel_count
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        // It depends on the speed, which may change while it plays.
        None
    }
}

/// The system's clipboard, through arboard. Device-only. It is kept
/// once opened: on X11 what this app put there lasts only while it is.
/// Without X11 on Wayland it does not open, and text goes through the
/// window's clipboard, without HTML, while a photo is not copied.
#[derive(Default)]
pub struct SystemClipboard {
    open: Option<arboard::Clipboard>,
}

impl SystemClipboard {
    fn open(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.open.is_none() {
            self.open = arboard::Clipboard::new().ok();
        }
        self.open.as_mut()
    }
}

impl Clipboard for SystemClipboard {
    fn put_text(&mut self, text: &str, html: Option<&str>) -> bool {
        let Some(clipboard) = self.open() else {
            return false;
        };
        match html {
            Some(html) => clipboard.set_html(html, Some(text)).is_ok(),
            None => clipboard.set_text(text).is_ok(),
        }
    }

    fn text(&mut self) -> Option<String> {
        let text = self.open()?.get_text().ok()?;
        (!text.trim().is_empty()).then(|| text.replace("\r\n", "\n"))
    }

    fn put_photo(&mut self, bytes: &[u8]) -> bool {
        let Ok(image) = image::load_from_memory(bytes) else {
            return false;
        };
        let rgba = image.to_rgba8();
        let (Ok(width), Ok(height)) = (
            usize::try_from(rgba.width()),
            usize::try_from(rgba.height()),
        ) else {
            return false;
        };
        let Some(clipboard) = self.open() else {
            return false;
        };
        let image = arboard::ImageData {
            width,
            height,
            bytes: rgba.into_raw().into(),
        };
        clipboard.set_image(image).is_ok()
    }

    fn photo(&mut self) -> Option<Vec<u8>> {
        let image = self.open()?.get_image().ok()?;
        let rgba = RgbaImage::from_raw(
            u32::try_from(image.width).ok()?,
            u32::try_from(image.height).ok()?,
            image.bytes.into_owned(),
        )?;
        Some(keep(DynamicImage::ImageRgba8(rgba)))
    }
}

/// What a file chosen with "From file…" or dropped in becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromFile {
    /// A photo, already made the phone's way.
    Photo(Vec<u8>),
    /// A voice recording, the sound file as it is, and why it cannot be
    /// transcribed here, if it cannot.
    Recording {
        bytes: Vec<u8>,
        problem: Option<oldbrainfix_whisper::Problem>,
    },
}

/// The endings of the sound files "From file…" offers and a drop takes.
pub const SOUND_EXTENSIONS: [&str; 7] = ["ogg", "oga", "opus", "m4a", "mp3", "wav", "aac"];

/// The endings of the pictures "From file…" offers.
pub const PICTURE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];

/// The file `bytes` named `name` as a photo or a voice recording; `None`
/// when it is neither a picture nor a sound file. A sound file is told
/// by its ending or its first bytes, not by whether it decodes here, so
/// one that cannot be decoded is still kept.
pub fn from_file(name: &str, bytes: &[u8]) -> Option<FromFile> {
    if let Some(photo) = photo_from_file(bytes) {
        return Some(FromFile::Photo(photo));
    }
    if bytes.is_empty() || !is_sound(name, bytes) {
        return None;
    }
    Some(FromFile::Recording {
        bytes: bytes.to_vec(),
        problem: oldbrainfix_whisper::problem(bytes),
    })
}

/// Whether the file `bytes` named `name` is a sound file: by its ending
/// ([`SOUND_EXTENSIONS`]) or by its first bytes, as the phone names a
/// recording (Voice.kt): Ogg, WAV, MP3 with its tag or an MPEG audio
/// frame, or MPEG-4.
fn is_sound(name: &str, bytes: &[u8]) -> bool {
    let ending = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);
    if ending.is_some_and(|e| SOUND_EXTENSIONS.contains(&e.as_str())) {
        return true;
    }
    let tag = |at: usize| bytes.get(at..at + 4).unwrap_or_default();
    let byte = |at: usize| bytes.get(at).copied().unwrap_or(0);
    tag(0) == b"OggS"
        || (tag(0) == b"RIFF" && tag(8) == b"WAVE")
        || bytes.starts_with(b"ID3")
        // An MPEG audio frame's sync bits, with a layer set: ADTS AAC
        // has the same sync but no layer.
        || (byte(0) == 0xFF && byte(1) & 0xE0 == 0xE0 && byte(1) & 0x06 != 0)
        || tag(4) == b"ftyp"
}

/// The file extension of the photo `bytes`: PNG, GIF or WebP by their
/// signatures, else JPEG, as the phone names a shared photo (#129).
pub fn photo_extension(bytes: &[u8]) -> &'static str {
    let tag = |at: usize| bytes.get(at..at + 4).unwrap_or_default();
    match tag(0) {
        b"\x89PNG" => "png",
        b"GIF8" => "gif",
        b"RIFF" if tag(8) == b"WEBP" => "webp",
        _ => "jpg",
    }
}

/// The file extension of the recording `bytes`, as the phone names a
/// shared recording (#129): WAV as the desktop records, Ogg as a
/// messenger's voice message is, MP3, else the phone's own AAC in
/// MPEG-4.
pub fn recording_extension(bytes: &[u8]) -> &'static str {
    let tag = |at: usize| bytes.get(at..at + 4).unwrap_or_default();
    let byte = |at: usize| bytes.get(at).copied().unwrap_or(0);
    if tag(0) == b"RIFF" && tag(8) == b"WAVE" {
        "wav"
    } else if tag(0) == b"OggS" {
        "ogg"
    } else if bytes.starts_with(b"ID3")
        // An MPEG audio frame's sync bits, with a layer set: ADTS AAC
        // has the same sync but no layer.
        || (byte(0) == 0xFF && byte(1) & 0xE0 == 0xE0 && byte(1) & 0x06 != 0)
    {
        "mp3"
    } else {
        "m4a"
    }
}

/// The picture file `bytes` (JPEG, PNG, WebP or GIF) as a photo to keep,
/// turned upright as its Exif says; `None` when it is no picture.
pub fn photo_from_file(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_decoder()
        .ok()?;
    let orientation = decoder.orientation().ok();
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    Some(keep(image))
}

/// `image` as a kept photo: at most [`MAX_EDGE`] on the long side, on
/// white, a JPEG at [`QUALITY`] without metadata.
fn keep(image: DynamicImage) -> Vec<u8> {
    let image = if image.width().max(image.height()) > MAX_EDGE {
        image.resize(MAX_EDGE, MAX_EDGE, image::imageops::FilterType::Triangle)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    // A JPEG has no transparency: a see-through screenshot goes on white.
    let rgb = RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let over = |c: u8| ((u16::from(c) * u16::from(a) + 255 * (255 - u16::from(a))) / 255) as u8;
        image::Rgb([over(r), over(g), over(b)])
    });
    let mut out = Vec::new();
    let encoded =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, QUALITY).encode_image(&rgb);
    if encoded.is_err() {
        out.clear();
    }
    out
}

/// The width and height of the photo `bytes`, read from its header
/// without decoding it; `None` when it cannot be read or has no pixels.
pub fn dimensions(bytes: &[u8]) -> Option<[u32; 2]> {
    let (width, height) = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;
    (width > 0 && height > 0).then_some([width, height])
}

/// The photo `jpeg` as pixels to show, at most `edge` on the long side;
/// `None` when it cannot be read.
pub fn pixels(jpeg: &[u8], edge: u32) -> Option<RgbaImage> {
    let image = image::load_from_memory(jpeg).ok()?;
    let image = if image.width().max(image.height()) > edge {
        image.thumbnail(edge, edge)
    } else {
        image
    };
    Some(image.to_rgba8())
}

#[cfg(test)]
mod tests;
