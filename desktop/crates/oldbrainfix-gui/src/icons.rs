//! The phone's icons on the desk: Material Icons, the font Flutter ships,
//! with the codepoints of the icons the phone draws (Flutter's
//! `material/icons.dart`).

use egui::{Align2, Color32, FontFamily, FontId, Painter, Rect, Ui};

static MATERIAL_ICONS: &[u8] = include_bytes!("../assets/MaterialIcons-Regular.otf");

/// The font family the icons live in.
pub const FAMILY: &str = "icons";

pub const ALL_NOTES: char = '\u{e44c}';
pub const UNSORTED: char = '\u{e33a}';
pub const TRASH: char = '\u{e1bb}';
/// Opens the Trash: a bin with an arrow up, the plain bin deletes (#166).
pub const OPEN_TRASH: char = '\u{e535}';
pub const IMPORTANT: char = '\u{e361}';
pub const NO_AREA_ICON: char = '\u{f091}';
pub const ADD: char = '\u{e047}';
pub const REMOVE: char = '\u{e516}';
pub const AGENDA: char = '\u{e66a}';
pub const SETTINGS: char = '\u{e57f}';
pub const ADD_ALARM: char = '\u{e049}';
pub const REPEAT: char = '\u{e51f}';
pub const REMINDER: char = '\u{e44f}';
pub const CHECKED: char = '\u{e157}';
pub const UNCHECKED: char = '\u{e158}';
pub const INBOX: char = '\u{e40c}';
pub const HISTORY: char = '\u{e314}';
pub const CLOSE: char = '\u{e16a}';
pub const NOTE: char = '\u{e605}';
pub const PREVIOUS: char = '\u{e15e}';
pub const NEXT: char = '\u{e15f}';
pub const MIC: char = '\u{e3e1}';
pub const STOP: char = '\u{e606}';
pub const PLAY: char = '\u{e4cb}';
pub const PASTE: char = '\u{e192}';
pub const SEARCH: char = '\u{e567}';
pub const PHOTO: char = '\u{e4b4}';
pub const FOLDER_OPEN: char = '\u{e2a4}';
pub const PERSON: char = '\u{e491}';
pub const MAIL_UNREAD: char = '\u{f1b6}';
/// `group_add_outlined`, as the phone marks Areas shared from afar.
pub const GROUP_ADD: char = '\u{f0d8}';
pub const ARROW_UP: char = '\u{e0a0}';
pub const ARROW_DOWN: char = '\u{e097}';
/// `check_circle_outline` and `radio_button_unchecked`, as the phone marks
/// a Session's picks with an entry and without.
pub const PICK_DONE: char = '\u{e15a}';
pub const PICK_OPEN: char = '\u{e504}';
pub const SWAP: char = '\u{e625}';
/// `edit`, as Edit in a Note card's ⋮ on the phone.
pub const EDIT: char = '\u{e21a}';
/// `view_agenda_outlined` and `view_headline`, as the phone's Note lists
/// show their full/compact switch while full and while compact (#114).
pub const NOTES_FULL: char = '\u{f48f}';
pub const NOTES_COMPACT: char = '\u{e6b3}';
/// `schedule`, `place_outlined`, `apps`, `event_outlined` and
/// `bluetooth`, as the phone's `CaptureContextLines` mark when, where, the
/// app shared from, the calendar events and the Bluetooth devices; and
/// `more_horiz`, as a compact card says there is more to see.
pub const CONTEXT_WHEN: char = '\u{e556}';
pub const CONTEXT_WHERE: char = '\u{f2ac}';
pub const CONTEXT_APP: char = '\u{e08f}';
pub const CONTEXT_CALENDAR: char = '\u{f02f}';
pub const CONTEXT_BLUETOOTH: char = '\u{e0e4}';
pub const MORE: char = '\u{e402}';
/// `format_list_bulleted`, as the phone's Agenda bar opens All Routines
/// and Reminders; `more_vert`, as an Occurrence's ⋮; and
/// `file_upload_outlined`, as that page's Export.
pub const ALL_ROUTINES: char = '\u{e2b8}';
pub const MORE_VERT: char = '\u{e404}';
/// `check_circle`, `cancel` and `timelapse`, as the phone's Routine
/// History marks what was Done, Missed, and a day partly both; it marks
/// what is still due with `radio_button_unchecked` ([`PICK_OPEN`]), what
/// is still to come with `schedule` ([`CONTEXT_WHEN`]) and a day with
/// nothing due with `remove` ([`REMOVE`]).
pub const STATUS_DONE: char = '\u{e159}';
pub const STATUS_MISSED: char = '\u{e139}';
pub const DAY_PARTLY: char = '\u{e660}';
pub const EXPORT: char = '\u{f05d}';
/// `shopping_cart_outlined`, as the phone marks a Shopping List's row on
/// the Agenda (#123).
pub const SHOPPING_CART: char = '\u{f37f}';
/// `checklist`, as the phone marks a Checklist's row on the Agenda
/// (#143).
pub const CHECKLIST: char = '\u{e15b}';
/// The format bar's, as the phone's (#127): `format_bold`,
/// `format_italic`, `format_strikethrough`, `format_list_bulleted`,
/// `format_list_numbered`, `format_quote`, `link`, `code`,
/// `format_indent_increase` and `format_indent_decrease`.
pub const FORMAT_BOLD: char = '\u{e2af}';
pub const FORMAT_ITALIC: char = '\u{e2b6}';
pub const FORMAT_STRIKE: char = '\u{e2bf}';
pub const FORMAT_BULLETS: char = '\u{e2b8}';
pub const FORMAT_NUMBERS: char = '\u{e2b9}';
pub const FORMAT_QUOTE: char = '\u{e2bc}';
pub const LINK: char = '\u{e380}';
pub const CODE: char = '\u{e176}';
/// The format bar's code block (#212), as the phone's: `data_object`.
pub const CODE_BLOCK: char = '\u{f04e6}';
/// The text format badge (#213), as the phone's: `text_format` for the
/// setting, `short_text` for a plain text; [`FORMAT_BOLD`] and [`CODE`]
/// for the others.
pub const TEXT_FORMAT: char = '\u{e64a}';
pub const SHORT_TEXT: char = '\u{e59d}';
pub const INDENT: char = '\u{e2b5}';
pub const OUTDENT: char = '\u{e2b4}';
/// The format bar's table and the table view's, as the phone's (#128):
/// `table_chart_outlined`, and `arrow_back` to leave it.
pub const TABLE: char = '\u{f41d}';
pub const ARROW_BACK: char = '\u{e092}';
/// Forward through the places the list showed (#145), beside `arrow_back`.
pub const ARROW_FORWARD: char = '\u{e09b}';
/// The frog that marks Frogs, from the emoji font, as there is none
/// among the Material icons ([`crate::frogs_view::FROG`]).
pub const FROG: char = '\u{1F438}';
/// The badges of a Note's settings (#160), as the phone's editor draws
/// them: `label_important_outline` for the Importance, `today_outlined`
/// for Show on the Agenda and `history_toggle_off` for Show ticks; for
/// a field of the New Tally page (#161) `functions` for its rule,
/// `straighten` for its unit and `flag_outlined` for its target; for the
/// New Session page `tag` for its count, `shuffle` for its rule and
/// `event_repeat` for its schedule, which also marks a Session; and
/// `bar_chart` for a Tally and `edit_note` for a Note in the menu of
/// "+".
pub const IMPORTANCE: char = '\u{e362}';
pub const ON_AGENDA: char = '\u{f44d}';
pub const SHOW_TICKS: char = '\u{e316}';
/// A Shopping List Line's reappear time, `autorenew` (#195).
pub const REAPPEAR: char = '\u{e0c1}';
pub const TALLY_RULE: char = '\u{e2cd}';
pub const TALLY_UNIT: char = '\u{e60e}';
pub const TALLY_TARGET: char = '\u{f07b}';
/// A Tally field hidden (#190), `visibility_off_outlined`.
pub const TALLY_HIDDEN: char = '\u{f4a0}';
/// What a Tally field multiplies (#200), `calculate_outlined`.
pub const TALLY_MULTIPLIES: char = '\u{ef10}';
pub const SESSION_COUNT: char = '\u{e640}';
pub const SESSION_RULE: char = '\u{e5a1}';
pub const SESSION: char = '\u{f04fb}';
/// A Session's weekdays' own counts (#191), `calendar_view_week`.
pub const SESSION_WEEKDAYS: char = '\u{e125}';
pub const TALLY: char = '\u{e0cc}';
pub const NEW_NOTE: char = '\u{f04f6}';
/// The badges' own icons and their values' (#192): `category_outlined`
/// for the kind; `priority_high`, `remove` and `low_priority` for
/// important, normal and unimportant; `toggle_on` and `toggle_off` for an
/// on-or-off setting; `numbers` for a Shopping List's count on the
/// Agenda, its Lines `format_list_bulleted` ([`FORMAT_BULLETS`]);
/// `add_circle_outline` and `vertical_align_top` for a Tally field's Sum
/// and Max; `alt_route` for a Session's rule, its Random `shuffle`
/// ([`SESSION_RULE`]), its Rolling `format_list_numbered`
/// ([`FORMAT_NUMBERS`]) and its Least `balance`; and `folder_outlined`
/// for Area and Topic ([`NO_AREA_ICON`]).
pub const KIND: char = '\u{ef37}';
pub const IMPORTANCE_HIGH: char = '\u{e4ec}';
pub const IMPORTANCE_NORMAL: char = '\u{e516}';
pub const IMPORTANCE_LOW: char = '\u{e3bd}';
pub const TOGGLE_ON: char = '\u{e66c}';
pub const TOGGLE_OFF: char = '\u{e66b}';
pub const COUNT: char = '\u{f0542}';
pub const TALLY_SUM: char = '\u{e050}';
pub const TALLY_MAX: char = '\u{e69d}';
pub const SESSION_RULE_BADGE: char = '\u{e080}';
pub const SESSION_LEAST: char = '\u{f04c6}';
/// A folded description of a Note of a kind (#188), `expand_more`, and
/// one unfolded, `expand_less`.
pub const EXPAND_MORE: char = '\u{e246}';
pub const EXPAND_LESS: char = '\u{e245}';

