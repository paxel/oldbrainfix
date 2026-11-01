//! A photo got out of a Note from the window (#149), driven headless by
//! egui_kittest with a fake save dialog, clipboard and opener: a
//! right-click on a photo in the Note editor or shown full screen offers
//! "Save image as…", which writes its file where the user says, "Copy
//! image", which puts it on the clipboard, and "Open in another
//! program", which hands a copy of it to the system's program. Done says
//! nothing; each failure, and a photo whose file is not here, is
//! announced at the top. The real dialog, clipboard and opener are
//! device-only.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, SigningKey, Store, parse_time};
use oldbrainfix_gui::media::Clipboard;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SAVE: &str = "Save image as…";
const COPY: &str = "Copy image";
const OPEN: &str = "Open in another program";

/// What the fake devices were asked and answer.
#[derive(Default)]
struct Devices {
    /// The names the save dialog proposed.
    proposed: Vec<String>,
    /// Where the save dialog says to save; `None` backs out.
    save_to: Option<PathBuf>,
    /// The photos put on the clipboard.
    copied: Vec<Vec<u8>>,
    /// Whether the clipboard takes a picture.
    clipboard_takes: bool,
    /// The files opened, with what they held then.
    opened: Vec<(PathBuf, Vec<u8>)>,
    /// Whether the opener finds a program.
    opener_finds: bool,
}

type Shared = Rc<RefCell<Devices>>;

struct FakeClipboard(Shared);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn put_photo(&mut self, bytes: &[u8]) -> bool {
        let mut d = self.0.borrow_mut();
        if d.clipboard_takes {
            d.copied.push(bytes.to_vec());
        }
        d.clipboard_takes
    }
}

/// A picture as a PNG file.
fn png() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image = image::RgbImage::from_pixel(80, 60, image::Rgb([20, 120, 200]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image).write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

/// A store on disk in `dir`.
fn store(dir: &std::path::Path) -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::open(&dir.join("log.db"), Box::new(move || now), || {
        Ok(SigningKey::from_seed([9; 32]))
    })?;
    Ok(Arc::new(store))
}

/// The window over `store` speaking `locale` with the Note `id` in the
/// editor and the fake devices, the copies to open written to `opened`.
fn window(
    store: Arc<Store>,
    id: &str,
    devices: &Shared,
    (locale, opened): (&str, PathBuf),
) -> Harness<'static, App> {
    let mut app = App::new(store, locale);
    app.set_clipboard(Box::new(FakeClipboard(devices.clone())));
    let saver = devices.clone();
    app.set_photo_saver(Box::new(move |_, name| {
        let mut d = saver.borrow_mut();
        d.proposed.push(name.to_string());
        d.save_to.clone()
    }));
    let opener = devices.clone();
    app.set_photo_opener(
        opened,
        Box::new(move |path| {
            let mut d = opener.borrow_mut();
            d.opened.push((path.to_path_buf(), std::fs::read(path)?));
            if d.opener_finds {
                Ok(())
            } else {
                Err(std::io::Error::other("xdg-open: no method available"))
            }
        }),
    );
    app.select(Selection::All);
    app.select_note(id);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness
}

/// A Note with one photo: the store, the Note's id, the photo's hash
/// and its bytes.
type PhotoNote = (Arc<Store>, String, String, Vec<u8>);

/// A Note with one photo, kept on disk in `dir`.
fn photo_note(dir: &std::path::Path) -> Result<PhotoNote, Box<dyn std::error::Error>> {
    let store = store(dir)?;
    let note = Notes::new(&store).capture_with("Receipt", None, None, &[png()?], &[])?;
    let hash = note.photos[0].clone();
    let bytes = store.photo(&hash).ok_or("no photo kept")?;
    Ok((store, note.id, hash, bytes))
}

/// Right-clicks the photo in the Note dialog, drawn over the list.
fn right_click_editor_photo(harness: &mut Harness<'_, App>) -> TestResult {
    let photo = harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .last()
        .ok_or("no photo in the editor")?;
    photo.click_secondary();
    harness.run();
    Ok(())
}

