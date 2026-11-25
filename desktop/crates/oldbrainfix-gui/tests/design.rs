//! The desktop's shared blocks of the design system (obf-1081): labels
//! and buttons 14, captions 12 and page titles 22 (obf-1121) in egui's
//! text styles, growing with Text size; a chip a full pill, 32 high and 36 in a band; the text
//! button with an icon in the accent's colour, tinted under the pointer;
//! "Open in the editor" in the outlined button's tonal fill; and a
//! single-line field as tall as a button in any font (obf-1128).

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(clippy::panic, clippy::indexing_slicing)]

use egui::accesskit::{Role, Toggled};
use egui::epaint::{ClippedShape, CornerRadius};
use egui::{Rect, Shape, TextStyle};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::TextSize;
use oldbrainfix_gui::design::{self, Chip};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::setting_badges::{self, BadgeValue};
use oldbrainfix_gui::theme::{self, ButtonKind, LIGHT};
use oldbrainfix_gui::{App, capture_choices, icons};

/// A harness showing `add` in the app's look, at `size`'s text size.
fn harness<'a>(size: TextSize, add: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    let mut add = add;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 300.0))
        .build_ui(move |ui| {
            App::install(ui.ctx());
            theme::set_text_size(ui.ctx(), size);
            ui.ctx().set_theme(egui::Theme::Light);
            add(ui);
        });
    harness.run_steps(4);
    harness
}

/// Every shape drawn, out of its lists.
fn drawn(harness: &Harness<'_>) -> Vec<Shape> {
    fn flat(shape: &Shape, into: &mut Vec<Shape>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| flat(s, into)),
            other => into.push(other.clone()),
        }
    }
    let mut shapes = Vec::new();
    for ClippedShape { shape, .. } in &harness.output().shapes {
        flat(shape, &mut shapes);
    }
    shapes
}

/// Where the widget of `role` labelled `label` lies, in points.
fn rect(harness: &Harness<'_>, role: Role, label: &str) -> Rect {
    let at = harness
        .query_all_by(|n| n.role() == role && n.label().as_deref() == Some(label))
        .next()
        .unwrap_or_else(|| panic!("no {role:?} {label}"))
        .rect();
    let scale = harness.ctx.pixels_per_point();
    Rect::from_min_max(
        (at.min.to_vec2() / scale).to_pos2(),
        (at.max.to_vec2() / scale).to_pos2(),
    )
}