/// How the phone draws each of the core's Area icons.
pub fn area_icon(key: Option<&str>) -> Option<char> {
    Some(match key? {
        "home" => '\u{e318}',
        "work" => '\u{e6f2}',
        "car" => '\u{e1d7}',
        "health" => '\u{e25b}',
        "school" => '\u{e559}',
        "kids" => '\u{e160}',
        "kitchen" => '\u{e35e}',
        "garden" => '\u{e6fb}',
        "shopping" => '\u{e59c}',
        "money" => '\u{e553}',
        "pets" => '\u{e4a1}',
        "sport" => '\u{e28d}',
        "travel" => '\u{e297}',
        "star" => '\u{e5f9}',
        // `psychology`, the phone's brain for the help Notes' Area (#180).
        "brain" => '\u{e4ef}',
        _ => return None,
    })
}

/// The fonts: egui's own, plus the icon font in a family of its own.
pub fn fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "material-icons".into(),
        std::sync::Arc::new(egui::FontData::from_static(MATERIAL_ICONS)),
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY.into()),
        vec!["material-icons".into()],
    );
    fonts
}

/// Paints `icon` centred in `rect`: a Material icon, nothing before the
/// fonts are bound (the very first pass), or one of the text's fonts,
/// as the frog, when it lies outside the Private Use Area.
pub fn paint(painter: &Painter, rect: Rect, icon: char, color: Color32) {
    // The Private Use Areas, the second one's for Material's newer
    // icons, such as `event_repeat`.
    let private =
        ('\u{e000}'..='\u{f8ff}').contains(&icon) || ('\u{f0000}'..='\u{ffffd}').contains(&icon);
    if !private {
        let size = rect.height() * 0.9;
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(size),
            color,
        );
        return;
    }
    let bound = painter
        .ctx()
        .fonts(|f| f.families().contains(&FontFamily::Name(FAMILY.into())));
    if bound {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon,
            FontId::new(rect.height(), FontFamily::Name(FAMILY.into())),
            color,
        );
    }
}

/// Takes a square of `size` in `ui` and paints `icon` in it.
pub fn show(ui: &mut Ui, icon: char, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    paint(ui.painter(), rect, icon, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_area_icon_of_the_core_has_a_glyph() {
        for key in oldbrainfix_core::AREA_ICONS {
            assert!(area_icon(Some(key)).is_some(), "{key}");
        }
        assert_eq!(area_icon(Some("rocket")), None);
        assert_eq!(area_icon(None), None);
    }
}
