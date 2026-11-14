//! The global hotkey that opens the capture popup from anywhere. On X11,
//! Windows and macOS the app grabs the key itself (global-hotkey); on
//! Wayland only the desktop can, so the app asks the GlobalShortcuts
//! portal, and the desktop confirms the key and may change it. Where
//! neither works there is no hotkey, and a key bound to
//! `oldbrainfix capture` in the desktop's own settings stands in.
//!
//! On Windows and macOS the key is heard through the window's event
//! loop, which runs for as long as the app, the window hidden into the
//! tray while closed; without a tray, closing the window ends the app.

use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

/// The id of the one shortcut the app asks the portal for.
pub const PORTAL_ID: &str = "capture";

/// A key chord such as Ctrl+Alt+N: at least one of Ctrl, Alt and Super,
/// maybe Shift, and a letter, a digit, F1 to F12 or Space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
    pub key: ChordKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordKey {
    /// 'A' to 'Z'.
    Letter(char),
    /// '0' to '9'.
    Digit(char),
    /// F1 to F12.
    F(u8),
    Space,
}

impl FromStr for Chord {
    type Err = ();

    /// Reads a chord as typed, in any case, with `+` between the parts:
    /// "ctrl+alt+n", "Super+Shift+F5", "Control+Option+Space".
    fn from_str(text: &str) -> Result<Chord, ()> {
        let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
        let key = parts.pop().ok_or(())?;
        let mut chord = Chord {
            ctrl: false,
            alt: false,
            shift: false,
            logo: false,
            key: parse_key(key).ok_or(())?,
        };
        for part in parts {
            let flag = match part.to_lowercase().as_str() {
                "ctrl" | "control" | "strg" => &mut chord.ctrl,
                "alt" | "option" => &mut chord.alt,
                "shift" | "umschalt" => &mut chord.shift,
                "super" | "meta" | "win" | "logo" | "cmd" | "command" => &mut chord.logo,
                _ => return Err(()),
            };
            if *flag {
                return Err(());
            }
            *flag = true;
        }
        // A key on its own, or with Shift only, is needed for typing.
        if !(chord.ctrl || chord.alt || chord.logo) {
            return Err(());
        }
        Ok(chord)
    }
}

fn parse_key(key: &str) -> Option<ChordKey> {
    let upper = key.to_uppercase();
    let mut chars = upper.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_uppercase() => return Some(ChordKey::Letter(c)),
        (Some(c), None) if c.is_ascii_digit() => return Some(ChordKey::Digit(c)),
        _ => {}
    }
    if upper == "SPACE" {
        return Some(ChordKey::Space);
    }
    upper
        .strip_prefix('F')
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=12).contains(n))
        .map(ChordKey::F)
}

impl fmt::Display for Chord {
    /// The chord as the settings show it, such as "Ctrl+Alt+N".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in [
            (self.ctrl, "Ctrl+"),
            (self.alt, "Alt+"),
            (self.shift, "Shift+"),
            (self.logo, "Super+"),
        ] {
            if on {
                f.write_str(name)?;
            }
        }
        match self.key {
            ChordKey::Letter(c) | ChordKey::Digit(c) => write!(f, "{c}"),
            ChordKey::F(n) => write!(f, "F{n}"),
            ChordKey::Space => f.write_str("Space"),
        }
    }
}

impl Chord {
    /// The chord of `key` pressed with `modifiers` in the window, as the
    /// Settings record it (#283): Ctrl, Alt, Shift and on macOS Cmd as
    /// Super; `None` for a key no chord takes, or one without Ctrl, Alt
    /// or Cmd. Off macOS the window hears no Super, so such a chord is
    /// typed.
    pub fn pressed(key: egui::Key, modifiers: egui::Modifiers) -> Option<Chord> {
        let chord = Chord {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            logo: modifiers.mac_cmd,
            key: parse_key(key.name())?,
        };
        (chord.ctrl || chord.alt || chord.logo).then_some(chord)
    }

