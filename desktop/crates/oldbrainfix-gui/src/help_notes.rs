//! The help Notes on the desktop (#180, ADR 0042): what the phone's
//! `help_notes.dart` hands the core, in the same words and with the same
//! pictures, so either app makes the same Notes. The pictures and the
//! What's new are the phone's assets, bundled at build time.

use oldbrainfix_core::app_link::AppLink;
use oldbrainfix_core::markdown;
use oldbrainfix_core::{HelpNotes, HelpTexts, Result, Store, parse_whats_new};

use crate::l10n::L10n;

/// The What's new of every release, as `tool/whats_new.py --bundle`
/// writes it.
pub static WHATS_NEW: &str = include_str!("../../../../assets/help/whats_new.json");

/// One icon of "What the icons mean": its name and its meaning.
type Named = (fn(&L10n) -> &'static str, fn(&L10n) -> &'static str);

/// A group of icons: its slug, as the picture is named, its heading and
/// its icons, as the phone's `helpIconGroups`.
pub struct Group {
    pub slug: &'static str,
    pub heading: fn(&L10n) -> &'static str,
    pub icons: &'static [Named],
    /// The picture, English and German.
    pub pictures: [&'static [u8]; 2],
}

/// The groups, in the phone's order, with its pictures.
pub static GROUPS: [Group; 3] = [
    Group {
        slug: "badges",
        heading: L10n::help_icons_badges,
        icons: &[
            (L10n::help_icon_kind, L10n::help_icon_kind_means),
            (L10n::help_icon_importance, L10n::help_icon_importance_means),
            (L10n::help_icon_place, L10n::help_icon_place_means),
            (L10n::help_icon_captured, L10n::help_icon_captured_means),
            (L10n::help_icon_reminder, L10n::help_icon_reminder_means),
            (L10n::help_icon_routine, L10n::help_icon_routine_means),
            (L10n::help_icon_frog, L10n::help_icon_frog_means),
            (L10n::help_icon_add, L10n::help_icon_add_means),
        ],
        pictures: [
            include_bytes!("../../../../assets/help/icons-badges-en.png"),
            include_bytes!("../../../../assets/help/icons-badges-de.png"),
        ],
    },
    Group {
        slug: "agenda",
        heading: L10n::help_icons_agenda,
        icons: &[
            (L10n::help_icon_browse, L10n::help_icon_browse_means),
            (
                L10n::help_icon_all_routines,
                L10n::help_icon_all_routines_means,
            ),
            (
                L10n::help_icon_new_routine,
                L10n::help_icon_new_routine_means,
            ),
            (
                L10n::help_icon_new_session,
                L10n::help_icon_new_session_means,
            ),
            (L10n::help_icon_unsorted, L10n::help_icon_unsorted_means),
        ],
        pictures: [
            include_bytes!("../../../../assets/help/icons-agenda-en.png"),
            include_bytes!("../../../../assets/help/icons-agenda-de.png"),
        ],
    },
    Group {
        slug: "card",
        heading: L10n::help_icons_card,
        icons: &[
            (L10n::help_icon_file_into, L10n::help_icon_file_into_means),
            (L10n::help_icon_alarm, L10n::help_icon_alarm_means),
            (L10n::help_icon_merge, L10n::help_icon_merge_means),
            (L10n::help_icon_more, L10n::help_icon_more_means),
        ],
        pictures: [
            include_bytes!("../../../../assets/help/icons-card-en.png"),
            include_bytes!("../../../../assets/help/icons-card-de.png"),
        ],
    },
];

/// The Lines of the Quick start, in `t`'s language.
pub fn quick_start_lines(t: &L10n) -> Vec<String> {
    [
        t.help_quick_start1(),
        t.help_quick_start2(),
        t.help_quick_start3(),
        t.help_quick_start4(),
        t.help_quick_start5(),
        t.help_quick_start6(),
        t.help_quick_start7(),
        t.help_quick_start8(),
    ]
    .map(String::from)
    .to_vec()
}

/// The Quick start's Description in `t`'s language (#260), as the phone
/// writes it: a link to the place of each Line, in the Lines' order
/// (#261), then "On the desktop" and the desktop's way for each Line, one
/// line each.
pub fn quick_start_description(t: &L10n) -> String {
    let mut lines = vec![t.help_quick_start_places_title().to_string()];
    for (i, place) in quick_start_places(t).iter().enumerate() {
        lines.push(format!("{}. {place}", i + 1));
    }
    lines.push(String::new());
    lines.extend(
        [
            t.help_quick_start_desktop_title(),
            t.help_quick_start_desktop1(),
            t.help_quick_start_desktop2(),
            t.help_quick_start_desktop3(),
            t.help_quick_start_desktop4(),
            t.help_quick_start_desktop5(),
            t.help_quick_start_desktop6(),
            t.help_quick_start_desktop7(),
            t.help_quick_start_desktop8(),
        ]
        .map(str::to_string),
    );
    lines.join("\n")
}

/// `label` as a Markdown link to `to`, as "Link to…" writes one.
fn linked(label: &str, to: &AppLink) -> String {
    markdown::link_labelled("", 0, 0, &to.url(), Some(label)).text
}

/// Where each Quick start Line is done, as links (#261): the Agenda,
/// Unsorted, All Routines and Reminders twice, this Checklist itself
/// (no link), the Agenda, Backups and Sync.
pub fn quick_start_places(t: &L10n) -> [String; 8] {
    let routines = linked(t.all_routines_title(), &AppLink::Routines);
    [
        linked(t.agenda(), &AppLink::Agenda),
        linked(t.unsorted(), &AppLink::Unsorted),
        routines.clone(),
        routines,
        markdown::escape(t.help_quick_start_place_here()),
        linked(t.agenda(), &AppLink::Agenda),
        linked(t.backups_title(), &AppLink::Backups),
        linked(t.sync_title(), &AppLink::Sync),
    ]
}

/// The text of "What the icons mean", as the phone writes it: its name,
/// then each group's heading and its icons, "name: meaning" one per line,
/// then a link to the user guide.
pub fn icons_text(t: &L10n) -> String {
    let mut out = t.help_icons_title().to_string();
    for group in &GROUPS {
        out.push_str("\n\n");
        out.push_str((group.heading)(t));
        for (name, means) in group.icons {
            out.push_str(&format!("\n{}: {}", name(t), means(t)));
        }
    }
    // A link to the guide's start page last (#261).
    out.push_str("\n\n");
    out.push_str(&linked(t.help_icons_guide(), &AppLink::Guide(None, None)));
    out
}

/// The pictures of the groups in `t`'s language.
pub fn pictures(t: &L10n) -> Vec<&'static [u8]> {
    let german = t.locale() == "de";
    GROUPS
        .iter()
        .map(|g| {
            let [english, in_german] = g.pictures;
            if german { in_german } else { english }
        })
        .collect()
}

/// Makes the help Notes due on this Device in `t`'s language, running
/// `version`; `fresh` right after the first start. Returns the ids made.
pub fn make_due(store: &Store, t: &L10n, version: &str, fresh: bool) -> Result<Vec<String>> {
    let texts = HelpTexts {
        area_name: t.help_area_name().to_string(),
        quick_start_title: t.help_quick_start_title().to_string(),
        quick_start_lines: quick_start_lines(t),
        quick_start_description: quick_start_description(t),
        icons_text: icons_text(t),
        icon_pictures: pictures(t),
        whats_new_title: Box::new(|v| t.help_whats_new_title(v)),
    };
    HelpNotes::new(store).make_due(
        version,
        fresh,
        t.locale(),
        &texts,
        &parse_whats_new(WHATS_NEW),
    )
}

#[cfg(test)]
mod tests;
