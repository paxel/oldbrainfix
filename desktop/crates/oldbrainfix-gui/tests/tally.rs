//! Tallies in the window (#103, ADR 0028): "Make it a Tally" in a Note's
//! editor opens the field page, whose name is required and whose units
//! show in this Device's system; "Add entry" opens the entry page
//! prefilled with the targets; the Tally part lists the entries newest
//! first, charts each field and edits, deletes and restores entries; a
//! hidden field comes back with its values; the unit setting follows the
//! region until chosen; in a Shared Area the own entries show until
//! everyone's are picked.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::lan::{Joining, answer_sync, answer_sync_inviting, apply_answer_as};
use oldbrainfix_core::lan::{sync_request, sync_request_as};
use oldbrainfix_core::{
    AreaRole, Day, DeviceSettings, Invite, Notes, OwnProfile, PairedDevice, Places, SigningKey,
    Store, Tallies, TallyRule, UnitSystem,
};
use oldbrainfix_gui::app::region_of;
use oldbrainfix_gui::tally_view::moved_to;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// A store on a clock that moves a second per read from Monday noon,
/// with its own Profile named `name`.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// A store with the Note "Morning" in the Area "Sport": the store and
/// the Note's id.
fn with_note() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device(3, "Desk")?;
    let sport = Places::new(&store).add_area("Sport")?;
    let note = Notes::new(&store).capture("Morning", Some(&sport.id), None)?;
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English on Berlin's clock in a metric
/// region, the Note `note` open in its editor.
fn window(store: &Arc<Store>, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(berlin));
    app.set_region(Some("DE"));
    app.select(Selection::All);
    if let Some(note) = note {
        app.select_note(note);
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
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

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn shows_part(harness: &Harness<'_, App>, part: &str) -> bool {
    harness.query_all_by_label_contains(part).next().is_some()
}

/// Clicks the last widget labelled `label`: a dialog's, over the window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

fn click_button(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_accesskit();
    harness.run();
}

fn click_radio(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .click_accesskit();
    harness.run();
}

fn radio_on(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .accesskit_node()
        .toggled()
        == Some(Toggled::True)
}

/// The entry whose row holds `part`.
fn entry_row<'a>(harness: &'a Harness<'_, App>, part: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by(move |n| n.role() == Role::Button && n.label().is_some_and(|l| l.contains(part)))
}

/// Types `text` over what the field labelled `label` holds.
fn type_into(harness: &mut Harness<'_, App>, label: &str, text: &str) {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .type_text(text);
    harness.run();
}

/// What the field labelled `label` holds.
fn holds(harness: &Harness<'_, App>, label: &str) -> Option<String> {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .accesskit_node()
        .value()
}

#[test]
fn make_it_a_tally_asks_for_a_name_and_saves_the_first_field() -> TestResult {
    let (store, note) = with_note()?;
    let mut harness = window(&store, Some(&note));
    click_button(&mut harness, "Make it a Tally");
    assert!(shows(&harness, "New field"));
    // A typed unit until one of the list is picked.
    assert!(radio_on(&harness, "Another unit"));
    click_button(&mut harness, "Save");
    assert!(shows(&harness, "Give the field a name."));
    assert!(!Tallies::new(&store).is_tally(&note)?);

    type_into(&mut harness, "Name", "Push-ups");
    type_into(&mut harness, "Another unit", "times");
    type_into(&mut harness, "Target (optional)", "20");
    click_button(&mut harness, "Save");
    let tally = Tallies::new(&store).by_note(&note)?.ok_or("a Tally")?;
    let field = &tally.fields[0];
    assert_eq!(
        (field.name.as_str(), field.unit.as_str(), field.target),
        ("Push-ups", "times", Some(20.0))
    );
    assert_eq!(field.rule, TallyRule::Sum);
    // The editor shows its Tally part instead of "Make it a Tally".
    assert!(!shows(&harness, "New field"));
    assert!(shows(&harness, "Tally"));
    assert!(shows(&harness, "Add entry"));
    assert!(shows(&harness, "times · Target 20 times"));
    assert!(!shows(&harness, "Make it a Tally"));
    assert!(shows(&harness, "No entries yet."));
    assert!(shows(&harness, "No values yet."));
    Ok(())
}

