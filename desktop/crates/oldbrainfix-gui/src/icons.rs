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

/// Paints `icon` centred in `rect`. Nothing before the fonts are bound
/// (the very first pass).
pub fn paint(painter: &Painter, rect: Rect, icon: char, color: Color32) {
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
