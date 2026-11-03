//! Desktop screenshots (#45), not a test suite: the
//! window over a small demo store, drawn by egui_kittest through wgpu and
//! written as PNG files to `$SCREENSHOTS_DIR` (else
//! `target/screenshots`). The release workflow renders them and
//! attaches them to the GitHub Release, which the metainfo points at.
//! Ignored: they need a GPU or a software Vulkan such as lavapipe.
//!
//! ```sh
//! SCREENSHOTS_DIR=/tmp/shots cargo test -p oldbrainfix-gui --test screenshots -- --ignored render_the_screenshots
//! ```
//!
//! The user guide's desktop frames (#134) come from the same demo store:
//! each screen in English and German, light and dark, under
//! `site/assets/frames/desktop/<language>/<theme>/`, a guide box's frame
//! in the folder of its page as `<page>/<box>.png` (#171). They are committed; a
//! run compares each frame with the committed one and fails when they
//! differ, and `UPDATE_FRAMES=1` writes them instead
//! (`tool/guide-frames.sh`):
//!
//! ```sh
//! cargo test -p oldbrainfix-gui --test screenshots -- --ignored guide_frames
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Duration, TimeZone, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::offers::Transport;
use oldbrainfix_core::{
    AreaRole, BoxState, Boxes, Checklists, ClockTime, Day, DeviceSettings, Frogs, ImportReport,
    Importance, Notes, OwnProfile, PairedDevice, PersonSettings, Places, QuickNoteControl,
    Routines, Schedule, ShoppingLists, SigningKey, Store, Tallies, parse_time, text_format,
};
use oldbrainfix_gui::popup::{CapturePopup, Outcome};
use oldbrainfix_gui::speech::{Download, SpeechModel, Status};
use oldbrainfix_gui::{App, Page, Selection};
use oldbrainfix_whisper::model::{ModelDir, Outcome as ModelOutcome};
use parking_lot::Mutex;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The file names the metainfo lists, in its order.
pub const FILES: [&str; 3] = [
    "screenshot-agenda.png",
    "screenshot-notes.png",
    "screenshot-settings.png",
];