    /// The chord as the XDG shortcuts specification writes a trigger,
    /// for the portal: "CTRL+ALT+n".
    pub fn portal_trigger(&self) -> String {
        let mut out = String::new();
        for (on, name) in [
            (self.ctrl, "CTRL+"),
            (self.alt, "ALT+"),
            (self.shift, "SHIFT+"),
            (self.logo, "LOGO+"),
        ] {
            if on {
                out.push_str(name);
            }
        }
        match self.key {
            ChordKey::Letter(c) => out.push(c.to_ascii_lowercase()),
            ChordKey::Digit(c) => out.push(c),
            ChordKey::F(n) => out.push_str(&format!("F{n}")),
            ChordKey::Space => out.push_str("space"),
        }
        out
    }

    /// The chord as a key the app grabs itself.
    pub fn hotkey(&self) -> Option<HotKey> {
        let mut mods = Modifiers::empty();
        for (on, m) in [
            (self.ctrl, Modifiers::CONTROL),
            (self.alt, Modifiers::ALT),
            (self.shift, Modifiers::SHIFT),
            (self.logo, Modifiers::META),
        ] {
            if on {
                mods.insert(m);
            }
        }
        let code = match self.key {
            ChordKey::Letter(c) => Code::from_str(&format!("Key{c}")).ok()?,
            ChordKey::Digit(c) => Code::from_str(&format!("Digit{c}")).ok()?,
            ChordKey::F(n) => Code::from_str(&format!("F{n}")).ok()?,
            ChordKey::Space => Code::Space,
        };
        Some(HotKey::new(Some(mods), code))
    }
}

/// How the hotkey stands, for the settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The app holds the chord.
    Active,
    /// Asking the desktop through the portal.
    Waiting,
    /// The desktop holds the shortcut, with its own description of the
    /// key it put it on, if it has one yet.
    Desktop(Option<String>),
    /// Another program holds the chord.
    Taken,
    /// No hotkey on this desktop.
    Unavailable,
}

/// The hotkey as the settings see it.
pub trait Control {
    fn status(&self) -> Status;
    /// Moves the hotkey to `chord`, where the app chooses the key.
    fn rebind(&mut self, chord: &Chord);
    /// Lets the key go while the Settings record a new one, so that it
    /// reaches the window, and with `false` takes it again (#283); the
    /// desktop's key through the portal stays.
    fn pause(&mut self, _paused: bool) {}
}

/// Which way to the hotkey this session offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    /// The app grabs the key: X11, Windows, macOS.
    Grab,
    /// Wayland: the GlobalShortcuts portal.
    Portal,
    /// No display to listen on.
    None,
}

/// The session for these environment variables: Wayland wins over X11,
/// since a key grabbed through XWayland only fires while an XWayland
/// window has the keyboard.
pub fn session(var: impl Fn(&str) -> Option<String>) -> Session {
    if !cfg!(target_os = "linux") {
        return Session::Grab;
    }
    let set = |name: &str| var(name).is_some_and(|v| !v.is_empty());
    if set("WAYLAND_DISPLAY") {
        Session::Portal
    } else if set("DISPLAY") {
        Session::Grab
    } else {
        Session::None
    }
}

enum Way {
    Grab {
        manager: GlobalHotKeyManager,
        held: Option<HotKey>,
    },
    Portal,
    None,
}

/// The hotkey while the app runs; it goes when this is dropped, except
/// the portal's, which lives as long as the process.
pub struct Hotkey {
    way: Way,
    status: Arc<Mutex<Status>>,
}

impl Hotkey {
    /// Sets up the hotkey on `chord` for this session; `pressed` runs on
    /// another thread whenever it is pressed. `description` names the
    /// shortcut in the desktop's settings. Call it on the main thread.
    pub fn start(
        chord: Option<Chord>,
        description: String,
        pressed: Box<dyn Fn() + Send + Sync>,
    ) -> Hotkey {
        let status = Arc::new(Mutex::new(Status::Unavailable));
        let way = match session(|name| std::env::var(name).ok()) {
            Session::Grab => match GlobalHotKeyManager::new() {
                Ok(manager) => {
                    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
                        if e.state() == HotKeyState::Pressed {
                            pressed();
                        }
                    }));
                    Way::Grab {
                        manager,
                        held: None,
                    }
                }
                Err(e) => {
                    eprintln!("oldbrainfix: no hotkey: {e}");
                    Way::None
                }
            },
            Session::Portal => {
                set(&status, Status::Waiting);
                portal::spawn(chord, description, status.clone(), pressed);
                Way::Portal
            }
            Session::None => Way::None,
        };
        let mut hotkey = Hotkey { way, status };
        if let (Way::Grab { .. }, Some(chord)) = (&hotkey.way, chord) {
            hotkey.rebind(&chord);
        }
        hotkey
    }
}

