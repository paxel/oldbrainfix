//! The desktop launcher: opens the store, starts the alarm layer, the
//! tray, the capture hotkey and the login start, and opens the native
//! window, which hands every frame to `oldbrainfix_gui::App`. Closing the
//! window leaves the app running in the tray, where Open brings the
//! window back; without a tray, closing it quits. `--tray` (the login
//! start) starts without the window. A second start hands its wish to the
//! running app and ends.
//!
//! The hotkey, the tray's Capture and `oldbrainfix capture` open the
//! capture popup: over the window while it is open, else on its own.
//! `oldbrainfix capture TEXT` keeps TEXT without any window, through the
//! running app when there is one, and `oldbrainfix import FILE` imports
//! the sync bundle FILE the same way. `oldbrainfix open FILE` shows the
//! Sent Item FILE in the window, the running app's or a new one. On
//! Windows it is built without a console window.
//!
//! Transcripts and Photo Texts are made on threads of their own for as
//! long as the app runs, the Transcripts once the speech model in the
//! data directory's `whisper` folder is checked. So are the alerts, and
//! the host paired Devices sync with on their own and the shared folder's
//! sync; after such a sync the alerts plan again and the window reads the
//! log again.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::cell::{Cell, RefCell};
use std::io::Read;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use oldbrainfix_core::{APP_ID, APP_NAME, DeviceSettings, Notes, SigningKey, Store};
use oldbrainfix_gui::alerts::{self, Alerts, Event};
use oldbrainfix_gui::auto_sync;
use oldbrainfix_gui::autostart::Autostart;
use oldbrainfix_gui::backups;
use oldbrainfix_gui::cli::{self, Launch};
use oldbrainfix_gui::enrich::{
    Enricher, Kind, OcrEngine, PromptsNow, WhisperEngine, transcript_prompts,
};
use oldbrainfix_gui::folder_sync;
use oldbrainfix_gui::hotkey::{self, Hotkey};
use oldbrainfix_gui::l10n::{L10n, locale_for_system};
use oldbrainfix_gui::login;
use oldbrainfix_gui::media::{MicRecorder, SpeakerPlayer, SystemClipboard};
use oldbrainfix_gui::notify::DesktopNotifier;
use oldbrainfix_gui::popup::{self, CapturePopup, Outcome};
use oldbrainfix_gui::ring::SpeakerRinger;
use oldbrainfix_gui::speech::SpeechModel;
use oldbrainfix_gui::sync_view::{device_name, import_bundle_text};
use oldbrainfix_gui::tray::{self, Command};
use oldbrainfix_gui::{App, instance, local_wall};

/// The window's context while it is open, to wake it from other threads.
type Window = Arc<Mutex<Option<egui::Context>>>;

/// Told after every Note kept outside the window: the window reads the
/// log again, the alerts plan its Nudge.
type Saved = Arc<dyn Fn() + Send + Sync>;

/// Makes a fresh capture popup.
type NewPopup = Rc<dyn Fn() -> CapturePopup>;

fn wake(window: &Window) {
    if let Ok(held) = window.lock()
        && let Some(ctx) = held.as_ref()
    {
        ctx.request_repaint();
    }
}

fn waiting(commands: &Mutex<Receiver<Command>>) -> Vec<Command> {
    match commands.lock() {
        Ok(rx) => rx.try_iter().collect(),
        Err(_) => Vec::new(),
    }
}

/// The capture popup over the window.
fn popup_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("capture popup")
}

struct Native {
    app: App,
    commands: Arc<Mutex<Receiver<Command>>>,
    quit: Arc<AtomicBool>,
    /// Whether closing the window leaves the app in the tray.
    tray: bool,
    new_popup: NewPopup,
    popup: Option<CapturePopup>,
}

