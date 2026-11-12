//! The window's size and position, the sidebar's width, the Note
//! dialog's size (#278) and a Note window's (#287), kept on this computer as settings of this
//! Device (ADR 0008) and brought back at the next start. Sizes and
//! positions are logical points, as the window is opened with them,
//! unzoomed.

use egui::{Pos2, Rect, Vec2};
use oldbrainfix_core::{Result, Store};

/// The window's inner size, `width,height`.
pub const WINDOW_SIZE: &str = "windowSize";
/// The window's outer position, `x,y`; none where the system tells no
/// position, as on Wayland.
pub const WINDOW_POSITION: &str = "windowPosition";
/// `true` while the window was maximized as it closed.
pub const WINDOW_MAXIMIZED: &str = "windowMaximized";
/// The sidebar's width.
pub const SIDEBAR_WIDTH: &str = "sidebarWidth";
/// The Note dialog's size, `width,height`.
pub const NOTE_DIALOG_SIZE: &str = "noteDialogSize";
/// The size of a Note in its own window (#287), `width,height`: the
/// last one closed, the one the next opens at. Unlike the others, in the
/// app's points, as a window of the running app is opened with them.
pub const NOTE_WINDOW_SIZE: &str = "noteWindowSize";

/// The window's size at the first start.
pub const DEFAULT_WINDOW: Vec2 = Vec2::new(1280.0, 800.0);
/// The smallest the window gets.
pub const MIN_WINDOW: Vec2 = Vec2::new(800.0, 480.0);
/// The sidebar's width at the first start, and its least.
pub const DEFAULT_SIDEBAR: f32 = 260.0;
pub const MIN_SIDEBAR: f32 = 180.0;
/// The smallest the Note dialog gets.
pub const MIN_NOTE_DIALOG: Vec2 = Vec2::new(320.0, 240.0);
/// A Note window's size until one was closed (#287).
pub const DEFAULT_NOTE_WINDOW: Vec2 = Vec2::new(560.0, 680.0);

/// How much of the window has to lie on a monitor for it to open where
/// it was, in each direction: enough to grab it.
const ON_MONITOR: f32 = 48.0;

/// Where and how large the window was.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlace {
    pub size: Vec2,
    pub position: Option<Pos2>,
    pub maximized: bool,
}

impl Default for WindowPlace {
    fn default() -> WindowPlace {
        WindowPlace {
            size: DEFAULT_WINDOW,
            position: None,
            maximized: false,
        }
    }
}

/// The window as it was kept, the first start's where nothing was or it
/// cannot be read; a size under the window's least as the first start's.
pub fn window(store: &Store) -> WindowPlace {
    let read = |key| store.local_setting(key).ok().flatten();
    let size = read(WINDOW_SIZE)
        .as_deref()
        .and_then(pair)
        .map(|(w, h)| Vec2::new(w, h))
        .filter(|s| s.x >= MIN_WINDOW.x && s.y >= MIN_WINDOW.y)
        .unwrap_or(DEFAULT_WINDOW);
    let position = read(WINDOW_POSITION)
        .as_deref()
        .and_then(pair)
        .map(|(x, y)| Pos2::new(x, y));
    let maximized = read(WINDOW_MAXIMIZED).as_deref() == Some("true");
    WindowPlace {
        size,
        position,
        maximized,
    }
}

/// Keeps where and how large the window is; a position not known is
/// not forgotten for it.
pub fn keep_window(store: &Store, place: &WindowPlace) -> Result<()> {
    keep(store, WINDOW_SIZE, &format_pair(place.size.x, place.size.y))?;
    if let Some(at) = place.position {
        keep(store, WINDOW_POSITION, &format_pair(at.x, at.y))?;
    }
    let maximized = if place.maximized { "true" } else { "false" };
    keep(store, WINDOW_MAXIMIZED, maximized)
}

/// The sidebar's width as kept, none where nothing was or it cannot be
/// read.
pub fn sidebar_width(store: &Store) -> Option<f32> {
    store
        .local_setting(SIDEBAR_WIDTH)
        .ok()
        .flatten()
        .and_then(|w| w.trim().parse::<f32>().ok())
        .filter(|w| w.is_finite() && *w >= MIN_SIDEBAR)
}

