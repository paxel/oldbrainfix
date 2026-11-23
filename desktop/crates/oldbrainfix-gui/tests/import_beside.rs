//! An Import beside the window (obf-1061): it runs on a thread of its
//! own with a connection of its own, the line under the menu bar moving;
//! the Import dialog counts its Notes as they come in, may be closed and
//! opened again while it runs, and at the end says what went where, with
//! the way into the new Area. The window draws and reads on meanwhile.
//! Driven headless by egui_kittest over a store on disk, the Import's
//! connection held at a Note by its clock.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::Duration;

use chrono::Utc;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn open(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

fn step_until(
    harness: &mut Harness<'_, App>,
    done: impl Fn(&Harness<'_, App>) -> bool,
) -> TestResult {
    for _ in 0..400 {
        if done(harness) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

/// The Import's own connection: it waits for the test to let it on
/// before it opens, and again each time its clock is asked, which every
/// entry written does, until the test lets go of it.
struct Held {
    on: Option<SyncSender<()>>,
    /// The threads the connection was opened on, one per Import.
    opened: Arc<Mutex<Vec<ThreadId>>>,
}

impl Held {
    /// The window over the database at `path`, each Import on a
    /// connection of its own whose clock `Held` holds.
    fn window(
        path: &Path,
        folder: std::path::PathBuf,
    ) -> Result<(Held, Harness<'static, App>), Box<dyn std::error::Error>> {
        let (on, waits) = sync_channel::<()>(0);
        let waits = Arc::new(Mutex::new(waits));
        let opened = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new(open(path)?, "en");
        app.set_import_picker(Box::new(move |_| Some(folder.clone())));
        // Opened here, its clock waiting only once the Import has it.
        let armed = Arc::new(AtomicBool::new(false));
        let (ticks, gate) = (armed.clone(), waits.clone());
        let clock = move || {
            // Let go of: the clock waits no more.
            if ticks.load(Ordering::SeqCst)
                && let Ok(gate) = gate.lock()
            {
                let _ = gate.recv();
            }
            Utc::now()
        };
        let own = Arc::new(Store::open(path, Box::new(clock), || {
            Ok(SigningKey::from_seed([3; 32]))
        })?);
        let seen = opened.clone();
        app.set_import_store(move || {
            if let Ok(mut seen) = seen.lock() {
                seen.push(std::thread::current().id());
            }
            let waits: Arc<Mutex<Receiver<()>>> = waits.clone();
            if let Ok(waits) = waits.lock() {
                let _ = waits.recv();
            }
            armed.store(true, Ordering::SeqCst);
            own.clone()
        });
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1280.0, 1600.0))
            .build_ui_state(
                |ui, app: &mut App| {
                    App::install(ui.ctx());
                    app.show(ui);
                },
                app,
            );
        harness.step();
        Ok((
            Held {
                on: Some(on),
                opened,
            },
            harness,
        ))
    }

    /// Lets the Import on one entry at a time until the window shows
    /// `label`; there it waits again.
    fn until(&self, harness: &mut Harness<'_, App>, label: &str) -> TestResult {
        let on = self.on.as_ref().ok_or("let go already")?;
        for _ in 0..2000 {
            harness.step();
            if shows(harness, label) {
                return Ok(());
            }
            // Back once the Import asked its clock the next time.
            on.send(())?;
        }
        Err(format!("the window never showed {label}").into())
    }

    /// Lets go of the Import's clock.
    fn let_go(&mut self) {
        self.on = None;
    }
}

/// Opens File ▸ Import….
fn import_dialog(harness: &mut Harness<'_, App>) {
    harness.get_by_label("File").click();
    harness.step();
    harness.get_by_label("Import…").click();
    harness.step();
    harness.step();
}

#[test]
fn an_import_runs_beside_the_window_counts_its_notes_and_opens_its_area() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Notes");
    std::fs::create_dir_all(&dir)?;
    for name in ["Dentist", "Garden", "Rome"] {
        std::fs::write(dir.join(format!("{name}.txt")), format!("{name} text"))?;
    }
    let path = root.path().join("log.db");
    let (mut held, mut harness) = Held::window(&path, dir)?;

    import_dialog(&mut harness);
    harness.get_by_label("A folder…").click_accesskit();
    harness.step();
    harness.step();
    // Held before its connection opens: it reads.
    assert!(harness.state().importing());
    assert!(shows(&harness, "Reading the folder…"));
    assert!(shows(&harness, "You can close this dialog."));
    assert!(
        harness.query_by_label("Importing…").is_some(),
        "the line moves"
    );
    let opened = held.opened.lock().map_err(|_| "opened")?.clone();
    assert_eq!(opened.len(), 1, "one connection of its own");
    assert_ne!(
        opened[0],
        std::thread::current().id(),
        "on a thread of its own"
    );

    // The dialog left, the window draws and reads on meanwhile.
    harness.get_by_label("Close").click_accesskit();
    harness.step();
    harness.step();
    assert!(!shows(&harness, "Reading the folder…"));
    harness.state_mut().select(Selection::All);
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert!(shows(&harness, "All Notes"));
    assert!(harness.state().importing(), "still held");
    assert!(harness.query_by_label("Importing…").is_some());
    assert_eq!(harness.state().notice(), None);

    // Opened again, it counts its Notes as they come in.
    import_dialog(&mut harness);
    assert!(shows(&harness, "Reading the folder…"));
    // Held inside a Note's transaction from here, which a window's
    // write, as the search's catching up on a read, waits for.
    held.until(&mut harness, "1 of 3 Notes")?;
    assert!(shows(&harness, "You can close this dialog."));
    held.until(&mut harness, "2 of 3 Notes")?;

    held.let_go();
    step_until(&mut harness, |h| h.state().notice().is_some())?;
    harness.step();
    harness.step();
    assert_eq!(harness.state().notice(), Some("Import finished."));
    assert!(!harness.state().importing());
    assert!(
        harness.query_by_label("Importing…").is_none(),
        "gone with it"
    );
    // The window's own connection sees what the Import's wrote.
    let desk = harness.state().store();
    let area = Places::new(desk)
        .areas()?
        .into_iter()
        .find(|a| a.name == "From Notes")
        .ok_or("the new Area")?;
    assert_eq!(Notes::new(desk).inside(&area.id)?.len(), 3);
    // The dialog says it in its place, with the way into the Area.
    assert!(shows(
        &harness,
        "3 Notes went into the new Area From Notes."
    ));
    harness.get_by_label("Open the Area").click_accesskit();
    harness.step();
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Place(area.id));
    assert!(!shows(&harness, "went into the new Area"));
    Ok(())
}

