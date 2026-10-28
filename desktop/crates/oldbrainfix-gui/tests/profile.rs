//! The own Profile driven headless (ADR 0017): its picture where the
//! settings gear was, the Profile page in its order, the name typed, a
//! picture chosen and its square picked, and the way on to Backups, the
//! user's Devices and this Device's settings.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::profile::{self, OwnProfile};
use oldbrainfix_core::{SigningKey, Store};
use oldbrainfix_gui::login::{self, Login};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store whose own Profile came from the login "Ada Lovelace".
fn store() -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([4; 32]))
    })?;
    let ada = Login {
        name: "Ada Lovelace".into(),
        picture: None,
    };
    login::apply_defaults(&store, &ada, "desk")?;
    Ok(store)
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
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
    harness.query_all_by_label_contains(label).next().is_some()
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_label(label).click_accesskit();
    // The Sync view keeps drawing while it hosts.
    harness.step();
    harness.step();
}

/// A PNG of `width` × `height`, red on the left half, blue on the right.
fn halves(width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image = image::RgbImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    });
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image).write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

#[test]
fn the_profile_picture_sits_where_the_gear_was_and_opens_the_profile_page() -> TestResult {
    let mut harness = window(App::new(store()?, "en"));
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    assert!(!shows(&harness, "Settings"));
    click(&mut harness, "Profile");
    assert_eq!(harness.state().selection(), &Selection::Profile);

    // Picture, name and code; then Backups, the Devices and the settings.
    let code = OwnProfile::new(harness.state().store())
        .code()?
        .ok_or("a code")?;
    assert_eq!(code.len(), 9);
    let top = |label: &str| harness.get_by_label_contains(label).rect().top();
    let order = [
        top("Change the picture"),
        harness
            .get_by_role_and_label(Role::TextInput, "Name")
            .rect()
            .top(),
        top(&code),
        top("Backups and recovery"),
        top("Your Devices"),
        top("Settings of this Device"),
    ];
    let mut sorted = order;
    sorted.sort_by(f32::total_cmp);
    assert_eq!(order, sorted);
    assert!(harness.query_all_by_value("Ada Lovelace").next().is_some());
    assert!(shows(&harness, "Set up Backups"));

    // The Devices and the settings are a click away, and back leads here.
    click(&mut harness, "Sync with another Device");
    assert_eq!(harness.state().selection(), &Selection::Sync);
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.step();
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Profile);
    click(&mut harness, "Settings");
    assert_eq!(harness.state().selection(), &Selection::Settings);
    // Sync and Backups left the settings.
    assert!(!shows(&harness, "Sync with another Device"));
    assert!(!shows(&harness, "Set up Backups"));
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Profile);
    Ok(())
}

#[test]
fn the_login_name_gives_the_initials_of_the_placeholder() -> TestResult {
    let store = store()?;
    let profile = OwnProfile::new(&store).get()?.ok_or("a Profile")?;
    assert_eq!(profile::initials(&profile.name), "AL");
    let harness = window(App::new(store, "en"));
    assert!(shows(&harness, "Profile"));
    Ok(())
}

#[test]
fn a_typed_name_is_kept_sanitised() -> TestResult {
    let mut app = App::new(store()?, "en");
    app.select(Selection::Profile);
    let mut harness = window(app);
    harness
        .get_by_role_and_label(Role::TextInput, "Name")
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness
        .get_by(|n| n.is_focused())
        .type_text("\u{202E}Grace\u{200B} Hopper ");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let kept = OwnProfile::new(harness.state().store())
        .get()?
        .ok_or("a Profile")?;
    assert_eq!((kept.name.as_str(), kept.version), ("Grace Hopper", 2));
    assert!(harness.query_all_by_value("Grace Hopper").next().is_some());

    // An empty name shows as "No name".
    harness
        .get_by_role_and_label(Role::TextInput, "Name")
        .click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness.key_press(egui::Key::Backspace);
    harness.key_press(egui::Key::Enter);
    harness.run();
    let kept = OwnProfile::new(harness.state().store())
        .get()?
        .ok_or("a Profile")?;
    assert_eq!(kept.name, "");
    // The empty field shows "No name"; found or it fails.
    harness.get_by(|n| n.placeholder() == Some("No name"));
    Ok(())
}

#[test]
fn a_chosen_picture_is_cropped_to_a_square_and_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("me.png");
    std::fs::write(&file, halves(600, 400)?)?;
    let mut app = App::new(store()?, "en");
    let picked = file.clone();
    app.set_picture_picker(Box::new(move |title| {
        assert_eq!(title, "Choose a picture");
        Some(picked.clone())
    }));
    app.select(Selection::Profile);
    let mut harness = window(app);
    click(&mut harness, "Choose a picture");
    assert!(shows(&harness, "Choose the square"));
    // Leaving keeps the picture as it was.
    click(&mut harness, "Cancel");
    assert!(!shows(&harness, "Choose the square"));
    assert_eq!(
        OwnProfile::new(harness.state().store())
            .get()?
            .ok_or("a Profile")?
            .picture,
        None
    );

    click(&mut harness, "Choose a picture");
    click(&mut harness, "Use this part");
    let kept = OwnProfile::new(harness.state().store())
        .get()?
        .ok_or("a Profile")?;
    let picture = kept.picture.ok_or("a picture")?;
    assert_eq!(&picture[..2], &[0xFF, 0xD8]);
    assert_eq!(profile::picture_size(&picture), Some((256, 256)));
    assert_eq!(kept.version, 2);

    // And back to the initials.
    click(&mut harness, "Show initials instead");
    let kept = OwnProfile::new(harness.state().store())
        .get()?
        .ok_or("a Profile")?;
    assert_eq!((kept.picture, kept.version), (None, 3));
    Ok(())
}

