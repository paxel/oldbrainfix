//! The units of a Tally's fields and the numbers they hold (ADR 0028),
//! as the Dart core's `tally_units.dart` has them.
//!
//! A known unit is stored in metric, in the base unit of its kind
//! (kilograms, metres, km/h, °C, degrees, seconds), and shown in the
//! Device's system; a typed unit is kept and shown as typed. Every number
//! in the log is a whole count of millionths of its unit, written as a
//! plain decimal, so both cores store, sum and compare the same numbers
//! without a floating-point text in between. Every conversion runs the
//! same operations in the same order as the Dart core's, so both come to
//! the same double.

/// What a unit measures; `Typed` for a unit the user typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    Mass,
    Distance,
    Speed,
    Temperature,
    Angle,
    Time,
    Typed,
}

impl UnitKind {
    /// The name the Dart core writes.
    pub fn name(self) -> &'static str {
        match self {
            UnitKind::Mass => "mass",
            UnitKind::Distance => "distance",
            UnitKind::Speed => "speed",
            UnitKind::Temperature => "temperature",
            UnitKind::Angle => "angle",
            UnitKind::Time => "time",
            UnitKind::Typed => "typed",
        }
    }
}

/// How a Device shows known units (ADR 0008).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    Metric,
    Imperial,
}

impl UnitSystem {
    pub const ALL: [UnitSystem; 2] = [UnitSystem::Metric, UnitSystem::Imperial];

    /// The name the Dart core keeps.
    pub fn name(self) -> &'static str {
        match self {
            UnitSystem::Metric => "metric",
            UnitSystem::Imperial => "imperial",
        }
    }

    /// The system `name` names; `None` for anything else.
    pub fn parse(name: &str) -> Option<UnitSystem> {
        UnitSystem::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// How a field counts a day with several entries: their sum, or the
/// largest of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TallyRule {
    Sum,
    Max,
}

impl TallyRule {
    /// The name the log keeps.
    pub fn name(self) -> &'static str {
        match self {
            TallyRule::Sum => "sum",
            TallyRule::Max => "max",
        }
    }

    /// The rule `name` names; `None` for anything else.
    pub fn parse(name: &str) -> Option<TallyRule> {
        [TallyRule::Sum, TallyRule::Max]
            .into_iter()
            .find(|r| r.name() == name)
    }
}

/// A unit of the known list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KnownUnit {
    /// The unit as stored and shown in metric, e.g. `kg`.
    pub code: &'static str,
    pub kind: UnitKind,
    /// How many base units of `kind` one of it is.
    pub factor: f64,
    /// The unit shown in imperial, e.g. `lb`; `code` when both systems
    /// share it.
    pub imperial: &'static str,
    /// How many base units one `imperial` is.
    pub imperial_factor: f64,
}

const fn unit(
    code: &'static str,
    kind: UnitKind,
    factor: f64,
    imperial: &'static str,
    imperial_factor: f64,
) -> KnownUnit {
    KnownUnit {
        code,
        kind,
        factor,
        imperial,
        imperial_factor,
    }
}

/// The known units, in the order the field page offers them.
pub static KNOWN: [KnownUnit; 11] = [
    unit("kg", UnitKind::Mass, 1.0, "lb", 0.45359237),
    unit("g", UnitKind::Mass, 0.001, "oz", 0.028349523125),
    unit("km", UnitKind::Distance, 1000.0, "mi", 1609.344),
    unit("m", UnitKind::Distance, 1.0, "ft", 0.3048),
    unit("cm", UnitKind::Distance, 0.01, "in", 0.0254),
    unit("km/h", UnitKind::Speed, 1.0, "mph", 1.609344),
    unit("°C", UnitKind::Temperature, 1.0, "°F", 1.0),
    unit("°", UnitKind::Angle, 1.0, "°", 1.0),
    unit("h", UnitKind::Time, 3600.0, "h", 3600.0),
    unit("min", UnitKind::Time, 60.0, "min", 60.0),
    unit("s", UnitKind::Time, 1.0, "s", 1.0),
];

