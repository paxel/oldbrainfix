//! Transcripts for the Old Brain Fix desktop (ADR 0007): Whisper from the
//! whisper.cpp sources the phone vendors, built unchanged and reached
//! through the small C bridge in `native/`, in English or German as on
//! the phone, led in with the prompt for the language it hears; the
//! speech model it needs, fetched from the phone's mirror
//! and checked against the phone's checksum ([`model`]); and voice
//! recordings decoded as Whisper hears them, the phone's AAC `.m4a` and
//! the desktop's WAV alike, and the sound files made into voice Notes:
//! MP3, Ogg Vorbis and Ogg Opus, as messengers send voice messages
//! ([`decode`]).

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

use opus_decoder::OpusDecoder;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, CODEC_TYPE_OPUS, Decoder, DecoderOptions};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub mod model;

unsafe extern "C" {
    fn obf_whisper_open(path: *const c_char) -> *mut c_void;
    fn obf_whisper_transcribe(
        ctx: *mut c_void,
        samples: *const f32,
        count: c_int,
        threads: c_int,
        english: *const c_char,
        german: *const c_char,
    ) -> *mut c_char;
    fn obf_whisper_free_text(text: *mut c_char);
    fn obf_whisper_close(ctx: *mut c_void);
}

/// What Whisper is led in with (its initial prompt), by the language it
/// hears; an empty one leads in nothing. The core's `TranscriptPrompts`
/// makes them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prompts {
    pub english: String,
    pub german: String,
}

/// `text` as a C string; a NUL inside it, which no name should hold, is
/// left out.
fn c_text(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

/// Whisper's own rate: 16 kHz.
pub const WHISPER_RATE: u32 = 16_000;

/// A loaded speech model. Used from one thread at a time; dropping it
/// lets the model's memory go.
pub struct Whisper {
    ctx: *mut c_void,
}

// SAFETY: a whisper context belongs to no thread; `&mut self` on every
// use keeps it to one thread at a time.
unsafe impl Send for Whisper {}

impl Whisper {
    /// The model in the file at `path`, or `None` when it cannot be
    /// loaded.
    pub fn open(path: &Path) -> Option<Whisper> {
        let path = CString::new(path.to_str()?).ok()?;
        // SAFETY: a valid C string; the bridge answers null on failure.
        let ctx = unsafe { obf_whisper_open(path.as_ptr()) };
        (!ctx.is_null()).then_some(Whisper { ctx })
    }

    /// The words in `samples`, 16 kHz mono from -1 to 1, in English or
    /// German, trimmed; `None` when they could not be transcribed.
    /// Whisper is led in with `prompts`, the one in the language it hears.
    pub fn transcribe(
        &mut self,
        samples: &[f32],
        threads: usize,
        prompts: &Prompts,
    ) -> Option<String> {
        let count = c_int::try_from(samples.len()).ok()?;
        let threads = c_int::try_from(threads).unwrap_or(1);
        let english = c_text(&prompts.english);
        let german = c_text(&prompts.german);
        // SAFETY: `ctx` is a live context, `samples` holds `count` floats,
        // and both prompts are C strings that live until the call returns.
        let text = unsafe {
            obf_whisper_transcribe(
                self.ctx,
                samples.as_ptr(),
                count,
                threads,
                english.as_ptr(),
                german.as_ptr(),
            )
        };
        if text.is_null() {
            return None;
        }
        // SAFETY: the bridge answers a NUL-terminated string it malloc'd.
        let out = unsafe { CStr::from_ptr(text) }
            .to_string_lossy()
            .trim()
            .to_string();
        // SAFETY: freed once, by the bridge that allocated it.
        unsafe { obf_whisper_free_text(text) };
        Some(out)
    }
}

impl Drop for Whisper {
    fn drop(&mut self) {
        // SAFETY: `ctx` came from `obf_whisper_open` and is freed once.
        unsafe { obf_whisper_close(self.ctx) };
    }
}

/// How many threads Whisper gets on `cores` cores: one is left for the
/// app, at most four, as on the phone.
pub fn threads_for(cores: usize) -> usize {
    cores.saturating_sub(1).clamp(1, 4)
}

/// The voice recording `bytes` as Whisper hears it: 16 kHz mono samples
/// from -1 to 1; `None` when it cannot be read. AAC in MPEG-4, as the
/// phone records, WAV, as the desktop records, and the sound files
/// made into voice Notes ([`pcm`]) are all read.
pub fn decode(bytes: Vec<u8>) -> Option<Vec<f32>> {
    let pcm = pcm(bytes)?;
    Some(to_whisper(&pcm.samples, pcm.channels, pcm.rate))
}

/// A sound file's samples as it holds them: interleaved, from -1 to 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub samples: Vec<f32>,
    pub channels: usize,
    pub rate: u32,
}

