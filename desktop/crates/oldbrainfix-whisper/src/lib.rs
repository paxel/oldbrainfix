//! Transcripts for the Old Brain Fix desktop (ADR 0007): Whisper from the
//! whisper.cpp sources the phone vendors, built unchanged and reached
//! through the small C bridge in `native/`, in English or German as on
//! the phone; the speech model it needs, fetched from the phone's mirror
//! and checked against the phone's checksum ([`model`]); and voice
//! recordings decoded as Whisper hears them, the phone's AAC `.m4a` and
//! the desktop's WAV alike ([`decode`]).

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as DecodeError;
use symphonia::core::formats::FormatOptions;
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
    ) -> *mut c_char;
    fn obf_whisper_free_text(text: *mut c_char);
    fn obf_whisper_close(ctx: *mut c_void);
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
    pub fn transcribe(&mut self, samples: &[f32], threads: usize) -> Option<String> {
        let count = c_int::try_from(samples.len()).ok()?;
        let threads = c_int::try_from(threads).unwrap_or(1);
        // SAFETY: `ctx` is a live context, `samples` holds `count` floats.
        let text = unsafe { obf_whisper_transcribe(self.ctx, samples.as_ptr(), count, threads) };
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
/// phone records, and WAV, as the desktop records, are both read.
pub fn decode(bytes: Vec<u8>) -> Option<Vec<f32>> {
    let source = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;
    let mut rate = track.codec_params.sample_rate;
    let mut channels = track.codec_params.channels.map(|c| c.count());
    let mut pcm: Vec<f32> = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(DecodeError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(_) => return None,
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                // What comes out may differ from what went in.
                let spec = *decoded.spec();
                rate = Some(spec.rate);
                channels = Some(spec.channels.count());
                let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                buffer.copy_interleaved_ref(decoded);
                pcm.extend_from_slice(buffer.samples());
            }
            // A damaged packet is skipped, as players do.
            Err(DecodeError::DecodeError(_)) => continue,
            Err(_) => return None,
        }
    }
    Some(to_whisper(&pcm, channels?, rate?))
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

    #[test]
    fn what_is_no_recording_does_not_decode() {
        assert_eq!(decode(b"not a recording".to_vec()), None);
        assert_eq!(decode(Vec::new()), None);
    }

    #[test]
    fn a_missing_model_does_not_load() {
        assert!(Whisper::open(Path::new("/nonexistent/ggml-base.bin")).is_none());
    }

    /// Device-only: needs the 148 MB model and a recording, named by
    /// `OBF_WHISPER_MODEL` and `OBF_VOICE_SAMPLE` (a phone .m4a or a
    /// desktop .wav):
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
        let mut whisper = Whisper::open(Path::new(&model));
        let text = whisper
            .as_mut()
            .and_then(|w| w.transcribe(&samples.unwrap_or_default(), 2));
        println!("Transcript: {text:?}");
        assert!(text.is_some());
    }
}
