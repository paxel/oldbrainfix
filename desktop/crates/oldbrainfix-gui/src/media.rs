//! Voice recordings and photos on the desktop: the microphone, the
//! speakers, the clipboard, and photos made the phone's way.
//!
//! A recording is WAV, 16-bit, mono, at 16 kHz: no AAC encoder is
//! available in pure Rust, and WAV at Whisper's own rate is what the
//! phone and the desktop both play and transcribe without converting
//! (the phone reads it with its own MediaExtractor). The phone records
//! AAC in `.m4a`; the desktop plays and transcribes that too.
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
        let Ok(decoder) = rodio::Decoder::new(Cursor::new(bytes)) else {
            return false;
        };
        let Ok(sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return false;
        };
        let player = rodio::Player::connect_new(sink.mixer());
        player.append(Stretch::new(decoder, self.speed.clone()));
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
        for i in 0..half {
            for c in 0..channels {
                stretch.tail[i * channels + c] =
                    stretch.fade[half + i] * stretch.sample(i as isize, c);
            }
        }
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
        for i in 0..self.half {
            for c in 0..self.channels {
                let rising = self.fade[i] * self.sample(start + i as isize, c);
                self.out.push(self.tail[i * self.channels + c] + rising);
            }
        }
        for i in 0..self.half {
            for c in 0..self.channels {
                let falling = self.fade[self.half + i] * self.sample(start + half + i as isize, c);
                self.tail[i * self.channels + c] = falling;
            }
        }
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
        let sample = self.out[self.at];
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

/// The system's clipboard, through arboard. Device-only.
#[derive(Default)]
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        let image = arboard::Clipboard::new().ok()?.get_image().ok()?;
        let rgba = RgbaImage::from_raw(
            u32::try_from(image.width).ok()?,
            u32::try_from(image.height).ok()?,
            image.bytes.into_owned(),
        )?;
        Some(keep(DynamicImage::ImageRgba8(rgba)))
    }
}

/// The picture file `bytes` (JPEG, PNG or WebP) as a photo to keep,
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
mod tests {
    use super::*;

    fn png(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
        let image = RgbaImage::from_pixel(width, height, image::Rgba(pixel));
        let mut out = Cursor::new(Vec::new());
        let written = DynamicImage::ImageRgba8(image).write_to(&mut out, image::ImageFormat::Png);
        assert!(written.is_ok());
        out.into_inner()
    }

