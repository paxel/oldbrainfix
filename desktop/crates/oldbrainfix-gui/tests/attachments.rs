//! Attachments on the desktop (#267, ADR 0047): any file dropped onto
//! the window or picked waits in the quick note and becomes its Note's
//! Attachment, onto the Note open in the Note dialog it goes onto that
//! Note; its row on the card and in the Note dialog opens it with the
//! system's program; its Attachment Text is read in the background, a
//! PDF page by page with its scans through the OCR, shown dimmed and
//! labelled, removed with Ctrl+Z to bring it back, and the file removed
//! while its text stays, asked once. Driven headless by egui_kittest with
//! a fake OCR and a fake opener.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{NewFile, Note, Notes, SigningKey, Store, parse_time};
use oldbrainfix_gui::enrich::{Engine, Enricher, Kind};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-13T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([7; 32])))?;
    Ok(Arc::new(store))
}

/// A PDF of `pages`, each a page's content stream; `/Im` draws a
/// 300 × 300 JPEG holding `scan`.
fn pdf(pages: &[&str], scan: &[u8]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", 3 + 2 * i))
        .collect();
    let image = 3 + 2 * pages.len();
    let font = image + 1;
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes(),
    );
    for (i, content) in pages.iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 {font} 0 R >> /XObject << /Im {image} 0 R >> >> /Contents {} 0 R >>",
                4 + 2 * i
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(content.as_bytes());
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
    }
    let mut picture = format!(
        "<< /Type /XObject /Subtype /Image /Width 300 /Height 300 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        scan.len()
    )
    .into_bytes();
    picture.extend_from_slice(scan);
    picture.extend_from_slice(b"\nendstream");
    objects.push(picture);
    objects.push(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_vec(),
    );
    let mut out = b"%PDF-1.7\n".to_vec();
    for (i, body) in objects.iter().enumerate() {
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    out.extend_from_slice(b"trailer\n<< /Root 1 0 R >>\n%%EOF\n");
    out
}

/// The JPEG the scanned page holds: only its start and end markers.
const SCAN: &[u8] = b"\xff\xd8\xff\xe0 the scanned page \xff\xd9";

/// A page with the text `text`, and a page that is a scan.
fn lease_pdf() -> Vec<u8> {
    pdf(
        &[
            "BT /F1 12 Tf 72 720 Td (Lease of the flat) Tj ET",
            "q 595 0 0 842 0 0 cm /Im Do Q",
        ],
        SCAN,
    )
}

/// The OCR: the JPEG of the scan reads as a line, anything else not.
struct FakeOcr;

impl Engine for FakeOcr {
    fn read(&mut self, bytes: Vec<u8>) -> Option<String> {
        (bytes == SCAN).then(|| "Rent 900 EUR".to_string())
    }
}

/// What the fake opener was handed, by path and content.
type Opened = Rc<RefCell<Vec<(PathBuf, Vec<u8>)>>>;

/// The window over `store` showing All Notes, its file dialog answering
/// `picked`, its opener noting what it opens into `opened`.
fn window(store: Arc<Store>, picked: Vec<PathBuf>, opened: &Opened) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.set_files_picker(Box::new(move |_| picked.clone()));
    let seen = opened.clone();
    // Where the copies to open go: beside the build, not in /tmp.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("opened");
    app.set_photo_opener(
        dir,
        Box::new(move |path| {
            let bytes = std::fs::read(path)?;
            seen.borrow_mut().push((path.to_path_buf(), bytes));
            Ok(())
        }),
    );
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

