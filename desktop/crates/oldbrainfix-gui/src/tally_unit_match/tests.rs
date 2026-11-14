use super::*;

fn codes(typed: &str, system: UnitSystem, kind: Option<UnitKind>) -> Vec<&'static str> {
    let names = unit_names(system);
    matching_units(typed, system, &names, kind)
        .iter()
        .map(|known| known.code)
        .collect()
}

fn metric(typed: &str) -> Vec<&'static str> {
    codes(typed, UnitSystem::Metric, None)
}

fn imperial(typed: &str) -> Vec<&'static str> {
    codes(typed, UnitSystem::Imperial, None)
}

#[test]
fn by_the_start_of_their_symbol_ignoring_case_and_spaces() {
    assert_eq!(metric("k"), ["kg", "km", "km/h"]);
    assert_eq!(metric(" KM "), ["km", "km/h"]);
    assert_eq!(metric("min"), ["min"]);
    assert_eq!(metric("°"), ["°C", "°"]);
}

#[test]
fn by_the_start_of_an_english_or_a_german_name() {
    assert_eq!(metric("kilo"), ["kg", "km", "km/h"]);
    assert_eq!(metric("minute"), ["min"]);
    assert_eq!(metric("Stun"), ["km/h", "h"]);
    assert_eq!(metric("sekunden"), ["s"]);
    assert_eq!(metric("celsius"), ["°C"]);
    assert_eq!(metric("grad"), ["°C", "°"]);
}

#[test]
fn in_imperial_by_the_imperial_symbols_and_names() {
    assert_eq!(imperial("lb"), ["kg"]);
    assert_eq!(imperial("pound"), ["kg"]);
    assert_eq!(imperial("Pfund"), ["kg"]);
    assert_eq!(imperial("Fuß"), ["m"]);
    assert_eq!(imperial("mi"), ["km", "km/h", "min"]);
    assert!(imperial("kilo").is_empty());
}

#[test]
fn every_unit_for_nothing_typed_and_none_for_a_text_of_its_own() {
    let every: Vec<_> = KNOWN.iter().map(|known| known.code).collect();
    assert_eq!(metric(""), every);
    assert!(metric("pages").is_empty());
}

#[test]
fn only_units_of_the_kind_fixed() {
    let distance = Some(UnitKind::Distance);
    assert_eq!(codes("", UnitSystem::Metric, distance), ["km", "m", "cm"]);
    assert_eq!(codes("m", UnitSystem::Metric, distance), ["m"]);
    assert!(codes("", UnitSystem::Metric, Some(UnitKind::Typed)).is_empty());
}

#[test]
fn the_unit_typed_is_a_known_code_as_typed_or_a_known_symbol_of_the_system() {
    assert_eq!(unit_of_typed("kg", UnitSystem::Metric), "kg");
    assert_eq!(unit_of_typed("kg", UnitSystem::Imperial), "kg");
    assert_eq!(unit_of_typed(" lb ", UnitSystem::Imperial), "kg");
    assert_eq!(unit_of_typed("°F", UnitSystem::Imperial), "°C");
}

#[test]
fn the_unit_typed_else_is_the_text_as_typed_a_unit_of_the_users_own() {
    assert_eq!(unit_of_typed(" pages ", UnitSystem::Metric), "pages");
    assert_eq!(unit_of_typed("lb", UnitSystem::Metric), "lb");
    assert_eq!(unit_of_typed("KG", UnitSystem::Metric), "KG");
    assert_eq!(unit_of_typed("", UnitSystem::Metric), "");
}

#[test]
fn the_names_come_from_every_language() {
    let names_of = |names: &UnitNames, code: &str| {
        names
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, names)| names.clone())
            .unwrap_or_default()
    };
    let metric = unit_names(UnitSystem::Metric);
    let imperial = unit_names(UnitSystem::Imperial);
    let kg = names_of(&metric, "kg");
    assert!(kg.contains(&"kilogram".to_string()) && kg.contains(&"Kilogramm".to_string()));
    let lb = names_of(&imperial, "kg");
    assert!(lb.contains(&"pound".to_string()) && lb.contains(&"Pfund".to_string()));
    let codes: Vec<_> = metric.iter().map(|(code, _)| *code).collect();
    let every: Vec<_> = KNOWN.iter().map(|known| known.code).collect();
    assert_eq!(codes, every);
    assert!(LOCALES.contains(&"de"), "the German names are matched");
}
