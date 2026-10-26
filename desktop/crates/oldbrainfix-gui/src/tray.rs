//! The tray icon with Open, Capture and Quit. On Linux it is a
//! StatusNotifierItem (ksni), which KDE, Xfce, Cinnamon and GNOME with
//! the AppIndicator extension show; where no tray host runs, or on other
//! systems, there is none and closing the window quits the app.

use std::sync::mpsc::Sender;

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
}

impl Drop for Tray {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        drop(self.handle.shutdown());
    }
}

/// The square icon of `size` pixels as ARGB32 rows, big-endian: the
/// packaged 256 px icon, scaled down. Empty if it does not decode.
pub fn icon_argb(size: i32) -> Vec<u8> {
    let Ok(icon) = image::load_from_memory(crate::launcher::ICON) else {
        return Vec::new();
    };
    let side = size.max(1) as u32;
    image::imageops::resize(
        &icon.to_rgba8(),
        side,
        side,
        image::imageops::FilterType::Lanczos3,
    )
    .pixels()
    .flat_map(|p| [p[3], p[0], p[1], p[2]])
    .collect()
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

/// Puts the tray icon up; there is none on this system yet.
#[cfg(not(target_os = "linux"))]
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