/// The regions whose Devices show imperial units unless set otherwise.
pub const IMPERIAL_REGIONS: [&str; 3] = ["US", "LR", "MM"];

/// The largest number a field holds, in its base unit, either way.
pub const MAX_VALUE: f64 = 1e9;

/// `MAX_VALUE` in millionths.
const MAX_MICRO: i64 = 1_000_000_000_000_000;

/// Beyond any number a field holds in any unit: what is shown as
/// nothing.
const LARGEST_SHOWN: f64 = 1e13;

/// The known unit `unit` is, or `None` for a typed one.
pub fn by_code(unit: &str) -> Option<&'static KnownUnit> {
    KNOWN.iter().find(|k| k.code == unit)
}

/// What `unit` measures.
pub fn kind_of(unit: &str) -> UnitKind {
    by_code(unit).map_or(UnitKind::Typed, |k| k.kind)
}

/// The rule a new field with a unit of `kind` gets: the largest for
/// what is lifted, run or measured at once, the sum for the rest.
pub fn default_rule(kind: UnitKind) -> TallyRule {
    match kind {
        UnitKind::Mass | UnitKind::Speed | UnitKind::Temperature | UnitKind::Angle => {
            TallyRule::Max
        }
        UnitKind::Distance | UnitKind::Time | UnitKind::Typed => TallyRule::Sum,
    }
}

/// The system a Device in `region` (an ISO country code) shows unless
/// it was set otherwise.
pub fn system_for_region(region: Option<&str>) -> UnitSystem {
    match region {
        Some(r) if IMPERIAL_REGIONS.contains(&r.to_uppercase().as_str()) => UnitSystem::Imperial,
        _ => UnitSystem::Metric,
    }
}

/// The unit `unit` is shown as in `system`.
pub fn shown_unit(unit: &str, system: UnitSystem) -> String {
    match by_code(unit) {
        Some(known) if system == UnitSystem::Imperial => known.imperial.to_string(),
        _ => unit.to_string(),
    }
}

/// `shown`, as entered in `unit` shown in `system`, in the base unit it
/// is stored in.
pub fn to_base(shown: f64, unit: &str, system: UnitSystem) -> f64 {
    let Some(known) = by_code(unit) else {
        return shown;
    };
    if system == UnitSystem::Metric {
        return shown * known.factor;
    }
    if known.kind == UnitKind::Temperature {
        return (shown - 32.0) * 5.0 / 9.0;
    }
    shown * known.imperial_factor
}

/// `base`, stored in the base unit of `unit`, as shown in `unit` in
/// `system`.
pub fn from_base(base: f64, unit: &str, system: UnitSystem) -> f64 {
    let Some(known) = by_code(unit) else {
        return base;
    };
    if system == UnitSystem::Metric {
        return base / known.factor;
    }
    if known.kind == UnitKind::Temperature {
        return base * 9.0 / 5.0 + 32.0;
    }
    base / known.imperial_factor
}

/// `base` as a field of `unit` shows it in `system`, without the unit: a
/// time as `m:ss` or `h:mm:ss`, anything else with up to two decimals,
/// `decimal` between them.
pub fn format_shown(base: f64, unit: &str, system: UnitSystem, decimal: &str) -> String {
    if !base.is_finite() {
        return String::new();
    }
    if kind_of(unit) == UnitKind::Time {
        let seconds = base.round() as i64;
        let sign = if seconds < 0 { "-" } else { "" };
        let s = seconds.abs();
        let (h, m, ss) = (s / 3600, s % 3600 / 60, s % 60);
        return if h > 0 {
            format!("{sign}{h}:{m:02}:{ss:02}")
        } else {
            format!("{sign}{m}:{ss:02}")
        };
    }
    let shown = from_base(base, unit, system);
    if !shown.is_finite() || shown.abs() > LARGEST_SHOWN {
        return String::new();
    }
    plain_decimal((shown * 100.0).round() as i64, 2).replacen('.', decimal, 1)
}

