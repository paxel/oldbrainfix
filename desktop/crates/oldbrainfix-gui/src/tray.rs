//! The tray icon with Open, Capture and Quit. On Linux it is a
//! StatusNotifierItem (ksni) on a thread of its own, which KDE, Xfce,
//! Cinnamon and GNOME with the AppIndicator extension show; where no tray
//! host runs there is none and closing the window quits the app. On
//! Windows it sits in the notification area and on macOS in the menu bar
//! (tray-icon); both hear it only while an event loop runs on the main
//! thread, so it goes up inside the window's, and closing the window
//! hides it there instead of ending that loop ([`IN_EVENT_LOOP`]).

use std::sync::mpsc::Sender;

/// Whether the tray lives in the window's event loop, as on Windows and
/// macOS: it is put up once that loop runs, and the window hides into it
/// rather than closing, so the loop goes on for as long as the app.
pub const IN_EVENT_LOOP: bool = cfg!(any(windows, target_os = "macos"));

/// What closing the window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnClose {
    /// The window hides; the app runs on in the tray (Windows, macOS).
    Hide,
    /// The window closes; the app runs on in the tray (Linux).
    Close,
    /// The window closes and the app ends.
    Quit,
}

/// What closing the window does with `tray` up or not, the tray in the
/// window's event loop when `in_event_loop`, while Quit was asked for
/// when `quitting`.
pub fn on_close(tray: bool, in_event_loop: bool, quitting: bool) -> OnClose {
    if quitting || !tray {
        OnClose::Quit
    } else if in_event_loop {
        OnClose::Hide
    } else {
        OnClose::Close
    }
}

/// What the tray, a second start or a notification asks of the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Show the window.
    Open,
    /// Show the window with the capture field ready.
    Capture,
    /// End the app.
    Quit,
}

/// The tray menu's texts.
#[derive(Debug, Clone)]
pub struct Labels {
    pub title: String,
    pub open: String,
    pub capture: String,
    pub quit: String,
}

/// The tray while it is up; it goes when this is dropped.
pub struct Tray {
    #[cfg(target_os = "linux")]
    handle: ksni::blocking::Handle<Item>,
    #[cfg(any(windows, target_os = "macos"))]
    _icon: tray_icon::TrayIcon,
}

impl Drop for Tray {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        drop(self.handle.shutdown());
    }
}

/// The packaged 256 px icon scaled down to `size` pixels square;
/// `None` if it does not decode.
fn icon_of(size: i32) -> Option<image::RgbaImage> {
    let icon = image::load_from_memory(crate::ICON).ok()?;
    let side = size.max(1) as u32;
    Some(image::imageops::resize(
        &icon.to_rgba8(),
        side,
        side,
        image::imageops::FilterType::Lanczos3,
    ))
}

/// The square icon of `size` pixels as ARGB32 rows, big-endian, as a
/// StatusNotifierItem takes it. Empty if it does not decode.
pub fn icon_argb(size: i32) -> Vec<u8> {
    icon_of(size)
        .map(|icon| {
            icon.pixels()
                .flat_map(|p| [p[3], p[0], p[1], p[2]])
                .collect()
        })
        .unwrap_or_default()
}

/// The square icon of `size` pixels as RGBA rows, as Windows and macOS
/// take it. Empty if it does not decode.
pub fn icon_rgba(size: i32) -> Vec<u8> {
    icon_of(size)
        .map(image::RgbaImage::into_raw)
        .unwrap_or_default()
}

/// The ids of the tray menu's entries on Windows and macOS.
const OPEN_ID: &str = "open";
const CAPTURE_ID: &str = "capture";
const QUIT_ID: &str = "quit";

