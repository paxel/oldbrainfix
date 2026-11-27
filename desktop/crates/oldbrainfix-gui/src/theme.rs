//! The look of the phone (UI laws, "Look"): a light teal ground or an
//! anthracite one, question marks a touch darker or brighter than it,
//! embossed cards that clearly stand out from both, and a teal accent.
//! The colours are the phone's `look.dart` tones; the text on them is
//! readable in both brightnesses (WCAG AA).

use egui::epaint::{CornerRadius, Shadow};
use egui::{
    Button, Color32, Context, Key, Modifiers, Rect, Response, Shape, Stroke, Theme,
    ThemePreference, Ui, Visuals,
};
use oldbrainfix_core::highlight::CodeColour;
use oldbrainfix_core::{TextSize, ThemeChoice};

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
/// `areaPalette`, today's nine, then lighter and darker shades
/// (obf-1156). A colour once offered stays, so every stored look still
/// reads.
pub const AREA_PALETTE: [&str; 18] = [
    "C62828", "EF6C00", "F9A825", "2E7D32", "00838F", "1565C0", "6A1B9A", "8D6E63", "546E7A",
    "F06292", "FF8A65", "AED581", "4FC3F7", "BA68C8", "AD1457", "9E9D24", "1B5E20", "283593",
];

/// How far an Area's colour stands out from the ground, the question
/// marks and the cards it is drawn on, at least (WCAG, as for a graphic
/// or an icon): the phone's `areaContrast`.
pub const AREA_CONTRAST: f64 = 3.0;

/// An Area's colour as it is drawn in the dark or the light look
/// (obf-1156), as the phone's `areaColour`: darkened on the light look
/// and brightened on the dark one, in twentieths toward black or white,
/// just as far as it takes to stand out [`AREA_CONTRAST`] from every
/// surface of that look.
pub fn area_shade(colour: Color32, dark: bool) -> Color32 {
    let tones = if dark { DARK } else { LIGHT };
    let surfaces = [tones.background, tones.mark, tones.card];
    let toward = if dark { 255.0 } else { 0.0 };
    for step in 0..=20u8 {
        let t = f64::from(step) / 20.0;
        let mix = |c: u8| {
            let c = f64::from(c);
            // Within 0..=255, as both ends are.
            (c + (toward - c) * t).round().clamp(0.0, 255.0) as u8
        };
        let mixed = Color32::from_rgb(mix(colour.r()), mix(colour.g()), mix(colour.b()));
        if surfaces
            .iter()
            .all(|&s| contrast(mixed, s) >= AREA_CONTRAST)
        {
            return mixed;
        }
    }
    if dark { Color32::WHITE } else { Color32::BLACK }
}

