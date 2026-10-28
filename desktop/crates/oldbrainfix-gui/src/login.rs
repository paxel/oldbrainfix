//! What the user's own Profile starts with on a computer (ADR 0017): the
//! full name and account picture of the computer login, else the
//! computer's name and the initials. Nothing is asked.

use oldbrainfix_core::profile::{self, OwnProfile, Profile};
use oldbrainfix_core::{Result, Store};

use crate::sync_view::device_name;

/// The login's full name, empty when it has none, and its account
/// picture as the file holds it, when one is found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Login {
    pub name: String,
    pub picture: Option<Vec<u8>>,
}

/// The login of whoever runs the app, as the system knows it.
pub fn login() -> Login {
    Login {
        name: full_name().unwrap_or_default(),
        picture: account_picture(),
    }
}

/// Makes the own Profile from `login` the first time, else takes the
/// defaults the user has not replaced: the login's full name or, without
/// one, `computer`'s name; its account picture, made a Profile picture,
/// or the initials.
pub fn apply_defaults(store: &Store, login: &Login, computer: &str) -> Result<Profile> {
    let name = if profile::sanitize_name(&login.name).is_empty() {
        computer
    } else {
        login.name.as_str()
    };
    let picture = login.picture.as_deref().and_then(profile::square_picture);
    OwnProfile::new(store).apply_defaults(name, picture)
}

/// [`apply_defaults`] with this computer's login and name.
pub fn apply_system_defaults(store: &Store) -> Result<Profile> {
    apply_defaults(store, &login(), &device_name())
}

/// [`apply_system_defaults`] at launch, but not while the first start's
/// view is due: a Device being added would hold a fresh Profile of its
/// own when it pairs, which can take over the one of the user's other
/// Devices (ADR 0017). The view makes it once answered.
pub fn apply_at_launch(store: &Store) -> Result<Option<Profile>> {
    if crate::backup_view::restore_due(store)? {
        return Ok(None);
    }
    apply_system_defaults(store).map(Some)
}

/// The full name in a passwd GECOS field: the part before the first
/// comma; an `&` stands for the login name, capitalised.
pub fn gecos_name(gecos: &str, login: &str) -> String {
    let full = gecos.split(',').next().unwrap_or_default().trim();
    let mut capitalised = login.chars();
    let login = capitalised
        .next()
        .map(|c| c.to_uppercase().chain(capitalised).collect::<String>())
        .unwrap_or_default();
    full.replace('&', &login)
}

/// The login's full name from the user database (`getpwuid_r`): on Linux
/// what AccountsService and the login screen show, inside the Flatpak
/// the copy of it the sandbox gets; on macOS the account's full name.
#[cfg(unix)]
fn full_name() -> Option<String> {
    let (gecos, login) = passwd()?;
    Some(gecos_name(&gecos, &login)).filter(|n| !n.is_empty())
}

/// The GECOS field and the login name of the user running the app.
#[cfg(unix)]
fn passwd() -> Option<(String, String)> {
    use std::ffi::CStr;
    let mut buffer = vec![0 as libc::c_char; 16 * 1024];
    // SAFETY: getpwuid_r writes `entry` and the strings it points to
    // into `buffer`, both alive and sized as passed, and sets `found`
    // to `entry` or null; nothing is read unless it did.
    unsafe {
        let mut entry: libc::passwd = std::mem::zeroed();
        let mut found: *mut libc::passwd = std::ptr::null_mut();
        let code = libc::getpwuid_r(
            libc::getuid(),
            &mut entry,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut found,
        );
        if code != 0 || found.is_null() || entry.pw_name.is_null() {
            return None;
        }
        let login = CStr::from_ptr(entry.pw_name).to_string_lossy().into_owned();
        let gecos = if entry.pw_gecos.is_null() {
            String::new()
        } else {
            CStr::from_ptr(entry.pw_gecos)
                .to_string_lossy()
                .into_owned()
        };
        Some((gecos, login))
    }
}

/// The login's display name (`GetUserNameExW`, `NameDisplay`); none for
/// a local account that has no full name.
#[cfg(windows)]
fn full_name() -> Option<String> {
    use windows_sys::Win32::Security::Authentication::Identity::{GetUserNameExW, NameDisplay};
    let mut size: u32 = 256;
    let mut buffer = vec![0u16; size as usize];
    // SAFETY: the buffer holds `size` UTF-16 units; on success the call
    // writes the name there and sets `size` to its length without the
    // closing zero.
    let ok = unsafe { GetUserNameExW(NameDisplay, buffer.as_mut_ptr(), &mut size) };
    if !ok {
        return None;
    }
    buffer.truncate(size as usize);
    Some(String::from_utf16_lossy(&buffer).trim().to_string()).filter(|n| !n.is_empty())
}

#[cfg(not(any(unix, windows)))]
fn full_name() -> Option<String> {
    None
}

/// The login's account picture: on Linux the one AccountsService keeps
/// for the login screen, else `~/.face`, as KDE and GNOME set them. Not
/// reachable inside the Flatpak, and not looked for on Windows and
/// macOS, where the initials stand in.
#[cfg(target_os = "linux")]
fn account_picture() -> Option<Vec<u8>> {
    let (_, login) = passwd()?;
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let mut places = vec![std::path::PathBuf::from("/var/lib/AccountsService/icons").join(&login)];
    if let Some(home) = home {
        places.push(home.join(".face"));
        places.push(home.join(".face.icon"));
    }
    places
        .into_iter()
        .filter_map(|path| std::fs::read(path).ok())
        // A picture far larger than an account picture is none.
        .find(|bytes| !bytes.is_empty() && bytes.len() <= 8 * 1024 * 1024)
}

#[cfg(not(target_os = "linux"))]
fn account_picture() -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod tests {
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
}
