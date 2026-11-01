//! The look of the phone (UI laws, "Look"): a light teal ground or an
//! anthracite one, question marks a touch darker or brighter than it,
//! embossed cards that clearly stand out from both, and a teal accent.
//! The colours are the phone's `look.dart` tones; the text on them is
//! readable in both brightnesses (WCAG AA).

use egui::epaint::{CornerRadius, Shadow};
use egui::{Color32, Context, Rect, Shape, Stroke, Theme, ThemePreference, Visuals};
use oldbrainfix_core::ThemeChoice;

/// The colours of one brightness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tones {
    pub background: Color32,
    pub mark: Color32,
    pub card: Color32,
    /// The emboss: a shade below and right, a highlight above and left.
    pub shade: Color32,
    pub highlight: Color32,
    /// The teal accent.
    pub accent: Color32,
}

/// A Flutter `Color(0xAARRGGBB)`.
const fn argb(argb: u32) -> Color32 {
    let [a, r, g, b] = argb.to_be_bytes();
    Color32::from_rgba_unmultiplied_const(r, g, b, a)
}

pub const LIGHT: Tones = Tones {
    background: argb(0xFFCDE8E4),
    mark: argb(0xFFBCDDD8),
    card: argb(0xFFF8FCFB),
    shade: argb(0x663E7A73),
    highlight: argb(0xCCFFFFFF),
    accent: argb(0xFF00796B),
};

pub const DARK: Tones = Tones {
    background: argb(0xFF26292C),
    mark: argb(0xFF31363A),
    card: argb(0xFF3E454A),
    shade: argb(0x99000000),
    highlight: argb(0x1FFFFFFF),
    accent: argb(0xFF4DB6AC),
};

/// The tones of the theme `visuals` belong to.
pub fn tones(visuals: &Visuals) -> Tones {
    if visuals.dark_mode { DARK } else { LIGHT }
}

/// The colours offered for an Area, as stored (`RRGGBB`): the phone's
/// `areaPalette`.
pub const AREA_PALETTE: [&str; 9] = [
    "C62828", "EF6C00", "F9A825", "2E7D32", "00838F", "1565C0", "6A1B9A", "8D6E63", "546E7A",
];

/// How round a card is.
pub const CARD_RADIUS: u8 = 16;

/// How much faded an unimportant card is drawn.
pub const UNIMPORTANT_OPACITY: f32 = 0.55;

/// The colours of the text on the [`Tones`] of one brightness, each at
/// least 4.5:1 against the ground, the question marks and the cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextTones {
    /// Plain text, also on a marked row.
    pub text: Color32,
    /// Text that matters less: hints, times, explanations.
    pub weak: Color32,
    /// Links and text in the accent; the accent itself is for marks and
    /// icons, 3:1.
    pub link: Color32,
    pub error: Color32,
    pub warn: Color32,
}

pub const LIGHT_TEXT: TextTones = TextTones {
    text: Color32::from_gray(60),
    weak: Color32::from_gray(90),
    link: argb(0xFF00695C),
    error: argb(0xFFA8231C),
    warn: argb(0xFF8A4500),
};

pub const DARK_TEXT: TextTones = TextTones {
    text: Color32::from_gray(220),
    weak: Color32::from_gray(180),
    link: argb(0xFF80CBC4),
    error: argb(0xFFFFB4AB),
    warn: argb(0xFFFFB74D),
};

/// Text and icons on a fill of the full accent: white on the light
/// one, black on the dark one.
pub fn on_accent(dark: bool) -> Color32 {
    if dark { Color32::BLACK } else { Color32::WHITE }
}

/// How strongly a marked row, a selected entry and selected text are
/// tinted with the accent.
const SELECTION_TINT: f32 = 0.25;