#[test]
fn a_known_unit_shows_in_imperial_and_its_kind_picks_the_rule() -> TestResult {
    let (store, note) = with_note()?;
    let mut harness = window(&store, Some(&note));
    harness.state_mut().set_region(Some("US"));
    harness.run();
    click_button(&mut harness, "Make it a Tally");
    for unit in ["lb", "oz", "mi", "ft", "in", "mph", "°F"] {
        assert!(
            harness
                .query_all_by_role_and_label(Role::RadioButton, unit)
                .next()
                .is_some(),
            "{unit}"
        );
    }
    assert!(shows(&harness, "Mass"), "the kind under the unit");
    assert!(radio_on(&harness, "Their sum"));
    click_radio(&mut harness, "lb");
    assert!(radio_on(&harness, "The largest"), "mass counts the largest");
    type_into(&mut harness, "Name", "Lift");
    type_into(&mut harness, "Target (optional)", "100");
    click_button(&mut harness, "Save");
    let tally = Tallies::new(&store).by_note(&note)?.ok_or("a Tally")?;
    let field = &tally.fields[0];
    assert_eq!(field.unit, "kg", "stored in metric");
    assert_eq!(field.rule, TallyRule::Max);
    assert!((field.target.ok_or("a target")? - 45.359237).abs() < 1e-9);
    assert!(shows(&harness, "lb · Target 100 lb"));
    Ok(())
}

#[test]
fn a_target_that_is_no_number_says_so_under_its_field() -> TestResult {
    let (store, note) = with_note()?;
    let mut harness = window(&store, Some(&note));
    click_button(&mut harness, "Make it a Tally");
    type_into(&mut harness, "Name", "Pages");
    type_into(&mut harness, "Target (optional)", "a few");
    click_button(&mut harness, "Save");
    assert!(shows(
        &harness,
        "Not a number. Type it like 12, 2.5 or 1:30."
    ));
    assert!(!Tallies::new(&store).is_tally(&note)?);
    // Cancel changes nothing.
    click_button(&mut harness, "Cancel");
    assert!(shows(&harness, "Make it a Tally"));
    assert!(!Tallies::new(&store).is_tally(&note)?);
    Ok(())
}

#[test]
fn add_entry_prefills_the_targets_and_one_click_saves_them() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    tallies.add_field(&note, "Push-ups", "times", Some(20.0), None)?;
    tallies.add_field(&note, "Run", "km", None, None)?;
    let mut harness = window(&store, Some(&note));
    click_button(&mut harness, "Add entry");
    assert!(shows(&harness, "New entry"));
    assert_eq!(holds(&harness, "Push-ups").as_deref(), Some("20"));
    assert_eq!(holds(&harness, "Run").as_deref(), Some(""));
    assert!(shows(&harness, "Mon, 3/2/2026"));
    click_button(&mut harness, "Save");
    let entries = tallies.entries(&note, &Default::default())?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].values,
        BTreeMap::from([(
            tallies.by_note(&note)?.ok_or("a Tally")?.fields[0]
                .id
                .clone(),
            20.0
        )])
    );
    assert!(shows_part(&harness, "Push-ups 20 times"));
    assert!(shows(&harness, "Chart of Push-ups, 1 bars"));
    assert!(shows(&harness, "Highest bar: 20 times"));
    assert!(shows(&harness, "3/2/2026 to 3/2/2026"));
    // Run holds no value yet.
    assert!(shows(&harness, "No values yet."));
    Ok(())
}

