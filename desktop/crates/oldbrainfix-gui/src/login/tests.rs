use super::*;
use oldbrainfix_core::SigningKey;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn new_store() -> std::result::Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

#[test]
fn the_gecos_field_gives_the_full_name() {
    assert_eq!(gecos_name("Ada Lovelace,,,", "ada"), "Ada Lovelace");
    assert_eq!(gecos_name("& Lovelace,Room 1", "ada"), "Ada Lovelace");
    assert_eq!(gecos_name("", "ada"), "");
}

#[test]
fn the_login_name_comes_first_then_the_computer() -> TestResult {
    let store = new_store()?;
    let login = Login {
        name: "Ada Lovelace".into(),
        picture: None,
    };
    assert_eq!(apply_defaults(&store, &login, "desk")?.name, "Ada Lovelace");
    let other = new_store()?;
    let nameless = Login {
        name: " \u{200B}".into(),
        picture: Some(b"no picture".to_vec()),
    };
    let made = apply_defaults(&other, &nameless, "desk")?;
    assert_eq!((made.name.as_str(), made.picture), ("desk", None));
    Ok(())
}

#[test]
fn the_account_picture_becomes_the_profile_picture() -> TestResult {
    let image = image::RgbImage::from_pixel(96, 64, image::Rgb([200, 30, 30]));
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image).write_to(&mut png, image::ImageFormat::Png)?;
    let login = Login {
        name: "Ada".into(),
        picture: Some(png.into_inner()),
    };
    let store = new_store()?;
    let picture = apply_defaults(&store, &login, "desk")?
        .picture
        .ok_or("a picture")?;
    assert_eq!(profile::picture_size(&picture), Some((256, 256)));
    Ok(())
}

#[test]
fn this_computer_gives_a_name_without_asking() -> TestResult {
    let store = new_store()?;
    let made = apply_system_defaults(&store)?;
    assert!(!made.name.is_empty());
    Ok(())
}

#[test]
fn no_profile_is_made_at_launch_before_the_first_start_is_answered() -> TestResult {
    let store = new_store()?;
    assert!(apply_at_launch(&store)?.is_none());
    assert!(OwnProfile::new(&store).get()?.is_none());
    crate::backup_view::restore_answered(&store)?;
    assert!(apply_at_launch(&store)?.is_some());
    assert!(OwnProfile::new(&store).get()?.is_some());
    Ok(())
}