impl eframe::App for Native {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        App::install(ui.ctx());
        for command in waiting(&self.commands) {
            match command {
                Command::Open => focus(ui.ctx()),
                // The window stays where it is, so the keyboard goes back
                // to whatever had it once the popup closes.
                Command::Capture => match &mut self.popup {
                    Some(popup) => popup.refocus(),
                    None => self.popup = Some((self.new_popup)()),
                },
                Command::Quit => {
                    self.quit.store(true, Ordering::SeqCst);
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if !self.tray && ui.input(|i| i.viewport().close_requested()) {
            self.quit.store(true, Ordering::SeqCst);
        }
        self.app.show(ui);
        if let Some(popup) = &mut self.popup {
            let builder = popup.viewport();
            let outcome = ui
                .ctx()
                .show_viewport_immediate(popup_id(), builder, |ui, _| popup.show(ui));
            if outcome == Outcome::Closed {
                self.popup = None;
            }
        }
    }

    fn on_exit(&mut self) {
        if let Some(popup) = &mut self.popup {
            popup.leave();
        }
        self.app.flush();
    }
}

/// The capture popup on its own, while the window is closed.
struct PopupWindow {
    popup: CapturePopup,
    commands: Arc<Mutex<Receiver<Command>>>,
    /// What the app does once the popup is gone: open the window, quit.
    next: Rc<Cell<Option<Command>>>,
}

impl eframe::App for PopupWindow {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        App::install(ui.ctx());
        let mut outcome = Outcome::Open;
        for command in waiting(&self.commands) {
            match command {
                Command::Capture => self.popup.refocus(),
                Command::Open | Command::Quit => {
                    outcome = self.popup.leave();
                    self.next.set(Some(command));
                }
            }
        }
        if outcome == Outcome::Open {
            outcome = self.popup.show(ui);
        }
        if outcome == Outcome::Closed {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self) {
        self.popup.leave();
    }
}

fn focus(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}

/// Everything the windows share while the app runs.
struct Shell {
    store: Arc<Store>,
    locale: &'static str,
    changes: Arc<AtomicU64>,
    events: Sender<Event>,
    commands: Arc<Mutex<Receiver<Command>>>,
    window: Window,
    autostart: Option<Autostart>,
    hotkey: Rc<RefCell<Hotkey>>,
    new_popup: NewPopup,
    transcriber: Enricher,
    reader: Enricher,
    speech: SpeechModel,
    folder_wanted: Arc<AtomicBool>,
    /// Raised by a click on the Changes Nudge, for the window to save the
    /// bundles.
    changes_wanted: Arc<AtomicBool>,
    /// Files for the window to open.
    handed: Arc<Mutex<Vec<std::path::PathBuf>>>,
}

impl Shell {
    /// Runs `app` in a native window with `viewport` until it closes.
    fn run(
        &self,
        viewport: egui::ViewportBuilder,
        app: Box<dyn eframe::App + '_>,
    ) -> Result<(), String> {
        let options = eframe::NativeOptions {
            viewport,
            ..Default::default()
        };
        let window = self.window.clone();
        let ran = eframe::run_native(
            APP_NAME,
            options,
            Box::new(move |cc| {
                if let Ok(mut held) = window.lock() {
                    *held = Some(cc.egui_ctx.clone());
                }
                Ok(app)
            }),
        );
        if let Ok(mut held) = self.window.lock() {
            *held = None;
        }
        ran.map_err(|e| e.to_string())
    }

    /// Opens the window and runs it until it closes; whether the app is
    /// to quit.
    fn window(&self, tray: bool) -> Result<bool, String> {
        let mut app = App::new(self.store.clone(), self.locale);
        app.watch_changes(self.changes.clone());
        app.set_wake(self.events.clone());
        app.set_folder_wanted(self.folder_wanted.clone());
        app.set_changes_wanted(self.changes_wanted.clone());
        app.set_handed_files(self.handed.clone());
        app.set_autostart(self.autostart.clone());
        // Nothing written here yet: a Backup may be restored first.
        app.offer_restore();
        let hotkey: Rc<RefCell<dyn hotkey::Control>> = self.hotkey.clone();
        app.set_hotkey(Some(hotkey));
        app.set_recorder(Box::new(MicRecorder::default()));
        app.set_player(Box::new(SpeakerPlayer::default()));
        app.set_clipboard(Box::new(SystemClipboard::default()));
        app.set_enrichers(
            Some(self.transcriber.clone()),
            Some(self.reader.clone()),
            Some(self.speech.clone()),
        );
        let quit = Arc::new(AtomicBool::new(false));
        let viewport = egui::ViewportBuilder::default()
            .with_title(app.t().app_title())
            // The app id a launcher entry's StartupWMClass names.
            .with_app_id(APP_ID)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 480.0]);
        let native = Native {
            app,
            commands: self.commands.clone(),
            quit: quit.clone(),
            tray,
            new_popup: self.new_popup.clone(),
            popup: None,
        };
        self.run(viewport, Box::new(native))?;
        Ok(quit.load(Ordering::SeqCst) || !tray)
    }