/// The number `text`, typed into a field of `unit` shown in `system`, in
/// its base unit; `None` when it is no number or out of range. A decimal
/// comma counts as a point; a time also reads as `m:ss` or `h:mm:ss`.
pub fn parse_shown(text: &str, unit: &str, system: UnitSystem) -> Option<f64> {
    let typed = text.trim();
    if kind_of(unit) == UnitKind::Time && typed.contains(':') {
        let (negative, rest) = match typed.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, typed),
        };
        let parts: Vec<&str> = rest.split(':').collect();
        let first = parts.first()?;
        let shaped = (2..=3).contains(&parts.len())
            && (1..=6).contains(&first.len())
            && digits(first)
            && parts[1..].iter().all(|p| p.len() == 2 && digits(p));
        if !shaped {
            return None;
        }
        let mut seconds: i64 = 0;
        for (i, part) in parts.iter().enumerate() {
            let n: i64 = part.parse().ok()?;
            if i > 0 && n > 59 {
                return None;
            }
            seconds = seconds * 60 + n;
        }
        let base = (if negative { -seconds } else { seconds }) as f64;
        return (base.abs() <= MAX_VALUE).then_some(base);
    }
    if !is_number(typed) {
        return None;
    }
    let shown: f64 = typed.replacen(',', ".", 1).parse().ok()?;
    let base = to_base(shown, unit, system);
    (base.is_finite() && base.abs() <= MAX_VALUE).then_some(base)
}

/// Whether `text` is nothing but ASCII digits, at least one.
fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// A number as typed: `-` or nothing, digits, then a point or a comma
/// and more digits or nothing.
fn is_number(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    match unsigned.find(['.', ',']) {
        Some(at) => digits(&unsigned[..at]) && digits(&unsigned[at + 1..]),
        None => digits(unsigned),
    }
}

/// `value` as the log keeps it: millionths, rounded half away from zero,
/// written as a plain decimal (`8046.72`, `-3.5`, `20`); `None` for a
/// number that is not finite or beyond [`MAX_VALUE`].
pub fn encode_number(value: f64) -> Option<String> {
    micro(value).map(|m| plain_decimal(m, 6))
}

/// The number `text` holds in millionths, as [`encode_number`] wrote it;
/// `None` for anything else.
pub fn decode_micro(text: Option<&str>) -> Option<i64> {
    let text = text?;
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, fraction) = match rest.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (rest, None),
    };
    let whole_ok = whole == "0" || (digits(whole) && whole.len() <= 10 && !whole.starts_with('0'));
    let fraction_ok = fraction.is_none_or(|f| digits(f) && f.len() <= 6 && !f.ends_with('0'));
    if !whole_ok || !fraction_ok {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction: i64 = format!("{:0<6}", fraction.unwrap_or("")).parse().ok()?;
    let micro = whole * 1_000_000 + fraction;
    if micro > MAX_MICRO {
        return None;
    }
    Some(if negative { -micro } else { micro })
}

/// The number `text` holds, as [`encode_number`] wrote it; `None` for
/// anything else.
pub fn decode_number(text: Option<&str>) -> Option<f64> {
    decode_micro(text).map(|m| m as f64 / 1_000_000.0)
}

/// `value` in millionths, rounded half away from zero; `None` when it is
/// not finite or beyond [`MAX_VALUE`].
pub fn micro(value: f64) -> Option<i64> {
    if !value.is_finite() || value.abs() > MAX_VALUE {
        return None;
    }
    Some((value * 1_000_000.0).round() as i64)
}

/// `scaled`, a count of 10^-`places`, as a plain decimal without
/// trailing zeros.
fn plain_decimal(scaled: i64, places: usize) -> String {
    let sign = if scaled < 0 { "-" } else { "" };
    let digits = format!("{:0>width$}", scaled.unsigned_abs(), width = places + 1);
    let (whole, fraction) = digits.split_at(digits.len() - places);
    let fraction = fraction.trim_end_matches('0');
    let text = if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    };
    if text == "0" {
        text
    } else {
        format!("{sign}{text}")
    }
}

#[cfg(test)]
mod tests {
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
}
