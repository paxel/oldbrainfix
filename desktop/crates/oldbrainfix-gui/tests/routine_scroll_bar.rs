//! The Routine dialog's scroll bar (obf-1120), driven headless by
//! egui_kittest: the scroll area leaves the bar's width as room on the
//! right, so the bar, hovered and at its widest too, draws beside the
//! cards and never over their edge.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{ClockTime, Day, Notes, OwnProfile, Routines, Schedule, SigningKey, Store};
use oldbrainfix_gui::{App, Selection, theme};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The wall clock shows UTC.
fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

/// A store whose clock runs a second on with each read from 26 October
/// 2026, 09:00.
fn device() -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-10-26T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([1; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    Ok(store)
}

/// Every filled rect drawn in the cards' colour, in points.
fn cards(harness: &Harness<'_, App>) -> Vec<egui::Rect> {
    fn flat(shape: &egui::Shape, card: egui::Color32, out: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| flat(s, card, out)),
            egui::Shape::Rect(r) if r.fill == card => out.push(r.rect),
            _ => {}
        }
    }
    let card = theme::tones(&harness.ctx.global_style().visuals).card;
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        flat(&clipped.shape, card, &mut out);
    }
    out
}

#[test]
fn the_bar_draws_beside_the_cards_not_over_them() -> TestResult {
    let store = Arc::new(device()?);
    let note = Notes::new(&store).capture("Water the plants", None, None)?;
    let at = ClockTime::new(10, 0).ok_or("10:00")?;
    let rule = oldbrainfix_core::Recurrence::parse("FREQ=DAILY")?;
    let schedule = Schedule::new(Day::new(2026, 10, 26), &[at], Some(rule)).ok_or("schedule")?;
    let id = Routines::new(&store).spawn(&note.id, &schedule)?.id;
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    // Little in the window behind the dialog.
    app.select(Selection::Trash);
    // Low enough that the editor scrolls.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness.state_mut().open_routine(&id);
    harness.run();
    let scale = harness.ctx.pixels_per_point();
    // The bar's node covers it at its widest, as when hovered.
    let px = harness
        .query_all_by_role(Role::ScrollBar)
        .map(|n| n.rect())
        .max_by(|a, b| a.height().total_cmp(&b.height()))
        .ok_or("the dialog's scroll bar")?;
    let bar = egui::Rect::from_min_max(
        (px.min.to_vec2() / scale).to_pos2(),
        (px.max.to_vec2() / scale).to_pos2(),
    );
    let under: Vec<egui::Rect> = cards(&harness)
        .into_iter()
        // The dialog's cards: those of the window behind it reach past
        // its right edge.
        .filter(|card| {
            card.y_range().intersects(bar.y_range())
                && card.left() < bar.left()
                && card.right() <= bar.right() + 0.5
        })
        .collect();
    assert!(!under.is_empty(), "no card beside the bar {bar:?}");
    for card in under {
        assert!(
            card.right() <= bar.left() + 0.5,
            "the card {card:?} reaches under the bar {bar:?}"
        );
    }
    Ok(())
}