/// Shows the editor's photo full screen with a click.
fn open_full_screen(harness: &mut Harness<'_, App>) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .last()
        .ok_or("no photo in the editor")?
        .click();
    harness.run();
    Ok(())
}

/// Right-clicks the photo shown full screen, the largest one.
fn right_click_full_screen(harness: &mut Harness<'_, App>) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .max_by(|a, b| a.rect().area().total_cmp(&b.rect().area()))
        .ok_or("no photo")?
        .click_secondary();
    harness.run();
    Ok(())
}

/// Picks the menu entry `entry` with the mouse.
fn pick(harness: &mut Harness<'_, App>, entry: &str) {
    harness.get_by_role_and_label(Role::Button, entry).click();
    harness.run();
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Whether a text in the window says `said`, the failure dialog's too.
fn says(harness: &Harness<'_, App>, said: &str) -> bool {
    harness.query_all_by_label_contains(said).next().is_some()
}

#[test]
fn the_editors_photo_saves_its_file_where_the_user_says_and_says_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, hash, bytes) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().save_to = Some(dir.path().join("receipt"));
    let mut harness = window(store, &id, &devices, ("en", dir.path().join("opened")));
    right_click_editor_photo(&mut harness)?;
    // The menu, and not the photo full screen.
    for entry in [SAVE, COPY, OPEN] {
        assert!(shows(&harness, entry), "{entry}");
    }
    assert_eq!(harness.state().viewed_photo(), None);
    pick(&mut harness, SAVE);
    // Proposed under the phone's name; a name typed without an ending
    // gets the photo's own.
    assert_eq!(
        devices.borrow().proposed,
        [format!("photo-{}.png", &hash[..12])]
    );
    assert_eq!(std::fs::read(dir.path().join("receipt.png"))?, bytes);
    assert_eq!(harness.state().notice(), None);
    assert!(!shows(&harness, SAVE), "the menu stays open");
    // Backing out of the dialog writes nothing and says nothing.
    devices.borrow_mut().save_to = None;
    right_click_editor_photo(&mut harness)?;
    pick(&mut harness, SAVE);
    assert_eq!(devices.borrow().proposed.len(), 2);
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn a_photo_that_cannot_be_written_there_is_announced() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, _, _) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().save_to = Some(dir.path().join("no such folder/receipt.png"));
    let mut harness = window(store, &id, &devices, ("en", dir.path().join("opened")));
    right_click_editor_photo(&mut harness)?;
    pick(&mut harness, SAVE);
    assert_eq!(harness.state().notice(), Some("The photo was not saved."));
    // Its click opens the cause and the fix.
    harness.get_by_label("The photo was not saved.").click();
    harness.run();
    assert!(says(
        &harness,
        "The photo could not be written where you chose."
    ));
    Ok(())
}

#[test]
fn the_editors_photo_goes_onto_the_clipboard_or_says_it_did_not() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, _, bytes) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().clipboard_takes = true;
    let mut harness = window(store, &id, &devices, ("en", dir.path().join("opened")));
    right_click_editor_photo(&mut harness)?;
    pick(&mut harness, COPY);
    assert_eq!(devices.borrow().copied, [bytes]);
    assert_eq!(harness.state().notice(), None);
    // A clipboard that takes no picture: the window's own takes none.
    devices.borrow_mut().clipboard_takes = false;
    right_click_editor_photo(&mut harness)?;
    pick(&mut harness, COPY);
    assert_eq!(devices.borrow().copied.len(), 1);
    assert_eq!(harness.state().notice(), Some("The photo was not copied."));
    Ok(())
}

