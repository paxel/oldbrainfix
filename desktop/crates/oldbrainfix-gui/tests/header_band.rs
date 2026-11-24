//! The header band of a list of Notes (obf-1064, as the phone's,
//! obf-1041): back, forward and the path with the place shown as its
//! title in the first row, the search field, the view switch, "Show
//! Notes by…" and "+" at one height in the second, a person the filter
//! keeps as a chip with ✕ there too, all on one band of the search's
//! fill with its line under it, growing with Text size and wrapping in
//! the narrowest window.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(clippy::panic, clippy::indexing_slicing)]

use std::sync::Arc;

use egui::accesskit::Role;
use egui::epaint::ClippedShape;
use egui::{Rect, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, Notes, OwnProfile, Places, SigningKey, Store, TextSize};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::theme::{self, DARK, LIGHT};
use oldbrainfix_gui::window_sizes::MIN_WINDOW;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Household with its Topic Kitchen and a Note in each, a Note in
/// Unsorted; Kitchen's id.
fn store(size: TextSize) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([5; 32]))
    })?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    DeviceSettings::new(&store).set_text_size(size)?;
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    let kitchen = places.add_topic(&house.id, "Kitchen")?;
    let notes = Notes::new(&store);
    notes.capture("Sort the cellar", Some(&house.id), None)?;
    notes.capture("Descale the kettle", Some(&kitchen.id), None)?;
    notes.capture("Buy milk", None, None)?;
    Ok((Arc::new(store), kitchen.id))
}

/// The window at `size` over `store`, showing `selection`.
fn window(store: &Arc<Store>, selection: Selection, size: egui::Vec2) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(selection);
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        |ui, app: &mut App| {
            App::install(ui.ctx());
            app.show(ui);
        },
        app,
    );
    harness.run_steps(20);
    harness
}

fn t() -> L10n {
    L10n::new("en")
}

