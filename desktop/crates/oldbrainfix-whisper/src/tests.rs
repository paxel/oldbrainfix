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
