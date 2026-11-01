//! The system's light or dark setting where the window system does not
//! report it (#147): on Linux, X11 and Wayland tell egui nothing, so the
//! desktop portal's colour scheme is read on start, inside the Flatpak
//! too, and followed as it changes. On Windows and macOS the window
//! system reports it, and this adds nothing.

use std::sync::{Arc, Mutex};

use egui::{Context, Theme};

/// The system's light or dark as last heard, shared with the thread that
/// listens for it.
#[derive(Clone, Default)]
pub struct SystemTheme(Arc<Mutex<Option<Theme>>>);

impl SystemTheme {
    /// Follows the system's setting from now on, calling `changed` (to
    /// wake the window) each time it is heard.
    pub fn follow(changed: Box<dyn Fn() + Send>) -> SystemTheme {
        let shared = SystemTheme::default();
        portal::spawn(shared.clone(), changed);
        shared
    }

    /// What was last heard; `None` while the system states no preference.
    pub fn get(&self) -> Option<Theme> {
        self.0.lock().map(|t| *t).unwrap_or(None)
    }

    /// Takes what the system says now.
    pub fn set(&self, theme: Option<Theme>) {
        if let Ok(mut held) = self.0.lock() {
            *held = theme;
        }
    }

    /// Hands what was heard to egui, for every window of `ctx`, the
    /// capture popup over the window too: egui shows it while the window
    /// system reports nothing, and Dark, as it always did, while nothing
    /// was heard either.
    pub fn hand_to(&self, ctx: &Context) {
        let theme = self.get().unwrap_or(Theme::Dark);
        if ctx.options(|o| o.fallback_theme) != theme {
            ctx.options_mut(|o| o.fallback_theme = theme);
        }
    }
}

#[cfg(target_os = "linux")]
mod portal {
    use ashpd::desktop::settings::{ColorScheme, Settings};
    use egui::Theme;
    use futures_lite::StreamExt;

    use super::SystemTheme;

    /// The theme a colour scheme asks for; none for no preference.
    pub(super) fn theme_of(scheme: ColorScheme) -> Option<Theme> {
        match scheme {
            ColorScheme::PreferDark => Some(Theme::Dark),
            ColorScheme::PreferLight => Some(Theme::Light),
            ColorScheme::NoPreference => None,
        }
    }

    /// Reads the colour scheme on a thread of its own, which then waits
    /// for its changes for as long as the app runs. Without a portal
    /// nothing is heard and the window stays as egui shows it.
    pub fn spawn(shared: SystemTheme, changed: Box<dyn Fn() + Send>) {
        std::thread::spawn(move || {
            if let Err(e) = async_io::block_on(listen(&shared, changed)) {
                eprintln!("oldbrainfix: no light or dark from the desktop: {e}");
            }
        });
    }

    async fn listen(shared: &SystemTheme, changed: Box<dyn Fn() + Send>) -> ashpd::Result<()> {
        let settings = Settings::new().await?;
        let changes = settings.receive_color_scheme_changed().await?;
        shared.set(theme_of(settings.color_scheme().await?));
        changed();
        let mut changes = std::pin::pin!(changes);
        while let Some(scheme) = changes.next().await {
            shared.set(theme_of(scheme));
            changed();
        }
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
mod portal {
    use super::SystemTheme;

    /// The window system reports it off Linux.
    pub fn spawn(_shared: SystemTheme, _changed: Box<dyn Fn() + Send>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The theme `ctx` shows after a frame on a system that reports
    /// `reported`.
    fn shown(ctx: &Context, reported: Option<Theme>) -> Theme {
        let input = egui::RawInput {
            system_theme: reported,
            ..egui::RawInput::default()
        };
        drop(ctx.run_ui(input, |_| {}));
        ctx.theme()
    }

    #[test]
    fn what_was_heard_shows_only_where_the_window_system_says_nothing() {
        let ctx = Context::default();
        let heard = SystemTheme::default();
        heard.hand_to(&ctx);
        assert_eq!(shown(&ctx, None), Theme::Dark);
        heard.set(Some(Theme::Light));
        heard.hand_to(&ctx);
        assert_eq!(shown(&ctx, None), Theme::Light);
        assert_eq!(shown(&ctx, Some(Theme::Dark)), Theme::Dark);
        heard.set(None);
        heard.hand_to(&ctx);
        assert_eq!(shown(&ctx, None), Theme::Dark);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_portals_colour_scheme_names_light_dark_or_none() {
        use ashpd::desktop::settings::ColorScheme;
        assert_eq!(portal::theme_of(ColorScheme::PreferDark), Some(Theme::Dark));
        assert_eq!(
            portal::theme_of(ColorScheme::PreferLight),
            Some(Theme::Light)
        );
        assert_eq!(portal::theme_of(ColorScheme::NoPreference), None);
    }
}