fn notes(store: &Store) -> Result<Vec<Note>, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).all()?)
}

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// Waits until `done` holds, for at most ten seconds.
fn eventually(done: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(10);
    while !done() {
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

fn drop_paths(harness: &mut Harness<'_, App>, paths: &[PathBuf]) {
    harness.input_mut().dropped_files = paths
        .iter()
        .map(|path| egui::DroppedFile {
            path: Some(path.clone()),
            ..Default::default()
        })
        .collect();
    harness.run();
}

#[test]
fn files_dropped_onto_the_window_wait_in_the_quick_note_and_are_attached() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let (lease, list) = (dir.path().join("Lease.pdf"), dir.path().join("list.zip"));
    std::fs::write(&lease, lease_pdf())?;
    std::fs::write(&list, b"PK\x03\x04 not really")?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    drop_paths(&mut harness, &[lease, list]);
    // They wait in the quick note, nothing written yet.
    assert!(harness.state().quick_note_open());
    assert!(notes(&store)?.is_empty());
    assert!(shows(&harness, "Lease.pdf") && shows(&harness, "list.zip"));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run();
    let saved = notes(&store)?;
    assert_eq!(saved.len(), 1);
    let files = &saved[0].attachments;
    assert_eq!(files.len(), 2);
    // The PDF's pages were counted: this computer reads it either way,
    // and a phone too, up to ten pages (Q645).
    assert_eq!(
        (files[0].name.as_str(), files[0].pages),
        ("Lease.pdf", Some(2))
    );
    assert!(!files[0].computer);
    assert_eq!(files[1].media_type, "application/zip");
    assert!(Notes::new(&store).has_file(&files[1].hash));
    // Each file is a row of the card that opens it.
    assert!(shows(&harness, "Open Lease.pdf") && shows(&harness, "Open list.zip"));
    Ok(())
}

#[test]
fn a_file_dropped_onto_the_note_dialog_goes_onto_its_note() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store).capture("Flat", None, None)?;
    let dir = tempfile::tempdir()?;
    let deed = dir.path().join("deed.odt");
    std::fs::write(&deed, b"not really odt")?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    harness.state_mut().select_note(&note.id);
    harness.run();
    drop_paths(&mut harness, &[deed]);
    assert!(!harness.state().quick_note_open());
    let flat = Notes::new(&store).by_id(&note.id)?.ok_or("Flat")?;
    assert_eq!(flat.attachments.len(), 1);
    assert_eq!(flat.attachments[0].name, "deed.odt");
    assert_eq!(notes(&store)?.len(), 1);
    // Its row in the dialog opens it.
    assert!(shows(&harness, "Open deed.odt"));
    Ok(())
}

#[test]
fn attach_a_file_picks_files_for_the_note_open_or_a_new_one() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store).capture("Flat", None, None)?;
    let dir = tempfile::tempdir()?;
    let deed = dir.path().join("deed.txt");
    std::fs::write(&deed, b"The deed")?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), vec![deed], &opened);
    // From the Note dialog, onto its Note.
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness.get_by_label("Attach a file…").click_accesskit();
    harness.run();
    let flat = Notes::new(&store).by_id(&note.id)?.ok_or("Flat")?;
    assert_eq!(flat.attachments.len(), 1);
    // From the File menu with no Note open, into the quick note.
    harness.state_mut().close_note();
    harness.run();
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Attach a file…").click_accesskit();
    harness.run();
    assert!(harness.state().quick_note_open());
    assert!(shows(&harness, "deed.txt"));
    Ok(())
}

#[test]
fn a_file_too_large_is_not_attached_and_is_announced() -> TestResult {
    let store = store()?;
    let dir = tempfile::tempdir()?;
    let huge = dir.path().join("film.mkv");
    // Sparse: told by its size, never read.
    std::fs::File::create(&huge)?.set_len(oldbrainfix_core::notes::MAX_FILE_BYTES + 1)?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    drop_paths(&mut harness, &[huge]);
    assert!(!harness.state().quick_note_open());
    assert_eq!(harness.state().notice(), Some("A file was not attached."));
    assert!(notes(&store)?.is_empty());
    Ok(())
}

#[test]
fn a_click_on_an_attachment_opens_a_copy_with_the_systems_program() -> TestResult {
    let store = store()?;
    let bytes = lease_pdf();
    let file = NewFile {
        name: "Lease.pdf",
        media_type: "application/pdf",
        bytes: &bytes,
        pages: Some(2),
    };
    let captured = Notes::new(&store).capture_with_files("", None, None, &[], &[], &[file])?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    // A Note of a file alone: the file is its card, titled by its name.
    assert!(captured.alone_file().is_some());
    harness.get_by_label("Open Lease.pdf").click();
    harness.run();
    let seen = opened.borrow();
    assert_eq!(seen.len(), 1);
    assert!(seen[0].0.ends_with("Lease.pdf"));
    assert_eq!(seen[0].1, bytes);
    // Nothing selected or opened besides.
    assert!(!harness.state().note_open());
    Ok(())
}