/// The visuals of one brightness: egui's own, on the phone's colours.
pub fn visuals(dark: bool) -> Visuals {
    let (t, x) = if dark {
        (DARK, DARK_TEXT)
    } else {
        (LIGHT, LIGHT_TEXT)
    };
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.override_text_color = None;
    v.widgets.noninteractive.fg_stroke.color = x.text;
    v.widgets.inactive.fg_stroke.color = x.text;
    v.weak_text_color = Some(x.weak);
    v.hyperlink_color = x.link;
    v.error_fg_color = x.error;
    v.warn_fg_color = x.warn;
    v.panel_fill = t.background;
    v.window_fill = t.card;
    v.extreme_bg_color = t.card;
    v.text_edit_bg_color = Some(t.card);
    v.selection.bg_fill = t.accent.gamma_multiply(SELECTION_TINT);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v
}

/// Installs the look for both brightnesses; the theme chosen picks one.
/// Cheap enough to call every frame.
pub fn install(ctx: &Context) {
    ctx.set_visuals_of(Theme::Light, visuals(false));
    ctx.set_visuals_of(Theme::Dark, visuals(true));
}

/// Shows the theme this Device chose: the system's, light or dark.
pub fn prefer(ctx: &Context, choice: ThemeChoice) {
    ctx.set_theme(match choice {
        ThemeChoice::System => ThemePreference::System,
        ThemeChoice::Light => ThemePreference::Light,
        ThemeChoice::Dark => ThemePreference::Dark,
    });
}

/// The shapes of a softly embossed panel over `rect`, visibly above the
/// background (UI laws, "Cards"): a shade below and right, a highlight
/// above and left, then the card. `strong` raises it further, for
/// important cards.
pub fn embossed(rect: Rect, tones: Tones, strong: bool) -> Vec<Shape> {
    let lift: i8 = if strong { 2 } else { 1 };
    let radius = CornerRadius::same(CARD_RADIUS);
    let shade = Shadow {
        offset: [4 * lift, 4 * lift],
        blur: 10 * lift as u8,
        spread: 0,
        color: tones.shade,
    };
    let highlight = Shadow {
        offset: [-3 * lift, -3 * lift],
        blur: 8 * lift as u8,
        spread: 0,
        color: tones.highlight,
    };
    vec![
        Shape::Rect(shade.as_shape(rect, radius)),
        Shape::Rect(highlight.as_shape(rect, radius)),
        Shape::rect_filled(rect, radius, tones.card),
    ]
}

