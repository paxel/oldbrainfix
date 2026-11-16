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
