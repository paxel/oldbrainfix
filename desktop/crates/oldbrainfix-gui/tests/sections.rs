//! A Note's Sections in the Note dialog (#183), driven headless by
//! egui_kittest: an Amend this Device may edit, its author's or a
//! Manager's, is edited in place in a field of its own and saved as the
//! dialog closes, keeping its place, its capture time and its boxes with
//! their ticks, the old text kept in the log; under its time it says when
//! it was edited and by whom, in the dialog and on its card. Another
//! person's Amend and every text of a Note in the trash are only read,
//! and select to be copied; "Copy as text" copies the Note and its
//! Sections as they show.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton};
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, Node};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::{
    AreaRole, BoxState, Boxes, Invite, Notes, OwnProfile, PairedDevice, Places, Section,
    SigningKey, Store,
};
use oldbrainfix_gui::media::Clipboard;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a microsecond per read, its Profile
/// named `name`.
fn store_of(name: &str, seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    Ok(Arc::new(store_of("Desk", 4)?))
}

/// What went to the fake clipboard: each text.
type Copied = Rc<RefCell<Vec<String>>>;

struct FakeClipboard(Copied);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn put_text(&mut self, text: &str, _html: Option<&str>) -> bool {
        self.0.borrow_mut().push(text.to_string());
        true
    }
}

/// The window over `app`, showing all Notes.
fn window_of(mut app: App) -> Harness<'static, App> {
    app.set_region(Some("DE"));
    app.select(Selection::All);
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

/// The window over `store` with the Note `id` open in its dialog.
fn opened(store: &Arc<Store>, id: &str) -> Harness<'static, App> {
    let mut harness = window_of(App::new(store.clone(), "en"));
    harness.state_mut().select_note(id);
    harness.run();
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// The fields of the Sections this Device may edit.
fn section_fields<'a>(harness: &'a Harness<'_, App>) -> Vec<Node<'a>> {
    harness.query_all_by_label("Section text").collect()
}

fn section(store: &Store, id: &str) -> Result<Section, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).section_by_id(id)?.ok_or("the Section")?)
}

/// Types `text` at the end of the first Section's field, then closes
/// the dialog with Esc, which saves.
fn append_and_close(harness: &mut Harness<'_, App>, text: &str) -> TestResult {
    section_fields(harness)
        .first()
        .ok_or("a Section's field")?
        .click();
    harness.run();
    harness.key_press(Key::End);
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().note_open());
    Ok(())
}

#[test]
fn an_amend_is_edited_in_place_and_keeps_its_place_and_old_text() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let tap = notes.capture("Fix the tap", None, None)?.id;
    let washer = notes.amend(&tap, "Washer bought")?;
    let later = notes.amend(&tap, "Plumber called")?;
    let mut harness = opened(&store, &tap);
    // Both are this Device's: each has its field.
    assert_eq!(section_fields(&harness).len(), 2);
    assert!(!shows(&harness, "Edited "));
    append_and_close(&mut harness, ", size 3/4")?;
    let edited = section(&store, &washer.id)?;
    assert_eq!(edited.text, "Washer bought, size 3/4");
    assert_eq!(edited.created(), washer.created());
    assert!(edited.modified.is_some());
    assert_eq!(edited.edited_by.map(|p| p.name).as_deref(), Some("Desk"));
    let order: Vec<String> = notes.sections(&tap)?.into_iter().map(|s| s.id).collect();
    assert_eq!(order, [washer.id.clone(), later.id.clone()]);
    assert!(section(&store, &later.id)?.modified.is_none());
    // The old text stays in the log.
    let texts: Vec<String> = store
        .history(&washer.id)?
        .into_iter()
        .filter(|e| e.field == note_keys::TEXT)
        .filter_map(|e| e.value)
        .collect();
    assert_eq!(texts.len(), 2, "{texts:?}");
    assert!(texts[0].contains("Washer bought") && !texts[0].contains("size"));
    // Under its time, in the dialog and on its card, who edited it.
    assert!(shows(&harness, " by Desk"));
    harness.state_mut().select_note(&tap);
    harness.run();
    assert_eq!(harness.query_all_by_label_contains(" by Desk").count(), 2);
    Ok(())
}

#[test]
fn an_emptied_section_keeps_its_text() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let tap = notes.capture("Fix the tap", None, None)?.id;
    let washer = notes.amend(&tap, "Washer bought")?;
    let mut harness = opened(&store, &tap);
    section_fields(&harness)
        .first()
        .ok_or("a Section's field")?
        .click();
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::Backspace);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    let kept = section(&store, &washer.id)?;
    assert_eq!(kept.text, "Washer bought");
    assert!(kept.modified.is_none());
    Ok(())
}

#[test]
fn an_edited_section_keeps_its_boxes_with_their_ids_and_ticks() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let trip = notes.capture("Trip", None, None)?.id;
    let tent = notes.amend(&trip, "☐ tent")?;
    let id = section(&store, &tent.id)?.boxes[0].id.clone();
    Boxes::new(&store).tick(&tent.id, &id, BoxState::Done)?;
    let mut harness = opened(&store, &trip);
    append_and_close(&mut harness, " and ☐ torch")?;
    let edited = section(&store, &tent.id)?;
    assert_eq!(edited.boxes.len(), 2);
    assert_eq!(edited.boxes[0].id, id);
    assert_eq!(edited.boxes[0].state, BoxState::Done);
    assert_eq!(edited.shown_text(), "✅ tent and ☐ torch");
    // Edited again, the new box keeps the id the first save gave it.
    let torch = edited.boxes[1].id.clone();
    harness.state_mut().select_note(&trip);
    harness.run();
    append_and_close(&mut harness, "!")?;
    let again = section(&store, &tent.id)?;
    assert_eq!(again.shown_text(), "✅ tent and ☐ torch!");
    assert_eq!(again.boxes[0].id, id);
    assert_eq!(again.boxes[1].id, torch);
    Ok(())
}

