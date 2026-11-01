//! The badges of a Note's settings (#160), as the phone's
//! `setting_badges.dart` draws them: like a shield, the setting's icon on
//! a tinted part, then its current value. A screen reader reads one as
//! "setting: value". A click opens its small editor, a dialog with the
//! same icon and the setting's name as its title, the editor itself and
//! Close; a badge this Device may not change only shows its value. The
//! New Tally and New Session pages (#161) draw their settings the same
//! way.

use egui::{
    Align2, Color32, FontId, Frame, Id, Margin, RichText, Sense, Ui, WidgetInfo, WidgetType,
};

use crate::icons;
use crate::l10n::L10n;
use crate::theme::{self, Tones};

/// The badge of `setting` with its `value`, `None` for one that only
/// adds, such as "+": `icon` on its tinted part, in `colour` or the
/// accent. It answers clicks while `active`.
pub fn badge(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    (icon, colour): (char, Option<Color32>),
    setting: &str,
    value: Option<&str>,
    active: bool,
) -> egui::Response {
    let text = ui.visuals().text_color();
    let font = FontId::proportional(14.0);
    let galley = value.map(|v| {
        ui.painter()
            .layout_no_wrap(v.to_string(), font.clone(), text)
    });
    let height = 32.0;
    let icon_part = 38.0;
    let pad = 10.0;
    let width = icon_part + galley.as_ref().map_or(0.0, |g| g.size().x + 2.0 * pad);
    let sense = if active {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    let label = match value {
        Some(value) => t.setting_badge_label(setting, value),
        None => setting.to_string(),
    };
    resp.widget_info(|| {
        if active {
            WidgetInfo::labeled(WidgetType::Button, true, &label)
        } else {
            WidgetInfo::labeled(WidgetType::Label, true, &label)
        }
    });
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = 8.0;
        let fill = if active && resp.hovered() {
            tones.accent.gamma_multiply(0.12)
        } else {
            tones.card
        };
        painter.rect_filled(rect, radius, fill);
        let left = egui::Rect::from_min_size(rect.min, egui::vec2(icon_part, height));
        let rounding = egui::CornerRadius {
            nw: radius as u8,
            sw: radius as u8,
            ne: if galley.is_some() { 0 } else { radius as u8 },
            se: if galley.is_some() { 0 } else { radius as u8 },
        };
        painter.rect_filled(left, rounding, tones.accent.gamma_multiply(0.22));
        let square = egui::Rect::from_center_size(left.center(), egui::vec2(18.0, 18.0));
        icons::paint(painter, square, icon, colour.unwrap_or(tones.accent));
        if let Some(galley) = galley {
            let at = egui::pos2(left.right() + pad, rect.center().y - galley.size().y / 2.0);
            painter.galley(at, galley, text);
        }
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.6)),
            egui::StrokeKind::Inside,
        );
    }
    if active {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// The small editor a badge opens, over everything: `icon` and the
/// `setting`'s name as its title, then `content`, then Close. True once
/// it is to close: Close, Esc or a click beside it.
pub fn badge_dialog(
    ctx: &egui::Context,
    (t, tones): (&L10n, Tones),
    (icon, colour): (char, Option<Color32>),
    setting: &str,
    content: impl FnOnce(&mut Ui),
) -> bool {
    let frame = Frame::NONE
        .fill(tones.card)
        .corner_radius(theme::CARD_RADIUS)
        .inner_margin(Margin::same(20));
    let mut closed = false;
    let modal = egui::Modal::new(Id::new("badge dialog"))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(340.0);
            ui.spacing_mut().item_spacing.y = 10.0;
            let text = ui.visuals().text_color();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover());
                icons::paint(ui.painter(), rect, icon, colour.unwrap_or(tones.accent));
                ui.label(RichText::new(setting).size(18.0).strong().color(text));
            });
            content(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.button(RichText::new(t.close()).color(text)).clicked() {
                    closed = true;
                }
            });
        });
    closed || modal.should_close()
}

/// The editor of an on-or-off setting in its [`badge_dialog`]: its value
/// in words, the switch below it (UI laws, "The control goes below its
/// text"), called by the setting. True when it was switched.
pub fn badge_switch(ui: &mut Ui, t: &L10n, setting: &str, on: &mut bool) -> bool {
    let text = ui.visuals().text_color();
    let value = if *on { t.setting_on() } else { t.setting_off() };
    ui.label(RichText::new(value).color(text));
    ui.checkbox(on, RichText::new(setting).color(text))
        .changed()
}

/// Lays `badges` out in one wrapping row, 8 points apart.
pub fn badge_row(ui: &mut Ui, badges: impl FnOnce(&mut Ui)) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
        badges(ui);
    });
}

/// The icon of a menu entry beside its words, as the phone's menus show
/// them.
pub fn menu_entry(ui: &mut Ui, (icon, colour): (char, Color32), label: &str) -> egui::Response {
    let text = ui.visuals().text_color();
    let font = FontId::proportional(14.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text);
    let pad = egui::vec2(8.0, 6.0);
    // As wide as its words, at least as a menu's entry is.
    let size = egui::vec2(
        (20.0 + 10.0 + galley.size().x + pad.x * 2.0).max(160.0),
        galley.size().y.max(20.0) + pad.y * 2.0,
    );
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter()
                .rect_filled(rect, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
        }
        let square = egui::Rect::from_min_size(
            egui::pos2(rect.left() + pad.x, rect.center().y - 10.0),
            egui::vec2(20.0, 20.0),
        );
        icons::paint(ui.painter(), square, icon, colour);
        ui.painter().text(
            egui::pos2(square.right() + 10.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            font,
            text,
        );
    }
    resp
}