fn out_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::var_os("SCREENSHOTS_DIR").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The demo store's words in one language.
struct Texts {
    household: &'static str,
    repairs: &'static str,
    work: &'static str,
    car: &'static str,
    /// The filed Notes: the tap, the bike lock, Dana, the parking, the
    /// tyres, and two Unsorted.
    filed: [&'static str; 7],
    washers: &'static str,
    snack: &'static str,
    bins: &'static str,
    garage: &'static str,
    /// The quick note's hint, and a thought typed into it.
    hint: &'static str,
    capture: &'static str,
    /// The "+" at the top of a list (#161).
    add_here: &'static str,
    /// A Note with a typo, and its text as an AI Task fixes it.
    typo: &'static str,
    fixed: &'static str,
    /// The menu bar's Note menu.
    note_menu: &'static str,
    /// The first start's "New to Old Brain Fix".
    new_here: &'static str,
    /// The menu bar's File menu, its Import…, and Google Keep… in the
    /// Import dialog.
    file_menu: &'static str,
    import: &'static str,
    keep: &'static str,
    /// The Note menu's History.
    history: &'static str,
    /// The Agenda's "New Routine or Reminder", the hint of its text, the
    /// schedule's Weekdays and the Escalation's switch.
    new_routine: &'static str,
    what_hint: &'static str,
    weekdays: &'static str,
    escalate: &'static str,
    /// The start of an Agenda row's ⋮, "More for {title}".
    more_for: &'static str,
    /// The Household's lists (#174): a packing Checklist, its first line
    /// its name, a group under one of its Lines; the weekly Shopping
    /// List; a Tally of the pages read in the evenings, with its field
    /// and unit; and the Frogs put off for a while.
    packing: &'static str,
    groceries: &'static str,
    reading: &'static str,
    pages_read: &'static str,
    pages: &'static str,
    frogs: [&'static str; 3],
    /// The Household's texts (#175): a formatted Note on the boiler's
    /// service with a #tag, a table of meter readings, a script kept as
    /// Code, a Note with boxes, a voice note's Transcript, a receipt's
    /// Note, the #tag and a word the search finds.
    formatted: &'static str,
    meters: &'static str,
    script: &'static str,
    checks: &'static str,
    spoken: &'static str,
    receipt: &'static str,
    tag: &'static str,
    search: &'static str,
    /// Sharing and sync (#176): the Note Lena sends, and the bike lock's
    /// text as this computer and the Laptop changed it before they synced.
    sent: &'static str,
    lock_here: &'static str,
    lock_there: &'static str,
}

const EN: Texts = Texts {
    household: "Household",
    repairs: "Repairs",
    work: "Work",
    car: "Car",
    filed: [
        "The kitchen tap drips again. Washer size is 1/2 inch, spare ones are in the blue box in the cellar.",
        "Bike lock code changed to 4-7-1-9",
        "Ask Dana about the slides for Thursday's review",
        "Parking at the new office: level 2, row F",
        "Winter tyres are in the cellar, left shelf, with the wheel bolts",
        "Where did I put the warranty for the washing machine?",
        "Book title Lena recommended: The Overstory",
    ],
    washers: "Bought two washers, 2.40 at the hardware store.",
    snack: "Snack: an apple and a handful of nuts",
    bins: "Put the bins out",
    garage: "Call the garage about the winter tyres",
    hint: "What's on your mind?",
    capture: "Ask Dana if she still needs the ladder",
    add_here: "Add here",
    typo: "Pick up the parcel for Dana at the post office tomorow",
    fixed: "Pick up the parcel for Dana at the post office tomorrow",
    note_menu: "Note",
    new_here: "New to Old Brain Fix",
    file_menu: "File",
    import: "Import…",
    keep: "Google Keep…",
    history: "History",
    new_routine: "New Routine or Reminder",
    what_hint: "What is to be done?",
    weekdays: "Weekdays",
    escalate: "Escalate to an urgent alarm with sound",
    more_for: "More for",
    packing: "Packing for the weekend at the lake\n✓ Toothbrush\nPhone charger\nHiking gear\n  ✓ Boots\n  Rain jacket",
    groceries: "Weekly shopping\nMilk\nBread\nCoffee\nApples\nWashing-up liquid",
    reading: "Reading before bed",
    pages_read: "Pages read",
    pages: "pages",
    frogs: [
        "Sort the receipts for the tax return",
        "Clear out the shelf in the cellar",
        "Cancel the old gym membership",
    ],
    formatted: "## Boiler service\nBooked for **Thursday, 8 to 10**. Have the *service booklet* ready.\n- Pressure checked: 1.5 bar\n- Spare valve in stock\n- Bleed the radiators\nManual: [the maker's page](https://example.org/boiler)\nError `F28` means no ignition. #boiler",
    meters: "Meter readings 2026\n\n<!-- first column headings -->\n| Meter | Jan | Feb | Mar | Apr | May | Jun |\n| --- | --: | --: | --: | --: | --: | --: |\n| Electricity kWh | 4210 | 4402 | 4588 | 4795 | 4960 | 5121 |\n| Water m³ | 312 | 318 | 324 | 330 | 337 | 343 |\n| Gas m³ | 8840 | 9012 | 9150 | 9241 | 9290 | 9318 |",
    script: "#!/bin/sh\n# Copy the photos to the USB disk every Sunday\nrsync -a --delete ~/Pictures/ /media/usb/photos/\necho \"Photos copied: $(date +%F)\"",
    checks: "Before the holiday\n✅ Turn the heating down\n✅ Give the spare key to the Müllers\n❌ Cancel the newspaper\n✅ Water the plants on Sunday",
    spoken: "Call the plumber about the boiler, Thursday morning between eight and ten.",
    receipt: "Bakery receipt for the party",
    tag: "#boiler",
    search: "boiler",
    sent: "Lena's birthday on Saturday: bring the cake at 3",
    lock_here: "Bike lock code changed to 4-7-1-9, the blue lock",
    lock_there: "Bike lock code changed to 4-7-2-9",
};

const DE: Texts = Texts {
    household: "Haushalt",
    repairs: "Reparaturen",
    work: "Arbeit",
    car: "Auto",
    filed: [
        "Der Küchenhahn tropft wieder. Dichtung ist 1/2 Zoll, Ersatz liegt in der blauen Kiste im Keller.",
        "Code vom Fahrradschloss geändert auf 4-7-1-9",
        "Dana wegen der Folien für die Besprechung am Donnerstag fragen",
        "Parken am neuen Büro: Ebene 2, Reihe F",
        "Winterreifen liegen im Keller, linkes Regal, mit den Radschrauben",
        "Wo ist die Garantie für die Waschmaschine?",
        "Buchtipp von Lena: Die Wurzeln des Lebens",
    ],
    washers: "Zwei Dichtungen gekauft, 2,40 im Baumarkt.",
    snack: "Snack: ein Apfel und eine Handvoll Nüsse",
    bins: "Mülltonnen rausstellen",
    garage: "Werkstatt wegen der Winterreifen anrufen",
    hint: "Was geht dir durch den Kopf?",
    capture: "Dana fragen, ob sie die Leiter noch braucht",
    add_here: "Hier hinzufügen",
    typo: "Packet für Dana morgen bei der Post abhohlen",
    fixed: "Paket für Dana morgen bei der Post abholen",
    note_menu: "Notiz",
    new_here: "Neu bei Old Brain Fix",
    file_menu: "Datei",
    import: "Importieren …",
    keep: "Google Keep …",
    history: "Verlauf",
    new_routine: "Neue Routine oder Erinnerung",
    what_hint: "Was ist zu tun?",
    weekdays: "Wochentage",
    escalate: "Zum dringenden Alarm mit Ton eskalieren",
    more_for: "Mehr zu",
    packing: "Packen fürs Wochenende am See\n✓ Zahnbürste\nLadekabel fürs Handy\nWandersachen\n  ✓ Wanderschuhe\n  Regenjacke",
    groceries: "Wocheneinkauf\nMilch\nBrot\nKaffee\nÄpfel\nSpülmittel",
    reading: "Lesen vor dem Schlafen",
    pages_read: "Gelesene Seiten",
    pages: "Seiten",
    frogs: [
        "Belege für die Steuererklärung sortieren",
        "Das Regal im Keller ausmisten",
        "Den alten Fitnessstudio-Vertrag kündigen",
    ],
    formatted: "## Wartung der Therme\nTermin am **Donnerstag, 8 bis 10 Uhr**. Das *Serviceheft* bereitlegen.\n- Druck geprüft: 1,5 bar\n- Ersatzventil vorrätig\n- Heizkörper entlüften\nAnleitung: [Seite des Herstellers](https://example.org/therme)\nFehler `F28` heißt: keine Zündung. #Therme",
    meters: "Zählerstände 2026\n\n<!-- first column headings -->\n| Zähler | Jan | Feb | Mär | Apr | Mai | Jun |\n| --- | --: | --: | --: | --: | --: | --: |\n| Strom kWh | 4210 | 4402 | 4588 | 4795 | 4960 | 5121 |\n| Wasser m³ | 312 | 318 | 324 | 330 | 337 | 343 |\n| Gas m³ | 8840 | 9012 | 9150 | 9241 | 9290 | 9318 |",
    script: "#!/bin/sh\n# Die Fotos jeden Sonntag auf die USB-Platte kopieren\nrsync -a --delete ~/Bilder/ /media/usb/fotos/\necho \"Fotos kopiert: $(date +%F)\"",
    checks: "Vor dem Urlaub\n✅ Heizung herunterdrehen\n✅ Ersatzschlüssel zu Müllers bringen\n❌ Zeitung abbestellen\n✅ Pflanzen am Sonntag gießen",
    spoken: "Den Installateur wegen der Therme anrufen, Donnerstag früh zwischen acht und zehn.",
    receipt: "Beleg vom Bäcker für die Feier",
    tag: "#Therme",
    search: "Therme",
    sent: "Lenas Geburtstag am Samstag: den Kuchen um 3 mitbringen",
    lock_here: "Code vom Fahrradschloss geändert auf 4-7-1-9, das blaue Schloss",
    lock_there: "Code vom Fahrradschloss geändert auf 4-7-2-9",
};

/// A store with a few days of a household's and a job's Notes and
/// Routines in `x`'s words, its clock at Monday 26 October 2026, 08:30 UTC.
fn demo(x: &Texts) -> Result<(Store, Vec<String>), Box<dyn std::error::Error>> {
    let (store, ids, _) = demo_with_clock(x)?;
    Ok((store, ids))
}

/// The store's clock, which a scene may set back and forth.
type Clock = Arc<Mutex<DateTime<Utc>>>;

/// [`demo`] with the store's clock.
fn demo_with_clock(x: &Texts) -> Result<(Store, Vec<String>, Clock), Box<dyn std::error::Error>> {
    let now = Arc::new(Mutex::new(parse_time("2026-10-22T17:40:00.000Z")?));
    let clock = now.clone();
    let mut store = Store::in_memory(Box::new(move || *clock.lock()), || {
        Ok(SigningKey::from_seed([45; 32]))
    })?;
    // Counted ids, not random ones: Notes captured at the same moment
    // keep one order on every machine.
    let next = AtomicU64::new(0);
    store.set_id_maker(Box::new(move || {
        Ok(format!("{:032x}", next.fetch_add(1, Ordering::SeqCst) + 1))
    }));
    store.set_local_setting("author", "Sam")?;
    let places = Places::new(&store);
    let household = places.add_area(x.household)?;
    places.set_icon(&household.id, Some("home"))?;
    places.set_colour(&household.id, Some("2E7D6B"))?;
    let repairs = places.add_topic(&household.id, x.repairs)?;
    let work = places.add_area(x.work)?;
    places.set_icon(&work.id, Some("work"))?;
    places.set_colour(&work.id, Some("5C6BC0"))?;
    let car = places.add_area(x.car)?;
    places.set_icon(&car.id, Some("car"))?;
    places.set_colour(&car.id, Some("C0504D"))?;

    let notes = Notes::new(&store);
    let mut ids = Vec::new();
    let later = |minutes: i64| {
        let mut held = now.lock();
        *held += Duration::minutes(minutes);
    };
    let places_of = [
        Some(repairs.id.as_str()),
        Some(household.id.as_str()),
        Some(work.id.as_str()),
        Some(work.id.as_str()),
        Some(car.id.as_str()),
        None,
        None,
    ];
    let filed = x.filed.into_iter().zip(places_of);
    for (text, place) in filed {
        later(95);
        ids.push(notes.capture(text, place, None)?.id);
    }
    notes.set_importance(&ids[0], Importance::Important)?;
    later(30);
    notes.amend(&ids[0], x.washers)?;

    let routines = Routines::new(&store);
    let monday = Day::new(2026, 10, 26);
    let at = |h, m| ClockTime::new(h, m).ok_or("a time");
    let daily = oldbrainfix_core::Recurrence::parse("FREQ=DAILY")?;
    let weekly = oldbrainfix_core::Recurrence::parse("FREQ=WEEKLY;BYDAY=MO")?;
    let snack = notes.capture(x.snack, Some(&household.id), None)?;
    let schedule = Schedule::new(monday, &[at(8, 0)?, at(15, 0)?], Some(daily));
    routines.spawn(&snack.id, &schedule.ok_or("schedule")?)?;
    let bins = notes.capture(x.bins, Some(&household.id), None)?;
    let schedule = Schedule::new(monday, &[at(19, 0)?], Some(weekly));
    routines.spawn(&bins.id, &schedule.ok_or("schedule")?)?;
    let schedule = Schedule::new(monday, &[at(11, 30)?], None);
    routines.spawn(&ids[2], &schedule.ok_or("schedule")?)?;
    let schedule = Schedule::new(monday.add_days(1), &[at(9, 0)?], None);
    routines.create(x.garage, &schedule.ok_or("schedule")?)?;

    *now.lock() = parse_time("2026-10-26T08:30:00.000Z")?;
    let snacks = routines.of_note(&snack.id)?;
    if let Some(routine) = snacks.first() {
        let morning = oldbrainfix_core::Occurrence::new(monday, Some(at(8, 0)?));
        routines.mark_done(&routine.id, &morning, None)?;
    }
    // The screens show Tallies, Sessions, Frogs, Topics, the format bar
    // and roles: Advanced, chosen (#197).
    PersonSettings::new(&store).set_advanced(true)?;
    Ok((store, ids, now))
}

fn utc(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// The Notes of the Household's lists (#174), as [`seed_lists`] made them.
#[derive(Default)]
struct Lists {
    packing: String,
    groceries: String,
}

/// Adds the Household's lists in `x`'s words to the demo store, each at
/// its own moment on `now`, which is set back after: a Tally of the pages
/// read most evenings since 13 October, 30 a day the target; the packing
/// Checklist, a Line and a grouped one checked; the weekly Shopping List,
/// two Lines bought on Saturday; and three Frogs, one of them at Work.
fn seed_lists(store: &Store, x: &Texts, now: &Clock) -> oldbrainfix_core::Result<Lists> {
    let back = *now.lock();
    let at = |day: u32, h: u32, m: u32| {
        *now.lock() = Utc
            .with_ymd_and_hms(2026, 10, day, h, m, 0)
            .single()
            .unwrap_or(back);
    };
    let places = Places::new(store);
    let area = |name: &str| -> oldbrainfix_core::Result<Option<String>> {
        Ok(places
            .areas()?
            .into_iter()
            .find(|a| a.name == name)
            .map(|a| a.id))
    };
    let household = area(x.household)?;
    let work = area(x.work)?;
    let notes = Notes::new(store);

    at(13, 22, 5);
    let reading = notes.capture(x.reading, household.as_deref(), None)?;
    let tallies = Tallies::new(store);
    let pages = tallies.add_field(&reading.id, x.pages_read, x.pages, Some(30.0), None)?;
    let read = [24, 31, 0, 18, 40, 35, 12, 0, 30, 27, 22, 45, 33];
    for (day, count) in (13..).zip(read) {
        if count == 0 {
            continue;
        }
        at(day, 22, 15);
        let values = BTreeMap::from([(pages.clone(), f64::from(count))]);
        tallies.add(&reading.id, &values, "", None)?;
    }

    at(24, 9, 30);
    let groceries = notes.capture(x.groceries, household.as_deref(), None)?;
    let shopping = ShoppingLists::new(store);
    shopping.make(&groceries.id)?;
    at(24, 11, 5);
    if let Some(list) = shopping.by_note(&groceries.id, &utc)? {
        for line in [&list.lines[1], &list.lines[3]] {
            shopping.check(&line.id, &utc)?;
        }
    }

    at(25, 19, 40);
    let packing = notes.capture(x.packing, household.as_deref(), None)?;
    Checklists::new(store).make(&packing.id)?;

    let frogs = Frogs::new(store);
    for (day, (text, place)) in (8..).step_by(3).zip([
        (x.frogs[0], &work),
        (x.frogs[1], &household),
        (x.frogs[2], &household),
    ]) {
        at(day, 20, 10);
        let note = notes.capture(text, place.as_deref(), None)?;
        frogs.spawn(&note.id)?;
    }
    *now.lock() = back;
    Ok(Lists {
        packing: packing.id,
        groceries: groceries.id,
    })
}

/// The window over the demo store with the Household's lists, in `x`'s
/// words, `locale` and `theme`, on the Agenda; with the lists' Notes.
fn window_with_lists(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<(Harness<'static, App>, Lists), Box<dyn std::error::Error>> {
    let mut lists = Lists::default();
    let harness = window_with_clock(x, locale, theme, |store, _, now| {
        lists = seed_lists(store, x, now)?;
        Ok(())
    })?;
    Ok((harness, lists))
}

/// The Notes of the Household's texts (#175), as [`seed_texts`] made them.
#[derive(Default)]
struct TextNotes {
    formatted: String,
    meters: String,
    script: String,
    voice: String,
    receipt: String,
}

/// The photo of a bakery's receipt the OCR tests read.
const RECEIPT: &[u8] = include_bytes!("../../../../android/ocr/testdata/german-receipt.jpg");

/// The words read from [`RECEIPT`], line by line.
const RECEIPT_TEXT: &str = "Bäckerei Müller GmbH\nHauptstraße 7, 80331 München\nBrötchen 6x 2,40 €\nKäsekuchen 3,90 €\nSüßes Teilchen 1,85 €\nSumme EUR 8,15\nMwSt 7% 0,53\nVielen Dank für Ihren Einkauf";

/// Adds the Household's texts in `x`'s words to the demo store, each at
/// its own moment on 25 October, the clock set back after: the table of
/// meter readings and the boiler's service, both formatted; the backup
/// script as Code; the holiday's boxes, written checked or failed, the
/// first failed and checked again so that it has a History; the receipt's photo
/// with its Photo Text; and a voice note with its Transcript, the newest.
fn seed_texts(store: &Store, x: &Texts, now: &Clock) -> oldbrainfix_core::Result<TextNotes> {
    let back = *now.lock();
    let at = |h: u32, m: u32| {
        *now.lock() = Utc
            .with_ymd_and_hms(2026, 10, 25, h, m, 0)
            .single()
            .unwrap_or(back);
    };
    let household = Places::new(store)
        .areas()?
        .into_iter()
        .find(|a| a.name == x.household)
        .map(|a| a.id);
    let place = household.as_deref();
    let notes = Notes::new(store);
    at(8, 50);
    let meters = notes.capture_as(x.meters, text_format::MARKDOWN, place, None)?;
    at(9, 10);
    let formatted = notes.capture_as(x.formatted, text_format::MARKDOWN, place, None)?;
    at(9, 40);
    let script = notes.capture_as(x.script, text_format::CODE, place, None)?;
    at(10, 5);
    let checks = notes.capture(x.checks, place, None)?;
    let boxes = Boxes::new(store);
    let shown = boxes.of(&checks.id)?;
    if let Some(first) = shown.first() {
        at(18, 20);
        boxes.tick(&checks.id, &first.id, BoxState::Failed)?;
        at(18, 25);
        boxes.tick(&checks.id, &first.id, BoxState::Done)?;
    }
    at(19, 30);
    let receipt = notes.capture_with(x.receipt, place, None, &[RECEIPT.to_vec()], &[])?;
    if let Some(hash) = receipt.photos.first() {
        notes.set_photo_text(&receipt.id, hash, RECEIPT_TEXT)?;
    }
    at(20, 15);
    let voice = notes.capture_with("", place, None, &[], &[vec![1, 2, 3]])?;
    if let Some(hash) = voice.recordings.first() {
        notes.set_transcript(&voice.id, hash, x.spoken)?;
    }
    *now.lock() = back;
    Ok(TextNotes {
        formatted: formatted.id,
        meters: meters.id,
        script: script.id,
        voice: voice.id,
        receipt: receipt.id,
    })
}

/// The window over the demo store with the Household's texts, in `x`'s
/// words, `locale` and `theme`, on the Agenda; with the texts' Notes.
fn window_with_texts(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<(Harness<'static, App>, TextNotes), Box<dyn std::error::Error>> {
    let mut texts = TextNotes::default();
    let harness = window_with_clock(x, locale, theme, |store, _, now| {
        texts = seed_texts(store, x, now)?;
        Ok(())
    })?;
    Ok((harness, texts))
}

/// Shows the Household's Notes in the window.
fn show_household(harness: &mut Harness<'static, App>, x: &Texts) -> TestResult {
    let household = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .find(|a| a.name == x.household)
        .ok_or("the Household Area")?;
    harness.state_mut().select(Selection::Place(household.id));
    harness.run();
    Ok(())
}

/// Clicks the last widget labelled `label`.
fn click_last(harness: &mut Harness<'static, App>, label: &str) -> TestResult {
    harness
        .get_all_by_label(label)
        .last()
        .ok_or_else(|| format!("{label:?}"))?
        .click();
    harness.run();
    Ok(())
}

/// The window's size in the screenshots.
const SIZE: (u32, u32) = (1280, 800);
/// The margin kittest draws around the window.
const MARGIN: u32 = 8;

/// Draws the window as it stands and writes it as `name`, without the
/// harness's margin.
fn shoot(harness: &mut Harness<'static, App>, name: &str) -> TestResult {
    harness.run_steps(4);
    let image = harness.render()?;
    let window = image::imageops::crop_imm(&image, MARGIN, MARGIN, SIZE.0, SIZE.1).to_image();
    window.save(out_dir()?.join(name))?;
    Ok(())
}

#[test]
#[ignore = "needs a GPU or a software Vulkan; run by the release workflow"]
fn render_the_screenshots() -> TestResult {
    let (store, ids) = demo(&EN)?;
    let mut app = App::new(store, "en");
    app.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (SIZE.0 + 2 * MARGIN) as f32,
            (SIZE.1 + 2 * MARGIN) as f32,
        ))
        .wgpu()
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run_steps(4);
    shoot(&mut harness, FILES[0])?;

    let household = Places::new(harness.state().store())
        .areas()?
        .into_iter()
        .find(|a| a.name == EN.household)
        .ok_or("the Household Area")?;
    harness.state_mut().select(Selection::Place(household.id));
    harness.state_mut().select_note(&ids[0]);
    shoot(&mut harness, FILES[1])?;

    harness.state_mut().open(Page::Settings);
    shoot(&mut harness, FILES[2])?;
    Ok(())
}

/// How far a guide frame may drift from the committed one before it
/// counts as changed: a pixel differs when a channel is more than
/// `CHANNEL_SLACK` off, and a frame fails with more than `PIXEL_SLACK`
/// differing pixels. The slack absorbs the edges another GPU or Mesa
/// version rasterises a shade differently; a changed word or button
/// exceeds it. The same rule as the phone's frames.
const CHANNEL_SLACK: u8 = 24;
const PIXEL_SLACK: usize = 200;

/// Where the repository keeps the guide frame `path`.
fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path)
}

/// Compares the window as it stands with the committed guide frame `path`,
/// or writes it there with `UPDATE_FRAMES=1`. A changed frame is written to
/// `build/guide-frames/` for a look and returned as the problem.
fn match_frame<S>(
    harness: &mut Harness<'static, S>,
    path: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    harness.run_steps(4);
    let image = harness.render()?;
    // The window, or the capture popup, without the harness's margin.
    let (width, height) = image.dimensions();
    let shot = image::imageops::crop_imm(
        &image,
        MARGIN,
        MARGIN,
        width - 2 * MARGIN,
        height - 2 * MARGIN,
    )
    .to_image();
    let file = repo(path);
    if std::env::var("UPDATE_FRAMES").as_deref() == Ok("1") {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        shot.save(&file)?;
        return Ok(None);
    }
    let problem = match image::open(&file) {
        Err(err) => Some(format!("no committed frame ({err})")),
        Ok(old) => {
            let old = old.to_rgba8();
            if old.dimensions() == shot.dimensions() {
                let differing = old
                    .pixels()
                    .zip(shot.pixels())
                    .filter(|(a, b)| {
                        a.0.iter()
                            .zip(b.0)
                            .any(|(x, y)| x.abs_diff(y) > CHANNEL_SLACK)
                    })
                    .count();
                (differing > PIXEL_SLACK).then(|| format!("{differing} pixels differ"))
            } else {
                Some(format!(
                    "size {:?}, committed {:?}",
                    shot.dimensions(),
                    old.dimensions()
                ))
            }
        }
    };
    let Some(problem) = problem else {
        return Ok(None);
    };
    let fresh = repo("build/guide-frames").join(path);
    if let Some(dir) = fresh.parent() {
        std::fs::create_dir_all(dir)?;
    }
    shot.save(&fresh)?;
    Ok(Some(format!(
        "{path} no longer matches its screen ({problem}); the new frame is build/guide-frames/{path}"
    )))
}

/// The window over the demo store in `x`'s words, in `locale` and `theme`,
/// on the Agenda.
fn window(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    window_with(x, locale, theme, |_, _| Ok(()))
}

/// The window as [`window`] opens it, once `prepare` changed the demo
/// store, given with the ids of its filed Notes.
fn window_with(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
    prepare: impl FnOnce(&Store, &[String]) -> oldbrainfix_core::Result<()>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    window_with_clock(x, locale, theme, |store, ids, _| prepare(store, ids))
}

/// [`window_with`], `prepare` also given the store's clock: what it sets
/// it to holds while the window shows.
fn window_with_clock(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
    prepare: impl FnOnce(&Store, &[String], &Clock) -> oldbrainfix_core::Result<()>,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let (store, ids, now) = demo_with_clock(x)?;
    prepare(&store, &ids, &now)?;
    let mut app = App::new(store, locale);
    app.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (SIZE.0 + 2 * MARGIN) as f32,
            (SIZE.1 + 2 * MARGIN) as f32,
        ))
        .with_theme(theme)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    // The window follows the system's theme (#147).
    harness.input_mut().system_theme = Some(theme);
    harness.run_steps(4);
    Ok(harness)
}