#[test]
fn a_file_that_is_no_picture_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("notes.png");
    std::fs::write(&file, b"not a picture")?;
    let mut app = App::new(store()?, "en");
    app.set_picture_picker(Box::new(move |_| Some(file.clone())));
    app.select(Selection::Profile);
    let mut harness = window(app);
    click(&mut harness, "Choose a picture");
    assert!(shows(&harness, "This picture cannot be read."));
    assert!(!shows(&harness, "Choose the square"));
    Ok(())
}

#[test]
fn the_profile_page_speaks_german() -> TestResult {
    let mut app = App::new(store()?, "de");
    app.select(Selection::Profile);
    let harness = window(app);
    for label in [
        "Profil",
        "Dein Code",
        "Sicherungen und Wiederherstellung",
        "Deine Geräte",
        "Einstellungen dieses Geräts",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    Ok(())
}

// ------------------------------------------- between one's own Devices

/// A phone of the same user with a Profile of its own, named by the user.
fn phone(name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([5; 32]))
    })?;
    let own = OwnProfile::new(&store);
    own.apply_defaults("Pixel", None)?;
    own.rename(name)?;
    Ok(store)
}

/// The phone joins the desktop's host once, as Pairing does.
fn pair(phone: &Store, desk: &Store) -> TestResult {
    use oldbrainfix_core::lan::{answer_sync, apply_answer, sync_request};
    let request = sync_request(phone, &desk.version_vector()?, "Phone", "", "")?;
    let (answer, _) = answer_sync(desk, &request, "Desk")?.ok_or("an answer")?;
    apply_answer(phone, &answer)?;
    Ok(())
}

/// The desktop's Profile named Ada by the user, paired with a phone whose
/// Profile is Ada L.: "Which one is you?" waits on both.
fn met() -> Result<(Store, Store), Box<dyn std::error::Error>> {
    let desk = store()?;
    OwnProfile::new(&desk).rename("Ada")?;
    let phone = phone("Ada L.")?;
    pair(&phone, &desk)?;
    Ok((desk, phone))
}

fn id(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

#[test]
fn two_profiles_meeting_ask_which_one_is_you_showing_both() -> TestResult {
    let (desk, phone) = met()?;
    let phone_code = OwnProfile::new(&phone).code()?.ok_or("a code")?;
    let desk_code = OwnProfile::new(&desk).code()?.ok_or("a code")?;
    let mut harness = window(App::new(desk, "en"));
    assert!(shows(&harness, "Which one is you?"));
    for label in [
        "Ada",
        "Ada L.",
        phone_code.as_str(),
        desk_code.as_str(),
        "On this Device",
        "On the other Device",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    // The phone's Profile is me: this Device takes it over.
    harness
        .get_all_by_label("This is me")
        .nth(1)
        .ok_or("two choices")?
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(!shows(&harness, "Which one is you?"));
    let store = harness.state().store();
    assert_eq!(id(store)?, id(&phone)?);
    assert_eq!(OwnProfile::new(store).code()?, Some(phone_code));
    Ok(())
}

#[test]
fn later_leaves_the_question_on_the_profile_page() -> TestResult {
    let (desk, phone) = met()?;
    let mut harness = window(App::new(desk, "en"));
    click(&mut harness, "Later");
    assert!(!shows(&harness, "On the other Device"));
    assert!(
        OwnProfile::new(harness.state().store())
            .question()?
            .is_some()
    );

    click(&mut harness, "Profile");
    assert!(shows(
        &harness,
        "Your Devices have two Profiles. Pick yours."
    ));
    click(&mut harness, "Which one is you?");
    assert!(shows(&harness, "On the other Device"));
    harness
        .get_all_by_label("This is me")
        .next()
        .ok_or("two choices")?
        .click_accesskit();
    harness.step();
    harness.step();
    let store = harness.state().store();
    assert!(OwnProfile::new(store).question()?.is_none());
    assert_eq!(OwnProfile::new(store).aliases()?, vec![id(&phone)?]);
    assert!(!shows(
        &harness,
        "Your Devices have two Profiles. Pick yours."
    ));
    Ok(())
}

#[test]
fn a_name_changed_on_the_phone_shows_here_at_once() -> TestResult {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    let desk = Arc::new(store()?);
    let phone = phone("Ada")?;
    pair(&phone, &desk)?;
    let changes = Arc::new(AtomicU64::new(0));
    let mut app = App::new(desk.clone(), "en");
    app.watch_changes(changes.clone());
    app.select(Selection::Profile);
    let mut harness = window(app);
    assert!(harness.query_all_by_value("Ada").next().is_some());

    OwnProfile::new(&phone).rename("Ada Lovelace")?;
    pair(&phone, &desk)?;
    changes.fetch_add(1, Ordering::SeqCst);
    harness.run();
    assert!(harness.query_all_by_value("Ada Lovelace").next().is_some());
    Ok(())
}