/// The WCAG contrast of two opaque colours.
pub fn contrast(a: Color32, b: Color32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// The WCAG relative luminance of an opaque colour.
fn luminance(colour: Color32) -> f64 {
    let linear = |c: u8| {
        let v = f64::from(c) / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(colour.r()) + 0.7152 * linear(colour.g()) + 0.0722 * linear(colour.b())
}

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
    /// Strong text, a heading's (obf-1126): set off from plain text,
    /// darker in light and brighter in dark, as egui's strong is only a
    /// colour.
    pub strong: Color32,
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
    strong: Color32::from_gray(15),
    weak: Color32::from_gray(90),
    link: argb(0xFF00695C),
    error: argb(0xFFA8231C),
    warn: argb(0xFF8A4500),
};

pub const DARK_TEXT: TextTones = TextTones {
    text: Color32::from_gray(220),
    strong: Color32::WHITE,
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

/// How round a button is (#218).
pub const BUTTON_RADIUS: u8 = 8;

/// The room between a button's words and its outline.
const BUTTON_PADDING: egui::Vec2 = egui::vec2(10.0, 4.0);

/// The room left and right of a field's text (obf-1128).
const FIELD_SIDE: i8 = 8;

/// The room around a single-line field's text in `font` (obf-1128):
/// [`FIELD_SIDE`] at its sides, and above and below what makes it as tall
/// as a button beside it, whatever its font and the text size.
pub fn field_margin(ui: &Ui, font: &egui::FontId) -> egui::Margin {
    let words = egui::TextStyle::Button.resolve(ui.style());
    // A button is as tall as its words' line, a field at least as its
    // font's row.
    let line = |f: &mut egui::epaint::text::FontsView<'_>, font: &egui::FontId| {
        f.layout_no_wrap("Mg".into(), font.clone(), Color32::PLACEHOLDER)
            .size()
            .y
    };
    let (button, own) = ui.ctx().fonts_mut(|f| {
        let own = line(f, font).max(f.row_height(font));
        (line(f, &words), own)
    });
    let height = (button + 2.0 * BUTTON_PADDING.y).max(ui.spacing().interact_size.y);
    let margin = ((height - own) / 2.0).max(0.0).round() as i8;
    egui::Margin::symmetric(FIELD_SIDE, margin)
}

/// A framed single-line field over `text` (UI laws, "Design system",
/// obf-1128): every field of the desktop the same height, a button's,
/// its text clear of its outline. A field in another font takes
/// [`field_margin`] of that font after it.
pub fn field<'t>(ui: &Ui, text: &'t mut dyn egui::TextBuffer) -> egui::TextEdit<'t> {
    let body = egui::TextStyle::Body.resolve(ui.style());
    egui::TextEdit::singleline(text).margin(field_margin(ui, &body))
}

/// What a button does (#218): the main action of a dialog or page, one
/// that deletes or removes, or any other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Secondary,
    Primary,
    Destructive,
}

/// The colours of a kind of button in one brightness: its fill idle,
/// under the pointer and pressed or focused, its 1 px outline, the
/// 2 px ring it wears while pressed or focused, and its text, readable
/// on all three fills (WCAG AA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonTones {
    pub idle: Color32,
    pub hovered: Color32,
    pub pressed: Color32,
    pub outline: Color32,
    pub ring: Color32,
    pub text: Color32,
}

/// The tones of a `kind` of button: secondary ones a tonal fill, darker
/// under the pointer in light and lighter in dark; primary ones the
/// accent, destructive ones the error colour, both with the text of
/// [`on_accent`].
pub fn button_tones(dark: bool, kind: ButtonKind) -> ButtonTones {
    let x = if dark { DARK_TEXT } else { LIGHT_TEXT };
    let filled = |[idle, hovered, pressed]: [u32; 3]| ButtonTones {
        idle: argb(idle),
        hovered: argb(hovered),
        pressed: argb(pressed),
        outline: argb(pressed),
        ring: x.text,
        text: on_accent(dark),
    };
    match (kind, dark) {
        (ButtonKind::Secondary, false) => ButtonTones {
            idle: argb(0xFFB2DFDB),
            hovered: argb(0xFF9ED5CF),
            pressed: argb(0xFF85C9C2),
            outline: argb(0xFF467F79),
            ring: LIGHT.accent,
            text: x.text,
        },
        (ButtonKind::Secondary, true) => ButtonTones {
            idle: argb(0xFF4A5359),
            hovered: argb(0xFF535D64),
            pressed: argb(0xFF3A4A4E),
            outline: argb(0xFF8A979E),
            ring: DARK.accent,
            text: x.text,
        },
        (ButtonKind::Primary, false) => filled([0xFF00796B, 0xFF00695C, 0xFF004D40]),
        (ButtonKind::Primary, true) => filled([0xFF4DB6AC, 0xFF80CBC4, 0xFFB2DFDB]),
        (ButtonKind::Destructive, false) => filled([0xFFA8231C, 0xFF8E1B15, 0xFF731410]),
        (ButtonKind::Destructive, true) => filled([0xFFFFB4AB, 0xFFFFCDC6, 0xFFFFDAD6]),
    }
}