/// The id of the Routine whose Note says `text`.
fn routine_of(store: &Store, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Routines::new(store)
        .all_with_notes()?
        .into_iter()
        .find(|(_, note)| note.as_ref().is_some_and(|n| n.text == text))
        .map(|(routine, _)| routine.id)
        .ok_or_else(|| format!("the Routine of {text}").into())
}

/// The capture popup's size, as its own window opens it.
const POPUP: (u32, u32) = (480, 340);

/// The capture popup over the demo store, as the hotkey opens it.
struct Popup {
    popup: CapturePopup,
    outcome: Outcome,
}

/// The capture popup the hotkey opens over all windows, over the demo
/// store in `x`'s words, in `locale` and `theme`.
fn popup(
    x: &Texts,
    locale: &str,
    theme: egui::Theme,
) -> Result<Harness<'static, Popup>, Box<dyn std::error::Error>> {
    let (store, _) = demo(x)?;
    let mut popup = CapturePopup::new(Arc::new(store), locale);
    popup.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (POPUP.0 + 2 * MARGIN) as f32,
            (POPUP.1 + 2 * MARGIN) as f32,
        ))
        .with_theme(theme)
        .wgpu()
        .build_ui_state(
            |ui, state: &mut Popup| {
                App::install(ui.ctx());
                if state.outcome == Outcome::Open {
                    state.outcome = state.popup.show(ui);
                }
            },
            Popup {
                popup,
                outcome: Outcome::Open,
            },
        );
    harness.input_mut().system_theme = Some(theme);
    harness.run_steps(4);
    Ok(harness)
}

