//! The Agenda read on a thread of its own with a connection of its own
//! (#207): the window draws on while it reads, the last rows showing,
//! and a read the log has changed since is dropped and read again.
//! Driven headless by egui_kittest over a store on disk.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::schedule::Recurrence;
use oldbrainfix_core::{ClockTime, Day, Notes, Occurrence, Routines, Schedule, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::app::local_wall;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn open(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

/// Today on the system's clock, as the window and its thread read it.
fn today() -> Day {
    Day::today(Utc::now(), &local_wall)
}

/// Every day at 20:00 since `days` ago.
fn daily(days: i64) -> Result<Schedule, Box<dyn std::error::Error>> {
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| e.source)?;
    let at = [ClockTime::new(20, 0).ok_or("a time")?];
    Ok(Schedule::new(today().add_days(-days), &at, Some(rule)).ok_or("a schedule")?)
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The thread's own connection, handed over once per read the test
/// lets go.
struct Gate {
    own: Arc<Store>,
    go: Sender<()>,
    gate: Arc<Mutex<Receiver<()>>>,
    /// Raised once a read let go holds the connection.
    taken: Arc<AtomicBool>,
}

impl Gate {
    fn new(own: Store) -> Gate {
        let (go, gate) = channel();
        Gate {
            own: Arc::new(own),
            go,
            gate: Arc::new(Mutex::new(gate)),
            taken: Arc::default(),
        }
    }

    /// Hands the Agenda's thread its connection, each read once it is
    /// let go.
    fn hand(&self, app: &mut App) {
        let (own, gate, taken) = (self.own.clone(), self.gate.clone(), self.taken.clone());
        app.set_agenda_store(move || {
            if let Ok(gate) = gate.lock() {
                let _ = gate.recv();
            }
            let store = own.clone();
            taken.store(true, Ordering::SeqCst);
            store
        });
    }

    /// Lets one read go and waits until it is done with the connection.
    fn let_go(&self) -> TestResult {
        self.taken.store(false, Ordering::SeqCst);
        self.go.send(())?;
        let started = Instant::now();
        // The thread holds a handle while it reads; the test and the
        // window's closure hold the other two.
        while !self.taken.load(Ordering::SeqCst) || Arc::strong_count(&self.own) > 2 {
            if started.elapsed() > Duration::from_secs(60) {
                return Err("the read never ended".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

/// Steps the window until `label` shows.
fn until_shown(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    for _ in 0..2000 {
        if shows(harness, label) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    Err(format!("{label:?} never showed").into())
}

#[test]
fn the_agenda_shows_its_last_rows_until_its_thread_read_is_in() -> TestResult {
    let db = tempfile::tempdir()?;
    let path = db.path().join("oldbrainfix.db");
    let shown = Arc::new(open(&path)?);
    let gate = Gate::new(open(&path)?);
    Routines::new(&shown).create("pills", &daily(3)?)?;
    let mut app = App::new(shown.clone(), "en");
    gate.hand(&mut app);
    let mut harness = window(app);
    assert!(shows(&harness, "pills"));

    // A change reads the Agenda again on its thread: the last rows show
    // meanwhile, frame after frame.
    Routines::new(&shown).create("tea", &daily(3)?)?;
    harness.state_mut().set_region(None);
    for _ in 0..3 {
        harness.step();
        assert!(shows(&harness, "pills"));
        assert!(!shows(&harness, "tea"));
    }
    gate.let_go()?;
    // The log changes before the window took the read: it is dropped,
    // and read again.
    Routines::new(&shown).create("water", &daily(3)?)?;
    for _ in 0..3 {
        harness.step();
        assert!(shows(&harness, "pills"));
        assert!(!shows(&harness, "tea"), "read before \"water\"");
    }
    gate.let_go()?;
    until_shown(&mut harness, "water")?;
    assert!(shows(&harness, "tea"));
    assert!(shows(&harness, "pills"));
    Ok(())
}

#[test]
fn the_agenda_read_on_its_thread_shows_what_one_here_does() -> TestResult {
    let db = tempfile::tempdir()?;
    let path = db.path().join("oldbrainfix.db");
    let shown = Arc::new(open(&path)?);
    let gate = Gate::new(open(&path)?);
    // Up before anything is due, so only the thread's read shows it.
    let mut app = App::new(shown.clone(), "en");
    gate.hand(&mut app);
    let mut there = window(app);
    let pills = Routines::new(&shown).create("pills", &daily(3)?)?;
    Routines::new(&shown).mark_done(
        &pills.id,
        &Occurrence::new(today(), ClockTime::new(20, 0)),
        None,
    )?;
    Notes::new(&shown).capture("nees mor sauce", None, None)?;
    Routines::new(&shown).create("tea", &daily(0)?)?;
    there.state_mut().set_region(None);
    gate.let_go()?;
    until_shown(&mut there, "tea")?;
    there.step();
    let here = window(App::new(shown.clone(), "en"));
    let labels = |harness: &Harness<'_, App>| -> Vec<String> {
        harness
            .query_all_by(|_| true)
            .filter_map(|n| n.accesskit_node().label())
            .collect()
    };
    assert!(shows(&there, "1 Unsorted"));
    assert!(shows(&there, "pills"));
    assert_eq!(labels(&there), labels(&here));
    Ok(())
}

#[test]
fn the_window_draws_on_while_its_thread_reads_a_long_agenda() -> TestResult {
    let db = tempfile::tempdir()?;
    let path = db.path().join("oldbrainfix.db");
    let shown = Arc::new(open(&path)?);
    // Months of use: daily Routines checked off most days.
    let schedule = daily(120)?;
    shown.batch(|| {
        let routines = Routines::new(&shown);
        for i in 0..20 {
            let routine = routines.create(&format!("Routine {i}"), &schedule)?;
            for k in 1..120 {
                if (k + i) % 5 != 0 {
                    let day = today().add_days(-k);
                    let occurrence = Occurrence::new(day, ClockTime::new(20, 0));
                    routines.mark_done(&routine.id, &occurrence, None)?;
                }
            }
        }
        Ok(())
    })?;
    // How long the window's own read takes, as a frame would wait.
    let mut here = App::new(shown.clone(), "en");
    let started = Instant::now();
    here.set_region(None);
    let read = started.elapsed();

    let gate = Gate::new(open(&path)?);
    let mut app = App::new(shown.clone(), "en");
    gate.hand(&mut app);
    let mut harness = window(app);
    Routines::new(&shown).create("tea", &daily(0)?)?;
    let mut longest = Duration::ZERO;
    let started = Instant::now();
    harness.state_mut().set_region(None);
    longest = longest.max(started.elapsed());
    gate.go.send(())?;
    let took = Instant::now();
    while !shows(&harness, "tea") {
        if took.elapsed() > Duration::from_secs(120) {
            return Err("the read never showed".into());
        }
        let frame = Instant::now();
        harness.step();
        longest = longest.max(frame.elapsed());
        std::thread::sleep(Duration::from_millis(2));
    }
    let took = took.elapsed();
    assert!(read > Duration::from_millis(50), "a long read: {read:?}");
    assert!(took > read / 2, "read there as long: {took:?} {read:?}");
    assert!(
        longest < read / 3,
        "no frame waits for it: {longest:?} of {read:?}"
    );
    Ok(())
}
