use super::*;

#[test]
fn known_units_are_stored_in_metric_and_shown_in_either_system() {
    assert_eq!(kind_of("kg"), UnitKind::Mass);
    assert_eq!(kind_of("pages"), UnitKind::Typed);
    assert_eq!(shown_unit("km", UnitSystem::Imperial), "mi");
    assert_eq!(shown_unit("pages", UnitSystem::Imperial), "pages");
    assert_eq!(to_base(5.0, "km", UnitSystem::Metric), 5000.0);
    assert_eq!(to_base(1.0, "kg", UnitSystem::Imperial), 0.45359237);
    assert_eq!(to_base(212.0, "°C", UnitSystem::Imperial), 100.0);
    assert_eq!(from_base(100.0, "°C", UnitSystem::Imperial), 212.0);
    assert_eq!(format_shown(8046.72, "km", UnitSystem::Imperial, "."), "5");
    assert_eq!(format_shown(82.5, "kg", UnitSystem::Metric, ","), "82,5");
    assert_eq!(format_shown(60.0, "min", UnitSystem::Metric, "."), "1:00");
    assert_eq!(
        format_shown(3725.0, "s", UnitSystem::Metric, "."),
        "1:02:05"
    );
    assert_eq!(parse_shown("1:30", "min", UnitSystem::Metric), Some(90.0));
    assert_eq!(parse_shown("1,5", "min", UnitSystem::Metric), Some(90.0));
    assert_eq!(parse_shown("1:75", "min", UnitSystem::Metric), None);
    assert_eq!(parse_shown("1e3", "kg", UnitSystem::Metric), None);
    assert_eq!(parse_shown("-4", "°C", UnitSystem::Metric), Some(-4.0));
    assert_eq!(system_for_region(Some("us")), UnitSystem::Imperial);
    assert_eq!(system_for_region(None), UnitSystem::Metric);
}

#[test]
fn numbers_are_kept_as_millionths_written_plain() {
    assert_eq!(encode_number(20.0).as_deref(), Some("20"));
    assert_eq!(encode_number(-3.5).as_deref(), Some("-3.5"));
    assert_eq!(encode_number(5.0 * 1609.344).as_deref(), Some("8046.72"));
    assert_eq!(encode_number(0.1 + 0.2).as_deref(), Some("0.3"));
    assert_eq!(encode_number(0.0000004).as_deref(), Some("0"));
    assert_eq!(encode_number(f64::NAN), None);
    assert_eq!(encode_number(2e9), None);
    assert_eq!(decode_number(Some("8046.72")), Some(8046.72));
    for bad in ["1.", "01", "1.50", "1e3", " 1", "1.0000001", ""] {
        assert_eq!(decode_number(Some(bad)), None, "{bad}");
    }
}
