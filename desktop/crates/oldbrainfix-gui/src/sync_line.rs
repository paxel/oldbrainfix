//! The line under the menu bar while a sync of this Device runs (#225,
//! UI laws "The sync line"), as the phone's under its top bar: a short
//! stretch in the accent moving back and forth along the bar's bottom
//! edge. It takes no room of its own, so nothing else in the window
//! moves. The desktop reads no "remove animations" setting yet, so it
//! always moves, as the question marks do. A screen reader finds it as
//! "Syncing…". A click on it opens the page Running (obf-1078); it takes
//! the click a little above and below itself, a 3 point line being hard
//! to hit, but not on the menus above it.

use std::time::Duration;

use egui::{Color32, CursorIcon, Id, Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2};

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

/// How far above and below the line a click still takes it, in points:
/// above only within the bar's own margin under its menus.
pub const REACH_ABOVE: f32 = 1.0;
pub const REACH_BELOW: f32 = 5.0;

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
/// shows; its response says when it was clicked.
pub fn show(ui: &Ui, bar: Rect, accent: Color32, label: &str) -> Response {
    let line = Rect::from_min_max(pos2(bar.left(), bar.bottom() - HEIGHT), bar.right_bottom());
    let width = line.width() * SHARE;
    let left = line.left() + (line.width() - width) * place(ui.input(|i| i.time));
    let stretch = Rect::from_min_max(pos2(left, line.top()), pos2(left + width, line.bottom()));
    ui.painter().rect_filled(stretch, 0.0, accent);
    // The pointer's reach first, so the line itself, named, lies over it.
    let reach = Rect::from_min_max(
        pos2(line.left(), line.top() - REACH_ABOVE),
        pos2(line.right(), line.bottom() + REACH_BELOW),
    );
    // Clicked, not a stop for Tab, which View ▸ Running stands in for: the
    // line comes and goes with the jobs.
    let around = ui.interact(reach, Id::new("sync line reach"), Sense::CLICK);
    let named = ui.interact(line, Id::new("sync line"), Sense::CLICK);
    named.widget_info(|| WidgetInfo::labeled(WidgetType::ProgressIndicator, true, label));
    ui.ctx().request_repaint_after(FRAME);
    named
        .union(around)
        .on_hover_cursor(CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests;
