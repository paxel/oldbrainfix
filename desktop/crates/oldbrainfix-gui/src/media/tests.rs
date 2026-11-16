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
fn a_gif_photo_shows_as_imported_from_keep() {
    let picture = RgbaImage::from_pixel(8, 4, image::Rgba([0, 0, 255, 255]));
    let mut gif = Vec::new();
    DynamicImage::ImageRgba8(picture)
        .write_to(&mut Cursor::new(&mut gif), image::ImageFormat::Gif)
        .unwrap();
    assert_eq!(photo_extension(&gif), "gif");
    assert_eq!(dimensions(&gif), Some([8, 4]));
    assert_eq!(pixels(&gif, 1024).map(|p| p.dimensions()), Some((8, 4)));
}

/// The phone's test voice message: 0.2 s of a 440 Hz tone in Ogg
/// Opus, as messengers send.
const VOICE_MESSAGE: &[u8] =
    include_bytes!("../../../oldbrainfix-whisper/testdata/voice-message.ogg");

#[test]
fn a_picture_file_becomes_a_photo_and_a_sound_file_a_recording() {
    let picture = from_file("receipt.png", &png(30, 20, [0, 0, 255, 255]));
    assert!(matches!(picture, Some(FromFile::Photo(ref p)) if p[0..2] == [0xFF, 0xD8]));
    // An Ogg voice message is kept as it is, by its ending or its
    // first bytes alone.
    for name in ["voice.ogg", "voice", "VOICE.OPUS"] {
        assert_eq!(
            from_file(name, VOICE_MESSAGE),
            Some(FromFile::Recording {
                bytes: VOICE_MESSAGE.to_vec(),
                problem: None,
            }),
            "{name}"
        );
    }
    assert!(matches!(
        from_file("talk.wav", &wav(&[0.1, 0.2])),
        Some(FromFile::Recording { problem: None, .. })
    ));
}

#[test]
fn a_sound_file_that_does_not_decode_is_kept_with_why() {
    assert_eq!(
        from_file("damaged.m4a", b"not a recording"),
        Some(FromFile::Recording {
            bytes: b"not a recording".to_vec(),
            problem: Some(oldbrainfix_whisper::Problem::Unreadable),
        })
    );
    // Neither a picture nor a sound file: left alone.
    assert_eq!(from_file("notes.txt", b"plain text"), None);
    assert_eq!(from_file("empty.ogg", b""), None);
}

#[test]
fn shared_photos_and_recordings_are_named_as_the_phone_names_them() {
    assert_eq!(photo_extension(b"\x89PNG\r\n"), "png");
    assert_eq!(photo_extension(b"GIF89a"), "gif");
    assert_eq!(photo_extension(b"RIFF\0\0\0\0WEBPVP8 "), "webp");
    assert_eq!(photo_extension(&[0xFF, 0xD8, 0xFF]), "jpg");
    assert_eq!(photo_extension(b""), "jpg");
    assert_eq!(recording_extension(&wav(&[0.0])), "wav");
    assert_eq!(recording_extension(b"OggS\0"), "ogg");
    assert_eq!(recording_extension(b"ID3\x04\0\0\0"), "mp3");
    assert_eq!(recording_extension(&[0xFF, 0xFB, 0x90, 0]), "mp3");
    assert_eq!(recording_extension(&[0xFF, 0xF1, 0x50, 0x80]), "m4a");
    assert_eq!(recording_extension(b"\0\0\0\x20ftypM4A "), "m4a");
}

#[test]
fn sound_files_are_told_by_their_first_bytes_as_the_phone_names_them() {
    assert!(is_sound("x", b"ID3\x04\0\0\0"));
    assert!(is_sound("x", &[0xFF, 0xFB, 0x90, 0]));
    assert!(!is_sound("x", &[0xFF, 0xF1, 0x50, 0x80]));
    assert!(is_sound("x", b"\0\0\0\x20ftypM4A "));
    assert!(!is_sound("x", b"RIFF\0\0\0\0AVI "));
    assert!(!is_sound("x.txt", b"plain"));
}

#[test]
fn ogg_opus_plays_through_the_whisper_decoder() {
    use rodio::Source;
    // rodio's own decoders take no Opus.
    assert!(rodio::Decoder::new(Cursor::new(VOICE_MESSAGE.to_vec())).is_err());
    let Some(sound) = playable(VOICE_MESSAGE.to_vec()) else {
        panic!("the voice message plays");
    };
    assert_eq!(
        (sound.channels().get(), sound.sample_rate().get()),
        (1, 48_000)
    );
    assert!(sound.count() > 8_000);
    assert!(playable(b"not a recording".to_vec()).is_none());
}

#[test]
fn a_kept_photo_has_no_metadata_left_to_strip() {
    let kept = photo_from_file(&png(8, 8, [1, 2, 3, 255])).unwrap_or_default();
    assert_eq!(oldbrainfix_core::photos::strip_jpeg_metadata(&kept), kept);
}
