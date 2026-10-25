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
use std::sync::atomic::{AtomicBool, Ordering};
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
#[derive(Default)]
pub struct SpeakerPlayer {
    playing: Option<(rodio::MixerDeviceSink, rodio::Player)>,
}

impl Player for SpeakerPlayer {
    fn play(&mut self, bytes: Vec<u8>) -> bool {
        self.stop();
        let Ok(sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return false;
        };
        match rodio::play(sink.mixer(), Cursor::new(bytes)) {
            Ok(player) => {
                self.playing = Some((sink, player));
                true
            }
            Err(_) => false,
        }
    }

    fn stop(&mut self) {
        if let Some((_, player)) = self.playing.take() {
            player.stop();
        }
    }

    fn playing(&self) -> bool {
        self.playing.as_ref().is_some_and(|(_, p)| !p.empty())
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