/// Paints `visuals`' interactive widgets in the button `tones`: filled,
/// outlined, rounded, the text in its colour in every state.
fn paint_buttons(visuals: &mut Visuals, b: ButtonTones) {
    let w = &mut visuals.widgets;
    let outline = Stroke::new(1.0, b.outline);
    for (state, fill, stroke) in [
        (&mut w.inactive, b.idle, outline),
        (&mut w.hovered, b.hovered, outline),
        (&mut w.active, b.pressed, Stroke::new(2.0, b.ring)),
        (&mut w.open, b.hovered, outline),
    ] {
        state.weak_bg_fill = fill;
        state.bg_stroke = stroke;
        state.corner_radius = CornerRadius::same(BUTTON_RADIUS);
        state.fg_stroke.color = b.text;
    }
}

/// A dialog's or page's main action (Save, Add, Done, Create): a button
/// on the accent, greyed unless `enabled`.
pub fn primary(ui: &mut Ui, enabled: bool, label: &str) -> Response {
    emphasised(ui, ButtonKind::Primary, enabled, label)
}

/// A dialog's main action as [`primary`] draws it, which Ctrl+Enter (Cmd
/// on macOS) also runs while its dialog is the one on top (#277);
/// whether it was asked for.
pub fn main_action(ui: &mut Ui, enabled: bool, label: &str) -> bool {
    let clicked = primary(ui, enabled, label).clicked();
    clicked || (enabled && ctrl_enter(ui))
}

/// Whether `ui` is in the dialog on top: no modal dialog lies over it.
pub fn on_top(ui: &Ui) -> bool {
    let layer = ui.layer_id();
    ui.ctx().memory(|m| m.is_above_modal_layer(layer))
}

/// Ctrl+Enter (Cmd on macOS) pressed in the dialog on top, taken (#277).
pub fn ctrl_enter(ui: &Ui) -> bool {
    on_top(ui) && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
}

/// An action that deletes or removes: a button on the error colour,
/// greyed unless `enabled`.
pub fn destructive(ui: &mut Ui, enabled: bool, label: &str) -> Response {
    emphasised(ui, ButtonKind::Destructive, enabled, label)
}

fn emphasised(ui: &mut Ui, kind: ButtonKind, enabled: bool, label: &str) -> Response {
    let tones = button_tones(ui.visuals().dark_mode, kind);
    ui.scope(|ui| {
        paint_buttons(ui.visuals_mut(), tones);
        ui.add_enabled(enabled, Button::new(label))
    })
    .inner
}

/// How round a checkbox's box is: a button's radius would make the
/// small box a circle, as a radio button is.
pub const CHECKBOX_RADIUS: u8 = 3;

/// Draws the checkboxes `add` adds as boxes with softly rounded corners,
/// not as the circles the buttons' radius would make of them.
pub fn square_boxes<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        for state in [&mut w.inactive, &mut w.hovered, &mut w.active] {
            state.corner_radius = CornerRadius::same(CHECKBOX_RADIUS);
        }
        add(ui)
    })
    .inner
}

/// A checkbox, its box square ([`square_boxes`]).
pub fn checkbox(ui: &mut Ui, checked: &mut bool, text: impl Into<egui::WidgetText>) -> Response {
    square_boxes(ui, |ui| ui.checkbox(checked, text))
}

/// The rounded background of an icon-only button over `rect` (#218):
/// the hovered fill under the pointer, the focus ring while it has the
/// keyboard.
pub fn icon_button_back(ui: &Ui, resp: &Response, rect: Rect) {
    let w = &ui.visuals().widgets;
    let radius = CornerRadius::same(BUTTON_RADIUS);
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, radius, w.hovered.weak_bg_fill);
    }
    if resp.has_focus() {
        ui.painter()
            .rect_stroke(rect, radius, w.active.bg_stroke, egui::StrokeKind::Inside);
    }
}

/// How strongly a marked row, a selected entry and selected text are
/// tinted with the accent.
const SELECTION_TINT: f32 = 0.25;

