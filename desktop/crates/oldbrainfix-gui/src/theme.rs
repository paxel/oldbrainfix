//! The look of the phone (UI laws, "Look"): a light teal ground or an
//! anthracite one, question marks a touch darker or brighter than it,
//! embossed cards that clearly stand out from both, and a teal accent.
//! The colours are the phone's `look.dart` tones.

use egui::epaint::{CornerRadius, Shadow};
use egui::{Color32, Context, Rect, Shape, Stroke, Theme, Visuals};

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

/// How round a card is.
pub const CARD_RADIUS: u8 = 16;

/// How much faded an unimportant card is drawn.
pub const UNIMPORTANT_OPACITY: f32 = 0.55;

/// The visuals of one brightness: egui's own, on the phone's colours.
pub fn visuals(dark: bool) -> Visuals {
    let t = if dark { DARK } else { LIGHT };
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.override_text_color = None;
    v.panel_fill = t.background;
    v.window_fill = t.card;
    v.extreme_bg_color = t.card;
    v.text_edit_bg_color = Some(t.card);
    v.hyperlink_color = t.accent;
    v.selection.bg_fill = t.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v
}

/// Installs the look for both brightnesses; the system picks one.
/// Cheap enough to call every frame.
pub fn install(ctx: &Context) {
    ctx.set_visuals_of(Theme::Light, visuals(false));
    ctx.set_visuals_of(Theme::Dark, visuals(true));
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