pub fn keep_sidebar_width(store: &Store, width: f32) -> Result<()> {
    keep(store, SIDEBAR_WIDTH, &format!("{}", width.round()))
}

/// The Note dialog's size as kept, none where nothing was or it cannot
/// be read.
pub fn note_dialog_size(store: &Store) -> Option<Vec2> {
    store
        .local_setting(NOTE_DIALOG_SIZE)
        .ok()
        .flatten()
        .as_deref()
        .and_then(pair)
        .map(|(w, h)| Vec2::new(w, h))
        .filter(|s| s.x >= MIN_NOTE_DIALOG.x && s.y >= MIN_NOTE_DIALOG.y)
}

pub fn keep_note_dialog_size(store: &Store, size: Vec2) -> Result<()> {
    keep(store, NOTE_DIALOG_SIZE, &format_pair(size.x, size.y))
}

/// The size a Note window opens at (#287): the last one's closed, the
/// first one's where nothing was kept or it cannot be read.
pub fn note_window_size(store: &Store) -> Vec2 {
    store
        .local_setting(NOTE_WINDOW_SIZE)
        .ok()
        .flatten()
        .as_deref()
        .and_then(pair)
        .map(|(w, h)| Vec2::new(w, h))
        .filter(|s| s.x >= MIN_NOTE_DIALOG.x && s.y >= MIN_NOTE_DIALOG.y)
        .unwrap_or(DEFAULT_NOTE_WINDOW)
}

pub fn keep_note_window_size(store: &Store, size: Vec2) -> Result<()> {
    keep(store, NOTE_WINDOW_SIZE, &format_pair(size.x, size.y))
}

/// Writes `value` unless it is there already.
fn keep(store: &Store, key: &str, value: &str) -> Result<()> {
    if store.local_setting(key)?.as_deref() == Some(value) {
        return Ok(());
    }
    store.set_local_setting(key, value)
}

fn pair(text: &str) -> Option<(f32, f32)> {
    let (a, b) = text.split_once(',')?;
    let a = a.trim().parse::<f32>().ok().filter(|v| v.is_finite())?;
    let b = b.trim().parse::<f32>().ok().filter(|v| v.is_finite())?;
    Some((a, b))
}

fn format_pair(a: f32, b: f32) -> String {
    format!("{},{}", a.round(), b.round())
}

/// `place` with what the window shows of itself in `info`, `zoom` its
/// zoom: its normal size and position only, not while minimized or full
/// screen, and only that it is maximized while it is, its normal size
/// kept for when it is not. A position the system does not tell, as on
/// Wayland, stays as it was.
pub fn noticed(place: WindowPlace, info: &egui::ViewportInfo, zoom: f32) -> WindowPlace {
    if info.minimized == Some(true) || info.fullscreen == Some(true) {
        return place;
    }
    if info.maximized == Some(true) {
        return WindowPlace {
            maximized: true,
            ..place
        };
    }
    let Some(inner) = info.inner_rect else {
        return place;
    };
    WindowPlace {
        size: inner.size() * zoom,
        position: info
            .outer_rect
            .map(|outer| outer.min * zoom)
            .or(place.position),
        maximized: false,
    }
}

/// Whether enough of `window` lies on one of `monitors` to grab it.
pub fn on_a_monitor(window: Rect, monitors: &[Rect]) -> bool {
    monitors.iter().any(|monitor| {
        let shown = window.intersect(*monitor);
        shown.width() >= ON_MONITOR.min(window.width())
            && shown.height() >= ON_MONITOR.min(window.height())
    })
}

/// Where a window of `size` lies centred on `monitor`, its top left
/// corner on the monitor however large it is.
pub fn centred(size: Vec2, monitor: Rect) -> Pos2 {
    let room = ((monitor.size() - size) / 2.0).max(Vec2::ZERO);
    monitor.min + room
}

