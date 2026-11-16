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