/// What the tray menu's entry `id` asks of the app.
pub fn command_of(id: &str) -> Option<Command> {
    match id {
        OPEN_ID => Some(Command::Open),
        CAPTURE_ID => Some(Command::Capture),
        QUIT_ID => Some(Command::Quit),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
pub struct Item {
    labels: Labels,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
}

#[cfg(target_os = "linux")]
impl Item {
    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
        (self.wake)();
    }
}

#[cfg(target_os = "linux")]
impl ksni::Tray for Item {
    fn id(&self) -> String {
        oldbrainfix_core::APP_ID.into()
    }

    fn title(&self) -> String {
        self.labels.title.clone()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        [22, 32, 64]
            .into_iter()
            .map(|size| ksni::Icon {
                width: size,
                height: size,
                data: icon_argb(size),
            })
            .collect()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.labels.title.clone(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(Command::Open);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        let item = |label: &str, command: Command| {
            StandardItem {
                label: label.into(),
                activate: Box::new(move |item: &mut Item| item.send(command)),
                ..Default::default()
            }
            .into()
        };
        vec![
            item(&self.labels.open, Command::Open),
            item(&self.labels.capture, Command::Capture),
            ksni::MenuItem::Separator,
            item(&self.labels.quit, Command::Quit),
        ]
    }
}

/// Puts the tray icon up; its menu sends to `commands` and then calls
/// `wake`. `None` where there is no tray.
#[cfg(target_os = "linux")]
pub fn spawn(
    labels: Labels,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
) -> Option<Tray> {
    use ksni::blocking::TrayMethods;
    let item = Item {
        labels,
        commands,
        wake,
    };
    match item.spawn() {
        Ok(handle) => Some(Tray { handle }),
        Err(e) => {
            eprintln!("oldbrainfix: no tray: {e}");
            None
        }
    }
}

/// Puts the tray icon up, in the notification area on Windows and the
/// menu bar on macOS; its menu, and on Windows a left click (Open), send
/// to `commands` and then call `wake`. Call it on the main thread once
/// its event loop runs. `None` where it cannot go up.
#[cfg(any(windows, target_os = "macos"))]
pub fn spawn(
    labels: Labels,
    commands: Sender<Command>,
    wake: Box<dyn Fn() + Send>,
) -> Option<Tray> {
    use std::sync::{Arc, Mutex};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let menu = Menu::new();
    let open = MenuItem::with_id(OPEN_ID, &labels.open, true, None);
    let capture = MenuItem::with_id(CAPTURE_ID, &labels.capture, true, None);
    let quit = MenuItem::with_id(QUIT_ID, &labels.quit, true, None);
    let separator = PredefinedMenuItem::separator();
    if let Err(e) = menu.append_items(&[&open, &capture, &separator, &quit]) {
        eprintln!("oldbrainfix: no tray: {e}");
        return None;
    }
    // The handlers run on the main thread, `wake` behind a lock as it
    // need not be shareable.
    let wake = Mutex::new(wake);
    let send = Arc::new(move |command: Command| {
        let _ = commands.send(command);
        if let Ok(wake) = wake.lock() {
            wake();
        }
    });
    {
        let send = send.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some(command) = command_of(event.id.as_ref()) {
                send(command);
            }
        }));
    }
    // On Windows a left click opens the window and the right click the
    // menu; on macOS either click opens the menu, as menu bar icons do.
    // The handler takes every event, so none piles up unread.
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if cfg!(windows)
            && let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
        {
            send(Command::Open);
        }
    }));
    let side = if cfg!(windows) { 32 } else { 64 };
    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(&labels.title)
        .with_menu_on_left_click(cfg!(target_os = "macos"));
    match Icon::from_rgba(icon_rgba(side), side as u32, side as u32) {
        Ok(icon) => builder = builder.with_icon(icon),
        Err(e) => eprintln!("oldbrainfix: tray icon: {e}"),
    }
    match builder.build() {
        Ok(icon) => Some(Tray { _icon: icon }),
        Err(e) => {
            eprintln!("oldbrainfix: no tray: {e}");
            None
        }
    }
}

/// Puts the tray icon up; there is none on this system.
#[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
pub fn spawn(
    _labels: Labels,
    _commands: Sender<Command>,
    _wake: Box<dyn Fn() + Send>,
) -> Option<Tray> {
    None
}

#[cfg(test)]
mod tests {
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
                &include_bytes!("../../../packaging/icon/icon_256.png")[..],
            ),
            (
                1024,
                &include_bytes!("../../../packaging/icon/icon.png")[..],
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
}
