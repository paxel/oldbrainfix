//! Links to places in the app (#261, ADR 0046), as `app_link.dart`: an
//! ordinary Markdown link whose destination has the app's scheme,
//! `oldbrainfix://`, such as `oldbrainfix://note/<id>` or
//! `oldbrainfix://settings/backups`. Ids are the same on every Device, so
//! a link survives sync; a plain Markdown app shows its label. [`parse`]
//! knows every form; anything else with the scheme is no app link and
//! opens nothing.

/// The app's scheme with its `//`.
pub const SCHEME: &str = "oldbrainfix://";

/// The website the user guide is on, a language's folder and `guide/`
/// after it.
pub const SITE: &str = "https://paxel.github.io/oldbrainfix";

/// The sections of the Settings a link can open, on either app; one
/// this app has not opens the Settings at their top.
pub const SETTINGS_SECTIONS: [&str; 14] = [
    "look",
    "units",
    "agenda",
    "alarms",
    "nudges",
    "capture",
    "quick-note",
    "calendar",
    "privacy",
    "typing",
    "history",
    "import",
    "features",
    "computer",
];

/// What an app link opens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppLink {
    /// A Note, by its id.
    Note(String),
    /// An Area or a Topic, by its id.
    Place(String),
    Agenda,
    Unsorted,
    /// The Settings, or one of their [`SETTINGS_SECTIONS`].
    Settings(Option<String>),
    /// All Routines and Reminders.
    Routines,
    Backups,
    Sync,
    Profile,
    Frogs,
    NewSession,
    /// A page of the user guide on the website and its box, the start
    /// page without one.
    Guide(Option<String>, Option<String>),
    /// A file in the app's shared folder, by its path inside it, its
    /// folders separated by `/`.
    File(String),
}

/// The link of `url`, or `None` when it is none: the scheme in any case
/// of ASCII letters, then one of the forms [`AppLink::url`] writes, as
/// written. No query; a `#` only after a guide's page. A file's path has
/// no empty part, no `.` or `..`, and no `\`, `:` or control character,
/// so it never leads out of the shared folder.
pub fn parse(url: &str) -> Option<AppLink> {
    let head = url.get(..SCHEME.len())?;
    if !head.eq_ignore_ascii_case(SCHEME) {
        return None;
    }
    let rest = url.get(SCHEME.len()..)?;
    if rest.contains('?') {
        return None;
    }
    let (rest, anchor) = match rest.split_once('#') {
        Some((rest, anchor)) => (rest, Some(anchor)),
        None => (rest, None),
    };
    let parts: Vec<&str> = rest.split('/').collect();
    if anchor.is_some() && !matches!(parts.as_slice(), ["guide", _]) {
        return None;
    }
    match parts.as_slice() {
        ["note", id] => Some(AppLink::Note(decode(id).filter(|id| !id.is_empty())?)),
        ["area", id] => Some(AppLink::Place(decode(id).filter(|id| !id.is_empty())?)),
        ["agenda"] => Some(AppLink::Agenda),
        ["unsorted"] => Some(AppLink::Unsorted),
        ["routines"] => Some(AppLink::Routines),
        ["profile"] => Some(AppLink::Profile),
        ["frogs"] => Some(AppLink::Frogs),
        ["session", "new"] => Some(AppLink::NewSession),
        ["settings"] => Some(AppLink::Settings(None)),
        ["settings", "backups"] => Some(AppLink::Backups),
        ["settings", "sync"] => Some(AppLink::Sync),
        ["settings", s] if SETTINGS_SECTIONS.contains(s) => {
            Some(AppLink::Settings(Some((*s).into())))
        }
        ["guide"] => Some(AppLink::Guide(None, None)),
        ["guide", page] if slug(page) && anchor.is_none_or(slug) => Some(AppLink::Guide(
            Some((*page).into()),
            anchor.map(str::to_string),
        )),
        ["file", names @ ..] if !names.is_empty() => {
            let mut path = Vec::new();
            for part in names {
                let name = decode(part)?;
                if name.is_empty()
                    || name == "."
                    || name == ".."
                    || name
                        .chars()
                        .any(|c| c < ' ' || c == '\u{7F}' || matches!(c, '/' | '\\' | ':'))
                {
                    return None;
                }
                path.push(name);
            }
            Some(AppLink::File(path.join("/")))
        }
        _ => None,
    }
}