/// How strongly code is tinted with the text's colour behind it, as on
/// the phone (#327).
pub const CODE_TINT: f32 = 0.08;

/// The ground of code: a light tint of the text's colour `text`, no
/// frame, in light and dark alike.
pub fn code_tint(text: Color32) -> Color32 {
    let alpha = (CODE_TINT * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(text.r(), text.g(), text.b(), alpha)
}

/// The tint behind a place a search or a find bar found (obf-1047), the
/// current one's stronger, in the dark look while `dark`: amber, which
/// the look uses nowhere else, behind the text's own colour, dark on the
/// light look's and light on the dark look's.
pub fn found_tint(dark: bool, current: bool) -> Color32 {
    match (dark, current) {
        (false, false) => argb(0xFFFFE9A8),
        (false, true) => argb(0xFFFFC34D),
        (true, false) => argb(0xFF5C4A12),
        (true, true) => argb(0xFF8A6400),
    }
}

/// The colour a part of code of the kind `part` is drawn in (obf-1036),
/// in the dark look while `dark`, readable on [`code_tint`]: GitHub's
/// palette, as the phone's.
pub fn code_colour(dark: bool, part: CodeColour) -> Color32 {
    let (light, dark_one) = match part {
        CodeColour::Comment => (0x6A737D, 0x8B949E),
        CodeColour::String => (0x0A3069, 0xA5D6FF),
        CodeColour::Number | CodeColour::Constant | CodeColour::Attribute => (0x0550AE, 0x79C0FF),
        CodeColour::Keyword | CodeColour::Operator => (0xCF222E, 0xFF7B72),
        CodeColour::Type | CodeColour::Variable => (0x953800, 0xFFA657),
        CodeColour::Function => (0x8250DF, 0xD2A8FF),
        CodeColour::Tag => (0x116329, 0x7EE787),
    };
    argb(0xFF00_0000 | if dark { dark_one } else { light })
}

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
    v.code_bg_color = code_tint(x.text);
    v.selection.bg_fill = t.accent.gamma_multiply(SELECTION_TINT);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    paint_buttons(&mut v, button_tones(dark, ButtonKind::Secondary));
    // egui reads strong text's colour from a pressed widget's (obf-1126);
    // a pressed button's words take it too, readable on its fill.
    v.widgets.active.fg_stroke.color = x.strong;
    v
}

/// Installs the look for both brightnesses; the theme chosen picks one.
/// Cheap enough to call every frame.
pub fn install(ctx: &Context) {
    ctx.set_visuals_of(Theme::Light, visuals(false));
    ctx.set_visuals_of(Theme::Dark, visuals(true));
    for theme in [Theme::Light, Theme::Dark] {
        ctx.style_mut_of(theme, |s| s.spacing.button_padding = BUTTON_PADDING);
    }
    text_styles(ctx);
}

/// The size of content, a Note's or Section's text, a Line, what is
/// typed (UI laws, "Text sizes", obf-1122), before the text size grows
/// it through [`content`].
pub const CONTENT: f32 = 16.0;

/// The size of a label's and a button's words (UI laws, "Spacing and
/// type", obf-1081), before the text size: egui's body and button text.
pub const LABEL: f32 = 14.0;

/// The size of a caption, a meta line or a hint (obf-1081), before the
/// text size: egui's small text, `.small()`.
pub const CAPTION: f32 = 12.0;

/// The size of a page's or band's title (obf-1121), before the text
/// size: egui's heading text, `ui.heading`.
pub const TITLE: f32 = 22.0;

/// Sets egui's text styles to the design's sizes (obf-1081): body and
/// buttons [`LABEL`], small text [`CAPTION`], headings [`TITLE`]
/// (obf-1121), grown by this Device's text size as a card's text is;
/// code keeps egui's.
fn text_styles(ctx: &Context) {
    let factor = text_factor(ctx);
    for theme in [Theme::Light, Theme::Dark] {
        ctx.style_mut_of(theme, |s| {
            for (style, size) in [
                (egui::TextStyle::Body, LABEL),
                (egui::TextStyle::Button, LABEL),
                (egui::TextStyle::Small, CAPTION),
                (egui::TextStyle::Heading, TITLE),
            ] {
                s.text_styles
                    .insert(style, egui::FontId::proportional(size * factor));
            }
        });
    }
}