/// The window of a computer on its first start, in `locale` and `theme`:
/// nothing written yet, so the first start's three choices show.
fn first_start(
    locale: &str,
    theme: egui::Theme,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T08:30:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || {
        Ok(SigningKey::from_seed([46; 32]))
    })?;
    let mut app = App::new(store, locale);
    app.set_wall_clock(Arc::new(utc));
    app.set_profile_defaults(Box::new(|store| {
        oldbrainfix_core::OwnProfile::new(store).apply_defaults("Sam", None)
    }));
    app.offer_restore();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(
            (SIZE.0 + 2 * MARGIN) as f32,
            (SIZE.1 + 2 * MARGIN) as f32,
        ))
        .with_theme(theme)
        .wgpu()
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.input_mut().system_theme = Some(theme);
    harness.run_steps(4);
    Ok(harness)
}

#[test]
#[ignore = "needs a GPU or a software Vulkan; run by tool/guide-frames.sh and CI"]
fn guide_frames() -> TestResult {
    let mut problems = Vec::new();
    for (language, texts) in [("en", &EN), ("de", &DE)] {
        for (theme, name) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let frame =
                |screen: &str| format!("site/assets/frames/desktop/{language}/{name}/{screen}.png");
            // The frame of the box `name` on the guide page `page` (#171),
            // as the site's box names it: a page's ticket only adds its
            // scenes here.
            let box_frame = |page: &str, name: &str| frame(&format!("{page}/{name}"));
            let mut harness = window(texts, language, theme)?;
            problems.extend(match_frame(&mut harness, &frame("agenda"))?);
            // Getting started: the first start's three choices, for adding
            // a Device and for a restore, whose own pages show a code and
            // a file of this run; the Starter Set; the Import dialog and
            // its question about a Keep export; the Settings.
            let mut harness = first_start(language, theme)?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "add-device"),
            )?);
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "restore"),
            )?);
            harness.get_by_label(texts.new_here).click_accesskit();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "start-new"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness.get_by_label(texts.file_menu).click();
            harness.run();
            harness.get_by_label(texts.import).click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "import-folder"),
            )?);
            harness.get_by_label(texts.keep).click_accesskit();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "import-keep"),
            )?);
            // A thought typed into the quick note, opened over the Agenda
            // as File ▸ New Note opens it (#146): the action row and the
            // Nudge times show.
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().new_note();
            harness.run();
            harness
                .get_by(|n| n.placeholder() == Some(texts.hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "capture"),
            )?);
            // Capture and sort: the quick note with what Settings ▸ Quick
            // note switched on, the capture popup of the hotkey, the
            // Unsorted Notes, the Note dialog and its History.
            let mut harness = window_with(texts, language, theme, |store, _| {
                let device = DeviceSettings::new(store);
                for control in [
                    QuickNoteControl::FileInto,
                    QuickNoteControl::Schedule,
                    QuickNoteControl::Importance,
                ] {
                    device.set_quick_note_shows(control, true)?;
                }
                Ok(())
            })?;
            harness.state_mut().new_note();
            harness.run();
            harness
                .get_by(|n| n.placeholder() == Some(texts.hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "sort-while-capturing"),
            )?);
            let mut harness = popup(texts, language, theme)?;
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "capture-outside"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().select(Selection::Unsorted);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "sort-later"),
            )?);
            // An AI Task that fixed a typo and filed two Unsorted Notes
            // into the Household waits for its review: since before the
            // window opened, so the review shows at once.
            let mut harness = window_with(texts, language, theme, |store, ids| {
                let notes = Notes::new(store);
                let parcel = notes.capture(texts.typo, None, None)?;
                let household = Places::new(store)
                    .areas()?
                    .into_iter()
                    .find(|a| a.name == texts.household)
                    .map(|a| a.id)
                    .unwrap_or_default();
                let task = store.open_ai_task()?;
                store.drafting(&task, || {
                    notes.edit(&parcel.id, texts.fixed)?;
                    notes.file(&parcel.id, &household)?;
                    notes.file(&ids[5], &household)
                })
            })?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("sharing-and-sync", "ai-task"),
            )?);
            // The Household's Notes under the kitchen tap's, filed into
            // Repairs, open in the Note dialog with its badges (#146,
            // #160).
            let mut tap = String::new();
            let mut harness = window_with(texts, language, theme, |_, ids| {
                tap = ids[0].clone();
                Ok(())
            })?;
            let household = Places::new(harness.state().store())
                .areas()?
                .into_iter()
                .find(|a| a.name == texts.household)
                .ok_or("the Household Area")?;
            harness.state_mut().select(Selection::Place(household.id));
            harness.state_mut().select_note(&tap);
            problems.extend(match_frame(&mut harness, &frame("window"))?);
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "edit"),
            )?);
            // The menu bar's Note menu over it, for that Note.
            harness
                .get_all_by_label(texts.note_menu)
                .next()
                .ok_or("the Note menu")?
                .click();
            harness.run();
            problems.extend(match_frame(&mut harness, &frame("menu"))?);
            // Note ▸ History of that Note.
            harness
                .get_all_by_label(texts.history)
                .last()
                .ok_or("Note ▸ History")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("capture-and-sort", "history"),
            )?);
            // Routines and the Agenda (#173): a new Routine typed from the
            // Agenda's bar, the snack's schedule on weekdays, its Re-nag
            // and Escalation, a Reminder due now in its Note's dialog, the
            // Missed Summary, the snack's History, the Agenda, All Routines
            // and Reminders, and an Occurrence's menu with Share.
            let mut harness = window(texts, language, theme)?;
            harness.get_by_label(texts.new_routine).click();
            harness.run();
            harness
                .get_by(|n| n.placeholder() == Some(texts.what_hint))
                .click();
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.capture);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "set-up"),
            )?);
            let mut harness = window(texts, language, theme)?;
            let snack = routine_of(harness.state().store(), texts.snack)?;
            harness.state_mut().open_routine(&snack);
            harness.run();
            harness.get_by_label(texts.weekdays).click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "schedule"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().open_routine(&snack);
            harness.run();
            harness
                .get_all_by_label(texts.escalate)
                .next()
                .ok_or("the Escalation's switch")?
                .scroll_to_me();
            harness.run();
            harness
                .get_all_by_label(texts.escalate)
                .next()
                .ok_or("the Escalation's switch")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "re-nag"),
            )?);
            // The parking's Note with a Reminder due at 08:00, set up the
            // evening before.
            let mut parking = String::new();
            let mut harness = window_with_clock(texts, language, theme, |store, ids, now| {
                parking = ids[3].clone();
                let due = ClockTime::new(8, 0)
                    .and_then(|at| Schedule::new(Day::new(2026, 10, 26), &[at], None));
                let back = *now.lock();
                *now.lock() = back - Duration::hours(12);
                if let Some(schedule) = due {
                    Routines::new(store).spawn(&ids[3], &schedule)?;
                }
                *now.lock() = back;
                Ok(())
            })?;
            harness.state_mut().select_note(&parking);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "check-off"),
            )?);
            // The tyres to be checked every evening since Tuesday, set up
            // the day before and never Done: Missed.
            let mut harness = window_with_clock(texts, language, theme, |store, ids, now| {
                let daily = oldbrainfix_core::Recurrence::parse("FREQ=DAILY").ok();
                let evenings = ClockTime::new(18, 0)
                    .and_then(|at| Schedule::new(Day::new(2026, 10, 20), &[at], daily));
                let back = *now.lock();
                *now.lock() = back - Duration::days(7);
                if let Some(schedule) = evenings {
                    Routines::new(store).spawn(&ids[4], &schedule)?;
                }
                *now.lock() = back;
                Ok(())
            })?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "missed"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness
                .state_mut()
                .select(Selection::History(snack.clone()));
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "history"),
            )?);
            let mut harness = window(texts, language, theme)?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "agenda"),
            )?);
            harness.state_mut().select(Selection::AllRoutines);
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "all-routines"),
            )?);
            let mut harness = window(texts, language, theme)?;
            harness
                .get_all_by_label(&format!("{} {}", texts.more_for, texts.snack))
                .nth(1)
                .ok_or("the snack's ⋮")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame("routines-and-agenda", "calendar"),
            )?);
            // What "+" at the top of the Household's list makes (#161).
            let mut harness = window(texts, language, theme)?;
            let household = Places::new(harness.state().store())
                .areas()?
                .into_iter()
                .find(|a| a.name == texts.household)
                .ok_or("the Household Area")?;
            harness.state_mut().select(Selection::Place(household.id));
            harness.run();
            harness.get_by_label(texts.add_here).click();
            harness.run();
            problems.extend(match_frame(&mut harness, &frame("add-menu"))?);
            // Lists, Tallies, Sessions and Frogs (#174): the same "+", the
            // Household's cards compact, a plain Note's kinds, the packing
            // Checklist in the Note dialog, the Shopping List's Agenda
            // choices and its card, the New Tally page, a Tally's card,
            // the New Session page and the Frog view.
            const LISTS: &str = "lists-tallies-sessions-frogs";
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "make"))?);
            let (mut harness, _) = window_with_lists(texts, language, theme)?;
            show_household(&mut harness, texts)?;
            problems.extend(match_frame(
                &mut harness,
                &box_frame(LISTS, "shopping-list"),
            )?);
            let add_entry = harness.state().t().add_tally_entry().to_string();
            harness
                .get_all_by_label(&add_entry)
                .next()
                .ok_or("the Tally's Add entry")?
                .scroll_to_me();
            harness.run();
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "tally-entry"))?);
            let (mut harness, _) = window_with_lists(texts, language, theme)?;
            show_household(&mut harness, texts)?;
            let view = harness.state().t().menu_view().to_string();
            let compact = harness.state().t().menu_compact_views().to_string();
            click_last(&mut harness, &view)?;
            click_last(&mut harness, &compact)?;
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "views"))?);
            let (mut harness, lists) = window_with_lists(texts, language, theme)?;
            harness.state_mut().select_note(&lists.packing);
            harness.run();
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "checklist"))?);
            let (mut harness, lists) = window_with_lists(texts, language, theme)?;
            harness.state_mut().select_note(&lists.groceries);
            harness.run();
            let agenda = harness.state().t().checklist_show_on_agenda().to_string();
            harness
                .query_all_by_label_contains(&format!("{agenda}: "))
                .next()
                .ok_or("the Show on the Agenda badge")?
                .click();
            harness.run();
            problems.extend(match_frame(
                &mut harness,
                &box_frame(LISTS, "list-on-agenda"),
            )?);
            // The bike lock's plain Note in the Note dialog, its kind
            // badge open.
            let mut lock = String::new();
            let mut harness = window_with(texts, language, theme, |_, ids| {
                lock = ids[1].clone();
                Ok(())
            })?;
            harness.state_mut().select_note(&lock);
            harness.run();
            let kind = {
                let t = harness.state().t();
                format!("{}: {}", t.badge_kind(), t.kind_plain())
            };
            click_last(&mut harness, &kind)?;
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "kind"))?);
            for (page, label) in [
                ("tally", harness.state().t().tally_heading().to_string()),
                ("session", harness.state().t().session().to_string()),
            ] {
                let (mut harness, _) = window_with_lists(texts, language, theme)?;
                show_household(&mut harness, texts)?;
                harness.get_by_label(texts.add_here).click();
                harness.run();
                click_last(&mut harness, &label)?;
                problems.extend(match_frame(&mut harness, &box_frame(LISTS, page))?);
            }
            let (mut harness, _) = window_with_lists(texts, language, theme)?;
            harness.state_mut().select(Selection::Frogs);
            harness.run();
            problems.extend(match_frame(&mut harness, &box_frame(LISTS, "frog"))?);
            // Formatted text, Transcripts and Photo Texts (#175).
            problems.extend(texts_frames(texts, language, theme, |name| {
                box_frame("formatted-text-transcripts-photo-texts", name)
            })?);
            // Sharing and sync (#176).
            problems.extend(sharing_frames(texts, language, theme, |name| {
                box_frame("sharing-and-sync", name)
            })?);
            // The Settings dialog with this computer's switches, the AI
            // port and the hotkey, over the Agenda.
            let mut harness = window(texts, language, theme)?;
            harness.state_mut().open(Page::Settings);
            problems.extend(match_frame(&mut harness, &frame("settings"))?);
            problems.extend(match_frame(
                &mut harness,
                &box_frame("getting-started", "set-up"),
            )?);
        }
    }
    assert!(
        problems.is_empty(),
        "{}\nRegenerate with tool/guide-frames.sh.",
        problems.join("\n")
    );
    Ok(())
}