impl AppLink {
    /// The link as it is written, which [`parse`] reads back: ids and a
    /// file's names %-encoded but for ASCII letters, digits, `-`, `.`,
    /// `_`, `~`, `:` and `@` (a file's `:` encoded too), upper-case hex.
    pub fn url(&self) -> String {
        let rest = match self {
            AppLink::Note(id) => format!("note/{}", encode(id, false)),
            AppLink::Place(id) => format!("area/{}", encode(id, false)),
            AppLink::Agenda => "agenda".into(),
            AppLink::Unsorted => "unsorted".into(),
            AppLink::Settings(None) => "settings".into(),
            AppLink::Settings(Some(s)) => format!("settings/{s}"),
            AppLink::Routines => "routines".into(),
            AppLink::Backups => "settings/backups".into(),
            AppLink::Sync => "settings/sync".into(),
            AppLink::Profile => "profile".into(),
            AppLink::Frogs => "frogs".into(),
            AppLink::NewSession => "session/new".into(),
            AppLink::Guide(None, _) => "guide".into(),
            AppLink::Guide(Some(page), None) => format!("guide/{page}"),
            AppLink::Guide(Some(page), Some(box_)) => format!("guide/{page}#{box_}"),
            AppLink::File(path) => {
                let names: Vec<String> = path.split('/').map(|n| encode(n, true)).collect();
                format!("file/{}", names.join("/"))
            }
        };
        format!("{SCHEME}{rest}")
    }

    /// The guide's page on the website in `language`, English where the
    /// site has none: its start page, or the page with its box; `None`
    /// for a link to anything else.
    pub fn guide_address(&self, language: &str) -> Option<String> {
        let AppLink::Guide(page, box_) = self else {
            return None;
        };
        let lang = if language == "de" { "de" } else { "en" };
        let page = page
            .as_ref()
            .map(|p| format!("{p}.html"))
            .unwrap_or_default();
        let box_ = box_.as_ref().map(|b| format!("#{b}")).unwrap_or_default();
        Some(format!("{SITE}/{lang}/guide/{page}{box_}"))
    }
}

/// Lower-case ASCII letters, digits and `-`, at least one.
fn slug(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

fn encode(s: &str, file: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'.' | b'_' | b'~' | b'@')
            || (b == b':' && !file)
        {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `s` with its `%XX` decoded as UTF-8; `None` when one is broken.
fn decode(s: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(s.len());
    let mut raw = s.bytes();
    while let Some(b) = raw.next() {
        if b != b'%' {
            bytes.push(b);
            continue;
        }
        let high = char::from(raw.next()?).to_digit(16)?;
        let low = char::from(raw.next()?).to_digit(16)?;
        bytes.push(u8::try_from(high * 16 + low).ok()?);
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_form_reads_back_from_the_link_it_writes() {
        for link in [
            AppLink::Note("note:3f2a".into()),
            AppLink::Place("topic:x y/ü".into()),
            AppLink::Agenda,
            AppLink::Unsorted,
            AppLink::Settings(None),
            AppLink::Settings(Some("alarms".into())),
            AppLink::Routines,
            AppLink::Backups,
            AppLink::Sync,
            AppLink::Profile,
            AppLink::Frogs,
            AppLink::NewSession,
            AppLink::Guide(None, None),
            AppLink::Guide(Some("getting-started".into()), Some("set-up".into())),
            AppLink::File("Scans/Tax 2026 (a).pdf".into()),
        ] {
            assert_eq!(parse(&link.url()), Some(link.clone()), "{}", link.url());
        }
        assert_eq!(
            AppLink::Place("topic:x y/ü".into()).url(),
            "oldbrainfix://area/topic:x%20y%2F%C3%BC"
        );
        assert_eq!(parse("OldBrainFix://agenda"), Some(AppLink::Agenda));
    }

    #[test]
    fn a_file_link_never_leads_out_of_the_shared_folder() {
        for url in [
            "oldbrainfix://file/../etc/passwd",
            "oldbrainfix://file/a/%2E%2E/b",
            "oldbrainfix://file//etc/passwd",
            "oldbrainfix://file/a%2Fb",
            "oldbrainfix://file/a%5Cb",
            "oldbrainfix://file/C%3A",
            "oldbrainfix://file/a%00",
            "oldbrainfix://file",
        ] {
            assert_eq!(parse(url), None, "{url}");
        }
    }

    #[test]
    fn anything_else_with_the_scheme_is_no_app_link() {
        for url in [
            "oldbrainfix://x",
            "oldbrainfix://capture/voice",
            "oldbrainfix://pick?session=s",
            "oldbrainfix://agenda/",
            "oldbrainfix://agenda#x",
            "oldbrainfix://note/%C3",
            "oldbrainfix://settings/nothing",
            "oldbrainfix://guide/a#B",
            "oldbrainfix:ü",
        ] {
            assert_eq!(parse(url), None, "{url}");
        }
        assert_eq!(
            AppLink::Guide(Some("a".into()), None).guide_address("de"),
            Some("https://paxel.github.io/oldbrainfix/de/guide/a.html".into())
        );
    }
}