    /// Opens the capture popup on its own and runs it until it closes;
    /// what was asked of the app meanwhile.
    fn popup(&self) -> Result<Option<Command>, String> {
        let popup = (self.new_popup)();
        let next = Rc::new(Cell::new(None));
        let viewport = popup.viewport();
        let window = PopupWindow {
            popup,
            commands: self.commands.clone(),
            next: next.clone(),
        };
        self.run(viewport, Box::new(window))?;
        Ok(next.get())
    }
}

/// Opens the store in `dir`.
fn open_store(dir: &Path) -> Result<Store, String> {
    Store::open(
        &dir.join("oldbrainfix.db"),
        Box::new(chrono::Utc::now),
        SigningKey::generate,
    )
    .map_err(|e| e.to_string())
}

/// `oldbrainfix capture TEXT`: keeps `text` through the running app, or
/// on its own when none runs; the exit code.
fn capture_text(dir: &Path, socket: &Path, t: &L10n, text: &str) -> i32 {
    if cli::note_text(text).is_none() {
        eprintln!("oldbrainfix: {}", t.cli_nothing());
        return cli::USAGE;
    }
    let kept = instance::hand_over_text(socket, text).unwrap_or_else(|| {
        let store = open_store(dir)?;
        popup::capture(&store, text, Notes::default_nudge())
            .map(drop)
            .map_err(|e| e.to_string())
    });
    match kept {
        Ok(()) => cli::OK,
        Err(e) => {
            eprintln!("oldbrainfix: {}", t.cli_failed(&e));
            cli::FAILED
        }
    }
}

/// `oldbrainfix import FILE`: imports the bundle through the running
/// app, or on its own when none runs, and prints what came in; the exit
/// code.
fn import_file(dir: &Path, socket: &Path, t: &L10n, file: &str) -> i32 {
    // The running app has a working directory of its own.
    let path = std::path::absolute(file).unwrap_or_else(|_| file.into());
    let done = instance::hand_over_import(socket, &path).unwrap_or_else(|| {
        let store = open_store(dir)?;
        import_bundle_text(t, &store, &path)
    });
    match done {
        Ok(said) => {
            println!("{said}");
            cli::OK
        }
        Err(e) => {
            eprintln!("oldbrainfix: {e}");
            cli::FAILED
        }
    }
}

