//! The building blocks of the design system the screens use (UI laws,
//! "Design system", obf-1081) beside those of [`crate::theme`]: the chip
//! and the text button with an icon. The filled buttons are
//! [`crate::theme::primary`], [`crate::theme::main_action`] and
//! [`crate::theme::destructive`]; every other button egui draws wears the
//! outlined one's tonal fill and outline, but Cancel and Close in a
//! dialog's action row, which are [`row_button`]s.

use egui::{Color32, CornerRadius, Id, Response, Sense, Ui, WidgetInfo, WidgetType};

use crate::icons;
use crate::theme;

/// How high a chip is outside a list's band, before the text size: the
/// phone's 32; in the band it takes the band's row.
pub const CHIP_HEIGHT: f32 = 32.0;

/// The room left and right of a chip's words.
const CHIP_PAD: f32 = 12.0;

/// The room between a chip's words and its ✕.
const CHIP_GAP: f32 = 6.0;

/// How strongly a picked chip is tinted with the accent, and one under
/// the pointer.
const PICKED_TINT: f32 = 0.3;
const PICKED_HOVER_TINT: f32 = 0.45;

/// A chip (UI laws, "Chips", obf-1081): a short word in a full pill, its
/// ends round at half its height, a 1 px outline and, while picked, a
/// fill tinted with the accent; its words in the button's text style.
/// Self-painted, so its words read in every state.
#[derive(Debug, Clone, Copy)]
pub struct Chip<'a> {
    words: &'a str,
    /// Whether it is picked, for a chip that is picked or not; `None`
    /// for one that acts, such as a tag's.
    on: Option<bool>,
    typ: WidgetType,
    said: Option<&'a str>,
    closes: bool,
    height: Option<f32>,
}

impl<'a> Chip<'a> {
    /// A chip of `words` that acts when clicked, a button to a screen
    /// reader.
    pub fn new(words: &'a str) -> Self {
        Self {
            words,
            on: None,
            typ: WidgetType::Button,
            said: None,
            closes: false,
            height: None,
        }
    }

    /// A chip that is picked or not, picked while `on`, as a screen
    /// reader hears it too.
    pub fn on(self, on: bool) -> Self {
        Self {
            on: Some(on),
            ..self
        }
    }

    /// A radio button to a screen reader: one of a choice chip row.
    pub fn radio(self) -> Self {
        Self {
            typ: WidgetType::RadioButton,
            ..self
        }
    }

    /// Heard as `said` by a screen reader instead of its words.
    pub fn said(self, said: &'a str) -> Self {
        Self {
            said: Some(said),
            ..self
        }
    }

    /// With ✕ after its words: a filter chip whose click takes it out.
    pub fn closing(self) -> Self {
        Self {
            closes: true,
            ..self
        }
    }

    /// `height` high, as a band's row is, instead of [`CHIP_HEIGHT`].
    pub fn height(self, height: f32) -> Self {
        Self {
            height: Some(height),
            ..self
        }
    }

    fn font(ui: &Ui) -> egui::FontId {
        egui::TextStyle::Button.resolve(ui.style())
    }

    fn tall(&self, ui: &Ui) -> f32 {
        self.height
            .unwrap_or_else(|| CHIP_HEIGHT * theme::text_factor(ui.ctx()))
    }

    /// How wide the chip is drawn in `ui`.
    pub fn width(&self, ui: &Ui) -> f32 {
        let font = Self::font(ui);
        let cross = if self.closes {
            CHIP_GAP + font.size
        } else {
            0.0
        };
        let colour = ui.visuals().text_color();
        let words = ui
            .painter()
            .layout_no_wrap(self.words.to_string(), font, colour);
        CHIP_PAD + words.size().x + cross + CHIP_PAD
    }

