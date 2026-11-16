use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_own_chime_rings_until_a_playable_file_is_chosen() -> TestResult {
    assert!(plays(OWN_SOUND));
    assert_eq!(alarm_sound(None), (Cow::Borrowed(OWN_SOUND), Rung::Own));
    let dir = tempfile::tempdir()?;
    let gone = dir.path().join("gone.wav");
    assert_eq!(alarm_sound(gone.to_str()).1, Rung::Own);
    let broken = dir.path().join("broken.mp3");
    std::fs::write(&broken, b"not a sound at all")?;
    assert_eq!(alarm_sound(broken.to_str()).1, Rung::Own);
    // A WAV with nothing in it would ring silently.
    let empty = dir.path().join("empty.wav");
    std::fs::write(&empty, silent_wav())?;
    assert_eq!(alarm_sound(empty.to_str()).1, Rung::Own);
    let bell = dir.path().join("bell.wav");
    let tone = tone_wav();
    std::fs::write(&bell, &tone)?;
    let (sound, rung) = alarm_sound(bell.to_str());
    assert_eq!(rung, Rung::Chosen);
    assert_eq!(sound.as_ref(), tone.as_slice());
    Ok(())
}

#[test]
fn a_recording_ringer_keeps_what_rang() {
    let ringer = RecordingRinger::default();
    let mut boxed: Box<dyn Ringer> = Box::new(ringer.clone());
    boxed.ring(Cow::Borrowed(OWN_SOUND));
    assert_eq!(ringer.take(), [OWN_SOUND.to_vec()]);
    assert!(ringer.take().is_empty());
}

/// A mono 16-bit WAV of `samples`.
fn wav(samples: &[i16]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&8000u32.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

fn silent_wav() -> Vec<u8> {
    wav(&[])
}

/// A tenth of a second of 440 Hz.
fn tone_wav() -> Vec<u8> {
    let samples: Vec<i16> = (0..800)
        .map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / 8000.0).sin() * 8000.0) as i16)
        .collect();
    wav(&samples)
}