/// Why a sound file cannot be decoded here, as the phone tells it
/// (Speech.kt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// No sound found: not a sound file, or a damaged one.
    Unreadable,
    /// A format this computer has no decoder for.
    Unsupported,
}

/// The rate Opus decodes at: its own, whatever rate the sound was
/// recorded at.
const OPUS_RATE: u32 = 48_000;

/// What decodes a sound file's track: symphonia, or the Opus decoder
/// for Ogg Opus, which symphonia only unpacks.
enum Sound {
    Symphonia(Box<dyn Decoder>),
    Opus {
        decoder: Box<OpusDecoder>,
        channels: usize,
        /// The samples at the start that only prime the decoder (the
        /// pre-skip), of all channels.
        skip: usize,
    },
}

/// A sound file opened for decoding.
struct Opened {
    format: Box<dyn FormatReader>,
    track_id: u32,
    sound: Sound,
    rate: Option<u32>,
    channels: Option<usize>,
}

/// The sound file `bytes` opened, its first track with sound ready to
/// decode; why not when it cannot be. Decodes nothing yet.
fn open(bytes: Vec<u8>) -> Result<Opened, Problem> {
    let source = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| Problem::Unreadable)?;
    let format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(Problem::Unreadable)?;
    let params = &track.codec_params;
    let track_id = track.id;
    let (sound, rate, channels) = if params.codec == CODEC_TYPE_OPUS {
        let channels = opus_channels(params.extra_data.as_deref()).ok_or(Problem::Unsupported)?;
        let decoder = OpusDecoder::new(OPUS_RATE, channels).map_err(|_| Problem::Unsupported)?;
        let decoder = Box::new(decoder);
        let skip = params.delay.unwrap_or(0) as usize * channels;
        let sound = Sound::Opus {
            decoder,
            channels,
            skip,
        };
        (sound, Some(OPUS_RATE), Some(channels))
    } else {
        let decoder = symphonia::default::get_codecs()
            .make(params, &DecoderOptions::default())
            .map_err(|_| Problem::Unsupported)?;
        let channels = params.channels.map(|c| c.count());
        (Sound::Symphonia(decoder), params.sample_rate, channels)
    };
    Ok(Opened {
        format,
        track_id,
        sound,
        rate,
        channels,
    })
}

/// The channels of the Ogg Opus stream whose identification header
/// (`OpusHead`, RFC 7845) is `head`: one or two, in one stream; `None`
/// for more, which this decoder does not take.
fn opus_channels(head: Option<&[u8]>) -> Option<usize> {
    let head = head?;
    if head.len() < 19 || &head[..8] != b"OpusHead" {
        return None;
    }
    let (channels, family) = (usize::from(head[9]), head[18]);
    ((family == 0 || family == 1) && (1..=2).contains(&channels)).then_some(channels)
}

/// Why the sound file `bytes` cannot be decoded here, or `None` when it
/// can, as the phone tells it: only opens the decoder, decodes nothing.
pub fn problem(bytes: &[u8]) -> Option<Problem> {
    open(bytes.to_vec()).err()
}

/// The sound file `bytes` decoded: AAC in MPEG-4, WAV, MP3, Ogg Vorbis
/// and Ogg Opus; `None` when it cannot be read. A damaged packet is
/// skipped, as players do.
pub fn pcm(bytes: Vec<u8>) -> Option<Pcm> {
    let Opened {
        mut format,
        track_id,
        mut sound,
        mut rate,
        mut channels,
    } = open(bytes).ok()?;
    let mut samples: Vec<f32> = Vec::new();
    let mut opus = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(DecodeError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return None,
        };
        if packet.track_id() != track_id {
            continue;
        }
        match &mut sound {
            Sound::Symphonia(decoder) => match decoder.decode(&packet) {
                Ok(decoded) => {
                    // What comes out may differ from what went in.
                    let spec = *decoded.spec();
                    rate = Some(spec.rate);
                    channels = Some(spec.channels.count());
                    let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                    buffer.copy_interleaved_ref(decoded);
                    samples.extend_from_slice(buffer.samples());
                }
                Err(DecodeError::DecodeError(_)) => continue,
                Err(_) => return None,
            },
            Sound::Opus {
                decoder, channels, ..
            } => {
                opus.resize(decoder.max_frame_size_per_channel() * *channels, 0.0);
                if let Ok(frames) = decoder.decode_float(&packet.data, &mut opus, false) {
                    samples.extend_from_slice(&opus[..(frames * *channels).min(opus.len())]);
                }
            }
        }
    }
    if let Sound::Opus { skip, .. } = sound {
        // Nothing decoded is no sound.
        if samples.is_empty() {
            return None;
        }
        samples.drain(..skip.min(samples.len()));
    }
    Some(Pcm {
        samples,
        channels: channels?,
        rate: rate?,
    })
}