#[test]
fn the_editors_photo_opens_as_a_copy_in_another_program_or_says_it_did_not() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, hash, bytes) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().opener_finds = true;
    let opened = dir.path().join("opened");
    let mut harness = window(store, &id, &devices, ("en", opened.clone()));
    right_click_editor_photo(&mut harness)?;
    pick(&mut harness, OPEN);
    // A copy with the photo's ending, never the stored file itself.
    let name = format!("photo-{}.png", &hash[..12]);
    assert_eq!(
        devices.borrow().opened,
        [(opened.join(&name), bytes.clone())]
    );
    assert_eq!(harness.state().notice(), None);
    // No program for pictures.
    devices.borrow_mut().opener_finds = false;
    right_click_editor_photo(&mut harness)?;
    pick(&mut harness, OPEN);
    assert_eq!(devices.borrow().opened.len(), 2);
    assert_eq!(harness.state().notice(), Some("The photo was not opened."));
    harness.get_by_label("The photo was not opened.").click();
    harness.run();
    assert!(says(
        &harness,
        "No program for pictures could be started on this computer."
    ));
    Ok(())
}

#[test]
fn the_photo_full_screen_offers_the_same_and_stays_open() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, hash, bytes) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    {
        let mut d = devices.borrow_mut();
        d.save_to = Some(dir.path().join("receipt.png"));
        d.clipboard_takes = true;
        d.opener_finds = true;
    }
    let opened = dir.path().join("opened");
    let mut harness = window(store, &id, &devices, ("en", opened.clone()));
    open_full_screen(&mut harness)?;
    assert!(harness.state().viewed_photo().is_some());
    for entry in [SAVE, COPY, OPEN] {
        right_click_full_screen(&mut harness)?;
        assert!(shows(&harness, SAVE) && shows(&harness, COPY) && shows(&harness, OPEN));
        pick(&mut harness, entry);
        assert_eq!(
            harness.state().viewed_photo().map(|(h, _)| h),
            Some(hash.as_str()),
            "{entry} closed the photo"
        );
        assert!(!shows(&harness, entry), "the menu stays open");
    }
    assert_eq!(std::fs::read(dir.path().join("receipt.png"))?, bytes);
    let d = devices.borrow();
    assert_eq!(d.copied, std::slice::from_ref(&bytes));
    let name = format!("photo-{}.png", &hash[..12]);
    assert_eq!(d.opened, [(opened.join(name), bytes)]);
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn a_photo_whose_file_went_is_announced_before_the_dialog() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, hash, _) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    devices.borrow_mut().save_to = Some(dir.path().join("receipt.png"));
    let mut harness = window(store, &id, &devices, ("en", dir.path().join("opened")));
    open_full_screen(&mut harness)?;
    // Its file goes while it shows full screen.
    std::fs::remove_file(dir.path().join("photos").join(&hash))?;
    right_click_full_screen(&mut harness)?;
    pick(&mut harness, SAVE);
    assert!(devices.borrow().proposed.is_empty());
    assert!(!dir.path().join("receipt.png").exists());
    assert_eq!(harness.state().notice(), Some("The photo was not saved."));
    Ok(())
}

#[test]
fn the_menu_speaks_german_in_german() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (store, id, _, _) = photo_note(dir.path())?;
    let devices: Shared = Rc::default();
    let mut harness = window(store, &id, &devices, ("de", dir.path().join("opened")));
    right_click_editor_photo_de(&mut harness)?;
    for entry in [
        "Bild speichern unter …",
        "Bild kopieren",
        "In anderem Programm öffnen",
    ] {
        assert!(shows(&harness, entry), "{entry}");
    }
    pick(&mut harness, "Bild kopieren");
    assert_eq!(
        harness.state().notice(),
        Some("Das Foto wurde nicht kopiert.")
    );
    Ok(())
}

/// Right-clicks the photo in the German Note dialog, drawn over the list.
fn right_click_editor_photo_de(harness: &mut Harness<'_, App>) -> TestResult {
    let photo = harness
        .query_all_by_role_and_label(Role::Image, "Foto")
        .last()
        .ok_or("no photo in the editor")?;
    photo.click_secondary();
    harness.run();
    Ok(())
}