#[test]
fn an_entry_with_nothing_in_it_asks_for_a_value_or_a_line() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    tallies.add_field(&note, "Pages", "pages", None, None)?;
    let mut harness = window(&store, Some(&note));
    click_button(&mut harness, "Add entry");
    click_button(&mut harness, "Save");
    assert!(shows(&harness, "Enter a value or a line of text."));
    type_into(&mut harness, "Pages", "1,5");
    type_into(&mut harness, "Line of text", "Chapter one");
    click_button(&mut harness, "Save");
    let entries = tallies.entries(&note, &Default::default())?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].line, "Chapter one");
    assert_eq!(
        entries[0].values.values().copied().collect::<Vec<_>>(),
        [1.5]
    );
    assert!(shows_part(&harness, "Chapter one"));
    Ok(())
}

#[test]
fn editing_an_entry_writes_only_what_was_retyped_and_keeps_its_time_of_day() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    let lift = tallies.add_field(&note, "Lift", "kg", None, None)?;
    let reps = tallies.add_field(&note, "Reps", "times", None, None)?;
    let values = BTreeMap::from([(lift.clone(), 82.5), (reps.clone(), 8.0)]);
    let entry = tallies.add(&note, &values, "", None)?;
    let mut harness = window(&store, Some(&note));
    entry_row(&harness, "Lift 82.5 kg").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Entry"));
    assert_eq!(holds(&harness, "Lift").as_deref(), Some("82.5"));
    type_into(&mut harness, "Reps", "10");
    click_button(&mut harness, "Day before");
    assert!(shows(&harness, "Sun, 3/1/2026"));
    click_button(&mut harness, "Save");
    let edited = tallies.entry_by_id(&entry.id)?.ok_or("the entry")?;
    assert_eq!(edited.values.get(&reps), Some(&10.0));
    assert_eq!(edited.at, entry.at - Duration::days(1));
    let lift_key = format!("v:{lift}");
    let written = store
        .history(&entry.id)?
        .into_iter()
        .filter(|e| e.field == lift_key)
        .count();
    assert_eq!(written, 1, "the value not retyped stays as it was");
    Ok(())
}

#[test]
fn a_day_moved_across_a_change_of_the_clocks_keeps_its_time_of_day() -> TestResult {
    // Noon in Berlin on summer time, moved back onto winter time.
    let at = DateTime::parse_from_rfc3339("2026-03-30T10:00:00Z")?.with_timezone(&Utc);
    let moved = moved_to(at, Day::new(2026, 3, 28), &berlin);
    assert_eq!(moved.to_rfc3339(), "2026-03-28T11:00:00+00:00");
    let back = moved_to(moved, Day::new(2026, 3, 30), &berlin);
    assert_eq!(back, at);
    Ok(())
}

#[test]
fn deleting_an_entry_asks_once_and_removed_entries_bring_it_back() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    let pages = tallies.add_field(&note, "Pages", "pages", None, None)?;
    let entry = tallies.add(&note, &BTreeMap::from([(pages, 12.0)]), "Book", None)?;
    let mut harness = window(&store, Some(&note));
    assert!(!shows(&harness, "Removed entries"));
    entry_row(&harness, "Book").click_secondary();
    harness.run();
    assert!(shows(&harness, "Edit"));
    click(&mut harness, "Delete")?;
    assert!(shows(
        &harness,
        "Delete this entry? It can be restored under \"Removed entries\"."
    ));
    assert!(
        !tallies
            .entry_by_id(&entry.id)?
            .ok_or("the entry")?
            .is_trashed()
    );
    click(&mut harness, "Delete")?;
    assert!(
        tallies
            .entry_by_id(&entry.id)?
            .ok_or("the entry")?
            .is_trashed()
    );
    assert!(shows(&harness, "No entries yet."));
    // Folded, closed until opened, and remembered on this Device.
    assert!(shows(&harness, "Removed entries"));
    assert!(!shows(&harness, "Restore"));
    click(&mut harness, "Removed entries")?;
    assert_eq!(
        store.local_setting("tallyRemovedOpen")?.as_deref(),
        Some("true")
    );
    click_button(&mut harness, "Restore");
    assert!(
        !tallies
            .entry_by_id(&entry.id)?
            .ok_or("the entry")?
            .is_trashed()
    );
    assert!(!shows(&harness, "Removed entries"));
    Ok(())
}