/// The rounded rectangle drawn over `at`: its fill and corner radius.
fn frame(harness: &Harness<'_>, at: Rect) -> Option<(egui::Color32, CornerRadius)> {
    drawn(harness).into_iter().find_map(|shape| match shape {
        Shape::Rect(r)
            if (r.rect.min - at.min).length() < 1.0 && (r.rect.max - at.max).length() < 1.0 =>
        {
            Some((r.fill, r.corner_radius))
        }
        _ => None,
    })
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn labels_and_buttons_are_14_captions_12_and_titles_22_growing_with_text_size() {
    for (size, factor) in [(TextSize::Normal, 1.0), (TextSize::Largest, 1.3)] {
        let harness = harness(size, |ui| {
            ui.label("Words");
        });
        let style = harness.ctx.global_style();
        let of = |s: TextStyle| style.text_styles[&s].size;
        assert!(near(of(TextStyle::Body), 14.0 * factor), "body at {size:?}");
        assert!(
            near(of(TextStyle::Button), 14.0 * factor),
            "button at {size:?}"
        );
        assert!(
            near(of(TextStyle::Small), 12.0 * factor),
            "small at {size:?}"
        );
        assert!(
            near(of(TextStyle::Heading), 22.0 * factor),
            "heading at {size:?}"
        );
    }
}

#[test]
fn a_chip_is_a_full_pill_32_high_and_36_in_a_band() {
    let mut harness = harness(TextSize::Normal, |ui| {
        ui.horizontal(|ui| {
            design::chip(ui, false, "Mon");
            Chip::new("Kitchen").height(36.0).show(ui);
            Chip::new("Ann").on(true).closing().radio().show(ui);
        });
    });
    harness.run_steps(2);
    let free = rect(&harness, Role::Button, "Mon");
    let band = rect(&harness, Role::Button, "Kitchen");
    let picked = rect(&harness, Role::RadioButton, "Ann");
    assert!(near(free.height(), 32.0), "free chip {free:?}");
    assert!(near(band.height(), 36.0), "band chip {band:?}");
    let (_, round) = frame(&harness, free).unwrap_or_else(|| panic!("no pill drawn"));
    assert_eq!(round, CornerRadius::same(16));
    let (_, round) = frame(&harness, band).unwrap_or_else(|| panic!("no pill drawn"));
    assert_eq!(round, CornerRadius::same(18));
    let (fill, _) = frame(&harness, picked).unwrap_or_else(|| panic!("no pill drawn"));
    assert_eq!(
        fill,
        LIGHT.accent.gamma_multiply(0.3),
        "a picked chip is tinted"
    );
}

#[test]
fn a_chip_that_acts_is_no_toggle_one_picked_is() {
    let harness = harness(TextSize::Normal, |ui| {
        ui.horizontal(|ui| {
            Chip::new("#shopping").show(ui);
            design::chip(ui, true, "Tue");
        });
    });
    let acts = harness.get_by_role_and_label(Role::Button, "#shopping");
    assert_eq!(acts.accesskit_node().toggled(), None);
    let picked = harness.get_by_role_and_label(Role::Button, "Tue");
    assert_eq!(picked.accesskit_node().toggled(), Some(Toggled::True));
}

#[test]
fn a_chip_grows_with_text_size() {
    let harness = harness(TextSize::Largest, |ui| {
        design::chip(ui, false, "Mon");
    });
    let chip = rect(&harness, Role::Button, "Mon");
    assert!(near(chip.height(), 32.0 * 1.3), "chip {chip:?}");
}

#[test]
fn a_text_button_is_in_the_accent_and_tinted_under_the_pointer() {
    let mut harness = harness(TextSize::Normal, |ui| {
        ui.add_space(40.0);
        design::text_button(ui, icons::PASTE, "Paste");
    });
    let at = rect(&harness, Role::Button, "Paste");
    assert!(frame(&harness, at).is_none(), "no frame while idle");
    let words = drawn(&harness).into_iter().any(|shape| match shape {
        Shape::Text(text) => {
            text.galley.text() == "Paste" && text.fallback_color == theme::LIGHT_TEXT.link
        }
        _ => false,
    });
    assert!(words, "the words in the accent's text colour");
    harness.get_by_label("Paste").hover();
    harness.run_steps(2);
    let (fill, round) = frame(&harness, at).unwrap_or_else(|| panic!("no tint under the pointer"));
    assert_eq!(fill, LIGHT.accent.gamma_multiply(0.15));
    assert_eq!(round, CornerRadius::same(theme::BUTTON_RADIUS));
}

#[test]
fn a_dialog_rows_cancel_is_a_text_button_without_its_icon() {
    let mut harness = harness(TextSize::Normal, |ui| {
        ui.add_space(40.0);
        design::row_button(ui, "Cancel");
    });
    let at = rect(&harness, Role::Button, "Cancel");
    assert!(frame(&harness, at).is_none(), "no frame while idle");
    let pad = harness.ctx.global_style().spacing.button_padding.x;
    let shapes = drawn(&harness);
    let words = shapes.iter().find_map(|shape| match shape {
        Shape::Text(text) if text.galley.text() == "Cancel" => Some(text.clone()),
        _ => None,
    });
    let words = words.unwrap_or_else(|| panic!("no words"));
    assert_eq!(words.fallback_color, theme::LIGHT_TEXT.link);
    // No icon: the words start where the padding ends, and nothing else
    // is drawn on it.
    assert!(
        near(words.pos.x, at.min.x + pad),
        "{} {}",
        words.pos.x,
        at.min.x
    );
    let on = shapes
        .iter()
        .filter(|shape| at.expand(1.0).contains_rect(shape.visual_bounding_rect()))
        .count();
    assert_eq!(on, 1, "only its words on it");
    harness.get_by_label("Cancel").hover();
    harness.run_steps(2);
    let (fill, _) = frame(&harness, at).unwrap_or_else(|| panic!("no tint under the pointer"));
    assert_eq!(fill, LIGHT.accent.gamma_multiply(0.15));
}

#[test]
fn open_in_the_editor_wears_the_outlined_buttons_tonal_fill() {
    let t = L10n::new("en");
    let harness = harness(TextSize::Normal, move |ui| {
        capture_choices::open_in_editor(ui, &t, true);
    });
    let label = L10n::new("en").capture_open_in_editor().to_string();
    let at = rect(&harness, Role::Button, &label);
    let tonal = theme::button_tones(false, ButtonKind::Secondary).idle;
    let fill = drawn(&harness).into_iter().any(|shape| match shape {
        Shape::Rect(r) => r.fill == tonal && r.rect.contains_rect(at.shrink(1.0)),
        _ => false,
    });
    assert!(fill, "the tonal fill under {at:?}");
}

#[test]
fn a_framed_icon_button_wears_the_outlined_buttons_frame_and_its_name() {
    let mut harness = harness(TextSize::Normal, |ui| {
        ui.add_space(40.0);
        design::framed_icon_button(ui, icons::ADD, "More");
    });
    let at = rect(&harness, Role::Button, "More");
    assert!(near(at.width(), at.height()), "square {at:?}");
    let look = harness.ctx.global_style().visuals.widgets.inactive;
    let (fill, round) = frame(&harness, at).unwrap_or_else(|| panic!("no frame while idle"));
    assert_eq!(fill, look.weak_bg_fill);
    assert_eq!(round, look.corner_radius);
    let outlined = drawn(&harness).into_iter().any(|shape| match shape {
        Shape::Rect(r) => r.stroke == look.bg_stroke && r.stroke.width > 0.0,
        _ => false,
    });
    assert!(outlined, "its outline");
    harness.get_by_label("More").hover();
    harness.run_steps(2);
    let hovered = harness
        .ctx
        .global_style()
        .visuals
        .widgets
        .hovered
        .weak_bg_fill;
    let (fill, _) = frame(&harness, at).unwrap_or_else(|| panic!("no frame under the pointer"));
    assert_eq!(fill, hovered);
}

#[test]
fn a_setting_badges_drop_down_mark_stands_4_after_its_value() {
    let t = L10n::new("en");
    let harness = harness(TextSize::Normal, move |ui| {
        ui.add_space(40.0);
        let tones = theme::tones(ui.visuals());
        setting_badges::badge(
            ui,
            (&t, tones),
            (icons::NOTE, None),
            "Kind",
            Some(BadgeValue::Choice("Plain Note")),
            true,
        );
    });
    let shapes = drawn(&harness);
    let text = |words: &str| {
        shapes.iter().find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text() == words => Some(text.clone()),
            _ => None,
        })
    };
    let value = text("Plain Note").unwrap_or_else(|| panic!("no value"));
    let mark = text(&icons::ARROW_DROP_DOWN.to_string()).unwrap_or_else(|| panic!("no ▾"));
    let end = value.pos.x + value.galley.size().x;
    assert!(
        near(mark.pos.x - end, setting_badges::ARROW_GAP),
        "{} {end}",
        mark.pos.x
    );
}