/// The band of an Area's colour down the left edge of a card over
/// `rect`, following its rounded corners.
pub fn area_band(rect: Rect, colour: Color32) -> Shape {
    const WIDTH: f32 = 6.0;
    let r = f32::from(CARD_RADIUS);
    let inner = rect.left() + WIDTH;
    // The rounded corner's arc, from where it meets the band's inner edge
    // to the straight left edge.
    let reach = ((r - WIDTH) / r).acos();
    let steps = 8;
    let arc = |centre_y: f32, down: bool| {
        (0..=steps).map(move |i| {
            let a = reach * (1.0 - i as f32 / steps as f32);
            let y = if down {
                centre_y - r * a.sin()
            } else {
                centre_y + r * a.sin()
            };
            egui::pos2(rect.left() + r - r * a.cos(), y)
        })
    };
    let top = rect.top() + r;
    let bottom = rect.bottom() - r;
    let mut points: Vec<egui::Pos2> = arc(top, true).collect();
    let mut lower: Vec<egui::Pos2> = arc(bottom, false).collect();
    lower.reverse();
    points.extend(lower);
    points.retain(|p| p.x <= inner + 0.01);
    Shape::convex_polygon(points, colour, Stroke::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tones_are_the_phones() {
        assert_eq!(LIGHT.background, Color32::from_rgb(0xCD, 0xE8, 0xE4));
        assert_eq!(DARK.accent, Color32::from_rgb(0x4D, 0xB6, 0xAC));
        assert_eq!(
            LIGHT.shade,
            Color32::from_rgba_unmultiplied(0x3E, 0x7A, 0x73, 0x66)
        );
        assert!(!visuals(false).dark_mode);
        assert!(visuals(true).dark_mode);
        assert_eq!(tones(&visuals(true)), DARK);
    }

    /// The WCAG contrast ratio of two opaque colours.
    fn contrast(a: Color32, b: Color32) -> f32 {
        fn channel(c: u8) -> f32 {
            let c = f32::from(c) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        let luminance = |c: Color32| {
            0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// `tint` (premultiplied) laid over the opaque `ground`.
    fn over(tint: Color32, ground: Color32) -> Color32 {
        let a = f32::from(tint.a()) / 255.0;
        let mix = |t: u8, g: u8| (f32::from(t) + f32::from(g) * (1.0 - a)).round() as u8;
        Color32::from_rgb(
            mix(tint.r(), ground.r()),
            mix(tint.g(), ground.g()),
            mix(tint.b(), ground.b()),
        )
    }

    fn both() -> [(&'static str, Tones, TextTones, Visuals); 2] {
        [
            ("light", LIGHT, LIGHT_TEXT, visuals(false)),
            ("dark", DARK, DARK_TEXT, visuals(true)),
        ]
    }

    #[test]
    fn every_text_is_readable_on_the_ground_the_marks_and_the_cards() {
        for (name, t, x, _) in both() {
            let texts = [
                ("text", x.text),
                ("weak", x.weak),
                ("link", x.link),
                ("error", x.error),
                ("warn", x.warn),
            ];
            let grounds = [
                ("ground", t.background),
                ("marks", t.mark),
                ("card", t.card),
            ];
            for (text, colour) in texts {
                for (ground, under) in grounds {
                    let ratio = contrast(colour, under);
                    assert!(ratio >= 4.5, "{name} {text} on the {ground}: {ratio:.2}");
                }
            }
        }
    }

    #[test]
    fn plain_text_stays_readable_on_a_marked_row() {
        for (name, t, x, v) in both() {
            for (ground, under) in [("ground", t.background), ("card", t.card)] {
                let marked = over(v.selection.bg_fill, under);
                let ratio = contrast(x.text, marked);
                assert!(
                    ratio >= 4.5,
                    "{name} text marked on the {ground}: {ratio:.2}"
                );
            }
        }
    }

    #[test]
    fn the_visuals_draw_with_the_readable_tones() {
        for (name, _, x, v) in both() {
            assert_eq!(v.text_color(), x.text, "{name}");
            assert_eq!(v.widgets.inactive.fg_stroke.color, x.text, "{name}");
            assert_eq!(v.weak_text_color(), x.weak, "{name}");
            assert_eq!(v.hyperlink_color, x.link, "{name}");
            assert_eq!(v.error_fg_color, x.error, "{name}");
            assert_eq!(v.warn_fg_color, x.warn, "{name}");
            assert!(v.override_text_color.is_none(), "{name}");
        }
    }

    #[test]
    fn the_accent_stands_out_as_a_mark_and_cards_from_the_ground() {
        for (name, t, _, _) in both() {
            for (ground, under) in [
                ("ground", t.background),
                ("marks", t.mark),
                ("card", t.card),
            ] {
                let ratio = contrast(t.accent, under);
                assert!(ratio >= 3.0, "{name} accent on the {ground}: {ratio:.2}");
            }
            let on = on_accent(name == "dark");
            let ratio = contrast(on, t.accent);
            assert!(ratio >= 4.5, "{name} text on the accent: {ratio:.2}");
            // As on the phone: a soft emboss, yet clearly a card.
            assert!(contrast(t.card, t.background) >= 1.2, "{name}");
            assert!(contrast(t.card, t.mark) >= 1.2, "{name}");
            assert!(contrast(t.mark, t.background) < 1.2, "{name}");
        }
    }

    #[test]
    fn an_embossed_card_has_two_shadows_under_it_and_its_band_stays_left() {
        let rect = Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(200.0, 80.0));
        assert_eq!(embossed(rect, LIGHT, false).len(), 3);
        let band = area_band(rect, Color32::RED);
        let Shape::Path(path) = band else {
            panic!("a path")
        };
        assert!(path.points.iter().all(|p| p.x >= 10.0 && p.x <= 16.01));
        assert!(path.points.iter().all(|p| p.y >= 10.0 && p.y <= 90.0));
    }
}
