use super::*;

#[test]
fn numbers_read_as_darts_double_try_parse_reads_them() {
    // What the Dart VM gives for each, probed on 2026-10-29.
    let read: [(&str, Option<f64>); 30] = [
        ("52.52", Some(52.52)),
        (" 1.5 ", Some(1.5)),
        ("\u{a0}1.5", Some(1.5)),
        ("\u{feff}1.5", Some(1.5)),
        ("\u{85}1.5\n", Some(1.5)),
        ("\u{3000}1", Some(1.0)),
        ("1.", Some(1.0)),
        (".5", Some(0.5)),
        ("+1", Some(1.0)),
        ("-.5e3", Some(-500.0)),
        ("1.e5", Some(100000.0)),
        ("1E+2", Some(100.0)),
        ("1e+05", Some(100000.0)),
        ("00.5", Some(0.5)),
        ("1e400", Some(f64::INFINITY)),
        ("1e-400", Some(0.0)),
        ("-Infinity", Some(f64::NEG_INFINITY)),
        ("0x10", None),
        ("inf", None),
        ("infinity", None),
        ("nan", None),
        ("1e", None),
        ("1e+", None),
        (".e5", None),
        ("1e5.5", None),
        (".", None),
        ("+-1", None),
        ("1,5", None),
        ("\u{200b}1", None),
        ("\u{ff15}", None),
    ];
    for (text, want) in read {
        assert_eq!(dart_double(text), want, "{text:?}");
    }
    for text in ["NaN", "-NaN", " +NaN "] {
        assert!(dart_double(text).is_some_and(f64::is_nan), "{text:?}");
    }
    assert_eq!(dart_double("Infinity"), Some(f64::INFINITY));
    assert!(dart_double("-0").is_some_and(|z| z == 0.0 && z.is_sign_negative()));
    for text in ["", " ", "-", "Infinity1", "1_0", "- 1", "1 2", "1d"] {
        assert_eq!(dart_double(text), None, "{text:?}");
    }
}

#[test]
fn coordinates_read_only_two_numbers_in_range() {
    let at = |lat, lon| Coordinates::new(lat, lon);
    assert_eq!(Coordinates::decode(Some("52.52,13.405")), at(52.52, 13.405));
    assert_eq!(
        Coordinates::decode(Some(" 52.52 , 13.405 ")),
        at(52.52, 13.405)
    );
    assert_eq!(Coordinates::decode(Some("-90,180")), at(-90.0, 180.0));
    for garbled in [
        "",
        "52.52",
        "52.52,13.405,0",
        "52.52;13.405",
        "91,0",
        "0,-180.5",
        "NaN,0",
        "0,Infinity",
        "somewhere",
        ",",
    ] {
        assert_eq!(Coordinates::decode(Some(garbled)), None, "{garbled:?}");
    }
    assert_eq!(Coordinates::decode(None), None);
    assert_eq!(Coordinates::new(f64::NAN, 0.0), None);
}

fn place(id: &str, lat: f64, lon: f64, radius: f64) -> Option<NamedPlace> {
    Some(NamedPlace {
        id: id.to_string(),
        name: id.to_string(),
        centre: Coordinates::new(lat, lon)?,
        radius,
    })
}

#[test]
fn the_smallest_named_place_holding_a_point_wins() {
    let (Some(home), Some(kitchen), Some(far)) = (
        place("home", 52.52, 13.405, 300.0),
        place("kitchen", 52.521, 13.405, 30.0),
        place("far", 48.0, 11.0, 1000.0),
    ) else {
        panic!("the places are in range");
    };
    let places = [home.clone(), kitchen.clone(), far];
    let at = |lat, lon| {
        Coordinates::new(lat, lon)
            .and_then(|p| named_place_at(&places, p))
            .map(|p| p.id.as_str())
    };
    assert_eq!(at(52.521, 13.405), Some("kitchen"));
    assert_eq!(at(52.52, 13.405), Some("home"));
    assert_eq!(at(52.53, 13.405), None);
    // The same size: the nearer centre, then the first.
    let (Some(a), Some(b)) = (
        place("a", 52.52, 13.405, 300.0),
        place("b", 52.521, 13.405, 300.0),
    ) else {
        panic!("the places are in range");
    };
    let twins = [a.clone(), b.clone()];
    let near_b = Coordinates::new(52.5209, 13.405).and_then(|p| named_place_at(&twins, p));
    assert_eq!(near_b.map(|p| p.id.as_str()), Some("b"));
    let same = [a.clone(), a.clone()];
    let first = Coordinates::new(52.52, 13.405).and_then(|p| named_place_at(&same, p));
    assert!(first.is_some_and(|p| std::ptr::eq(p, &same[0])));
    // A NaN radius holds every point, as `distance > NaN` is false
    // in Dart too, and loses to any other.
    let Some(nan) = place("nan", 0.0, 0.0, f64::NAN) else {
        panic!("the place is in range");
    };
    let odd = [nan.clone(), kitchen.clone()];
    let anywhere = Coordinates::new(-10.0, 100.0).and_then(|p| named_place_at(&odd, p));
    assert_eq!(anywhere.map(|p| p.id.as_str()), Some("nan"));
    assert_eq!(nan, nan.clone());
    assert_ne!(home, kitchen);
}

#[test]
fn named_places_without_a_readable_centre_or_radius_are_skipped() -> Result<()> {
    let store = crate::store::tests::store();
    let write = |id: &str, name: &str, centre: &str, radius: &str| -> Result<()> {
        store.append(id, keys::TYPE, Some(keys::TYPE_NAMED_PLACE), None)?;
        store.append(id, keys::NAME, Some(name), None)?;
        store.append(id, keys::CENTRE, Some(centre), None)?;
        store.append(id, keys::RADIUS, Some(radius), None)?;
        Ok(())
    };
    write("namedplace:b", "home", "52.52,13.405", "300.0")?;
    write("namedplace:a", "Kitchen", "52.521,13.405", " 3e1 ")?;
    write("namedplace:c", "Wide", "52.52,13.405", "wide")?;
    write("namedplace:d", "Nowhere", "52.52", "10")?;
    write("namedplace:e", "Gone", "52.52,13.405", "10")?;
    store.append("namedplace:e", keys::TYPE, None, None)?;
    store.append(
        "namedplace:f",
        keys::TYPE,
        Some(keys::TYPE_NAMED_PLACE),
        None,
    )?;
    store.append("namedplace:f", keys::CENTRE, Some("1,1"), None)?;
    store.append("namedplace:f", keys::RADIUS, Some("5"), None)?;
    let places = NamedPlaces::new(&store);
    let names: Vec<String> = places.all()?.into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["", "home", "Kitchen"]);
    assert_eq!(places.by_id("namedplace:a")?.map(|p| p.radius), Some(30.0));
    let at = |lat, lon| -> Result<Option<String>> {
        Ok(match Coordinates::new(lat, lon) {
            Some(point) => places.at(point)?.map(|p| p.id),
            None => None,
        })
    };
    assert_eq!(at(52.521, 13.405)?.as_deref(), Some("namedplace:a"));
    assert_eq!(at(52.52, 13.405)?.as_deref(), Some("namedplace:b"));
    assert_eq!(at(1.0, 1.0)?.as_deref(), Some("namedplace:f"));
    assert_eq!(at(10.0, 10.0)?, None);
    Ok(())
}
