//! Paste in the capture (#130), in the window and the capture popup,
//! driven headless by egui_kittest with a fake clipboard: the Paste
//! button and Ctrl+V in the empty field take the clipboard's text into
//! the field and its picture into the Note; a clipboard with neither
//! leaves a grey line, no announcement. The real clipboard is
//! device-only.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, SigningKey, Store, parse_time};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::media::{self, Clipboard};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn t() -> L10n {
    L10n::new("en")
}

fn empty_store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-30T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

/// What the fake clipboard holds, and how often it was read.
#[derive(Default)]
struct Board {
    text: Option<String>,
    photo: Option<Vec<u8>>,
    reads: u32,
}

type Shared = Rc<RefCell<Board>>;

struct FakeClipboard(Shared);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        let mut board = self.0.borrow_mut();
        board.reads += 1;
        board.photo.clone()
    }

    fn text(&mut self) -> Option<String> {
        self.0.borrow().text.clone()
    }
}

/// A picture as a photo to keep.
fn photo() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image = image::RgbaImage::from_pixel(40, 30, image::Rgba([20, 120, 200, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut out, image::ImageFormat::Png)?;
    Ok(media::photo_from_file(&out.into_inner()).ok_or("a photo")?)
}

/// The window over `store` with the fake clipboard `board`.
fn window(store: Arc<Store>, board: &Shared) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.set_clipboard(Box::new(FakeClipboard(board.clone())));
    app.select(Selection::All);
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

/// What the capture field holds.
fn typed<S>(harness: &Harness<'_, S>) -> Option<String> {
    let hint = t().capture_hint();
    harness.get_by(|n| n.placeholder() == Some(hint)).value()
}

/// Gives the capture field the keyboard.
fn focus<S>(harness: &mut Harness<'_, S>) {
    let hint = t().capture_hint();
    harness.get_by(|n| n.placeholder() == Some(hint)).click();
    harness.run();
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Ctrl+V let go, which is all the window brings of it when the
/// clipboard holds only a picture.
fn ctrl_v_released<S>(harness: &mut Harness<'_, S>) {
    harness.event(Event::Key {
        key: Key::V,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    });
    harness.run();
}

#[test]
fn paste_puts_the_text_into_the_field_without_saving_it() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().text = Some("Call the dentist".into());
    let mut harness = window(store.clone(), &board);
    harness.get_by_label(t().paste()).click();
    harness.run();
    assert_eq!(typed(&harness).as_deref(), Some("Call the dentist"));
    assert!(Notes::new(&store).all()?.is_empty());
    assert_eq!(harness.state().notice(), None);
    // Kept as typed text is.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Call the dentist");
    Ok(())
}

#[test]
fn paste_with_text_and_a_picture_keeps_both_in_one_note() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    let photo = photo()?;
    board.borrow_mut().text = Some("Parking spot".into());
    board.borrow_mut().photo = Some(photo.clone());
    let mut harness = window(store.clone(), &board);
    harness.get_by_label(t().paste()).click();
    harness.run();
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Parking spot");
    assert_eq!(notes[0].photos.len(), 1);
    assert_eq!(store.photo(&notes[0].photos[0]), Some(photo));
    Ok(())
}

#[test]
fn pasted_text_goes_after_what_is_typed_at_the_cursor() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().text = Some("milk".into());
    let mut harness = window(store, &board);
    focus(&mut harness);
    harness.get_by(|n| n.is_focused()).type_text("Buy ");
    harness.run();
    harness.get_by_label(t().paste()).click();
    harness.run();
    assert_eq!(typed(&harness).as_deref(), Some("Buy milk"));
    Ok(())
}

#[test]
fn pasted_markdown_makes_the_note_a_formatted_one() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().text = Some("# Shop\n- milk\n- eggs".into());
    let mut harness = window(store.clone(), &board);
    harness.get_by_label(t().paste()).click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "# Shop\n- milk\n- eggs");
    assert_eq!(notes[0].text_format, MARKDOWN);
    Ok(())
}

