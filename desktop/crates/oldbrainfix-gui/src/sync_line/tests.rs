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
