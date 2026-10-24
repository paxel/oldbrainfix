//! The desktop launcher: opens the store, starts the alarm layer, the
//! tray and the login start, and opens the native window, which hands
//! every frame to `oldbrainfix_gui::App`. Closing the window leaves the
//! app running in the tray, where Open brings the window back; without a
//! tray, closing it quits. `--tray` (the login start) starts without the
//! window. A second start hands its wish to the running app and ends. On
//! Windows it is built without a console window.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use oldbrainfix_core::{APP_ID, APP_NAME, DeviceSettings, SigningKey, Store};
use oldbrainfix_gui::alerts::{self, Alerts, Event};
use oldbrainfix_gui::autostart::{Autostart, TRAY_ARG};
use oldbrainfix_gui::l10n::{L10n, locale_for_system};
use oldbrainfix_gui::notify::DesktopNotifier;
use oldbrainfix_gui::tray::{self, Command};
use oldbrainfix_gui::{App, instance, local_wall};

/// The window's context while it is open, to wake it from other threads.
type Window = Arc<Mutex<Option<egui::Context>>>;

fn wake(window: &Window) {
    if let Ok(held) = window.lock()
        && let Some(ctx) = held.as_ref()
    {
        ctx.request_repaint();
    }
}

struct Native {
    app: App,
    commands: Arc<Mutex<Receiver<Command>>>,
    quit: Arc<AtomicBool>,
    /// Whether closing the window leaves the app in the tray.
    tray: bool,
}

impl eframe::App for Native {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        App::install(ui.ctx());
        let waiting: Vec<Command> = match self.commands.lock() {
            Ok(rx) => rx.try_iter().collect(),
            Err(_) => Vec::new(),
        };
        for command in waiting {
            match command {
                Command::Open => focus(ui.ctx()),
                Command::Capture => {
                    focus(ui.ctx());
                    self.app.focus_capture();
                }
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
    }

    fn on_exit(&mut self) {
        self.app.flush();
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
}

impl Shell {
    /// Opens the window and runs it until it closes; whether the app is
    /// to quit.
    fn window(&self, tray: bool, capture: bool) -> Result<bool, String> {
        let mut app = App::new(self.store.clone(), self.locale);
        app.watch_changes(self.changes.clone());
        app.set_wake(self.events.clone());
        app.set_autostart(self.autostart.clone());
        if capture {
            app.focus_capture();
        }
        let quit = Arc::new(AtomicBool::new(false));
        let native = Native {
            app,
            commands: self.commands.clone(),
            quit: quit.clone(),
            tray,
        };
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title(native.app.t().app_title())
                // The app id a launcher entry's StartupWMClass names.
                .with_app_id(APP_ID)
                .with_inner_size([1280.0, 800.0])
                .with_min_inner_size([800.0, 480.0]),
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
                Ok(Box::new(native))
            }),
        );
        if let Ok(mut held) = self.window.lock() {
            *held = None;
        }
        ran.map_err(|e| e.to_string())?;
        Ok(quit.load(Ordering::SeqCst) || !tray)
    }
}

fn run() -> Result<(), String> {
    let login_start = std::env::args().skip(1).any(|a| a == TRAY_ARG);
    let dir = oldbrainfix_gui::data_dir().ok_or("no data directory for this user")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let socket = dir.join("instance.sock");
    // Already running: the window opens there, a login start adds nothing.
    let wish = (!login_start).then_some(Command::Open);
    if instance::hand_over(&socket, wish) {
        return Ok(());
    }
    let store = Arc::new(
        Store::open(
            &dir.join("oldbrainfix.db"),
            Box::new(chrono::Utc::now),
            SigningKey::generate,
        )
        .map_err(|e| e.to_string())?,
    );
    let locale = locale_for_system(sys_locale::get_locale().as_deref());
    let t = L10n::new(locale);

    // On by default: set up unless switched off, so a moved program is
    // found again.
    let autostart = Autostart::for_this_user();
    if let Some(start) = &autostart {
        let on = DeviceSettings::new(&store).autostart_on().unwrap_or(true);
        if let Err(e) = start.set(on) {
            eprintln!("oldbrainfix: {}", t.autostart_failed(&e));
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
    {
        let store = store.clone();
        let notifier = DesktopNotifier::new(events_tx.clone());
        let mut alerts = Alerts::new(Box::new(notifier), t, changes.clone());
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

    if let Err(e) = instance::listen(&socket, command_tx.clone(), waker(&window)) {
        eprintln!("oldbrainfix: {e}");
    }
    let labels = tray::Labels {
        title: t.app_title().into(),
        open: t.tray_open().into(),
        capture: t.capture_tooltip().into(),
        quit: t.tray_quit().into(),
    };
    let tray = tray::spawn(labels, command_tx, waker(&window));

    let shell = Shell {
        store,
        locale,
        changes,
        events: events_tx,
        commands: commands.clone(),
        window,
        autostart,
    };
    // A login start stays in the tray; without one it shows the window.
    let mut open = (!login_start || tray.is_none()).then_some(false);
    loop {
        if let Some(capture) = open.take()
            && shell.window(tray.is_some(), capture)?
        {
            break;
        }
        let next = match commands.lock() {
            Ok(rx) => rx.recv(),
            Err(_) => break,
        };
        match next {
            Ok(Command::Open) => open = Some(false),
            Ok(Command::Capture) => open = Some(true),
            Ok(Command::Quit) | Err(_) => break,
        }
    }
    drop(tray);
    let _ = std::fs::remove_file(&socket);
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("oldbrainfix: {e}");
        std::process::exit(1);
    }
}