#[test]
fn the_entry_page_deletes_after_asking_too() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    tallies.add_field(&note, "Pages", "pages", None, None)?;
    let entry = tallies.add(&note, &BTreeMap::new(), "Skimmed", None)?;
    let mut harness = window(&store, Some(&note));
    entry_row(&harness, "Skimmed").click_accesskit();
    harness.run();
    click_button(&mut harness, "Delete");
    click(&mut harness, "Delete")?;
    assert!(
        tallies
            .entry_by_id(&entry.id)?
            .ok_or("the entry")?
            .is_trashed()
    );
    assert!(!shows(&harness, "Entry"), "the page closes");
    Ok(())
}

#[test]
fn a_hidden_field_returns_with_its_values() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    let pages = tallies.add_field(&note, "Pages", "pages", None, None)?;
    tallies.add(&note, &BTreeMap::from([(pages.clone(), 30.0)]), "", None)?;
    let mut harness = window(&store, Some(&note));
    assert!(shows(&harness, "Chart of Pages, 1 bars"));
    click_button(&mut harness, "Pages");
    assert!(shows(&harness, "Field"));
    assert!(shows(&harness, "Hide field"));
    click_button(&mut harness, "Hide field");
    assert!(tallies.by_note(&note)?.ok_or("a Tally")?.fields[0].hidden);
    assert!(shows(&harness, "pages · Hidden"));
    assert!(!shows(&harness, "Chart of Pages, 1 bars"));
    click_button(&mut harness, "Pages");
    click_button(&mut harness, "Show field again");
    assert!(!tallies.by_note(&note)?.ok_or("a Tally")?.fields[0].hidden);
    assert!(shows(&harness, "Chart of Pages, 1 bars"));
    assert!(shows_part(&harness, "Pages 30 pages"));
    Ok(())
}

#[test]
fn once_entries_hold_values_only_units_of_the_same_kind_are_offered() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    let lift = tallies.add_field(&note, "Lift", "kg", None, None)?;
    tallies.add(&note, &BTreeMap::from([(lift, 80.0)]), "", None)?;
    let mut harness = window(&store, Some(&note));
    click_button(&mut harness, "Lift");
    assert!(shows(
        &harness,
        "Entries hold values for this field, so its kind of unit stays."
    ));
    let offered = |harness: &Harness<'_, App>, unit: &str| {
        harness
            .query_all_by_role_and_label(Role::RadioButton, unit)
            .next()
            .is_some()
    };
    assert!(offered(&harness, "kg") && offered(&harness, "g"));
    for other in ["km", "s", "Another unit"] {
        assert!(!offered(&harness, other), "{other}");
    }
    click_radio(&mut harness, "g");
    click_button(&mut harness, "Save");
    assert_eq!(
        tallies.by_note(&note)?.ok_or("a Tally")?.fields[0].unit,
        "g"
    );
    assert!(shows_part(&harness, "Lift 80000 g"));
    Ok(())
}

#[test]
fn the_bars_follow_the_length_picked_and_show_more_shows_the_next_entries() -> TestResult {
    let (store, note) = with_note()?;
    let tallies = Tallies::new(&store);
    let pages = tallies.add_field(&note, "Pages", "pages", None, None)?;
    let now = store.now();
    // On ten days, each entry at an instant of its own.
    for day in 0..31 {
        let at = now - Duration::days(day % 10) - Duration::seconds(day);
        let values = BTreeMap::from([(pages.clone(), 1.0)]);
        tallies.add(&note, &values, &format!("Read #{day}#"), Some(at))?;
    }
    let mut harness = window(&store, Some(&note));
    assert!(radio_on(&harness, "Day"));
    assert!(shows(&harness, "Chart of Pages, 10 bars"));
    assert!(shows(&harness, "Highest bar: 4 pages"));
    // Newest first, 30 at a time: the oldest waits.
    assert!(shows_part(&harness, "#30#"));
    assert!(!shows_part(&harness, "#29#"));
    let first = entry_row(&harness, "#0#").rect().top();
    assert!(first < entry_row(&harness, "#1#").rect().top());
    click_button(&mut harness, "Show more");
    assert!(shows_part(&harness, "#29#"));
    assert!(!shows(&harness, "Show more"));
    click_radio(&mut harness, "Month");
    // From the 21st of February to the 2nd of March.
    assert!(shows(&harness, "Chart of Pages, 2 bars"));
    assert!(shows(&harness, "Feb 2026 to Mar 2026"));
    assert!(shows(&harness, "Highest bar: 24 pages"));
    Ok(())
}