/// Where the context keeps the factor of this Device's text size.
fn text_size_id() -> egui::Id {
    egui::Id::new("oldbrainfix-text-size")
}

/// Shows the text of Notes, Lines, Sections, the Agenda's rows and Ask's
/// answers at `size`'s factor (#326, obf-1019), and the labels, buttons
/// and captions around them (obf-1081), on top of the zoom of "Bigger
/// text and icons"; the context keeps it, for every window.
pub fn set_text_size(ctx: &Context, size: TextSize) {
    ctx.data_mut(|d| d.insert_temp(text_size_id(), size.factor()));
    text_styles(ctx);
}

/// The factor of the text size shown around `ctx`; 1 until one is set.
pub fn text_factor(ctx: &Context) -> f32 {
    ctx.data(|d| d.get_temp(text_size_id())).unwrap_or(1.0)
}

/// `size`, a content text's usual size in points, at the text size of
/// this Device (#326, obf-1019). Labels, buttons and captions grow with
/// it through egui's text styles ([`LABEL`], [`CAPTION`]); icons keep
/// their size.
pub fn content(ui: &Ui, size: f32) -> f32 {
    size * text_factor(ui.ctx())
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

/// How long a dash of a tear line is, and the gap after it (#307), as
/// on the phone.
pub const TEAR_DASH: f32 = 4.0;
pub const TEAR_GAP: f32 = 4.0;

/// The tear line from `left` to `right` at `y` (#307): a row of short
/// dashes like the perforation of a roll of paper, parting a Note's
/// text from each of its Sections, which read on as one text.
pub fn tear_shapes(left: f32, right: f32, y: f32, stroke: Stroke) -> Vec<Shape> {
    let ends = [egui::pos2(left, y), egui::pos2(right, y)];
    Shape::dashed_line(&ends, stroke, TEAR_DASH, TEAR_GAP)
}

/// A tear line across `ui` (#307), in the weak text colour of the time
/// stamp under it, wherever it is drawn, taking its room as a separator
/// does.
pub fn tear_line(ui: &mut Ui) -> Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 8.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let stroke = Stroke::new(1.0, ui.visuals().weak_text_color());
        let y = rect.center().y;
        let shapes = tear_shapes(rect.left(), rect.right(), y, stroke);
        ui.painter().extend(shapes);
    }
    response
}

/// The fill of the band a search field stands on over what it lists
/// (obf-1088): the marks' tone, clearly set off from a dialog's card and
/// a shade off the window's ground, where its line sets it off most, in
/// the light and the dark look.
pub fn band_fill(tones: Tones) -> Color32 {
    tones.mark
}

/// The band a search field stands on over what scrolls under it
/// (obf-1088, the phone's `SearchBand`): `add`'s field on [`band_fill`]
/// across the width, a separator's line along its bottom, and nothing
/// between it and what comes next, so a list right under it scrolls
/// under the line, not under an edge in the page's own colour.
pub fn search_band<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let fill = band_fill(tones(ui.visuals()));
    let line = ui.visuals().widgets.noninteractive.bg_stroke;
    let spacing = ui.spacing().item_spacing.y;
    // The room after an item is taken as it is placed: none after the band.
    ui.spacing_mut().item_spacing.y = 0.0;
    let band = egui::Frame::NONE
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = spacing;
            ui.set_min_width(ui.available_width());
            add(ui)
        });
    ui.spacing_mut().item_spacing.y = spacing;
    let rect = band.response.rect;
    // Inside the band, so what scrolls under it does not cover it.
    let y = rect.bottom() - line.width / 2.0;
    ui.painter().hline(rect.x_range(), y, line);
    band.inner
}

#[cfg(test)]
mod tests;