/// Ben's store, the Note's, Ann's Amend's and Ben's Amend's ids.
type Shared = (Arc<Store>, String, String, String);

/// Ann's Household with her Note and her Amend in it, shared with Ben as
/// a Member, who amends it too.
fn shared() -> Result<Shared, Box<dyn std::error::Error>> {
    let ann = store_of("Ann", 4)?;
    let house = Places::new(&ann).add_area("Household")?.id;
    let notes = Notes::new(&ann);
    let tap = notes.capture("Fix the tap", Some(&house), None)?.id;
    let hers = notes.amend(&tap, "☐ Washer bought")?.id;
    let ben = store_of("Ben", 5)?;
    let area = ann.share(&house)?;
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let invite = Invite {
        area,
        role: AreaRole::Member,
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    ben.remember_device(&PairedDevice {
        id: ann.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: ben.now(),
        shared: true,
    })?;
    let his = Notes::new(&ben).amend(&tap, "Plumber called")?.id;
    Ok((Arc::new(ben), tap, hers, his))
}

#[test]
fn a_member_edits_their_own_amend_on_another_persons_note_only() -> TestResult {
    let (ben, tap, hers, his) = shared()?;
    let notes = Notes::new(&ben);
    assert!(!notes.may_edit_section(&hers)?);
    assert!(notes.may_edit_section(&his)?);
    let mut harness = opened(&ben, &tap);
    // Ann's Note and her Amend are only read; Ben's Amend has its field.
    assert!(harness.query_all_by_label("Note text").next().is_none());
    assert_eq!(section_fields(&harness).len(), 1);
    assert!(shows(&harness, "Washer bought"));
    // Her text selects to be copied, and a click on its box still ticks
    // it.
    harness
        .query_all_by_role_and_label(Role::Button, "Open")
        .last()
        .ok_or("her box")?
        .click();
    harness.run();
    harness
        .query_all_by_label("✅ Check")
        .last()
        .ok_or("✅ Check")?
        .click_accesskit();
    harness.run();
    assert_eq!(section(&ben, &hers)?.shown_text(), "✅ Washer bought");
    append_and_close(&mut harness, " at nine")?;
    assert_eq!(section(&ben, &his)?.text, "Plumber called at nine");
    assert!(section(&ben, &hers)?.modified.is_none());
    harness.state_mut().select_note(&tap);
    harness.run();
    assert!(shows(&harness, " by Ben"));
    Ok(())
}

#[test]
fn a_note_in_the_trash_reads_its_sections_and_they_select_to_be_copied() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let tap = notes.capture("Fix the tap", None, None)?.id;
    notes.amend(&tap, "Washer bought")?;
    notes.trash(&tap)?;
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(Selection::Trash);
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
    harness.state_mut().select_note(&tap);
    harness.run();
    assert!(harness.state().note_open());
    assert!(section_fields(&harness).is_empty());
    assert!(harness.query_all_by_label("Note text").next().is_none());
    // A drag over the Amend's text selects it, and Copy copies it.
    let rect = harness
        .query_all_by_label("Washer bought")
        .last()
        .ok_or("the Amend's text")?
        .rect();
    let (from, to) = (
        egui::pos2(rect.left() + 1.0, rect.center().y),
        egui::pos2(rect.right() - 1.0, rect.center().y),
    );
    harness.event(Event::PointerMoved(from));
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.event(Event::PointerMoved(to));
    harness.step();
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.event(Event::Copy);
    let mut copied = None;
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                copied = Some(text.clone());
            }
        }
    }
    let copied = copied.ok_or("nothing copied")?;
    assert!(copied.contains("Washer bought"), "{copied}");
    Ok(())
}

#[test]
fn copy_as_text_copies_the_note_and_its_edited_sections_as_they_show() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let trip = notes.capture("Trip", None, None)?.id;
    let tent = notes.amend(&trip, "☐ tent")?;
    let id = section(&store, &tent.id)?.boxes[0].id.clone();
    Boxes::new(&store).tick(&tent.id, &id, BoxState::Done)?;
    let copied = Copied::default();
    let mut app = App::new(store.clone(), "en");
    app.set_clipboard(Box::new(FakeClipboard(copied.clone())));
    let mut harness = window_of(app);
    harness.state_mut().select_note(&trip);
    harness.run();
    append_and_close(&mut harness, " and torch")?;
    harness
        .query_all_by_label("More: Trip")
        .last()
        .ok_or("the card's ⋮")?
        .click_accesskit();
    harness.run();
    harness
        .query_all_by_label("Copy as text")
        .last()
        .ok_or("Copy as text")?
        .click_accesskit();
    harness.run();
    let copied = copied.borrow();
    let [text] = copied.as_slice() else {
        return Err(format!("{copied:?}").into());
    };
    assert!(text.starts_with("Trip\n\n"), "{text}");
    assert!(text.ends_with("\n✅ tent and torch"), "{text}");
    // No id of a box goes with it.
    assert!(
        !text
            .chars()
            .any(|c| ('\u{E0000}'..='\u{E007F}').contains(&c)),
        "{text:?}"
    );
    assert_eq!(harness.state().notice(), None);
    Ok(())
}