/// Every shape drawn, out of its lists.
fn drawn(harness: &Harness<'_, App>) -> Vec<Shape> {
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

/// The band under the search field, and whether a line runs along its
/// bottom.
fn band(harness: &Harness<'_, App>) -> (Rect, bool) {
    let field = rect(harness, Role::TextInput, None);
    let shapes = drawn(harness);
    let fills = [theme::band_fill(LIGHT), theme::band_fill(DARK)];
    let band = shapes
        .iter()
        .find_map(|shape| match shape {
            Shape::Rect(r) if fills.contains(&r.fill) && r.rect.contains_rect(field) => {
                Some(r.rect)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no band under the search field {field:?}"));
    let lined = shapes.iter().any(|shape| match shape {
        Shape::LineSegment { points, stroke } => {
            stroke.width > 0.0
                && points.iter().all(|p| (p.y - band.bottom()).abs() <= 1.0)
                && (points[0].x - band.left()).abs() <= 1.0
                && (points[1].x - band.right()).abs() <= 1.0
        }
        _ => false,
    });
    (band, lined)
}

/// Where the widget of `role` labelled `label` lies; any of the role
/// without a label.
fn rect(harness: &Harness<'_, App>, role: Role, label: Option<&str>) -> Rect {
    let at = harness
        .query_all_by(|n| n.role() == role && label.is_none_or(|l| n.label().as_deref() == Some(l)))
        .next()
        .unwrap_or_else(|| panic!("no {role:?} {label:?}"))
        .rect();
    points(harness, at)
}

/// `rect`, as a screen reader is given it in pixels, in the points the
/// shapes are drawn in.
fn points(harness: &Harness<'_, App>, rect: Rect) -> Rect {
    let scale = harness.ctx.pixels_per_point();
    Rect::from_min_max(
        (rect.min.to_vec2() / scale).to_pos2(),
        (rect.max.to_vec2() / scale).to_pos2(),
    )
}

fn button(harness: &Harness<'_, App>, label: &str) -> Rect {
    rect(harness, Role::Button, Some(label))
}

/// The title: the label holding the place shown, on the band, not the
/// sidebar's row of the same name.
fn title(harness: &Harness<'_, App>, name: &str) -> Rect {
    let (band, _) = band(harness);
    harness
        .query_all_by(|n| n.role() == Role::Label && n.value().as_deref() == Some(name))
        .map(|n| points(harness, n.rect()))
        .find(|r| band.contains_rect(*r))
        .unwrap_or_else(|| panic!("no title {name:?} on the band"))
}

/// The second row's widgets of a Topic: the field, the view switch,
/// "Show Notes by…" and "+".
fn row_two(harness: &Harness<'_, App>) -> [Rect; 4] {
    let t = t();
    [
        rect(harness, Role::TextInput, None),
        button(harness, t.show_notes_compact()),
        button(harness, t.person_filter()),
        button(harness, t.add_here()),
    ]
}

#[test]
fn a_topics_top_is_one_band_in_two_rows() -> TestResult {
    let (store, kitchen) = store(TextSize::Normal)?;
    let harness = window(&store, Selection::Place(kitchen), egui::vec2(1280.0, 800.0));
    let t = t();
    let (band, lined) = band(&harness);
    assert!(lined, "a line under the band");
    let row_one = [
        button(&harness, t.nav_back()),
        button(&harness, t.nav_forward()),
        button(&harness, "Up to Areas"),
        button(&harness, "Up to Household"),
        title(&harness, "Kitchen"),
    ];
    let row_two = row_two(&harness);
    for r in row_one.iter().chain(&row_two) {
        assert!(band.contains_rect(*r), "{r:?} outside the band {band:?}");
    }
    // The first row over the second, the place shown large.
    let one = row_one.iter().map(|r| r.bottom()).fold(f32::MIN, f32::max);
    let two = row_two.iter().map(|r| r.top()).fold(f32::MAX, f32::min);
    assert!(one <= two, "row one ends at {one}, row two starts at {two}");
    let crumb = button(&harness, "Up to Household");
    assert!(title(&harness, "Kitchen").height() > crumb.height());
    // The second row at one height, side by side.
    let [field, ..] = row_two;
    for r in &row_two {
        assert!(
            (r.height() - field.height()).abs() <= 1.0,
            "{r:?} not as tall as the field {field:?}"
        );
        assert!(
            (r.center().y - field.center().y).abs() <= 1.0,
            "{r:?} not beside the field {field:?}"
        );
    }
    for pair in row_two.windows(2) {
        assert!(pair[0].right() <= pair[1].left(), "{pair:?} out of order");
    }
    // The old heading's title, "Household / Kitchen", is gone.
    assert!(
        harness
            .query_all_by(|n| n.value().as_deref() == Some("Household / Kitchen"))
            .next()
            .is_none()
    );
    Ok(())
}

#[test]
fn a_person_the_filter_keeps_is_a_chip_in_row_two() -> TestResult {
    let (store, kitchen) = store(TextSize::Normal)?;
    let mut harness = window(&store, Selection::Place(kitchen), egui::vec2(1280.0, 800.0));
    let t = t();
    harness
        .get_by_role_and_label(Role::Button, t.person_filter())
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::CheckBox, t.person_filter_me())
        .click();
    harness.run();
    assert!(!harness.state().people_filter().is_everyone());
    // Closed again, the filter kept.
    harness.key_press(egui::Key::Escape);
    harness.run();
    let menu = button(&harness, t.person_filter());
    let chip = button(&harness, t.person_filter_me());
    let (band, _) = band(&harness);
    assert!(band.contains_rect(chip));
    assert!(
        chip.left() >= menu.right(),
        "the chip {chip:?} after the menu {menu:?}"
    );
    assert!((chip.center().y - menu.center().y).abs() <= 1.0);
    assert!((chip.height() - menu.height()).abs() <= 1.0);
    assert!(
        harness
            .query_all_by(|n| n.value().as_deref() == Some(t.person_filter_showing()))
            .next()
            .is_none(),
        "no row of its own"
    );
    // Its ✕ takes Me out again.
    harness
        .get_by_role_and_label(Role::Button, t.person_filter_me())
        .click();
    harness.run();
    assert!(harness.state().people_filter().is_everyone());
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, t.person_filter_me())
            .next()
            .is_none()
    );
    Ok(())
}

#[test]
fn the_band_grows_with_the_text_size() -> TestResult {
    let measure = |size: TextSize| -> Result<(f32, f32, f32), Box<dyn std::error::Error>> {
        let (store, kitchen) = store(size)?;
        let harness = window(&store, Selection::Place(kitchen), egui::vec2(1280.0, 800.0));
        let (band, _) = band(&harness);
        let menu = button(&harness, t().person_filter());
        Ok((
            band.height(),
            title(&harness, "Kitchen").height(),
            menu.height(),
        ))
    };
    let (band, title, row) = measure(TextSize::Normal)?;
    let (grown_band, grown_title, grown_row) = measure(TextSize::Largest)?;
    let factor = TextSize::Largest.factor();
    assert!(grown_band > band * 1.15, "the band {band} → {grown_band}");
    assert!(
        (grown_title / title - factor).abs() < 0.1,
        "the title {title} → {grown_title}"
    );
    assert!(
        (grown_row / row - factor).abs() < 0.1,
        "the second row {row} → {grown_row}"
    );
    Ok(())
}

#[test]
fn the_narrowest_window_keeps_the_band_inside_at_the_largest_size() -> TestResult {
    let (store, kitchen) = store(TextSize::Largest)?;
    DeviceSettings::new(&store).set_fat_fingers(true)?;
    let harness = window(&store, Selection::Place(kitchen), MIN_WINDOW);
    let (band, lined) = band(&harness);
    assert!(lined);
    let width = harness.ctx.content_rect().right();
    assert!(
        band.right() <= width + 0.5,
        "the band {band:?} past {width}"
    );
    for r in row_two(&harness) {
        assert!(band.contains_rect(r), "{r:?} outside the band {band:?}");
    }
    Ok(())
}

#[test]
fn unsorted_all_notes_and_the_trash_have_the_band_too() -> TestResult {
    let (store, _) = store(TextSize::Normal)?;
    let t = t();
    for (selection, name) in [
        (Selection::Unsorted, t.unsorted()),
        (Selection::All, t.all_notes_title()),
        (Selection::Trash, t.trash_title()),
    ] {
        let harness = window(&store, selection.clone(), egui::vec2(1280.0, 800.0));
        let (band, lined) = band(&harness);
        assert!(lined, "{selection:?}: a line under the band");
        assert!(band.contains_rect(button(&harness, t.nav_back())));
        // On the band, or it panics.
        title(&harness, name);
        let filter = harness
            .query_all_by_role_and_label(Role::Button, t.person_filter())
            .next()
            .is_some();
        assert_eq!(filter, selection != Selection::Trash, "{selection:?}");
    }
    Ok(())
}

#[test]
fn the_second_row_stays_one_line_at_any_wide_enough_width() -> TestResult {
    let (store, kitchen) = store(TextSize::Normal)?;
    for width in [1000.0, 1037.0, 1111.0, 1199.0, 1283.0, 1421.0, 1777.0] {
        let harness = window(
            &store,
            Selection::Place(kitchen.clone()),
            egui::vec2(width, 800.0),
        );
        let row = row_two(&harness);
        let [field, ..] = row;
        for r in &row {
            assert!(
                (r.center().y - field.center().y).abs() <= 1.0,
                "at {width}: {r:?} not beside the field {field:?}"
            );
        }
    }
    Ok(())
}

/// Where the text shape reading `words` lies inside `within`, at which
/// size; `None` if none does.
fn text_in(harness: &Harness<'_, App>, words: &str, within: Rect) -> Option<(Rect, f32)> {
    drawn(harness).iter().find_map(|shape| match shape {
        Shape::Text(text) if text.galley.text() == words => {
            let size = text.galley.job.sections.first()?.format.font_id.size;
            let at = text.visual_bounding_rect();
            within.expand(1.0).contains_rect(at).then_some((at, size))
        }
        _ => None,
    })
}

#[test]
fn the_search_field_is_as_tall_as_a_button_its_text_a_cards() -> TestResult {
    for size in [TextSize::Normal, TextSize::Largest] {
        let (store, kitchen) = store(size)?;
        let mut harness = window(&store, Selection::Place(kitchen), egui::vec2(1280.0, 800.0));
        let field = rect(&harness, Role::TextInput, None);
        let menu = button(&harness, t().person_filter());
        assert!(
            (field.height() - menu.height()).abs() <= 2.5,
            "{size:?}: the field {field:?}, the button {menu:?}"
        );
        // A card's text size, grown as a card's.
        let card = 16.0 * size.factor();
        let whole = Rect::EVERYTHING;
        let (_, shown) = text_in(&harness, "Descale the kettle", whole)
            .unwrap_or_else(|| panic!("{size:?}: no card"));
        assert!((shown - card).abs() < 0.01, "{size:?}: the card at {shown}");
        let (_, hint) = text_in(&harness, t().search_topic_hint(), field)
            .unwrap_or_else(|| panic!("{size:?}: no hint in the field"));
        assert!((hint - card).abs() < 0.01, "{size:?}: the hint at {hint}");
        harness.get_by_role(Role::TextInput).click();
        harness.run_steps(4);
        harness.get_by(|n| n.is_focused()).type_text("kettle");
        harness.run_steps(20);
        let (_, typed) = text_in(&harness, "kettle", field)
            .unwrap_or_else(|| panic!("{size:?}: nothing typed in the field"));
        assert!((typed - card).abs() < 0.01, "{size:?}: typed at {typed}");
    }
    Ok(())
}

#[test]
fn the_magnifier_and_clear_lie_inside_the_search_field() -> TestResult {
    let (store, kitchen) = store(TextSize::Normal)?;
    let mut harness = window(&store, Selection::Place(kitchen), egui::vec2(1280.0, 800.0));
    let t = t();
    let field = rect(&harness, Role::TextInput, None);
    let glass = oldbrainfix_gui::icons::SEARCH.to_string();
    let (at, _) = text_in(&harness, &glass, field)
        .unwrap_or_else(|| panic!("no magnifier in the field {field:?}"));
    assert!(at.center().x < field.left() + 30.0, "the magnifier {at:?}");
    // No ✕ while nothing is typed.
    let none = harness
        .query_all_by_role_and_label(Role::Button, t.clear_search())
        .next()
        .is_none();
    assert!(none, "a ✕ in the empty field");
    harness.get_by_role(Role::TextInput).click();
    harness.run_steps(4);
    harness.get_by(|n| n.is_focused()).type_text("kettle");
    harness.run_steps(20);
    let typed = rect(&harness, Role::TextInput, None);
    assert_eq!(typed, field, "the field moved as it filled");
    let clear = button(&harness, t.clear_search());
    assert!(field.contains_rect(clear), "✕ {clear:?} outside {field:?}");
    assert!(clear.center().x > field.right() - 30.0, "✕ at {clear:?}");
    harness.get_by_label(t.clear_search()).click();
    harness.run_steps(20);
    assert_eq!(harness.state().query(), "", "the search cleared");
    let gone = harness
        .query_all_by_role_and_label(Role::Button, t.clear_search())
        .next()
        .is_none();
    assert!(gone, "✕ stays after clearing");
    Ok(())
}

/// Whether a frame is drawn around `button`: a filled or stroked
/// rectangle within a point or two of it.
fn framed(harness: &Harness<'_, App>, button: Rect) -> bool {
    drawn(harness).iter().any(|shape| match shape {
        Shape::Rect(r) => {
            let near = (r.rect.min - button.min).length() <= 2.0
                && (r.rect.max - button.max).length() <= 2.0;
            near && (r.fill.a() > 0 || r.stroke.width > 0.0)
        }
        _ => false,
    })
}

#[test]
fn view_and_new_note_are_icon_and_word_in_a_button_frame() -> TestResult {
    let t = t();
    for size in [TextSize::Normal, TextSize::Largest] {
        let (store, kitchen) = store(size)?;
        for (selection, add) in [
            (Selection::Place(kitchen.clone()), t.add_here()),
            (Selection::Unsorted, t.menu_new_note()),
        ] {
            let harness = window(&store, selection.clone(), egui::vec2(1280.0, 800.0));
            let menu = button(&harness, t.person_filter());
            assert!(framed(&harness, menu), "{size:?}: no frame to compare with");
            let view = button(&harness, t.show_notes_compact());
            let new = button(&harness, add);
            for (r, word) in [(view, t.menu_view()), (new, t.menu_new_note())] {
                let at = format!("{size:?} {selection:?} {word}");
                let (words, shown) =
                    text_in(&harness, word, r).unwrap_or_else(|| panic!("{at}: no words in {r:?}"));
                assert!(shown > 0.0);
                assert!(r.width() > r.height() + words.width(), "{at}: {r:?}");
                assert!((r.height() - menu.height()).abs() <= 1.0, "{at}: {r:?}");
                assert!(framed(&harness, r), "{at}: no frame around {r:?}");
                // The icon before the word, inside the button.
                let icon = drawn(&harness).iter().any(|shape| match shape {
                    Shape::Text(text) => {
                        let b = text.visual_bounding_rect();
                        r.expand(1.0).contains_rect(b) && b.right() <= words.left()
                    }
                    _ => false,
                });
                assert!(icon, "{at}: no icon before the word");
            }
        }
    }
    Ok(())
}
