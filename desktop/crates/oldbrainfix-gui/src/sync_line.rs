//! The line under the menu bar while a sync of this Device runs (#225,
//! UI laws "The sync line"), as the phone's under its top bar: a short
//! stretch in the accent moving back and forth along the bar's bottom
//! edge. It takes no room of its own, so nothing else in the window
//! moves. The desktop reads no "remove animations" setting yet, so it
//! always moves, as the question marks do. A screen reader finds it as
//! "Syncing…".

use std::time::Duration;

use egui::{Color32, Id, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2};

/// How thick the line is, in points.
pub const HEIGHT: f32 = 3.0;

/// How long the stretch takes from one side to the other, in seconds.
pub const SWEEP: f64 = 0.9;

/// How long the line waits before its next frame: about 30 a second,
/// smooth enough for the stretch, and a sync that runs for long keeps
/// the window from drawing as fast as it can (#292).
pub const FRAME: Duration = Duration::from_millis(33);

/// How long the line shows for one sync at most: a sync that hangs past
/// it keeps running, the line goes (#292).
pub const LIMIT: Duration = Duration::from_secs(120);

/// The stretch's share of the bar's width.
const SHARE: f32 = 0.25;

/// Where the stretch is at `time` seconds: 0 at the left, 1 at the
/// right, back and forth, easing at both ends.
pub fn place(time: f64) -> f32 {
    let phase = (time / SWEEP).rem_euclid(2.0);
    let there = if phase < 1.0 { phase } else { 2.0 - phase };
    // Ease in and out, as the phone's curve.
    (there * there * (3.0 - 2.0 * there)) as f32
}

/// Draws the line along the bottom of `bar`, named `label` for screen
/// readers, and keeps the window drawing at [`FRAME`]'s pace while it
/// shows.
pub fn show(ui: &Ui, bar: Rect, accent: Color32, label: &str) {
    let line = Rect::from_min_max(pos2(bar.left(), bar.bottom() - HEIGHT), bar.right_bottom());
    let width = line.width() * SHARE;
    let left = line.left() + (line.width() - width) * place(ui.input(|i| i.time));
    let stretch = Rect::from_min_max(pos2(left, line.top()), pos2(left + width, line.bottom()));
    ui.painter().rect_filled(stretch, 0.0, accent);
    let named = ui.interact(line, Id::new("sync line"), Sense::hover());
    named.widget_info(|| WidgetInfo::labeled(WidgetType::ProgressIndicator, true, label));
    ui.ctx().request_repaint_after(FRAME);
}

#[cfg(test)]
mod tests;