#[test]
fn an_import_left_alone_is_announced_at_the_top() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Notes");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("Dentist.txt"), "Tuesday")?;
    let path = root.path().join("log.db");
    let (mut held, mut harness) = Held::window(&path, dir)?;
    import_dialog(&mut harness);
    harness.get_by_label("A folder…").click_accesskit();
    harness.step();
    held.until(&mut harness, "0 of 1 Note")?;
    harness.get_by_label("Close").click_accesskit();
    harness.step();
    held.let_go();
    step_until(&mut harness, |h| h.state().notice().is_some())?;
    harness.step();
    // Only that it finished; the click shows what went where.
    assert_eq!(harness.state().notice(), Some("Import finished."));
    assert!(!shows(&harness, "went into the new Area"));
    harness.get_by_label("Import finished.").click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "1 Note went into the new Area From Notes."));
    Ok(())
}

/// The count the Import dialog shows, "12 of 600 Notes" as 12.
fn count_shown(harness: &Harness<'_, App>, of: &str) -> Option<usize> {
    harness.query_all_by_label_contains(of).find_map(|n| {
        let node = n.accesskit_node();
        let text = node.label().or_else(|| node.value())?;
        text.strip_suffix(of)?.trim().parse().ok()
    })
}

#[test]
fn the_window_reads_and_counts_on_while_an_import_writes() -> TestResult {
    let root = tempfile::tempdir()?;
    let dir = root.path().join("Notes");
    std::fs::create_dir_all(&dir)?;
    for n in 0..600 {
        std::fs::write(dir.join(format!("Note {n}.txt")), format!("Text {n}"))?;
    }
    let path = root.path().join("log.db");
    let mut app = App::new(open(&path)?, "en");
    app.set_import_picker(Box::new(move |_| Some(dir.clone())));
    let own = Arc::new(open(&path)?);
    app.set_import_store(move || own.clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    import_dialog(&mut harness);
    harness.get_by_label("A folder…").click_accesskit();
    harness.step();
    // Each frame reads the list anew, which catches the search up on
    // the window's own connection: a write, which the Import's Notes,
    // a transaction each with a pause after it, leave room for.
    let mut counts = Vec::new();
    for n in 0..2000 {
        if !harness.state().importing() {
            break;
        }
        let place = if n % 2 == 0 {
            Selection::All
        } else {
            Selection::Agenda
        };
        harness.state_mut().select(place);
        harness.step();
        assert_eq!(harness.state().error(), None);
        if let Some(count) = count_shown(&harness, " of 600 Notes")
            && counts.last() != Some(&count)
        {
            counts.push(count);
        }
    }
    assert!(!harness.state().importing(), "it ended");
    assert!(
        counts.len() >= 5 && counts.windows(2).all(|w| w[0] < w[1]),
        "the window drew on, counting: {counts:?}"
    );
    Ok(())
}