/// The Note dialog's size where `wanted` is kept, none for the first
/// start's (#146): as wide as 760 at most, in what `room` it has.
pub fn note_dialog(wanted: Option<Vec2>, room: Vec2) -> Vec2 {
    match wanted {
        None => Vec2::new(
            (room.x - 120.0).clamp(MIN_NOTE_DIALOG.x, 760.0),
            (room.y - 60.0).max(MIN_NOTE_DIALOG.y),
        ),
        // The frame around it and a margin stay.
        Some(wanted) => wanted.min(room - Vec2::splat(48.0)).max(MIN_NOTE_DIALOG),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oldbrainfix_core::SigningKey;

    fn store() -> Store {
        let made = Store::in_memory(Box::new(chrono::Utc::now), || {
            Ok(SigningKey::from_seed([3; 32]))
        });
        match made {
            Ok(store) => store,
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn the_window_comes_back_as_it_was_kept() -> Result<()> {
        let store = store();
        assert_eq!(window(&store), WindowPlace::default());
        let place = WindowPlace {
            size: Vec2::new(1500.0, 900.0),
            position: Some(Pos2::new(-1200.0, 40.0)),
            maximized: true,
        };
        keep_window(&store, &place)?;
        assert_eq!(window(&store), place);
        // Where no position is known, as on Wayland, the last one stays.
        let unplaced = WindowPlace {
            position: None,
            maximized: false,
            ..place
        };
        keep_window(&store, &unplaced)?;
        assert_eq!(window(&store).position, place.position);
        assert!(!window(&store).maximized);
        Ok(())
    }

    #[test]
    fn what_cannot_be_read_opens_the_first_start_window() -> Result<()> {
        let store = store();
        store.set_local_setting(WINDOW_SIZE, "wide")?;
        store.set_local_setting(WINDOW_POSITION, "10")?;
        assert_eq!(window(&store), WindowPlace::default());
        store.set_local_setting(WINDOW_SIZE, "300,200")?;
        assert_eq!(window(&store).size, DEFAULT_WINDOW);
        store.set_local_setting(SIDEBAR_WIDTH, "12")?;
        assert_eq!(sidebar_width(&store), None);
        store.set_local_setting(NOTE_DIALOG_SIZE, "NaN,400")?;
        assert_eq!(note_dialog_size(&store), None);
        Ok(())
    }

    #[test]
    fn the_sidebar_and_the_note_dialog_come_back() -> Result<()> {
        let store = store();
        assert_eq!(sidebar_width(&store), None);
        assert_eq!(note_dialog_size(&store), None);
        keep_sidebar_width(&store, 333.4)?;
        keep_note_dialog_size(&store, Vec2::new(1000.2, 650.0))?;
        assert_eq!(sidebar_width(&store), Some(333.0));
        assert_eq!(note_dialog_size(&store), Some(Vec2::new(1000.0, 650.0)));
        Ok(())
    }

    #[test]
    fn a_note_window_opens_as_large_as_the_last_one_closed() -> Result<()> {
        let store = store();
        assert_eq!(note_window_size(&store), DEFAULT_NOTE_WINDOW);
        keep_note_window_size(&store, Vec2::new(480.4, 900.0))?;
        assert_eq!(note_window_size(&store), Vec2::new(480.0, 900.0));
        // Too small or unreadable: the first one's.
        store.set_local_setting(NOTE_WINDOW_SIZE, "100,100")?;
        assert_eq!(note_window_size(&store), DEFAULT_NOTE_WINDOW);
        store.set_local_setting(NOTE_WINDOW_SIZE, "tall")?;
        assert_eq!(note_window_size(&store), DEFAULT_NOTE_WINDOW);
        Ok(())
    }

    #[test]
    fn the_window_is_noticed_at_its_normal_size() {
        let kept = WindowPlace {
            size: Vec2::new(1500.0, 900.0),
            position: Some(Pos2::new(40.0, 60.0)),
            maximized: false,
        };
        let shown = egui::ViewportInfo {
            inner_rect: Some(Rect::from_min_size(
                Pos2::new(110.0, 130.0),
                Vec2::new(1000.0, 600.0),
            )),
            outer_rect: Some(Rect::from_min_size(
                Pos2::new(100.0, 100.0),
                Vec2::new(1020.0, 640.0),
            )),
            maximized: Some(false),
            minimized: Some(false),
            ..Default::default()
        };
        // Zoomed in, the points are fewer than those it opens with.
        let normal = noticed(kept, &shown, 1.5);
        assert_eq!(normal.size, Vec2::new(1500.0, 900.0));
        assert_eq!(normal.position, Some(Pos2::new(150.0, 150.0)));
        assert!(!normal.maximized);
        // Maximized it keeps the size it had before, whatever it shows.
        let maximized = egui::ViewportInfo {
            maximized: Some(true),
            ..shown.clone()
        };
        assert_eq!(
            noticed(kept, &maximized, 1.0),
            WindowPlace {
                maximized: true,
                ..kept
            }
        );
        // Minimized or full screen, nothing changes.
        for info in [
            egui::ViewportInfo {
                minimized: Some(true),
                ..shown.clone()
            },
            egui::ViewportInfo {
                fullscreen: Some(true),
                ..shown.clone()
            },
        ] {
            assert_eq!(noticed(kept, &info, 1.0), kept);
        }
        // No position told, as on Wayland: the one kept stays.
        let unplaced = egui::ViewportInfo {
            outer_rect: None,
            ..shown
        };
        let seen = noticed(kept, &unplaced, 1.0);
        assert_eq!(seen.size, Vec2::new(1000.0, 600.0));
        assert_eq!(seen.position, kept.position);
    }

    #[test]
    fn a_window_off_every_monitor_is_not_on_one() {
        let left = Rect::from_min_size(Pos2::new(-1920.0, 0.0), Vec2::new(1920.0, 1080.0));
        let main = Rect::from_min_size(Pos2::ZERO, Vec2::new(2560.0, 1440.0));
        let size = Vec2::new(1280.0, 800.0);
        let at = |x, y| Rect::from_min_size(Pos2::new(x, y), size);
        assert!(on_a_monitor(at(100.0, 100.0), &[main]));
        // On the monitor to the left, while it is there.
        assert!(on_a_monitor(at(-1500.0, 100.0), &[left, main]));
        assert!(!on_a_monitor(at(-1500.0, 100.0), &[main]));
        // Barely over an edge is not enough to grab it.
        assert!(!on_a_monitor(at(2540.0, 100.0), &[main]));
        assert!(on_a_monitor(at(2400.0, 100.0), &[main]));
        assert!(!on_a_monitor(at(100.0, 3000.0), &[left, main]));
        assert!(!on_a_monitor(at(100.0, 100.0), &[]));
    }

    #[test]
    fn a_window_is_centred_on_its_monitor() {
        let monitor = Rect::from_min_size(Pos2::new(1920.0, 0.0), Vec2::new(2560.0, 1440.0));
        assert_eq!(
            centred(Vec2::new(1280.0, 800.0), monitor),
            Pos2::new(2560.0, 320.0)
        );
        // Larger than the monitor: its corner on it.
        assert_eq!(
            centred(Vec2::new(3000.0, 2000.0), monitor),
            Pos2::new(1920.0, 0.0)
        );
    }

    #[test]
    fn the_note_dialog_keeps_its_size_in_the_room_it_has() {
        let room = Vec2::new(1280.0, 760.0);
        // The first start's, as before (#146).
        assert_eq!(note_dialog(None, room), Vec2::new(760.0, 700.0));
        assert_eq!(
            note_dialog(Some(Vec2::new(1100.0, 500.0)), room),
            Vec2::new(1100.0, 500.0)
        );
        // Larger than the window: as large as it has room for.
        assert_eq!(
            note_dialog(Some(Vec2::new(3000.0, 3000.0)), room),
            Vec2::new(1232.0, 712.0)
        );
        // Never under its least.
        assert_eq!(
            note_dialog(Some(Vec2::new(400.0, 300.0)), Vec2::new(200.0, 200.0)),
            MIN_NOTE_DIALOG
        );
    }
}