#[test]
fn a_dialog_title_is_18_strong_at_every_text_size() {
    for size in [TextSize::Normal, TextSize::Largest] {
        let harness = harness(size, |ui| {
            design::dialog_title(ui, "Pair a Device");
        });
        let strong = harness.ctx.global_style().visuals.strong_text_color();
        let title = drawn(&harness)
            .into_iter()
            .find_map(|shape| match shape {
                Shape::Text(text) if text.galley.text() == "Pair a Device" => Some(text),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no title at {size:?}"));
        let format = &title.galley.job.sections[0].format;
        assert!(near(format.font_id.size, 18.0), "at {size:?}");
        assert_eq!(format.color, strong, "at {size:?}");
    }
}

#[test]
fn a_field_is_as_tall_as_a_button_in_any_font_its_text_clear_of_the_outline() {
    for size in [TextSize::Normal, TextSize::Largest] {
        let mut body = String::from("Body");
        let mut mono = String::from("Mono");
        let mut content = String::from("Content");
        let harness = harness(size, move |ui| {
            ui.horizontal(|ui| {
                ui.add(theme::field(ui, &mut body).desired_width(100.0));
                let font = TextStyle::Monospace.resolve(ui.style());
                let margin = theme::field_margin(ui, &font);
                ui.add(
                    theme::field(ui, &mut mono)
                        .font(font)
                        .margin(margin)
                        .desired_width(100.0),
                );
                let font = egui::FontId::proportional(theme::content(ui, theme::CONTENT));
                let margin = theme::field_margin(ui, &font);
                ui.add(
                    theme::field(ui, &mut content)
                        .font(font)
                        .margin(margin)
                        .desired_width(120.0),
                );
                let _ = ui.button("Go");
            });
        });
        let button = rect(&harness, Role::Button, "Go");
        let scale = harness.ctx.pixels_per_point();
        let fields: Vec<Rect> = harness
            .query_all_by(|n| n.role() == Role::TextInput)
            .map(|n| {
                let at = n.rect();
                Rect::from_min_max(
                    (at.min.to_vec2() / scale).to_pos2(),
                    (at.max.to_vec2() / scale).to_pos2(),
                )
            })
            .collect();
        assert_eq!(fields.len(), 3, "at {size:?}");
        for field in &fields {
            assert!(
                (field.height() - button.height()).abs() <= 1.0,
                "at {size:?}: the field {field:?}, the button {button:?}"
            );
        }
        // The typed text 8 inside the outline, never touching it.
        let shapes = drawn(&harness);
        for (words, field) in ["Body", "Mono", "Content"].iter().zip(&fields) {
            let at = shapes
                .iter()
                .find_map(|shape| match shape {
                    Shape::Text(text) if text.galley.text() == *words => {
                        Some(text.visual_bounding_rect())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no {words} at {size:?}"));
            assert!(
                at.left() - field.left() >= 7.5,
                "{words} at {at:?} in {field:?}"
            );
            assert!(
                at.top() > field.top() && at.bottom() < field.bottom(),
                "{words} {at:?}"
            );
        }
    }
}