/// `interleaved` samples of `channels` channels at `rate` as Whisper
/// wants them: mono, the channels averaged, at 16 kHz. A recording
/// already so comes back as it is.
pub fn to_whisper(interleaved: &[f32], channels: usize, rate: u32) -> Vec<f32> {
    resample(&downmix(interleaved, channels), rate, WHISPER_RATE)
}

/// The average of the `channels` channels of `interleaved`; a cut-off
/// last frame is dropped.
pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// `samples` at `from` Hz linearly interpolated to `to` Hz.
pub fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || samples.is_empty() || from == 0 {
        return samples.to_vec();
    }
    let count = (samples.len() as u64 * u64::from(to) / u64::from(from)) as usize;
    let step = f64::from(from) / f64::from(to);
    let last = samples.len() - 1;
    (0..count)
        .map(|i| {
            let at = i as f64 * step;
            let left = (at as usize).min(last);
            let right = (left + 1).min(last);
            let part = (at - left as f64) as f32;
            samples[left] + (samples[right] - samples[left]) * part
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAV file of 16-bit `samples` with `channels` at `rate`.
    fn wav(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
        let data = (samples.len() * 2) as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        out.extend_from_slice(&(channels * 2).to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data.to_le_bytes());
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
        out
    }

    #[test]
    fn channels_are_averaged_and_a_cut_off_frame_dropped() {
        assert_eq!(downmix(&[0.2, 0.4, 1.0, 0.0, 0.5], 2), vec![0.3, 0.5]);
        assert_eq!(downmix(&[0.1, 0.2], 1), vec![0.1, 0.2]);
    }

    #[test]
    fn resampling_interpolates_and_leaves_whisper_rate_alone() {
        assert_eq!(resample(&[0.0, 1.0], 16_000, 16_000), vec![0.0, 1.0]);
        assert_eq!(
            resample(&[0.0, 1.0, 0.0, 1.0], 32_000, 16_000),
            vec![0.0, 0.0]
        );
        assert_eq!(
            resample(&[0.0, 1.0], 8_000, 16_000),
            vec![0.0, 0.5, 1.0, 1.0]
        );
        assert!(resample(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn threads_leave_one_core_and_stop_at_four() {
        assert_eq!(threads_for(1), 1);
        assert_eq!(threads_for(3), 2);
        assert_eq!(threads_for(16), 4);
    }

    #[test]
    fn a_wav_recording_decodes_to_whisper_samples() {
        let samples = decode(wav(&[0, 16_384, -16_384, 32_767], 1, 16_000));
        let samples = samples.unwrap_or_default();
        assert_eq!(samples.len(), 4);
        assert!((samples[1] - 0.5).abs() < 1e-3, "{samples:?}");
        assert!((samples[2] + 0.5).abs() < 1e-3, "{samples:?}");
        let stereo = decode(wav(&[16_384, 0, 16_384, 0], 2, 32_000)).unwrap_or_default();
        assert_eq!(stereo.len(), 1);
        assert!((stereo[0] - 0.25).abs() < 1e-3, "{stereo:?}");
    }

    /// The phone's test voice message: 0.2 s of a 440 Hz tone, 16 kHz
    /// mono, encoded with libopus into an Ogg file, as messengers send.
    const VOICE_MESSAGE: &[u8] = include_bytes!("../testdata/voice-message.ogg");

    /// The pitch of `samples` at `rate`, from its rising zero crossings.
    fn hertz(samples: &[f32], rate: u32) -> f32 {
        let rising = samples
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        rising as f32 * rate as f32 / samples.len() as f32
    }

    #[test]
    fn an_ogg_opus_voice_message_decodes_without_its_pre_skip() {
        assert_eq!(problem(VOICE_MESSAGE), None);
        let pcm = pcm(VOICE_MESSAGE.to_vec()).unwrap_or_else(|| Pcm {
            samples: Vec::new(),
            channels: 0,
            rate: 0,
        });
        assert_eq!((pcm.channels, pcm.rate), (1, 48_000));
        // 0.2 s, give or take a frame of the encoder's.
        assert!(
            (9_600 - 960..=9_600 + 960).contains(&pcm.samples.len()),
            "{}",
            pcm.samples.len()
        );
        let heard = decode(VOICE_MESSAGE.to_vec()).unwrap_or_default();
        assert!((2_880..=3_520).contains(&heard.len()), "{}", heard.len());
        let middle = &heard[400..2_800];
        let pitch = hertz(middle, WHISPER_RATE);
        assert!((pitch - 440.0).abs() < 20.0, "{pitch} Hz");
        assert!(middle.iter().any(|s| s.abs() > 0.1));
    }

    #[test]
    fn opus_in_more_than_one_stream_is_not_taken() {
        let head = |channels: u8, family: u8| {
            let mut head = b"OpusHead".to_vec();
            head.extend_from_slice(&[1, channels, 0x38, 1, 0x80, 0x3E, 0, 0, 0, 0, family]);
            head
        };
        assert_eq!(opus_channels(Some(&head(1, 0))), Some(1));
        assert_eq!(opus_channels(Some(&head(2, 1))), Some(2));
        assert_eq!(opus_channels(Some(&head(6, 1))), None);
        assert_eq!(opus_channels(Some(&head(1, 255))), None);
        assert_eq!(opus_channels(Some(b"OpusHead")), None);
        assert_eq!(opus_channels(None), None);
    }

    #[test]
    fn what_is_no_sound_file_is_unreadable() {
        assert_eq!(problem(b"not a recording"), Some(Problem::Unreadable));
        assert_eq!(problem(&[]), Some(Problem::Unreadable));
        // An Ogg file cut off before its sound.
        assert_eq!(pcm(VOICE_MESSAGE[..100].to_vec()), None);
        assert_eq!(problem(&wav(&[0, 1], 1, 16_000)), None);
    }

    #[test]
    fn what_is_no_recording_does_not_decode() {
        assert_eq!(decode(b"not a recording".to_vec()), None);
        assert_eq!(decode(Vec::new()), None);
    }

    #[test]
    fn a_prompt_with_a_nul_inside_still_goes_without_it() {
        assert_eq!(c_text("Ar\0beit").as_bytes(), b"Arbeit");
        assert_eq!(c_text("").as_bytes(), b"");
    }

    #[test]
    fn a_missing_model_does_not_load() {
        assert!(Whisper::open(Path::new("/nonexistent/ggml-base.bin")).is_none());
    }

    /// Device-only: needs a speech model and a recording, named by
    /// `OBF_WHISPER_MODEL` and `OBF_VOICE_SAMPLE` (a phone .m4a, a
    /// desktop .wav, or a sound file such as an Ogg voice message), and leads in with `OBF_PROMPT` if set:
    /// `cargo test -p oldbrainfix-whisper -- --ignored --nocapture`.
    #[test]
    #[ignore = "needs the speech model and a recording"]
    fn the_pinned_model_transcribes_a_recording() {
        let (Ok(model), Ok(sample)) = (
            std::env::var("OBF_WHISPER_MODEL"),
            std::env::var("OBF_VOICE_SAMPLE"),
        ) else {
            panic!("set OBF_WHISPER_MODEL and OBF_VOICE_SAMPLE");
        };
        let bytes = std::fs::read(sample).unwrap_or_default();
        let samples = decode(bytes);
        assert!(samples.as_ref().is_some_and(|s| !s.is_empty()));
        // `OBF_PROMPT` leads in both languages, if set.
        let prompt = std::env::var("OBF_PROMPT").unwrap_or_default();
        let prompts = Prompts {
            english: prompt.clone(),
            german: prompt,
        };
        let mut whisper = Whisper::open(Path::new(&model));
        let text = whisper
            .as_mut()
            .and_then(|w| w.transcribe(&samples.unwrap_or_default(), 2, &prompts));
        println!("Transcript: {text:?}");
        assert!(text.is_some());
    }
}
