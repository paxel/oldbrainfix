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
    let luminance =
        |c: Color32| 0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b());
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

const KINDS: [ButtonKind; 3] = [
    ButtonKind::Secondary,
    ButtonKind::Primary,
    ButtonKind::Destructive,
];

#[test]
fn the_text_of_every_button_is_readable_on_its_fill_in_every_state() {
    for (name, _, _, _) in both() {
        for kind in KINDS {
            let b = button_tones(name == "dark", kind);
            for (state, fill) in [
                ("idle", b.idle),
                ("hovered", b.hovered),
                ("pressed", b.pressed),
            ] {
                assert_eq!(fill.a(), 255, "{name} {kind:?} {state} is opaque");
                let ratio = contrast(b.text, fill);
                assert!(ratio >= 4.5, "{name} {kind:?} text {state}: {ratio:.2}");
            }
            assert_ne!(b.idle, b.hovered, "{name} {kind:?} hover shows");
            assert_ne!(b.hovered, b.pressed, "{name} {kind:?} press shows");
        }
    }
}

#[test]
fn a_button_stands_out_by_its_outline_and_its_focus_ring() {
    for (name, t, _, _) in both() {
        for kind in KINDS {
            let b = button_tones(name == "dark", kind);
            for (ground, under) in [
                ("ground", t.background),
                ("marks", t.mark),
                ("card", t.card),
            ] {
                let outline = contrast(b.outline, under);
                let ring = contrast(b.ring, under);
                assert!(
                    outline >= 3.0 || kind != ButtonKind::Secondary,
                    "{name} {kind:?} outline on the {ground}: {outline:.2}"
                );
                assert!(
                    ring >= 3.0,
                    "{name} {kind:?} ring on the {ground}: {ring:.2}"
                );
            }
        }
        // The primary and destructive ones differ from the rest.
        let dark = name == "dark";
        let fills = KINDS.map(|kind| button_tones(dark, kind).idle);
        assert_ne!(fills[0], fills[1], "{name}");
        assert_ne!(fills[1], fills[2], "{name}");
        assert_ne!(fills[0], fills[2], "{name}");
    }
}

#[test]
fn every_button_draws_rounded_filled_and_outlined_in_the_secondary_tones() {
    for (name, _, x, v) in both() {
        let b = button_tones(v.dark_mode, ButtonKind::Secondary);
        assert_eq!(b.text, x.text, "{name}");
        let w = &v.widgets;
        for (state, fill, stroke) in [
            ("idle", b.idle, Stroke::new(1.0, b.outline)),
            ("hovered", b.hovered, Stroke::new(1.0, b.outline)),
            ("pressed", b.pressed, Stroke::new(2.0, b.ring)),
            ("open", b.hovered, Stroke::new(1.0, b.outline)),
        ] {
            let s = match state {
                "idle" => w.inactive,
                "hovered" => w.hovered,
                "pressed" => w.active,
                _ => w.open,
            };
            assert_eq!(s.weak_bg_fill, fill, "{name} {state}");
            assert_eq!(s.bg_stroke, stroke, "{name} {state}");
            assert_eq!(s.corner_radius, CornerRadius::same(BUTTON_RADIUS), "{name}");
            assert_eq!(s.fg_stroke.color, x.text, "{name} {state} text");
        }
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
