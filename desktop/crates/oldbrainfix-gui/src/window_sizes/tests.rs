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
