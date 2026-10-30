//! A Note handed to another app from the window (#129): the card's ⋮
//! and the editor offer "Send to a person", "Copy as text" and "Save as
//! Markdown…". Copying puts the Note's text and its Sections on the
//! clipboard, with HTML beside formatted text, or plain text through the
//! window's clipboard when the system's cannot be reached; saving writes
//! the Markdown file, plain text escaped, with its photos and recordings
//! in a folder of its own beside it, under the phone's names. Copied and saved say nothing; a Note without words
//! to copy is announced.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::text_format;
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::media::{self, Clipboard};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What went to the fake clipboard: each text with its HTML.
type Copied = Rc<RefCell<Vec<(String, Option<String>)>>>;

struct FakeClipboard(Copied);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn put_text(&mut self, text: &str, html: Option<&str>) -> bool {
        self.0
            .borrow_mut()
            .push((text.to_string(), html.map(str::to_string)));
        true
    }
}

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([4; 32]))
    })?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(Arc::new(store))
}

/// A picture as a PNG file.
fn png() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image = image::RgbImage::from_pixel(8, 6, image::Rgb([20, 120, 200]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image).write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1200.0))
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

/// Clicks the last of the widgets labelled `label`.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Opens the ⋮ of the card `line` and picks `entry`.
fn pick(harness: &mut Harness<'_, App>, line: &str, entry: &str) -> TestResult {
    click(harness, &format!("More: {line}"))?;
    click(harness, entry)
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

#[test]
fn the_editor_copies_formatted_text_with_html_beside_it_and_says_nothing() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture_as("**Milk** now", text_format::MARKDOWN, None, None)?;
    notes.amend_as(&note.id, "2*3", text_format::PLAIN)?;
    let copied = Copied::default();
    let mut app = App::new(store, "en");
    app.set_clipboard(Box::new(FakeClipboard(copied.clone())));
    app.select(Selection::All);
    app.select_note(&note.id);
    let mut harness = window(app);
    click(&mut harness, "Copy as text")?;
    let copied = copied.borrow();
    let [(text, html)] = copied.as_slice() else {
        return Err(format!("{copied:?}").into());
    };
    assert!(text.starts_with("**Milk** now\n\n"), "{text}");
    assert!(text.ends_with("\n2*3"), "{text}");
    let html = html.as_deref().ok_or("no html")?;
    assert!(
        html.starts_with("<p><strong>Milk</strong> now</p>\n<p>"),
        "{html}"
    );
    assert!(html.ends_with("<br>2*3</p>\n"), "{html}");
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn without_the_systems_clipboard_the_card_copies_plain_text_through_the_window() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture("Milk\nEggs", None, None)?;
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let mut harness = window(app);
    click(&mut harness, "More: Milk")?;
    harness
        .query_all_by_label("Copy as text")
        .last()
        .ok_or("Copy as text")?
        .click_accesskit();
    let mut copied = None;
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                copied = Some(text.clone());
            }
        }
    }
    harness.run();
    assert_eq!(copied.as_deref(), Some("Milk\nEggs"));
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn a_note_without_words_is_not_copied_and_says_why() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture_with("", None, None, &[png()?], &[])?;
    let copied = Copied::default();
    let mut app = App::new(store, "en");
    app.set_clipboard(Box::new(FakeClipboard(copied.clone())));
    app.select(Selection::All);
    let mut harness = window(app);
    harness
        .query_all_by_label_contains("More: ")
        .last()
        .ok_or("no ⋮")?
        .click_accesskit();
    harness.run();
    click(&mut harness, "Copy as text")?;
    assert!(copied.borrow().is_empty());
    assert_eq!(harness.state().notice(), Some("The Note was not shared."));
    Ok(())
}

#[test]
fn the_card_saves_markdown_with_its_photos_and_recordings_in_a_folder_of_its_own() -> TestResult {
    let store = store()?;
    let photo = png()?;
    let voice = media::wav(&[0.0; 160]);
    let notes = Notes::new(&store);
    let note = notes.capture_with(
        "Milk *2*, eggs",
        None,
        None,
        std::slice::from_ref(&photo),
        std::slice::from_ref(&voice),
    )?;
    let other = image::RgbImage::from_pixel(4, 4, image::Rgb([200, 20, 20]));
    let mut bread_photo = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(other).write_to(&mut bread_photo, image::ImageFormat::Png)?;
    let bread_photo = bread_photo.into_inner();
    let bread = notes.capture_with("Bread", None, None, std::slice::from_ref(&bread_photo), &[])?;
    let dir = tempfile::tempdir()?;
    let targets = Rc::new(RefCell::new(vec![
        dir.path().join("Bakery"),
        dir.path().join("Shopping"),
    ]));
    let asked: Rc<RefCell<Vec<String>>> = Rc::default();
    let mut app = App::new(store, "en");
    {
        let asked = asked.clone();
        let targets = targets.clone();
        app.set_markdown_saver(Box::new(move |_, name| {
            asked.borrow_mut().push(name.to_string());
            targets.borrow_mut().pop()
        }));
    }
    app.select(Selection::All);
    let mut harness = window(app);
    pick(&mut harness, "Milk *2*, eggs", "Save as Markdown…")?;
    pick(&mut harness, "Bread", "Save as Markdown…")?;
    assert_eq!(asked.borrow().as_slice(), ["Milk 2 eggs.md", "Bread.md"]);
    // Each Note's files in the folder named as its file, beside it.
    let photo_name = format!("photo-{}.png", &note.photos[0][..12]);
    let voice_name = format!("recording-{}.wav", &note.recordings[0][..12]);
    let shopping = dir.path().join("Shopping");
    assert_eq!(std::fs::read(shopping.join(&photo_name))?, photo);
    assert_eq!(std::fs::read(shopping.join(&voice_name))?, voice);
    let written = std::fs::read_to_string(dir.path().join("Shopping.md"))?;
    // The plain text is escaped, so it reads as written.
    assert_eq!(
        written,
        format!(
            "Milk \\*2\\*, eggs\n\n![{photo_name}](Shopping/{photo_name})\n\n[{voice_name}](Shopping/{voice_name})\n"
        )
    );
    let bread_name = format!("photo-{}.png", &bread.photos[0][..12]);
    let bakery: Vec<_> = std::fs::read_dir(dir.path().join("Bakery"))?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<_>>()?;
    assert_eq!(bakery, vec![bread_name.clone()]);
    let written = std::fs::read_to_string(dir.path().join("Bakery.md"))?;
    assert_eq!(
        written,
        format!("Bread\n\n![{bread_name}](Bakery/{bread_name})\n")
    );
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn backing_out_of_the_save_dialog_writes_nothing() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture("Milk", None, None)?;
    let mut app = App::new(store, "en");
    app.set_markdown_saver(Box::new(|_, _| None::<PathBuf>));
    app.select(Selection::All);
    let mut harness = window(app);
    pick(&mut harness, "Milk", "Save as Markdown…")?;
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn the_card_still_sends_to_a_person() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture("Milk", None, None)?;
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let mut harness = window(app);
    pick(&mut harness, "Milk", "Send to a person")?;
    let title = harness.state().t().send_title().to_string();
    assert!(shows(&harness, &title));
    Ok(())
}
