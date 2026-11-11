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

    #[test]
    fn it_asks_for_the_next_frame_at_its_pace_not_at_once() {
        let ctx = egui::Context::default();
        let frame = || {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let bar = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 30.0));
                show(ui, bar, Color32::RED, "Syncing…");
            })
        };
        // egui's first pass always asks for another; the second is the line's own.
        frame();
        let output = frame();
        // egui waits the delay after drawing the frame, so it takes the
        // frame's own predicted time off what was asked for.
        let drawing = Duration::from_secs_f32(1.0 / 60.0);
        let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay + drawing;
        assert!(
            delay + Duration::from_millis(1) >= FRAME,
            "asked again after {delay:?}"
        );
        assert!(
            delay <= FRAME + Duration::from_millis(1),
            "asked again after {delay:?}"
        );
    }
}