fn set(status: &Mutex<Status>, to: Status) {
    if let Ok(mut held) = status.lock() {
        *held = to;
    }
}

impl Control for Hotkey {
    fn status(&self) -> Status {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(Status::Unavailable)
    }

    fn rebind(&mut self, chord: &Chord) {
        let Way::Grab { manager, held } = &mut self.way else {
            // The desktop chooses the portal's key, in its own settings.
            return;
        };
        if let Some(old) = held.take() {
            drop(manager.unregister(old));
        }
        let to = match chord.hotkey().map(|key| (key, manager.register(key))) {
            Some((key, Ok(()))) => {
                *held = Some(key);
                Status::Active
            }
            Some((_, Err(_))) => Status::Taken,
            None => Status::Unavailable,
        };
        set(&self.status, to);
    }

    fn pause(&mut self, paused: bool) {
        let Way::Grab {
            manager,
            held: Some(key),
        } = &self.way
        else {
            return;
        };
        if paused {
            drop(manager.unregister(*key));
        } else if manager.register(*key).is_err() {
            set(&self.status, Status::Taken);
        }
    }
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        if let Way::Grab {
            manager,
            held: Some(key),
        } = &self.way
        {
            drop(manager.unregister(*key));
        }
    }
}

#[cfg(target_os = "linux")]
mod portal {
    use std::sync::{Arc, Mutex};

    use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
    use futures_lite::StreamExt;

    use super::{Chord, PORTAL_ID, Status, set};

    /// Asks the portal for the shortcut on a thread of its own, which
    /// then waits for it for as long as the app runs.
    pub fn spawn(
        chord: Option<Chord>,
        description: String,
        status: Arc<Mutex<Status>>,
        pressed: Box<dyn Fn() + Send + Sync>,
    ) {
        std::thread::spawn(move || {
            let ran = async_io::block_on(listen(chord, description, &status, pressed));
            if let Err(e) = ran {
                eprintln!("oldbrainfix: no hotkey from the desktop: {e}");
                set(&status, Status::Unavailable);
            }
        });
    }

    async fn listen(
        chord: Option<Chord>,
        description: String,
        status: &Mutex<Status>,
        pressed: Box<dyn Fn() + Send + Sync>,
    ) -> ashpd::Result<()> {
        // Outside a sandbox the desktop knows the app by its id only once
        // told; older portals do not ask, so a refusal is no matter.
        if let Ok(id) = ashpd::AppID::try_from(oldbrainfix_core::APP_ID) {
            drop(ashpd::register_host_app(id).await);
        }
        let portal = GlobalShortcuts::new().await?;
        let session = portal.create_session(Default::default()).await?;
        let trigger = chord.map(|c| c.portal_trigger());
        let shortcut =
            NewShortcut::new(PORTAL_ID, description).preferred_trigger(trigger.as_deref());
        let activated = portal.receive_activated().await?;
        let bound = portal
            .bind_shortcuts(&session, &[shortcut], None, Default::default())
            .await?
            .response()?;
        let key = bound
            .shortcuts()
            .iter()
            .find(|s| s.id() == PORTAL_ID)
            .map(|s| s.trigger_description().to_string())
            .filter(|d| !d.is_empty());
        set(status, Status::Desktop(key));
        let mut activated = std::pin::pin!(activated);
        while let Some(a) = activated.next().await {
            if a.shortcut_id() == PORTAL_ID {
                pressed();
            }
        }
        drop(session);
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
mod portal {
    use std::sync::{Arc, Mutex};

    use super::{Chord, Status};

    /// There is no portal off Linux.
    pub fn spawn(
        _chord: Option<Chord>,
        _description: String,
        _status: Arc<Mutex<Status>>,
        _pressed: Box<dyn Fn() + Send + Sync>,
    ) {
    }
}

#[cfg(test)]
mod tests;
