//! The desktop launcher: opens the store and the native window and hands
//! every frame to `oldbrainfix_gui::App`. On Windows it is built without
//! a console window.
#![cfg_attr(windows, windows_subsystem = "windows")]

use oldbrainfix_core::{APP_ID, APP_NAME, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::l10n::locale_for_system;

struct Native {
    app: App,
}

impl eframe::App for Native {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        App::install(ui.ctx());
        self.app.show(ui);
    }
}

fn run() -> Result<(), String> {
    let dir = oldbrainfix_gui::data_dir().ok_or("no data directory for this user")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let store = Store::open(
        &dir.join("oldbrainfix.db"),
        Box::new(chrono::Utc::now),
        SigningKey::generate,
    )
    .map_err(|e| e.to_string())?;
    let locale = locale_for_system(sys_locale::get_locale().as_deref());
    let app = App::new(store, locale);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(app.t().app_title())
            // The app id a launcher entry's StartupWMClass names.
            .with_app_id(APP_ID)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(Native { app }))),
    )
    .map_err(|e| e.to_string())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("oldbrainfix: {e}");
        std::process::exit(1);
    }
}