#[test]
fn the_reader_reads_every_waiting_file_a_pdf_page_by_page_with_its_scans() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let lease = lease_pdf();
    let broken = b"%PDF-1.7 cut off".to_vec();
    let files = [
        ("Lease.pdf", lease.as_slice()),
        ("Broken.pdf", broken.as_slice()),
        ("milk.txt", b"Milk\r\neggs".as_slice()),
        ("photos.zip", b"PK\x03\x04".as_slice()),
    ];
    let mut hashes = Vec::new();
    let note = notes.capture("Papers", None, None)?;
    for (name, bytes) in files {
        let file = NewFile {
            name,
            media_type: "",
            bytes,
            // Not counted: left to a computer, which this is.
            pages: None,
        };
        hashes.push(notes.attach_file(&note.id, &file)?);
    }
    let reader = Enricher::spawn(
        store.clone(),
        Kind::AttachmentTexts,
        Box::new(FakeOcr),
        Box::new(|| {}),
    );
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    harness
        .state_mut()
        .set_attachment_reader(Some(reader.clone()));
    let text = |hash: &str| {
        notes
            .by_id(&note.id)
            .ok()
            .flatten()
            .and_then(|n| n.attachment_texts.get(hash).cloned())
    };
    assert!(eventually(|| text(&hashes[2]).is_some()));
    assert!(eventually(|| text(&hashes[0]).is_some()));
    // The text page and the scan, a blank line between them.
    assert_eq!(
        text(&hashes[0]).as_deref(),
        Some("Lease of the flat\n\nRent 900 EUR")
    );
    // A PDF that does not read gets an empty text, so it is not read
    // again and again.
    assert!(eventually(|| text(&hashes[1]).is_some()));
    assert_eq!(text(&hashes[1]).as_deref(), Some(""));
    assert_eq!(text(&hashes[2]).as_deref(), Some("Milk\neggs"));
    // A file of a kind without text is never read.
    assert_eq!(text(&hashes[3]), None);
    assert!(notes.unread_attachments(true)?.is_empty());
    // The Note dialog shows the text dimmed, under its label.
    harness.state_mut().select_note(&note.id);
    harness.run();
    assert!(shows(&harness, "Attachment Text"));
    assert!(shows(&harness, "Lease of the flat\n\nRent 900 EUR"));
    Ok(())
}

#[test]
fn the_text_comes_off_with_an_undo_and_the_file_goes_asked_once() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture("Shopping", None, None)?;
    let file = NewFile {
        name: "milk.txt",
        media_type: "text/plain",
        bytes: b"Milk",
        pages: None,
    };
    let hash = notes.attach_file(&note.id, &file)?;
    notes.set_attachment_text(&note.id, &hash, "Milk")?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    harness.state_mut().select_note(&note.id);
    harness.run();
    // Removed at once, and back with Ctrl+Z.
    harness
        .get_by_label("Remove generated text")
        .click_accesskit();
    harness.run();
    let shown = || notes.by_id(&note.id).ok().flatten().ok_or("Shopping");
    assert!(!shown()?.attachment_texts.contains_key(&hash));
    assert!(!shows(&harness, "Remove Attachment, keep text"));
    harness.state_mut().close_note();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(
        shown()?.attachment_texts.get(&hash).map(String::as_str),
        Some("Milk")
    );
    // The file goes after one question; its text stays.
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness
        .get_by_label("Remove Attachment, keep text")
        .click_accesskit();
    harness.run();
    assert!(shows(
        &harness,
        "Remove this Attachment and keep its Attachment Text?"
    ));
    assert_eq!(shown()?.attachments.len(), 1);
    harness
        .query_all_by_label("Remove")
        .next_back()
        .ok_or("Remove")?
        .click_accesskit();
    harness.run();
    let after = shown()?;
    assert!(after.attachments.is_empty());
    assert_eq!(after.removed_attachments.len(), 1);
    assert_eq!(after.attachment_texts[&hash], "Milk");
    // The text still shows, where the file was.
    assert!(shows(&harness, "Attachment Text"));
    Ok(())
}