/// The window of [`window_with_texts`], held on the heap: a page of
/// scenes keeps the test's stack small.
type TextsWindow = Box<Harness<'static, App>>;

/// A scene over the Household's texts, given its window, the texts' Notes
/// and the words.
type TextsScene = fn(&mut TextsWindow, &TextNotes, &Texts) -> TestResult;

/// The frame `path` of a scene over the Household's texts: `scene` brings
/// a fresh window, given the texts' Notes, to its screen.
fn texts_scene(
    texts: &Texts,
    language: &str,
    theme: egui::Theme,
    path: &str,
    scene: TextsScene,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let (harness, notes) = window_with_texts(texts, language, theme)?;
    let mut harness = Box::new(harness);
    scene(&mut harness, &notes, texts)?;
    match_frame(&mut harness, path)
}

/// The frames of the page "Formatted text, Transcripts and Photo Texts"
/// (#175), in `texts`' words, `language` and `theme`: the boiler's service
/// being edited under the format bar, the script's Text format, the meter
/// readings full screen, a box's History, a #tag's and a word's search,
/// the receipt's Photo Text, the voice note's Transcript with its buttons
/// and the speech model in the Settings.
fn texts_frames(
    texts: &Texts,
    language: &str,
    theme: egui::Theme,
    box_frame: impl Fn(&str) -> String,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let scenes: [(&str, TextsScene); 8] = [
        ("format", |harness, notes, _| {
            harness.state_mut().edit_note(&notes.formatted);
            harness.run();
            Ok(())
        }),
        ("code", |harness, notes, _| {
            harness.state_mut().select_note(&notes.script);
            harness.run();
            let format = {
                let t = harness.state().t();
                format!("{}: {}", t.badge_text_format(), t.text_format_code())
            };
            click_last(harness, &format)
        }),
        ("table", |harness, notes, _| {
            harness.state_mut().select_note(&notes.meters);
            harness.run();
            let open = harness.state().t().table_open().to_string();
            harness
                .query_all_by_role_and_label(Role::Button, &open)
                .last()
                .ok_or("the table's Open the table")?
                .click_accesskit();
            harness.run();
            Ok(())
        }),
        ("box", |harness, _, texts| {
            show_household(harness, texts)?;
            let (checked, history) = {
                let t = harness.state().t();
                (
                    t.text_mark_checked().to_string(),
                    t.history_title().to_string(),
                )
            };
            harness
                .query_all_by_role_and_label(Role::Button, &checked)
                .next()
                .ok_or("the holiday's first box")?
                .click_accesskit();
            harness.run();
            click_last(harness, &history)
        }),
        ("tag", |harness, _, texts| {
            show_household(harness, texts)?;
            let tag = harness.state().t().search_for_tag(texts.tag);
            harness
                .query_all_by_label(&tag)
                .next()
                .ok_or("the boiler's #tag")?
                .click_accesskit();
            harness.run();
            Ok(())
        }),
        ("search", |harness, _, texts| {
            // Ctrl+F on the Agenda: All Notes, the cursor in its search.
            harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
            harness.run();
            harness.get_by(|n| n.is_focused()).type_text(texts.search);
            harness.run();
            Ok(())
        }),
        ("photo-text", |harness, notes, _| {
            harness.state_mut().select_note(&notes.receipt);
            harness.run();
            // The Photo Text under the photo in view, to its last line.
            harness
                .query_all_by_label_contains("Vielen Dank")
                .last()
                .ok_or("the receipt's Photo Text")?
                .scroll_to_me();
            harness.run();
            Ok(())
        }),
        ("generated", |harness, notes, _| {
            harness.state_mut().select_note(&notes.voice);
            harness.run();
            let replace = harness.state().t().replace_note_text().to_string();
            harness.get_by_label(&replace).scroll_to_me();
            harness.run();
            Ok(())
        }),
    ];
    let mut problems = Vec::new();
    for (name, scene) in scenes {
        problems.extend(texts_scene(
            texts,
            language,
            theme,
            &box_frame(name),
            scene,
        )?);
    }
    problems.extend(speech_model_frame(
        texts,
        language,
        theme,
        &box_frame("transcript"),
    )?);
    Ok(problems)
}

