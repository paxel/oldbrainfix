//! An idle window asks for no frames it does not need (#292): a sync
//! running while the window is hidden into the tray draws nothing and asks
//! for nothing, and shown, its line asks at its own pace.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use egui_kittest::Harness;
use oldbrainfix_core::{OwnProfile, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::auto_sync::Syncing;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn device() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([7; 32])))?;
    OwnProfile::new(&store).apply_defaults("Device", None)?;
    Ok(Arc::new(store))
}

fn harness(app: App) -> Harness<'static, App> {
    Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        )
}

/// When the window asks for its next frame after a few steps of `app`.
fn next_frame(app: App) -> StdDuration {
    let mut harness = harness(app);
    harness.run_steps(4);
    harness.output().viewport_output[&egui::ViewportId::ROOT].repaint_delay
}

/// Runs `look` while a sync of this Device runs.
fn while_syncing<T>(syncing: &Syncing, look: impl FnOnce() -> T) -> T {
    let (started, start) = channel();
    let (done, finish) = channel::<()>();
    std::thread::scope(|s| {
        s.spawn(move || {
            syncing.during(|| {
                started.send(()).ok();
                finish.recv().ok();
            })
        });
        start.recv().ok();
        let seen = look();
        done.send(()).ok();
        seen
    })
}

#[test]
fn a_sync_in_a_hidden_window_asks_for_no_frames() -> TestResult {
    let store = device()?;
    let syncing = Syncing::default();
    let mut app = App::new(store.clone(), "en");
    app.set_syncing(syncing.clone());
    app.set_shown(false);
    let delay = while_syncing(&syncing, || next_frame(app));
    assert!(
        delay > StdDuration::from_secs(1),
        "a hidden window asks again in {delay:?}"
    );
    Ok(())
}

#[test]
fn a_sync_in_a_shown_window_draws_its_line() -> TestResult {
    let store = device()?;
    let syncing = Syncing::default();
    let mut app = App::new(store.clone(), "en");
    app.set_syncing(syncing.clone());
    // The line's own pace is tested in sync_line.rs; the harness predicts
    // a long frame, under which any short pace reads as none.
    let causes = while_syncing(&syncing, || {
        let mut harness = harness(app);
        harness.run_steps(4);
        harness.ctx.repaint_causes()
    });
    let named: Vec<String> = causes.iter().map(|c| format!("{c:?}")).collect();
    assert!(
        named.iter().any(|c| c.contains("sync_line.rs")),
        "the line asks for its frames: {named:?}"
    );
    Ok(())
}
