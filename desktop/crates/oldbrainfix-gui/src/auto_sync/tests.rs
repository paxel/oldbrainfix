use super::*;
use std::sync::mpsc::channel;

#[test]
fn the_line_shows_while_a_sync_runs_and_goes_past_its_limit() {
    let syncing = Syncing::default();
    assert!(!syncing.shows_line(Duration::from_secs(3600)));
    let (started, start) = channel();
    let (done, finish) = channel::<()>();
    std::thread::scope(|s| {
        s.spawn(|| {
            syncing.during(move || {
                started.send(()).ok();
                finish.recv().ok();
            })
        });
        start.recv().ok();
        assert!(syncing.running());
        assert!(syncing.shows_line(Duration::from_secs(3600)));
        // Past its limit the sync still runs, the line is gone.
        std::thread::sleep(Duration::from_millis(5));
        assert!(!syncing.shows_line(Duration::from_millis(1)));
        assert!(syncing.running());
        done.send(()).ok();
    });
    assert!(!syncing.running());
    assert!(!syncing.shows_line(Duration::from_secs(3600)));
}