/// The frame `path` of the speech model's card in the Settings, none
/// fetched yet.
fn speech_model_frame(
    texts: &Texts,
    language: &str,
    theme: egui::Theme,
    path: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let models = tempfile::tempdir()?;
    let none: Download = Arc::new(|_, _, _, _| ModelOutcome::Mismatch);
    let speech = SpeechModel::with_download(
        ModelDir::new(models.path().join("whisper")),
        none,
        Arc::new(|| {}),
    );
    for _ in 0..100 {
        if speech.status() != Status::Checking {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let mut harness = Box::new(window(texts, language, theme)?);
    harness.state_mut().set_enrichers(None, None, Some(speech));
    harness.state_mut().open(Page::Settings);
    harness.run();
    let title = harness.state().t().speech_model_title().to_string();
    harness
        .query_all_by_label(&title)
        .last()
        .ok_or("the speech model's card")?
        .scroll_to_me();
    harness.run();
    let problem = match_frame(&mut harness, path)?;
    drop(harness);
    models.close()?;
    Ok(problem)
}

/// What [`window_with_people`] seeded: Lena's Profile id, the bike
/// lock's Note in the Household, and the file of the Note she sent,
/// while its folder lasts.
struct People {
    lena: String,
    note: String,
    sent: Option<(tempfile::TempDir, PathBuf)>,
}

/// How a scene of the page "Sharing and sync" starts: Lena in People;
/// with `shared`, Work shared with her from afar as a Member, its
/// changes travelling by bundles; with `sends`, a Note she sent, not
/// encrypted; with `conflict`, the Laptop paired, and it and this
/// computer changed the bike lock's code before they synced.
/// With `devices`, five Devices are paired, one of them Lena's; with
/// `folder`, a shared folder is chosen. Both push the Sync dialog's code
/// out of view, which differs on every run.
#[derive(Clone, Copy, Default)]
struct PeopleSeed {
    shared: bool,
    sends: bool,
    conflict: bool,
    devices: bool,
    folder: bool,
}

/// Lena's store, on a clock of its own and with counted ids: the same
/// Profile id and short code on every run.
fn lena_store() -> oldbrainfix_core::Result<Store> {
    let at = parse_time("2026-10-20T18:00:00.000Z")?;
    let mut lena = Store::in_memory(Box::new(move || at), || Ok(SigningKey::from_seed([47; 32])))?;
    let next = AtomicU64::new(0);
    lena.set_id_maker(Box::new(move || {
        Ok(format!(
            "{:032x}",
            0x1000 + next.fetch_add(1, Ordering::SeqCst)
        ))
    }));
    OwnProfile::new(&lena).apply_defaults("Lena", None)?;
    Ok(lena)
}

/// Everything `from` holds that `to` lacks, taken in as a sync would.
fn send_entries(from: &Store, to: &Store) -> oldbrainfix_core::Result<()> {
    to.apply_entries(
        &oldbrainfix_core::sync::wire(&from.entries_since(&to.version_vector()?)?),
        &from.key_records()?,
        None,
        &mut ImportReport::default(),
    )?;
    Ok(())
}

/// The window over the demo store as Sam's, seeded as `seed` says.
fn window_with_people(
    texts: &Texts,
    language: &str,
    theme: egui::Theme,
    seed: PeopleSeed,
) -> Result<(Harness<'static, App>, People), Box<dyn std::error::Error>> {
    let mut people = People {
        lena: String::new(),
        note: String::new(),
        sent: None,
    };
    let harness = window_with_clock(texts, language, theme, |store, ids, now| {
        OwnProfile::new(store).apply_defaults("Sam", None)?;
        people.note = ids[1].clone();
        let lena = lena_store()?;
        let card = lena.profile_card(false)?.unwrap_or_default();
        people.lena = OwnProfile::new(&lena)
            .get()?
            .map(|p| p.id)
            .unwrap_or_default();
        if seed.sends {
            let party = Notes::new(&lena).capture(texts.sent, None, None)?;
            let at = ClockTime::new(14, 30).into_iter().collect::<Vec<_>>();
            if let Some(saturday) = Schedule::new(Day::new(2026, 10, 31), &at, None) {
                Routines::new(&lena).spawn(&party.id, &saturday)?;
            }
            let bytes = lena.send_note(&party.id, None, false)?;
            let dir =
                tempfile::tempdir().map_err(|e| oldbrainfix_core::Error::Invalid(e.to_string()))?;
            let path = dir.path().join("lena.oldbrain");
            std::fs::write(&path, bytes)
                .map_err(|e| oldbrainfix_core::Error::Invalid(e.to_string()))?;
            people.sent = Some((dir, path));
        }
        store.open_profile_card(&card, false)?;
        if seed.shared {
            let work = Places::new(store)
                .areas()?
                .into_iter()
                .find(|a| a.name == texts.work)
                .map(|a| a.id)
                .unwrap_or_default();
            store.share_area_with(&work, &people.lena, AreaRole::Member, Transport::Bundle)?;
        }
        if seed.devices {
            let at = parse_time("2026-10-26T07:55:00.000Z")?;
            for (n, (name, shared)) in [
                ("Laptop", false),
                ("Phone", false),
                ("Tablet", false),
                ("Work laptop", false),
                ("Lena", true),
            ]
            .into_iter()
            .enumerate()
            {
                store.remember_device(&PairedDevice {
                    id: format!("{:032x}", 0x2000 + n),
                    name: name.into(),
                    fingerprint: String::new(),
                    host: None,
                    port: None,
                    last_sync: at - Duration::minutes(n as i64 * 40),
                    shared,
                })?;
            }
        }
        if seed.folder {
            store.choose_sync_folder("/home/sam/Sync/Old Brain Fix")?;
        }
        if seed.conflict {
            let then = parse_time("2026-10-26T08:00:00.000Z")?;
            let laptop = Store::in_memory(Box::new(move || then), || {
                Ok(SigningKey::from_seed([48; 32]))
            })?;
            send_entries(store, &laptop)?;
            let lock = &ids[1];
            *now.lock() = parse_time("2026-10-26T08:05:00.000Z")?;
            Notes::new(&laptop).edit(lock, texts.lock_there)?;
            Notes::new(store).edit(lock, texts.lock_here)?;
            send_entries(&laptop, store)?;
            store.remember_device(&PairedDevice {
                id: laptop.device_id()?,
                name: "Laptop".into(),
                fingerprint: String::new(),
                host: None,
                port: None,
                last_sync: *now.lock(),
                shared: false,
            })?;
            store.set_conflicts_arrived(true)?;
            *now.lock() = parse_time("2026-10-26T08:30:00.000Z")?;
        }
        Ok(())
    })?;
    Ok((harness, people))
}

/// The window of [`window_with_people`], held on the heap: a page of
/// scenes keeps the test's stack small.
type PeopleWindow = Box<Harness<'static, App>>;

/// A scene of the page "Sharing and sync", given its window, what was
/// seeded and the words.
type PeopleScene = fn(&mut PeopleWindow, &People, &Texts) -> TestResult;

/// Clicks the first button labelled `label`, by its accessibility
/// action, and lets the window settle.
fn click_first(harness: &mut Harness<'static, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .next()
        .ok_or_else(|| format!("no {label}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Opens the context menu of the left pane's Area `name` and clicks its
/// entry `entry`.
fn area_menu(harness: &mut Harness<'static, App>, name: &str, entry: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, name)
        .next()
        .ok_or_else(|| format!("the Area {name}"))?
        .click_secondary();
    harness.run();
    harness
        .query_all_by_label(entry)
        .last()
        .ok_or_else(|| format!("the entry {entry}"))?
        .click();
    harness.run();
    Ok(())
}

/// Opens Lena's Profile from Sync ▸ People.
fn lena_profile(harness: &mut Harness<'static, App>) -> TestResult {
    harness.state_mut().open(Page::People);
    harness.run();
    let open = harness.state().t().person_open("Lena");
    click_first(harness, &open)
}

/// Opens the Sync dialog and scrolls `label` into view. The dialog hosts
/// while it shows and keeps repainting: a few steps, not a settled run.
fn sync_to(harness: &mut Harness<'static, App>, label: &str) -> TestResult {
    harness.state_mut().open(Page::Sync);
    harness.run_steps(4);
    harness
        .query_all_by_label(label)
        .last()
        .ok_or_else(|| format!("no {label}"))?
        .scroll_to_me();
    harness.run_steps(4);
    Ok(())
}

/// The frames of the page "Sharing and sync" (#176), in `texts`' words,
/// `language` and `theme`, but the AI Task's review, which
/// [`guide_frames`] draws: the own Profile, People, Lena's Profile, the
/// Household's Share dialog and its "Share with…", Work's People, a
/// Note's dialog with Send…, the Sent Item Lena sent, the Sync menu,
/// the Sync dialog's shared folder and bundles, Lena's Profile with
/// "Send changes to Lena", and the announcement of a Conflict.
fn sharing_frames(
    texts: &Texts,
    language: &str,
    theme: egui::Theme,
    box_frame: impl Fn(&str) -> String,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let plain = PeopleSeed::default();
    let shared = PeopleSeed {
        shared: true,
        ..plain
    };
    let synced = PeopleSeed {
        devices: true,
        ..plain
    };
    let scenes: [(&str, PeopleSeed, PeopleScene); 14] = [
        ("send-profile", plain, |harness, _, _| {
            harness.state_mut().open(Page::Profile);
            harness.run();
            Ok(())
        }),
        ("add-person", plain, |harness, _, _| {
            harness.state_mut().open(Page::People);
            harness.run();
            Ok(())
        }),
        ("verify", plain, |harness, _, _| lena_profile(harness)),
        ("share-in-person", plain, |harness, _, texts| {
            let share = harness.state().t().share_area().to_string();
            area_menu(harness, texts.household, &share)
        }),
        ("share-afar", plain, |harness, _, texts| {
            let (share, afar) = {
                let t = harness.state().t();
                (t.share_area().to_string(), t.share_afar_pick().to_string())
            };
            area_menu(harness, texts.household, &share)?;
            click_first(harness, &afar)
        }),
        ("roles", shared, |harness, _, texts| {
            let people = harness.state().t().people_title().to_string();
            area_menu(harness, texts.work, &people)
        }),
        ("share-to-app", plain, |harness, people, _| {
            harness.state_mut().select_note(&people.note);
            harness.run();
            let copy = harness.state().t().copy_as_text().to_string();
            harness
                .query_all_by_label(&copy)
                .last()
                .ok_or("Copy as text")?
                .scroll_to_me();
            harness.run();
            Ok(())
        }),
        ("send-note", plain, |harness, people, _| {
            harness.state_mut().select_note(&people.note);
            harness.run();
            let send = harness.state().t().send_button().to_string();
            click_first(harness, &send)
        }),
        ("receive-note", plain, |harness, people, _| {
            let (_, path) = people.sent.as_ref().ok_or("Lena's Sent Item")?;
            harness.input_mut().dropped_files = vec![egui::DroppedFile {
                path: Some(path.clone()),
                ..Default::default()
            }];
            harness.run();
            Ok(())
        }),
        ("pairing", plain, |harness, _, _| {
            let sync = harness.state().t().menu_sync().to_string();
            harness
                .query_all_by_label(&sync)
                .next()
                .ok_or("the Sync menu")?
                .click();
            harness.run();
            Ok(())
        }),
        ("shared-folder", synced, |harness, _, _| {
            let stay = harness.state().t().sync_settings_stay().to_string();
            sync_to(harness, &stay)
        }),
        ("bundle", synced, |harness, _, _| {
            let stay = harness.state().t().sync_settings_stay().to_string();
            sync_to(harness, &stay)
        }),
        ("send-changes", shared, |harness, _, _| {
            lena_profile(harness)
        }),
        ("conflicts", plain, |_, _, _| Ok(())),
    ];
    let mut problems = Vec::new();
    for (name, seed, scene) in scenes {
        let seed = PeopleSeed {
            sends: name == "receive-note",
            conflict: name == "conflicts",
            folder: name == "shared-folder",
            ..seed
        };
        let (harness, people) = window_with_people(texts, language, theme, seed)?;
        let mut harness = Box::new(harness);
        scene(&mut harness, &people, texts)?;
        problems.extend(match_frame(&mut harness, &box_frame(name))?);
        drop(harness);
        if let Some((dir, _)) = people.sent {
            dir.close()?;
        }
    }
    Ok(problems)
}