fn run() -> Result<i32, String> {
    let locale = locale_for_system(sys_locale::get_locale().as_deref());
    let t = L10n::new(locale);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Ok(launch) = cli::parse(&args) else {
        eprintln!("{}", t.cli_usage());
        return Ok(cli::USAGE);
    };
    if launch == Launch::Help {
        println!("{}", t.cli_usage());
        return Ok(cli::OK);
    }
    let dir = oldbrainfix_gui::data_dir().ok_or("no data directory for this user")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let socket = dir.join("instance.sock");
    // Files for the window to open: from `open`, and later starts.
    let handed: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::default();
    let first = match launch {
        Launch::Text(text) => return Ok(capture_text(&dir, &socket, &t, &text)),
        Launch::Stdin => {
            let mut text = String::new();
            if let Err(e) = std::io::stdin().read_to_string(&mut text) {
                eprintln!("oldbrainfix: {}", t.cli_failed(&e.to_string()));
                return Ok(cli::FAILED);
            }
            return Ok(capture_text(&dir, &socket, &t, &text));
        }
        Launch::Import(file) => return Ok(import_file(&dir, &socket, &t, &file)),
        Launch::Open(file) => {
            // The running app has a working directory of its own.
            let path = std::path::absolute(&file).unwrap_or_else(|_| file.into());
            if instance::hand_over_file(&socket, &path) {
                return Ok(cli::OK);
            }
            if let Ok(mut held) = handed.lock() {
                held.push(path);
            }
            Some(Command::Open)
        }
        Launch::Window | Launch::Help => Some(Command::Open),
        Launch::Popup => Some(Command::Capture),
        // A login start adds nothing to a running app.
        Launch::Tray => None,
    };
    // Already running: the window or the popup opens there.
    if instance::hand_over(&socket, first) {
        return Ok(cli::OK);
    }
    // The menu entry and icon the window's app id names, kept current.
    #[cfg(target_os = "linux")]
    std::thread::spawn(oldbrainfix_gui::launcher::register);
    let store = Arc::new(open_store(&dir)?);
    // The own Profile from the login's full name and account picture,
    // asking nothing; the window opens without it if that fails. On the
    // first start only once its view is answered (ADR 0017).
    if let Err(e) = login::apply_at_launch(&store) {
        eprintln!("oldbrainfix: {e}");
    }

    // On by default: set up unless switched off, so a moved program is
    // found again.
    let autostart = Autostart::for_this_user();
    if let Some(start) = &autostart {
        let on = DeviceSettings::new(&store).autostart_on().unwrap_or(true);
        if let Err(e) = start.set(on) {
            eprintln!("oldbrainfix: {} ({e})", t.autostart_failed());
        }
    }

    let window: Window = Arc::new(Mutex::new(None));
    let (command_tx, command_rx) = std::sync::mpsc::channel::<Command>();
    let commands = Arc::new(Mutex::new(command_rx));
    let waker = |window: &Window| -> Box<dyn Fn() + Send> {
        let window = window.clone();
        Box::new(move || wake(&window))
    };

    // The alarm layer, on its own thread for as long as the app runs.
    let (events_tx, events_rx) = std::sync::mpsc::channel::<Event>();
    let changes = Arc::new(AtomicU64::new(0));
    let changes_wanted = Arc::new(AtomicBool::new(false));
    {
        let store = store.clone();
        let notifier = DesktopNotifier::new(events_tx.clone());
        let mut alerts = Alerts::new(Box::new(notifier), t, changes.clone());
        alerts.set_changes_wanted(changes_wanted.clone());
        alerts.set_ringer(Box::new(SpeakerRinger::new()));
        let open = command_tx.clone();
        let wake_window = waker(&window);
        std::thread::spawn(move || {
            alerts::run(
                &store,
                &local_wall,
                &mut alerts,
                &events_rx,
                |open_window| {
                    if open_window {
                        let _ = open.send(Command::Open);
                    }
                    // The window reads a Done or a snooze at once.
                    wake_window();
                },
            );
        });
    }
    // Paired Devices sync on their own while the app runs, in the tray
    // too: the alerts plan again after it (a Done elsewhere closes the
    // notification here), the window reads the log again.
    let auto_stop = Arc::new(AtomicBool::new(false));
    let auto_sync = {
        let (store, changes, events) = (store.clone(), changes.clone(), events_tx.clone());
        let (stop, wake_window) = (auto_stop.clone(), waker(&window));
        std::thread::spawn(move || {
            auto_sync::run(&store, &device_name(), &stop, |session| {
                if !session.applied.is_empty() || session.profile.changed() {
                    changes.fetch_add(1, Ordering::SeqCst);
                    let _ = events.send(Event::Wake);
                }
                wake_window();
            });
        })
    };
    // The shared folder syncs every half minute while the app runs, at
    // once when the Sync view asks; after a round the alerts plan again
    // and the window reads the log and the status again.
    let folder_wanted = Arc::new(AtomicBool::new(false));
    let folder_sync = {
        let (store, changes, events) = (store.clone(), changes.clone(), events_tx.clone());
        let (stop, wanted, wake_window) =
            (auto_stop.clone(), folder_wanted.clone(), waker(&window));
        std::thread::spawn(move || {
            folder_sync::run(&store, &stop, &wanted, |result| {
                // Photos and recordings come on their own too, after
                // the entries naming them.
                if !result.applied.is_empty() || result.blobs_in > 0 || result.profile_changed {
                    changes.fetch_add(1, Ordering::SeqCst);
                    let _ = events.send(Event::Wake);
                }
                wake_window();
            });
        })
    };
    // The weekly Backup while the app runs; the window reads its status
    // again after each.
    let backup_thread = {
        let store = store.clone();
        let (stop, wake_window) = (auto_stop.clone(), waker(&window));
        std::thread::spawn(move || backups::run(&store, &stop, wake_window))
    };
    // Transcripts and Photo Texts, each kept one tells the window.
    let enriched = {
        let (changes, window) = (changes.clone(), window.clone());
        move || -> Box<dyn Fn() + Send> {
            let (changes, window) = (changes.clone(), window.clone());
            Box::new(move || {
                changes.fetch_add(1, Ordering::SeqCst);
                wake(&window);
            })
        }
    };
    let speech = {
        let window = window.clone();
        SpeechModel::new(dir.join("whisper"), Arc::new(move || wake(&window)))
    };
    let prompts: PromptsNow = {
        let store = store.clone();
        Box::new(move || transcript_prompts(&store, &L10n::new(locale)))
    };
    let transcriber = Enricher::spawn(
        store.clone(),
        Kind::Transcripts,
        Box::new(WhisperEngine::new(speech.model(), prompts)),
        enriched(),
    );
    let reader = Enricher::spawn(
        store.clone(),
        Kind::PhotoTexts,
        Box::new(OcrEngine::default()),
        enriched(),
    );
    let saved: Saved = {
        let (changes, events, window) = (changes.clone(), events_tx.clone(), window.clone());
        let (transcriber, reader, speech) = (transcriber.clone(), reader.clone(), speech.clone());
        Arc::new(move || {
            changes.fetch_add(1, Ordering::SeqCst);
            let _ = events.send(Event::Wake);
            wake(&window);
            // A voice Note from the popup is transcribed without the window.
            reader.kick();
            if speech.current().is_some() {
                transcriber.kick();
            }
        })
    };
    let new_popup: NewPopup = {
        let (store, saved) = (store.clone(), saved.clone());
        let (transcriber, speech) = (transcriber.clone(), speech.clone());
        Rc::new(move || {
            let mut popup = CapturePopup::new(store.clone(), locale);
            popup.set_recorder(Box::new(MicRecorder::default()));
            popup.set_clipboard(Box::new(SystemClipboard::default()));
            popup.set_transcriber(transcriber.clone(), Some(speech.clone()));
            let saved = saved.clone();
            popup.on_saved(Box::new(move || saved()));
            popup
        })
    };

    let capture: instance::Capture = {
        let (store, saved) = (store.clone(), saved.clone());
        Box::new(move |text| {
            popup::capture(&store, text, Notes::default_nudge())
                .map_err(|e| e.to_string())?
                .ok_or_else(|| t.cli_nothing().to_string())?;
            saved();
            Ok(())
        })
    };
    let import: instance::Import = {
        let (store, saved) = (store.clone(), saved.clone());
        Box::new(move |path| {
            let said = import_bundle_text(&t, &store, path);
            saved();
            said
        })
    };
    let open_file: instance::OpenFile = {
        let handed = handed.clone();
        Box::new(move |path| {
            if let Ok(mut held) = handed.lock() {
                held.push(path.to_path_buf());
            }
        })
    };
    if let Err(e) = instance::listen(
        &socket,
        command_tx.clone(),
        waker(&window),
        (capture, import, open_file),
    ) {
        eprintln!("oldbrainfix: {e}");
    }
    let labels = tray::Labels {
        title: t.app_title().into(),
        open: t.tray_open().into(),
        capture: t.capture_tooltip().into(),
        quit: t.tray_quit().into(),
    };
    let tray = tray::spawn(labels, command_tx.clone(), waker(&window));

    // On the main thread, where Windows and macOS hear the key.
    let chord = DeviceSettings::new(&store)
        .capture_hotkey()
        .ok()
        .and_then(|c| c.parse().ok());
    let pressed = {
        let (commands, window) = (command_tx, window.clone());
        Box::new(move || {
            let _ = commands.send(Command::Capture);
            wake(&window);
        })
    };
    let hotkey = Rc::new(RefCell::new(Hotkey::start(
        chord,
        t.capture_tooltip().into(),
        pressed,
    )));

    let shell = Shell {
        store,
        locale,
        changes,
        events: events_tx,
        commands: commands.clone(),
        window,
        autostart,
        hotkey,
        new_popup,
        transcriber,
        reader,
        speech,
        folder_wanted,
        changes_wanted,
        handed,
    };
    // A login start stays in the tray; without one it shows the window.
    let mut next = first.or(tray.is_none().then_some(Command::Open));
    loop {
        let quit = match next.take() {
            Some(Command::Open) => shell.window(tray.is_some())?,
            Some(Command::Capture) => match shell.popup()? {
                Some(Command::Open) => {
                    next = Some(Command::Open);
                    false
                }
                Some(Command::Quit) => true,
                // Without a tray nothing is left to come back to.
                _ => tray.is_none(),
            },
            Some(Command::Quit) => true,
            None => false,
        };
        if quit {
            break;
        }
        if next.is_some() {
            continue;
        }
        let waited = match commands.lock() {
            Ok(rx) => rx.recv(),
            Err(_) => break,
        };
        match waited {
            Ok(command) => next = Some(command),
            Err(_) => break,
        }
    }
    drop(tray);
    // The mDNS record says goodbye before the app ends.
    auto_stop.store(true, Ordering::SeqCst);
    let _ = auto_sync.join();
    let _ = folder_sync.join();
    let _ = backup_thread.join();
    let _ = std::fs::remove_file(&socket);
    Ok(cli::OK)
}

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("oldbrainfix: {e}");
            cli::FAILED
        }
    };
    std::process::exit(code);
}
