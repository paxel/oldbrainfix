//! A Note in a window of its own (#287), beside the main window: one
//! window per Note, several at once, each optionally kept on top of the
//! others. The window is an [`crate::App`] of its own that shows only
//! that Note, drawn as a viewport of the main window's; this holds what
//! needs no App.

use egui::{Vec2, ViewportBuilder, ViewportId, WindowLevel};
use oldbrainfix_core::APP_ID;

use crate::hotkey::{self, Session};
use crate::window_sizes::MIN_NOTE_DIALOG;

/// Whether a window can be kept on top of the others here, for these
/// environment variables: not on Wayland, which has no standard way for
/// an app to ask for it.
pub fn pin_works(var: impl Fn(&str) -> Option<String>) -> bool {
    hotkey::session(var) != Session::Portal
}

/// Whether a window can be kept on top of the others on this computer.
pub fn pin_works_here() -> bool {
    pin_works(|name| std::env::var(name).ok())
}

/// The viewport of the `serial`th Note window opened while the app runs.
pub fn viewport_id(serial: u64) -> ViewportId {
    ViewportId::from_hash_of(("note window", serial))
}

/// How a Note window lies among the others: over all of them while it
/// is kept on top.
pub fn level(on_top: bool) -> WindowLevel {
    if on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

/// The window a Note shows in, titled `title`, opened as large as `size`
/// and over all others while `on_top`.
pub fn viewport(title: &str, size: Vec2, on_top: bool) -> ViewportBuilder {
    ViewportBuilder::default()
        .with_title(title)
        .with_app_id(APP_ID)
        .with_inner_size(size)
        .with_min_inner_size(MIN_NOTE_DIALOG)
        .with_window_level(level(on_top))
}

/// The longest a Note window's title gets, in characters.
const TITLE_CHARS: usize = 60;

/// A Note window's title for the Note's `text` as its card shows it:
/// its first line, shortened, the app's name where it has none.
pub fn title(text: &str, app: &str) -> String {
    let line = text.lines().map(str::trim).find(|l| !l.is_empty());
    match line {
        None => app.to_string(),
        Some(line) if line.chars().count() <= TITLE_CHARS => line.to_string(),
        Some(line) => {
            let short: String = line.chars().take(TITLE_CHARS - 1).collect();
            format!("{}…", short.trim_end())
        }
    }
}

/// What the App of a Note window holds of being one.
#[derive(Debug, Clone, PartialEq)]
pub struct Alone {
    /// The Note it shows.
    pub note: String,
    /// Whether it is kept on top of the other windows; off as it opens.
    pub on_top: bool,
    /// Whether that can be asked here (see [`pin_works`]).
    pub pin_works: bool,
    /// Whether closing it was asked, by the system's cross or by the main
    /// window going.
    pub close_asked: bool,
    /// Its size as drawn last, in logical points, kept as it closes.
    pub size: Option<Vec2>,
}

impl Alone {
    /// A window opening on the Note `note`.
    pub fn of(note: &str, pin_works: bool) -> Alone {
        Alone {
            note: note.to_string(),
            on_top: false,
            pin_works,
            close_asked: false,
            size: None,
        }
    }

    /// Whether the window lies over all others: only where that can be
    /// asked, so a pin left on elsewhere does nothing here.
    pub fn kept_on_top(&self) -> bool {
        self.on_top && self.pin_works
    }
}

/// How a Note window's frame ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    Open,
    /// It closed, what was typed kept: by its cross, or as its Note left
    /// it, went to the trash or was deleted.
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn on_wayland_no_window_is_kept_on_top() {
        let wayland = pin_works(env(&[("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")]));
        let x11 = pin_works(env(&[("WAYLAND_DISPLAY", ""), ("DISPLAY", ":0")]));
        if cfg!(target_os = "linux") {
            assert!(!wayland);
        } else {
            assert!(wayland);
        }
        assert!(x11);
    }

    #[test]
    fn the_pin_puts_the_window_over_the_others() {
        let size = Vec2::new(500.0, 600.0);
        let normal = viewport("Recipe", size, false);
        assert_eq!(normal.window_level, Some(WindowLevel::Normal));
        assert_eq!(normal.inner_size, Some(size));
        assert_eq!(normal.title.as_deref(), Some("Recipe"));
        let pinned = viewport("Recipe", size, true);
        assert_eq!(pinned.window_level, Some(WindowLevel::AlwaysOnTop));
    }

    #[test]
    fn a_pin_where_it_cannot_work_keeps_nothing_on_top() {
        let mut alone = Alone::of("n1", false);
        alone.on_top = true;
        assert!(!alone.kept_on_top());
        let mut alone = Alone::of("n1", true);
        assert!(!alone.kept_on_top());
        alone.on_top = true;
        assert!(alone.kept_on_top());
    }

    #[test]
    fn the_title_is_the_first_line() {
        assert_eq!(title("\n  Pancakes \n200 g flour", "App"), "Pancakes");
        assert_eq!(title("  ", "App"), "App");
        let long = "a".repeat(80);
        let shown = title(&long, "App");
        assert_eq!(shown.chars().count(), TITLE_CHARS);
        assert!(shown.ends_with('…'));
    }

    #[test]
    fn each_window_has_a_viewport_of_its_own() {
        assert_ne!(viewport_id(1), viewport_id(2));
        assert_ne!(viewport_id(1), ViewportId::ROOT);
    }
}
