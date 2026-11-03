//! The line under the menu bar while a sync of this Device runs (#225,
//! UI laws "The sync line"), as the phone's under its top bar: a short
//! stretch in the accent moving back and forth along the bar's bottom
//! edge. It takes no room of its own, so nothing else in the window
//! moves. The desktop reads no "remove animations" setting yet, so it
//! always moves, as the question marks do. A screen reader finds it as
//! "Syncing…".

use egui::{Color32, Id, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2};

/// How thick the line is, in points.
pub const HEIGHT: f32 = 3.0;

/// How long the stretch takes from one side to the other, in seconds.
pub const SWEEP: f64 = 0.9;

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
/// readers, and keeps the window drawing while it shows.
pub fn show(ui: &Ui, bar: Rect, accent: Color32, label: &str) {
    let line = Rect::from_min_max(pos2(bar.left(), bar.bottom() - HEIGHT), bar.right_bottom());
    let width = line.width() * SHARE;
    let left = line.left() + (line.width() - width) * place(ui.input(|i| i.time));
    let stretch = Rect::from_min_max(pos2(left, line.top()), pos2(left + width, line.bottom()));
    ui.painter().rect_filled(stretch, 0.0, accent);
    let named = ui.interact(line, Id::new("sync line"), Sense::hover());
    named.widget_info(|| WidgetInfo::labeled(WidgetType::ProgressIndicator, true, label));
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_goes_back_and_forth() {
        assert_eq!(place(0.0), 0.0);
        assert_eq!(place(SWEEP), 1.0);
        assert_eq!(place(2.0 * SWEEP), 0.0);
        assert!((place(SWEEP / 2.0) - 0.5).abs() < 1e-6);
        assert!(place(SWEEP * 1.25) < 1.0 && place(SWEEP * 1.25) > 0.5);
    }
}
