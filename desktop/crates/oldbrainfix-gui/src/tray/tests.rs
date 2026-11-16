use super::*;

/// The pixel at `x`, `y` of a square ARGB icon `size` wide.
fn at(icon: &[u8], size: usize, x: usize, y: usize) -> &[u8] {
    &icon[(y * size + x) * 4..(y * size + x) * 4 + 4]
}

#[test]
fn closing_the_window_hides_it_into_a_tray_in_its_event_loop() {
    // Windows and macOS: the event loop goes on, the window hidden.
    assert_eq!(on_close(true, true, false), OnClose::Hide);
    // Linux: the window closes, ksni's thread keeps the tray.
    assert_eq!(on_close(true, false, false), OnClose::Close);
    // Without a tray nothing is left to come back to, and Quit ends
    // the app with a tray too.
    for in_event_loop in [true, false] {
        assert_eq!(on_close(false, in_event_loop, false), OnClose::Quit);
        assert_eq!(on_close(true, in_event_loop, true), OnClose::Quit);
        assert_eq!(on_close(false, in_event_loop, true), OnClose::Quit);
    }
    assert_eq!(IN_EVENT_LOOP, cfg!(any(windows, target_os = "macos")));
}

#[test]
fn the_tray_menu_entries_ask_for_open_capture_and_quit() {
    assert_eq!(command_of("open"), Some(Command::Open));
    assert_eq!(command_of("capture"), Some(Command::Capture));
    assert_eq!(command_of("quit"), Some(Command::Quit));
    assert_eq!(command_of("about"), None);
}

#[test]
fn the_rgba_tray_icon_is_the_argb_one_reordered() {
    let (argb, rgba) = (icon_argb(32), icon_rgba(32));
    assert_eq!(rgba.len(), 32 * 32 * 4);
    let reordered: Vec<u8> = rgba
        .chunks(4)
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect();
    assert_eq!(reordered, argb);
}

#[test]
fn the_tray_icon_is_the_round_anthracite_app_icon() {
    for size in [22, 32, 64] {
        let icon = icon_argb(size);
        let size = size as usize;
        assert_eq!(icon.len(), size * size * 4);
        // The corner is clear, a point inside the disc is anthracite.
        assert_eq!(at(&icon, size, 0, 0)[0], 0);
        assert_eq!(
            at(&icon, size, size / 8, size / 2),
            &[255, 0x26, 0x29, 0x2C]
        );
    }
}

#[test]
fn the_packaged_icons_are_the_same_round_icon() {
    // Launcher, .deb, AppImage, Flatpak, macOS and Windows ship these.
    for (size, png) in [
        (
            256,
            &include_bytes!("../../../../packaging/icon/icon_256.png")[..],
        ),
        (
            1024,
            &include_bytes!("../../../../packaging/icon/icon.png")[..],
        ),
    ] {
        let rgba = image::load_from_memory(png).unwrap().to_rgba8();
        assert_eq!(rgba.dimensions(), (size, size));
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        assert_eq!(
            rgba.get_pixel(size / 8, size / 2).0,
            [0x26, 0x29, 0x2C, 255]
        );
    }
}
