//! The sound an Occurrence rings with on this computer (#102): Old Brain
//! Fix's own chime, which `tool/alarm_sound.py` synthesizes for the phone
//! too, or a sound file chosen in the Settings (ADR 0008). The app plays
//! it itself through rodio, so the notification stays silent; an
//! escalated alarm keeps the system's alarm sound instead.

use std::borrow::Cow;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};

/// Old Brain Fix's own chime, as the phone rings it.
pub static OWN_SOUND: &[u8] =
    include_bytes!("../../../../android/app/src/main/res/raw/obf_alarm.wav");

/// Which sound rings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    /// The app's own chime.
    Own,
    /// The sound file chosen in the Settings.
    Chosen,
}

/// The sound to ring for the file `chosen`, and which it is: the file
/// when it can be read and plays, the own chime when none is chosen, the
/// file is gone, or it is no sound the desktop can play.
pub fn alarm_sound(chosen: Option<&str>) -> (Cow<'static, [u8]>, Rung) {
    if let Some(path) = chosen
        && let Ok(bytes) = std::fs::read(path)
        && plays(&bytes)
    {
        return (Cow::Owned(bytes), Rung::Chosen);
    }
    (Cow::Borrowed(OWN_SOUND), Rung::Own)
}

/// Whether `bytes` decode to a sound with something in it.
fn plays(bytes: &[u8]) -> bool {
    crate::media::playable(bytes.to_vec()).is_some_and(|mut sound| sound.next().is_some())
}

/// Plays the sound of an Occurrence that rings.
pub trait Ringer: Send {
    /// Plays `sound` once, in place of one still playing.
    fn ring(&mut self, sound: Cow<'static, [u8]>);
}

/// The system's default speakers, through rodio, on a thread of their
/// own: a sink is opened for each ring and closed once it played out.
pub struct SpeakerRinger {
    sounds: Sender<Vec<u8>>,
}

impl SpeakerRinger {
    pub fn new() -> SpeakerRinger {
        let (sounds, played) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("ring".into())
            .spawn(move || play(&played));
        if let Err(e) = spawned {
            eprintln!("oldbrainfix: alarm sound: {e}");
        }
        SpeakerRinger { sounds }
    }
}

impl Default for SpeakerRinger {
    fn default() -> Self {
        SpeakerRinger::new()
    }
}

impl Ringer for SpeakerRinger {
    fn ring(&mut self, sound: Cow<'static, [u8]>) {
        // Without the thread nothing rings; the notification still shows.
        drop(self.sounds.send(sound.into_owned()));
    }
}

/// Plays each sound `played` brings until it is closed, a new one in
/// place of one still playing.
fn play(played: &Receiver<Vec<u8>>) {
    let mut playing: Option<(rodio::MixerDeviceSink, rodio::Player)> = None;
    loop {
        let next = if let Some((_, player)) = &playing {
            match played.recv_timeout(std::time::Duration::from_millis(500)) {
                Ok(bytes) => Some(bytes),
                Err(RecvTimeoutError::Timeout) => {
                    if player.empty() {
                        playing = None;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            played.recv().ok()
        };
        let Some(bytes) = next else {
            return;
        };
        if let Some((_, player)) = playing.take() {
            player.stop();
        }
        let Some(sound) = crate::media::playable(bytes) else {
            continue;
        };
        match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);
                let player = rodio::Player::connect_new(sink.mixer());
                player.append(sound);
                playing = Some((sink, player));
            }
            Err(e) => eprintln!("oldbrainfix: alarm sound: {e}"),
        }
    }
}

/// Keeps what would have rung, readable from outside.
#[derive(Debug, Default, Clone)]
pub struct RecordingRinger {
    pub rung: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl RecordingRinger {
    /// Everything rung so far, taken out.
    pub fn take(&self) -> Vec<Vec<u8>> {
        self.rung
            .lock()
            .map(|mut r| std::mem::take(&mut *r))
            .unwrap_or_default()
    }
}

impl Ringer for RecordingRinger {
    fn ring(&mut self, sound: Cow<'static, [u8]>) {
        if let Ok(mut rung) = self.rung.lock() {
            rung.push(sound.into_owned());
        }
    }
}

#[cfg(test)]
mod tests;
