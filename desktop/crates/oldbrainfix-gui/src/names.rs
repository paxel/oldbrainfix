//! Display-time translation of the Starter Set (Catlog's ADR 0005), as
//! the phone's `place_names.dart` does it: a planted Area or Topic shows
//! in the app's language while its stored name is still the canonical
//! English one. Once the user renamed it, or when the user made it, the
//! typed name shows, untranslated.

use oldbrainfix_core::places::starter_name;
use oldbrainfix_core::{Area, Topic};

use crate::l10n::L10n;

pub fn area_name(t: &L10n, area: &Area) -> String {
    shown(t, area.starter.as_deref(), &area.name)
}

pub fn topic_name(t: &L10n, topic: &Topic) -> String {
    shown(t, topic.starter.as_deref(), &topic.name)
}

fn shown(t: &L10n, slug: Option<&str>, name: &str) -> String {
    let Some(slug) = slug else {
        return name.to_string();
    };
    if starter_name(slug) != Some(name) {
        return name.to_string();
    }
    translated(t, slug).unwrap_or(name).to_string()
}

fn translated(t: &L10n, slug: &str) -> Option<&'static str> {
    Some(match slug {
        "household" => t.starter_household(),
        "kitchen" => t.starter_kitchen(),
        "garden" => t.starter_garden(),
        "cleaning" => t.starter_cleaning(),
        "work" => t.starter_work(),
        "meetings" => t.starter_meetings(),
        "to-sort-out" => t.starter_to_sort_out(),
        "car" => t.starter_car(),
        "service" => t.starter_service(),
        "repairs" => t.starter_repairs(),
        "health" => t.starter_health(),
        "doctor" => t.starter_doctor(),
        "medication" => t.starter_medication(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(name: &str, starter: Option<&str>) -> Area {
        Area {
            id: "area:a".into(),
            name: name.into(),
            starter: starter.map(str::to_string),
            colour: None,
            icon: None,
        }
    }

    #[test]
    fn starters_translate_until_renamed() {
        let de = L10n::new("de");
        let en = L10n::new("en");
        assert_eq!(
            area_name(&de, &area("Household", Some("household"))),
            "Haushalt"
        );
        assert_eq!(
            area_name(&en, &area("Household", Some("household"))),
            "Household"
        );
        assert_eq!(area_name(&de, &area("Bike", Some("car"))), "Bike");
        assert_eq!(area_name(&de, &area("Household", None)), "Household");
        assert_eq!(area_name(&de, &area("Odd", Some("odd"))), "Odd");
        let topic = Topic {
            id: "topic:t".into(),
            area_id: "area:a".into(),
            name: "To sort out".into(),
            starter: Some("to-sort-out".into()),
        };
        assert_eq!(
            topic_name(&de, &topic),
            L10n::new("de").starter_to_sort_out()
        );
    }
}