    #[test]
    fn a_recording_is_mono_16_bit_wav_at_whisper_rate() {
        let bytes = wav(&[0.0, 0.5, -1.0, 2.0]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(bytes.len(), 44 + 8);
        let samples = oldbrainfix_whisper::decode(bytes.clone()).unwrap_or_default();
        assert_eq!(samples.len(), 4);
        assert!((samples[1] - 0.5).abs() < 1e-3, "{samples:?}");
        // Beyond full scale is clipped, not wrapped.
        assert_eq!(i16::from_le_bytes([bytes[50], bytes[51]]), 32767);
        assert_eq!(i16::from_le_bytes([bytes[48], bytes[49]]), -32767);
    }

    /// `seconds` of a tone of `hertz`, as a recording, decoded as the
    /// speakers decode it.
    fn tone(hertz: f32, seconds: f32) -> Option<rodio::Decoder<Cursor<Vec<u8>>>> {
        let rate = SAMPLE_RATE as f32;
        let samples: Vec<f32> = (0..(seconds * rate) as usize)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * hertz * i as f32 / rate).sin())
            .collect();
        rodio::Decoder::new(Cursor::new(wav(&samples))).ok()
    }

    /// The pitch of `samples` at [`SAMPLE_RATE`], from its rising zero
    /// crossings, leaving out the first and last tenth.
    fn hertz(samples: &[f32]) -> f32 {
        let middle = &samples[samples.len() / 10..samples.len() * 9 / 10];
        let rising = middle
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        rising as f32 * SAMPLE_RATE as f32 / middle.len() as f32
    }

    fn stretched(speed: f32) -> Vec<f32> {
        let Some(decoded) = tone(440.0, 2.0) else {
            return Vec::new();
        };
        Stretch::new(decoded, Arc::new(AtomicU32::new(speed.to_bits()))).collect()
    }

    #[test]
    fn faster_or_slower_playback_keeps_the_pitch_and_the_loudness() {
        let heard = 2 * SAMPLE_RATE as usize;
        for speed in [0.5f32, 0.75, 1.5, 2.0] {
            let played = stretched(speed);
            let expected = heard as f32 / speed;
            let off = (played.len() as f32 - expected).abs();
            assert!(
                off < expected * 0.02 + 480.0,
                "{speed}: {} samples",
                played.len()
            );
            let pitch = hertz(&played);
            assert!((pitch - 440.0).abs() < 440.0 * 0.02, "{speed}: {pitch} Hz");
            // Pieces joined out of step would cancel each other out.
            let full = 0.5 / 2f32.sqrt();
            let middle = &played[played.len() / 10..played.len() * 9 / 10];
            for window in middle.as_chunks::<320>().0 {
                let loudness = (window.iter().map(|s| s * s).sum::<f32>() / 320.0).sqrt();
                assert!((loudness - full).abs() < full * 0.1, "{speed}: {loudness}");
            }
        }
    }

    #[test]
    fn at_normal_speed_the_recording_plays_unchanged() {
        let Some(decoded) = tone(440.0, 2.0) else {
            panic!("the tone decodes");
        };
        let heard: Vec<f32> = decoded.collect();
        let played = stretched(1.0);
        assert!(played.len() >= heard.len() && played.len() < heard.len() + 480);
        for (i, (a, b)) in heard.iter().zip(&played).enumerate() {
            assert!((a - b).abs() < 1e-5, "{i}: {a} {b}");
        }
        assert!(played[heard.len()..].iter().all(|s| s.abs() < 1e-5));
    }

    #[test]
    fn a_stereo_recording_keeps_each_channels_pitch() {
        let (rate, frames) = (48_000usize, 96_000usize);
        let wave = |hertz: f32, i: usize| {
            0.5 * (2.0 * std::f32::consts::PI * hertz * i as f32 / rate as f32).sin()
        };
        let samples: Vec<f32> = (0..frames)
            .flat_map(|i| [wave(440.0, i), wave(660.0, i)])
            .collect();
        let (Some(channels), Some(per_second)) = (
            std::num::NonZero::new(2u16),
            std::num::NonZero::new(rate as u32),
        ) else {
            panic!("two channels at 48 kHz");
        };
        let decoded = rodio::buffer::SamplesBuffer::new(channels, per_second, samples);
        let played: Vec<f32> =
            Stretch::new(decoded, Arc::new(AtomicU32::new(2.0f32.to_bits()))).collect();
        assert_eq!(played.len() % 2, 0);
        for (channel, expected) in [(0, 440.0), (1, 660.0)] {
            let one: Vec<f32> = played.iter().skip(channel).step_by(2).copied().collect();
            assert!(
                (one.len() as f32 - 48_000.0).abs() < 2_000.0,
                "{}",
                one.len()
            );
            // [`hertz`] counts at 16 kHz; this is three times that.
            let pitch = 3.0 * hertz(&one);
            assert!(
                (pitch - expected).abs() < expected * 0.02,
                "{channel}: {pitch} Hz"
            );
        }
    }

    #[test]
    fn a_new_speed_applies_to_the_recording_playing() {
        let Some(decoded) = tone(440.0, 2.0) else {
            panic!("the tone decodes");
        };
        let speed = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let mut playing = Stretch::new(decoded, speed.clone());
        let first: Vec<f32> = playing.by_ref().take(SAMPLE_RATE as usize).collect();
        speed.store(2.0f32.to_bits(), Ordering::Relaxed);
        let rest: Vec<f32> = playing.collect();
        // One second at normal speed, the other at double, but for the
        // piece already playing and the last one's fade-out.
        let expected = SAMPLE_RATE as f32 / 2.0;
        assert!(
            (rest.len() as f32 - expected).abs() < 960.0,
            "{}",
            rest.len()
        );
        assert!((hertz(&first) - 440.0).abs() < 9.0);
        assert!((hertz(&rest) - 440.0).abs() < 9.0, "{}", hertz(&rest));
    }

    #[test]
    fn a_big_see_through_picture_is_kept_small_on_white() {
        let kept = photo_from_file(&png(4096, 1024, [255, 0, 0, 0])).unwrap_or_default();
        assert_eq!(&kept[0..2], &[0xFF, 0xD8]);
        let back = image::load_from_memory(&kept).map(|i| i.to_rgb8());
        let back = back.unwrap_or_default();
        assert_eq!(back.dimensions(), (2048, 512));
        let [r, g, b] = back.get_pixel(10, 10).0;
        assert!(r > 240 && g > 240 && b > 240, "{r} {g} {b}");
    }

    #[test]
    fn a_small_picture_keeps_its_size_and_what_is_no_picture_is_refused() {
        let kept = photo_from_file(&png(30, 20, [0, 0, 255, 255])).unwrap_or_default();
        let shown = pixels(&kept, 1024).map(|p| p.dimensions());
        assert_eq!(shown, Some((30, 20)));
        assert_eq!(pixels(&kept, 15).map(|p| p.dimensions()), Some((15, 10)));
        assert_eq!(photo_from_file(b"not a picture"), None);
        assert_eq!(pixels(b"not a picture", 100), None);
    }

    #[test]
    fn a_kept_photo_has_no_metadata_left_to_strip() {
        let kept = photo_from_file(&png(8, 8, [1, 2, 3, 255])).unwrap_or_default();
        assert_eq!(oldbrainfix_core::photos::strip_jpeg_metadata(&kept), kept);
    }
}