#[test]
fn a_file_dropped_over_a_dialog_on_the_note_dialog_waits_for_a_new_note() -> TestResult {
    let store = store()?;
    let note = Notes::new(&store).capture("Flat", None, None)?;
    let dir = tempfile::tempdir()?;
    let deed = dir.path().join("deed.odt");
    std::fs::write(&deed, b"not really odt")?;
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    harness.state_mut().select_note(&note.id);
    harness.run();
    // The Settings lie over the Note dialog: the file is not the Note's.
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert!(harness.state().note_open());
    drop_paths(&mut harness, &[deed]);
    let flat = Notes::new(&store).by_id(&note.id)?.ok_or("Flat")?;
    assert!(flat.attachments.is_empty());
    assert!(harness.state().quick_note_open());
    assert!(shows(&harness, "deed.odt"));
    Ok(())
}

#[test]
fn a_file_whose_bytes_are_not_here_yet_is_not_read_and_says_so_when_opened() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let file = NewFile {
        name: "milk.txt",
        media_type: "text/plain",
        bytes: b"Milk",
        pages: None,
    };
    let note = notes.capture_with_files("Shopping", None, None, &[], &[], &[file])?;
    let hash = note.attachments[0].hash.clone();
    // As a file another Device attached arrives before its bytes; one
    // beside it is here, read first.
    store.drop_blob("file", &hash)?;
    let here = NewFile {
        name: "eggs.txt",
        media_type: "text/plain",
        bytes: b"Eggs",
        pages: None,
    };
    let other = notes.capture_with_files("Breakfast", None, None, &[], &[], &[here])?;
    let reader = Enricher::spawn(
        store.clone(),
        Kind::AttachmentTexts,
        Box::new(FakeOcr),
        Box::new(|| {}),
    );
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    harness
        .state_mut()
        .set_attachment_reader(Some(reader.clone()));
    // The one here is read, so the reader ran past the other.
    assert!(eventually(|| {
        notes
            .by_id(&other.id)
            .ok()
            .flatten()
            .is_some_and(|n| !n.attachment_texts.is_empty())
    }));
    assert!(eventually(|| reader.current().is_none()));
    let kept = notes.by_id(&note.id)?.ok_or("Shopping")?;
    assert!(kept.attachment_texts.is_empty());
    assert_eq!(notes.unread_attachments(true)?.len(), 1);
    harness.get_by_label("Open milk.txt").click();
    harness.run();
    assert!(opened.borrow().is_empty());
    assert_eq!(
        harness.state().notice(),
        Some("An Attachment was not opened.")
    );
    Ok(())
}

#[test]
fn a_files_text_is_read_again_and_taken_into_the_notes_text() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture("", None, None)?;
    let file = NewFile {
        name: "list.md",
        media_type: "",
        bytes: b"Milk\neggs",
        pages: None,
    };
    let hash = notes.attach_file(&note.id, &file)?;
    notes.set_attachment_text(&note.id, &hash, "Old text")?;
    let reader = Enricher::spawn(
        store.clone(),
        Kind::AttachmentTexts,
        Box::new(FakeOcr),
        Box::new(|| {}),
    );
    let opened = Opened::default();
    let mut harness = window(store.clone(), Vec::new(), &opened);
    harness.state_mut().set_attachment_reader(Some(reader));
    harness.state_mut().select_note(&note.id);
    harness.run();
    let text = || {
        notes
            .by_id(&note.id)
            .ok()
            .flatten()
            .and_then(|n| n.attachment_texts.get(&hash).cloned())
    };
    harness
        .get_by_label("Read the text again")
        .click_accesskit();
    harness.run();
    assert!(eventually(|| text().as_deref() == Some("Milk\neggs")));
    harness.state_mut().close_note();
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness.get_by_label("Replace Note text").click_accesskit();
    harness.run();
    let shown = notes.by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(shown.text, "Milk\neggs");
    Ok(())
}
