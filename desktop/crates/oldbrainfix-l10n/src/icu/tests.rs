use super::*;

#[test]
fn arguments_plurals_and_selects_format() {
    assert_eq!(
        format("en", "CSV saved to {path}", &[("path", Arg::Str("/tmp/x"))]),
        "CSV saved to /tmp/x"
    );
    assert_eq!(format("en", "Hello {name}", &[]), "Hello {name}");
    let msg =
        "{n, plural, =0{Nothing restored.} one{1 catalog restored.} other{{n} catalogs restored.}}";
    assert_eq!(
        format("en", msg, &[("n", Arg::Int(0))]),
        "Nothing restored."
    );
    assert_eq!(
        format("en", msg, &[("n", Arg::Int(1))]),
        "1 catalog restored."
    );
    assert_eq!(
        format("en", msg, &[("n", Arg::Int(3))]),
        "3 catalogs restored."
    );
    assert_eq!(
        format("en", msg, &[("n", Arg::Str("2"))]),
        "2 catalogs restored."
    );
    assert_eq!(format("en", msg, &[]), "{n} catalogs restored.");
    assert_eq!(
        format(
            "en",
            "{n, plural, one{# day} other{# days}}",
            &[("n", Arg::Int(2))]
        ),
        "2 days"
    );
    assert_eq!(
        format(
            "en",
            "{n, plural, one{one} other{many}} left",
            &[("n", Arg::Int(5))]
        ),
        "many left"
    );
    let sel = "{g, select, female{she} male{he} other{they}} did it";
    assert_eq!(format("en", sel, &[("g", Arg::Str("male"))]), "he did it");
    assert_eq!(format("en", sel, &[("g", Arg::Str("x"))]), "they did it");
    assert_eq!(format("en", sel, &[]), "they did it");
    assert_eq!(
        format(
            "en",
            "it's {a} and {b, number}",
            &[("a", Arg::Int(1)), ("b", Arg::Int(2))]
        ),
        "it's 1 and 2"
    );
    assert_eq!(
        format("en", "{n, plural, one{x}}", &[("n", Arg::Int(3))]),
        ""
    );
    assert_eq!(
        format("en", "broken {n, plural, one{x", &[("n", Arg::Int(1))]),
        "broken x"
    );
    assert_eq!(format("en", "# outside", &[]), "# outside");
}

#[test]
fn variables_are_found_with_their_kind() {
    let vars =
        variables("{n, plural, one{{n} for {who}} other{#}} and {who} {g, select, a{x} other{y}}");
    assert_eq!(
        vars,
        vec![
            ("n".to_string(), VariableKind::Plural),
            ("who".to_string(), VariableKind::Plain),
            ("g".to_string(), VariableKind::Select),
        ]
    );
    assert_eq!(
        variables("{x} then {x, plural, other{y}}")[0].1,
        VariableKind::Plural
    );
}

#[test]
fn plural_categories_follow_cldr_for_every_app_locale() {
    assert_eq!(plural_category("en", 1), "one");
    assert_eq!(plural_category("en", 0), "other");
    assert_eq!(plural_category("de-DE", 5), "other");
    assert_eq!(plural_category("fr", 0), "one");
    assert_eq!(plural_category("fr", 2), "other");
    assert_eq!(plural_category("ja", 1), "other");
    assert_eq!(plural_category("ar", 0), "zero");
    assert_eq!(plural_category("ar", 2), "two");
    assert_eq!(plural_category("ar", 5), "few");
    assert_eq!(plural_category("ar", 15), "many");
    assert_eq!(plural_category("ar", 100), "other");
    assert_eq!(plural_category("he", 2), "two");
    assert_eq!(plural_category("he", 20), "many");
    assert_eq!(plural_category("he", 3), "other");
    assert_eq!(plural_category("ru", 21), "one");
    assert_eq!(plural_category("ru", 22), "few");
    assert_eq!(plural_category("ru", 12), "many");
    assert_eq!(plural_category("uk", 11), "many");
    assert_eq!(plural_category("hr", 5), "other");
    assert_eq!(plural_category("sr", 3), "few");
    assert_eq!(plural_category("cs", 3), "few");
    assert_eq!(plural_category("sk", 5), "other");
    assert_eq!(plural_category("pl", 22), "few");
    assert_eq!(plural_category("pl", 5), "many");
    assert_eq!(plural_category("pl", 1), "one");
    assert_eq!(plural_category("lt", 11), "other");
    assert_eq!(plural_category("lt", 22), "few");
    assert_eq!(plural_category("lt", 21), "one");
    assert_eq!(plural_category("lv", 0), "zero");
    assert_eq!(plural_category("lv", 21), "one");
    assert_eq!(plural_category("lv", 2), "other");
    assert_eq!(plural_category("ga", 4), "few");
    assert_eq!(plural_category("ga", 8), "many");
    assert_eq!(plural_category("ga", 2), "two");
    assert_eq!(plural_category("mt", 0), "few");
    assert_eq!(plural_category("mt", 15), "many");
    assert_eq!(plural_category("mt", 1), "one");
    assert_eq!(plural_category("mt", 25), "other");
    assert_eq!(plural_category("ro", 5), "few");
    assert_eq!(plural_category("ro", 25), "other");
    assert_eq!(plural_category("sl", 101), "one");
    assert_eq!(plural_category("sl", 102), "two");
    assert_eq!(plural_category("sl", 3), "few");
    assert_eq!(plural_category("sl", 5), "other");
    assert_eq!(plural_category("mk", 11), "other");
    assert_eq!(plural_category("is", 21), "one");
    assert_eq!(plural_category("hu", 1), "one");
    assert_eq!(plural_category("tr", -1), "one");
}
