//! A Note in a window of its own (#287), beside the main window: one
//! window per Note, several at once, each optionally kept on top of the
//! others, each maximized as the system's windows are. The window is an
//! [`crate::App`] of its own that shows only that Note, drawn as a
//! viewport of the main window's; this holds what needs no App.
//!
//! Its reading view (#322, obf-1015) shows only the Note's content, a
//! column no wider than [`measure`] in the middle of the window, scrolled
//! by the wheel and by the keys of [`Scroll`].

use egui::{InputState, Key, Modifiers, Vec2, ViewportBuilder, ViewportId, WindowLevel};
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
/// and over all others while `on_top`; it is resized and maximized as the
/// system's windows are (#322).
pub fn viewport(title: &str, size: Vec2, on_top: bool) -> ViewportBuilder {
    ViewportBuilder::default()
        .with_title(title)
        .with_app_id(APP_ID)
        .with_inner_size(size)
        .with_min_inner_size(MIN_NOTE_DIALOG)
        .with_resizable(true)
        .with_maximize_button(true)
        .with_window_level(level(on_top))
}

/// The reading view's widest column, in parts of its text's size (#322):
/// about 70 characters, half a size each on average, so a maximized
/// window does not run its lines across a wide screen.
pub const MEASURE: f32 = 35.0;

/// The reading view's widest column for its text at `size` points, the
/// Device's text size in it (#322).
pub fn measure(size: f32) -> f32 {
    MEASURE * size
}

/// How far a page up or down keeps of the page before, in points: the
/// line read last stays in sight.
pub const PAGE_OVERLAP: f32 = 40.0;

/// How far Up and Down scroll the reading view, in points.
pub const LINE_STEP: f32 = 40.0;

/// A key scrolling the reading view (#322): Page Up and Page Down (Space
/// and Shift+Space too), Up and Down, Home and End.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scroll {
    PageUp,
    PageDown,
    LineUp,
    LineDown,
    Top,
    End,
}

/// The key of [`Scroll`] pressed, taken: Shift+Space before Space,
/// which egui would take for it too. While a widget has the keyboard
/// (`free` false), Space and the arrows are its own: they press it and
/// move the keyboard on, and only the page keys scroll.
pub fn scroll_key(input: &mut InputState, free: bool) -> Option<Scroll> {
    let pages = [
        (Modifiers::NONE, Key::PageUp, Scroll::PageUp),
        (Modifiers::NONE, Key::PageDown, Scroll::PageDown),
        (Modifiers::NONE, Key::Home, Scroll::Top),
        (Modifiers::NONE, Key::End, Scroll::End),
    ];
    if let Some((_, _, scroll)) = pages
        .into_iter()
        .find(|(modifiers, key, _)| input.consume_key(*modifiers, *key))
    {
        return Some(scroll);
    }
    if !free {
        return None;
    }
    let keys = [
        (Modifiers::SHIFT, Key::Space, Scroll::PageUp),
        (Modifiers::NONE, Key::Space, Scroll::PageDown),
        (Modifiers::NONE, Key::ArrowUp, Scroll::LineUp),
        (Modifiers::NONE, Key::ArrowDown, Scroll::LineDown),
    ];
    keys.into_iter()
        .find(|(modifiers, key, _)| input.consume_key(*modifiers, *key))
        .map(|(_, _, scroll)| scroll)
}

/// Where the reading view was scrolled as it was drawn last, in points:
/// how far down, how high its window and how high all of it is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Scrolled {
    pub offset: f32,
    pub page: f32,
    pub height: f32,
}

impl Scrolled {
    /// How far down the reading view lies after `key`, never past its
    /// ends.
    pub fn to(&self, key: Scroll) -> f32 {
        let end = (self.height - self.page).max(0.0);
        let page = (self.page - PAGE_OVERLAP).max(LINE_STEP);
        let to = match key {
            Scroll::PageUp => self.offset - page,
            Scroll::PageDown => self.offset + page,
            Scroll::LineUp => self.offset - LINE_STEP,
            Scroll::LineDown => self.offset + LINE_STEP,
            Scroll::Top => 0.0,
            Scroll::End => end,
        };
        to.clamp(0.0, end)
    }
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
    /// Its size as drawn last, in logical points, kept as it closes; not
    /// while it is maximized.
    pub size: Option<Vec2>,
    /// Whether it shows its reading view (#322): off as it opens, kept
    /// while it is open, never stored.
    pub reading: bool,
    /// Where the reading view was scrolled as it was drawn last.
    pub scrolled: Scrolled,
    /// Where a key scrolls the reading view to in the next frame.
    pub scroll_to: Option<f32>,
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
            reading: false,
            scrolled: Scrolled::default(),
            scroll_to: None,
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
mod tests;