    /// Draws the chip; a click on it in its response.
    pub fn show(self, ui: &mut Ui) -> Response {
        let tones = theme::tones(ui.visuals());
        let text = ui.visuals().text_color();
        let font = Self::font(ui);
        let size = egui::vec2(self.width(ui), self.tall(ui));
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let said = self.said.unwrap_or(self.words);
        let enabled = ui.is_enabled();
        resp.widget_info(|| match self.on {
            Some(on) => WidgetInfo::selected(self.typ, enabled, on, said),
            None => WidgetInfo::labeled(self.typ, enabled, said),
        });
        let on = self.on.unwrap_or(false);
        if ui.is_rect_visible(rect) {
            let radius = CornerRadius::from(rect.height() / 2.0);
            let fill = match (on, resp.hovered()) {
                (true, false) => tones.accent.gamma_multiply(PICKED_TINT),
                (true, true) => tones.accent.gamma_multiply(PICKED_HOVER_TINT),
                (false, true) => ui.visuals().widgets.hovered.weak_bg_fill,
                (false, false) => Color32::TRANSPARENT,
            };
            let outline = if on {
                tones.accent
            } else {
                ui.visuals().weak_text_color()
            };
            let painter = ui.painter();
            painter.rect(
                rect,
                radius,
                fill,
                egui::Stroke::new(1.0, outline),
                egui::StrokeKind::Inside,
            );
            let cross = font.size;
            let galley = painter.layout_no_wrap(self.words.to_string(), font, text);
            let at = egui::pos2(
                rect.left() + CHIP_PAD,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(at, galley, text);
            if self.closes {
                let x = egui::Rect::from_center_size(
                    egui::pos2(rect.right() - CHIP_PAD - cross / 2.0, rect.center().y),
                    egui::vec2(cross, cross),
                );
                icons::paint(painter, x, icons::CLOSE, text);
            }
            if resp.has_focus() {
                let ring = ui.visuals().widgets.active.bg_stroke;
                let around = rect.expand(2.0);
                let round = CornerRadius::from(around.height() / 2.0);
                painter.rect_stroke(around, round, ring, egui::StrokeKind::Outside);
            }
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

/// A chip of `words`, picked while `on`: [`Chip`] as most rows want it.
pub fn chip(ui: &mut Ui, on: bool, words: &str) -> Response {
    Chip::new(words).on(on).show(ui)
}

/// How strongly the ground under a text button is tinted with the
/// accent while the pointer is on it.
const TEXT_BUTTON_TINT: f32 = 0.15;

/// A lesser action inside a card or a row (UI laws, "Buttons",
/// obf-1081): `icon` and `words`, both in the accent's text colour,
/// without a frame, a rounded tint of the accent under the pointer and
/// the ring while it has the keyboard; greyed while `ui` is disabled.
pub fn text_button(ui: &mut Ui, icon: char, words: &str) -> Response {
    plain_button(ui, Some(icon), words)
}

/// Cancel or Close in a dialog's action row (UI laws, "Buttons",
/// obf-1102): a [`text_button`] without its icon, the one place it goes
/// without one, as the row shows these are its buttons.
pub fn row_button(ui: &mut Ui, words: &str) -> Response {
    plain_button(ui, None, words)
}

/// A [`text_button`], with `icon` before its words where there is one.
fn plain_button(ui: &mut Ui, icon: Option<char>, words: &str) -> Response {
    let (placed, resp) = lay(ui, icon, words, Placing::Laid(Sense::click()));
    placed.answer(ui, &resp);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A [`text_button`] drawn under a click laid over it afterwards, as a
/// card's fold whose click the card's own would cover (obf-1110): its
/// icon and words now, without taking a click; its tint and ring once
/// [`UnderClick::answer`] has the click's response.
pub fn text_button_under(ui: &mut Ui, icon: char, words: &str) -> UnderClick {
    lay(ui, Some(icon), words, Placing::Laid(Sense::hover())).0
}

/// Where a text button lies and the room kept under its words for its
/// tint and ring.
#[derive(Clone)]
pub struct UnderClick {
    pub rect: egui::Rect,
    ground: egui::layers::ShapeIdx,
    /// The painter it was drawn with: its layer and clip.
    painter: egui::Painter,
}

impl UnderClick {
    /// Tints the ground under the pointer and rings it while `resp` has
    /// the keyboard.
    pub fn answer(&self, ui: &Ui, resp: &Response) {
        let radius = CornerRadius::same(theme::BUTTON_RADIUS);
        let mut ground = Vec::new();
        if resp.hovered() {
            let tint = theme::tones(ui.visuals())
                .accent
                .gamma_multiply(TEXT_BUTTON_TINT);
            ground.push(egui::Shape::rect_filled(self.rect, radius, tint));
        }
        if resp.has_focus() {
            let ring = ui.visuals().widgets.active.bg_stroke;
            ground.push(egui::Shape::rect_stroke(
                self.rect,
                radius,
                ring,
                egui::StrokeKind::Inside,
            ));
        }
        self.painter.set(self.ground, egui::Shape::Vec(ground));
    }
}

/// A [`text_button`] at `rect`, which [`text_button_size`] measured,
/// clicked by `id`: for a place a layout does not reach, as a table
/// grid's switch to its lines (obf-1111).
pub fn text_button_at(
    ui: &mut Ui,
    (rect, id): (egui::Rect, Id),
    icon: char,
    words: &str,
) -> Response {
    let (placed, resp) = lay(ui, Some(icon), words, Placing::At(rect, id));
    placed.answer(ui, &resp);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// How much room the [`text_button`] of `words` takes.
pub fn text_button_size(ui: &Ui, words: &str) -> egui::Vec2 {
    measure(ui, true, words).size
}

/// Where a text button goes: laid out with its own sense, or at a
/// place, clicked by an id.
enum Placing {
    Laid(Sense),
    At(egui::Rect, Id),
}

/// A text button's words laid out, its room and the parts of it.
struct Measured {
    galley: std::sync::Arc<egui::Galley>,
    colour: Color32,
    size: egui::Vec2,
    glyph: f32,
    before: f32,
}

/// Lays out a text button's words, with room for an icon where it has
/// one.
fn measure(ui: &Ui, icon: bool, words: &str) -> Measured {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let colour = if ui.is_enabled() {
        ui.visuals().hyperlink_color
    } else {
        ui.visuals().weak_text_color()
    };
    let galley = ui
        .painter()
        .layout_no_wrap(words.to_string(), font.clone(), colour);
    let pad = ui.spacing().button_padding;
    let gap = ui.spacing().icon_spacing;
    let glyph = font.size * 1.25;
    let before = if icon { glyph + gap } else { 0.0 };
    let inner = egui::vec2(before + galley.size().x, glyph.max(galley.size().y));
    let size = (inner + pad * 2.0).max(egui::vec2(0.0, ui.spacing().interact_size.y));
    Measured {
        galley,
        colour,
        size,
        glyph,
        before,
    }
}

/// Lays out and draws a text button's icon and words, placed as `placing`
/// says.
fn lay(ui: &mut Ui, icon: Option<char>, words: &str, placing: Placing) -> (UnderClick, Response) {
    let Measured {
        galley,
        colour,
        size,
        glyph,
        before,
    } = measure(ui, icon.is_some(), words);
    let ground = ui.painter().add(egui::Shape::Noop);
    let (rect, resp) = match placing {
        Placing::Laid(sense) => ui.allocate_exact_size(size, sense),
        Placing::At(rect, id) => (rect, ui.interact(rect, id, Sense::click())),
    };
    if resp.sense.senses_click() {
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), words));
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let pad = ui.spacing().button_padding;
        let at = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad.x, rect.center().y - glyph / 2.0),
            egui::vec2(glyph, glyph),
        );
        if let Some(icon) = icon {
            icons::paint(painter, at, icon, colour);
        }
        let left = at.left() + before;
        let top = rect.center().y - galley.size().y / 2.0;
        painter.galley(egui::pos2(left, top), galley, colour);
    }
    let painter = ui.painter().clone();
    (
        UnderClick {
            rect,
            ground,
            painter,
        },
        resp,
    )
}