#[test]
fn the_unit_setting_follows_the_region_until_a_system_is_chosen() -> TestResult {
    let store = Arc::new(device(4, "Desk")?);
    let mut harness = window(&store, None);
    harness.state_mut().set_region(Some("US"));
    harness.state_mut().select(Selection::Settings);
    harness.run();
    assert!(shows(&harness, "Units"));
    assert!(radio_on(&harness, "As this computer's region: imperial"));
    click_radio(&mut harness, "Metric (kg, km, °C)");
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.chosen_unit_system()?, Some(UnitSystem::Metric));
    assert!(radio_on(&harness, "Metric (kg, km, °C)"));
    click_radio(&mut harness, "As this computer's region: imperial");
    assert_eq!(settings.chosen_unit_system()?, None);
    Ok(())
}

#[test]
fn the_region_is_read_from_the_systems_locale() {
    assert_eq!(region_of(Some("en-US")).as_deref(), Some("US"));
    assert_eq!(region_of(Some("de_DE.UTF-8")).as_deref(), Some("DE"));
    assert_eq!(region_of(Some("zh-Hans-CN")).as_deref(), Some("CN"));
    assert_eq!(region_of(Some("en")), None);
    assert_eq!(region_of(None), None);
}

/// Ann's Household with her Note "Push-ups" in it, a Tally, shared with
/// Ben as a Member: both stores and the Note's id.
fn shared() -> Result<(Arc<Store>, Store, String), Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let note = Notes::new(&ann).capture("Push-ups", Some(&house.id), None)?;
    Tallies::new(&ann).add_field(&note.id, "Count", "times", None, None)?;
    let area = ann.share(&house.id)?;
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
    for (store, peer) in [(&ann, &ben), (&ben, &ann)] {
        store.remember_device(&PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared: true,
        })?;
    }
    Ok((Arc::new(ann), ben, note.id))
}

#[test]
fn in_a_shared_area_the_own_entries_show_until_everyones_are_picked() -> TestResult {
    let (ann, ben, note) = shared()?;
    let count = Tallies::new(&ann).by_note(&note)?.ok_or("a Tally")?.fields[0]
        .id
        .clone();
    Tallies::new(&ann).add(
        &note,
        &BTreeMap::from([(count.clone(), 10.0)]),
        "Ann's",
        None,
    )?;
    Tallies::new(&ben).add(&note, &BTreeMap::from([(count, 15.0)]), "Ben's", None)?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    let mut harness = window(&ann, Some(&note));
    assert!(shows(&harness, "Show entries"));
    assert!(radio_on(&harness, "Mine"));
    assert!(shows_part(&harness, "Ann's"));
    assert!(!shows_part(&harness, "Ben's"));
    assert!(shows(&harness, "Highest bar: 10 times"));
    click_radio(&mut harness, "Everyone's");
    assert!(shows_part(&harness, "Ann's"));
    assert!(shows_part(&harness, "Ben · Count 15 times · Ben's"));
    // Both on one day: their sum.
    assert!(shows(&harness, "Highest bar: 25 times"));
    click_radio(&mut harness, "Ben");
    assert!(!shows_part(&harness, "Ann's"));
    assert!(shows_part(&harness, "Ben's"));
    Ok(())
}
