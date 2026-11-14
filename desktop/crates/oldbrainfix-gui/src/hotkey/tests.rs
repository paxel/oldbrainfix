use super::*;

fn chord(text: &str) -> Option<Chord> {
    text.parse().ok()
}

#[test]
fn chords_read_as_typed_and_show_one_way() {
    let c = chord("ctrl + alt + n");
    assert_eq!(c.map(|c| c.to_string()).as_deref(), Some("Ctrl+Alt+N"));
    let c = chord("Super+Shift+f5");
    assert_eq!(c.map(|c| c.to_string()).as_deref(), Some("Shift+Super+F5"));
    let c = chord("Control+Option+space");
    assert_eq!(c.map(|c| c.to_string()).as_deref(), Some("Ctrl+Alt+Space"));
    assert_eq!(
        chord("Strg+Alt+7").map(|c| c.key),
        Some(ChordKey::Digit('7'))
    );
    // A typing key is no hotkey, nor is half a chord.
    for bad in [
        "",
        "N",
        "Shift+N",
        "Ctrl+",
        "Ctrl+Alt",
        "Ctrl+Ctrl+N",
        "Ctrl+F13",
        "Ctrl+Hyper+N",
        "Ctrl+Ä",
    ] {
        assert_eq!(chord(bad), None, "{bad}");
    }
}

#[test]
fn the_portal_hears_the_chord_in_the_shortcuts_spec() {
    let trigger = |t: &str| chord(t).map(|c| c.portal_trigger());
    assert_eq!(trigger("Ctrl+Alt+N").as_deref(), Some("CTRL+ALT+n"));
    assert_eq!(trigger("Super+Shift+F5").as_deref(), Some("SHIFT+LOGO+F5"));
    assert_eq!(trigger("Alt+Space").as_deref(), Some("ALT+space"));
}

#[test]
fn the_app_grabs_the_chord_it_shows() {
    let key = chord("Ctrl+Alt+N").and_then(|c| c.hotkey());
    let want = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyN);
    assert_eq!(key, Some(want));
    let key = chord("Super+1").and_then(|c| c.hotkey());
    assert_eq!(key, Some(HotKey::new(Some(Modifiers::META), Code::Digit1)));
    let key = chord("Alt+F12").and_then(|c| c.hotkey());
    assert_eq!(key, Some(HotKey::new(Some(Modifiers::ALT), Code::F12)));
}

#[test]
fn a_chord_pressed_is_recorded_as_typed() {
    use egui::{Key, Modifiers};
    let pressed = |key, modifiers| Chord::pressed(key, modifiers).map(|c| c.to_string());
    let ctrl_alt = Modifiers::CTRL | Modifiers::ALT;
    assert_eq!(pressed(Key::K, ctrl_alt).as_deref(), Some("Ctrl+Alt+K"));
    assert_eq!(
        pressed(Key::F5, Modifiers::ALT | Modifiers::SHIFT).as_deref(),
        Some("Alt+Shift+F5")
    );
    assert_eq!(
        pressed(Key::Num7, Modifiers::CTRL).as_deref(),
        Some("Ctrl+7")
    );
    assert_eq!(
        pressed(Key::Space, Modifiers::MAC_CMD).as_deref(),
        Some("Super+Space")
    );
    // A typing key, a key no chord takes, and Shift alone are none.
    assert_eq!(pressed(Key::N, Modifiers::NONE), None);
    assert_eq!(pressed(Key::N, Modifiers::SHIFT), None);
    assert_eq!(pressed(Key::Escape, ctrl_alt), None);
    assert_eq!(pressed(Key::ArrowUp, ctrl_alt), None);
    assert_eq!(pressed(Key::F13, ctrl_alt), None);
}

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    }
}

#[test]
fn wayland_asks_the_portal_and_x11_grabs() {
    if cfg!(target_os = "linux") {
        let both = [("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")];
        assert_eq!(session(env(&both)), Session::Portal);
        assert_eq!(session(env(&[("DISPLAY", ":0")])), Session::Grab);
        assert_eq!(
            session(env(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", ":1")])),
            Session::Grab
        );
        assert_eq!(session(env(&[])), Session::None);
    } else {
        assert_eq!(session(env(&[])), Session::Grab);
    }
}