#[test]
fn an_empty_clipboard_leaves_a_grey_line_until_something_is_typed() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    let mut harness = window(store.clone(), &board);
    harness.get_by_label(t().paste()).click();
    harness.run();
    assert!(shows(&harness, t().paste_empty()));
    assert_eq!(harness.state().notice(), None);
    assert!(Notes::new(&store).all()?.is_empty());
    focus(&mut harness);
    harness.get_by(|n| n.is_focused()).type_text("T");
    harness.run();
    assert!(!shows(&harness, t().paste_empty()));
    Ok(())
}

#[test]
fn ctrl_v_in_the_empty_field_pastes_its_text_once() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().text = Some("Milk".into());
    let mut harness = window(store, &board);
    focus(&mut harness);
    harness.event(Event::Paste("Milk".into()));
    harness.run();
    ctrl_v_released(&mut harness);
    assert_eq!(typed(&harness).as_deref(), Some("Milk"));
    // The picture was looked for once, not again on the release.
    assert_eq!(board.borrow().reads, 1);
    Ok(())
}

#[test]
fn ctrl_v_in_the_empty_field_takes_a_picture_alone() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().photo = Some(photo()?);
    let mut harness = window(store.clone(), &board);
    focus(&mut harness);
    ctrl_v_released(&mut harness);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].photos.len(), 1);
    Ok(())
}

#[test]
fn ctrl_v_in_a_field_with_text_pastes_as_always() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().photo = Some(photo()?);
    let mut harness = window(store.clone(), &board);
    focus(&mut harness);
    harness.get_by(|n| n.is_focused()).type_text("Buy");
    harness.run();
    harness.event(Event::Paste(" milk".into()));
    harness.run();
    ctrl_v_released(&mut harness);
    assert_eq!(typed(&harness).as_deref(), Some("Buy milk"));
    assert_eq!(board.borrow().reads, 0);
    assert!(Notes::new(&store).all()?.is_empty());
    Ok(())
}

struct Popup {
    popup: CapturePopup,
    closed: bool,
}

fn popup(store: Arc<Store>, board: &Shared) -> Harness<'static, Popup> {
    let mut popup = CapturePopup::new(store, "en");
    popup.set_clipboard(Box::new(FakeClipboard(board.clone())));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 340.0))
        .build_ui_state(
            |ui, state: &mut Popup| {
                App::install(ui.ctx());
                if state.popup.show(ui) == Outcome::Closed {
                    state.closed = true;
                }
            },
            Popup {
                popup,
                closed: false,
            },
        );
    harness.run();
    harness
}

#[test]
fn the_popup_holds_what_is_pasted_until_it_is_left() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    board.borrow_mut().text = Some("Receipt".into());
    board.borrow_mut().photo = Some(photo()?);
    let mut harness = popup(store.clone(), &board);
    harness.get_by_label(t().paste()).click();
    harness.run();
    assert_eq!(harness.state().popup.text(), "Receipt");
    assert!(shows(&harness, t().photo()));
    // All of it still fits the popup's window.
    let hint = harness.get_by_label_contains("Enter saves it with a Nudge in 1 h.");
    assert!(hint.rect().bottom() <= 340.0, "{:?}", hint.rect());
    assert!(Notes::new(&store).all()?.is_empty());
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().closed);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].text, "Receipt");
    assert_eq!(notes[0].photos.len(), 1);
    Ok(())
}

#[test]
fn ctrl_v_in_the_empty_popup_takes_a_picture_and_says_when_there_is_none() -> TestResult {
    let store = empty_store()?;
    let board: Shared = Rc::default();
    let mut harness = popup(store.clone(), &board);
    ctrl_v_released(&mut harness);
    assert!(shows(&harness, t().paste_empty()));
    assert_eq!(harness.state().popup.error(), None);
    board.borrow_mut().photo = Some(photo()?);
    ctrl_v_released(&mut harness);
    assert!(!shows(&harness, t().paste_empty()));
    assert!(shows(&harness, t().photo()));
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().closed);
    let notes = Notes::new(&store).all()?;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].photos.len(), 1);
    Ok(())
}
