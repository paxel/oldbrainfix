//! The known units a Tally field's unit typed so far may mean (#159),
//! as the phone's `tally_unit_match.dart`: the suggestions under the
//! unit's field, and the unit the text typed is.

use oldbrainfix_core::tally_units::{self, KNOWN, KnownUnit};
use oldbrainfix_core::{UnitKind, UnitSystem};

use crate::l10n::{L10n, LOCALES};

/// The names of each known unit, in the order of [`KNOWN`], by its code.
pub type UnitNames = Vec<(&'static str, Vec<String>)>;

/// The known units `typed` so far may mean, in the order of [`KNOWN`]:
/// those whose symbol in `system`, or one of whose `names` in it,
/// starts with it, ignoring case and the spaces around it; every one
/// for nothing typed. Only units of `kind` when given, so none for
/// [`UnitKind::Typed`].
pub fn matching_units(
    typed: &str,
    system: UnitSystem,
    names: &UnitNames,
    kind: Option<UnitKind>,
) -> Vec<&'static KnownUnit> {
    let start = typed.trim().to_lowercase();
    let starts = |name: &str| name.to_lowercase().starts_with(&start);
    KNOWN
        .iter()
        .filter(|known| kind.is_none_or(|k| k == known.kind))
        .filter(|known| {
            starts(&tally_units::shown_unit(known.code, system))
                || names
                    .iter()
                    .filter(|(code, _)| *code == known.code)
                    .flat_map(|(_, names)| names)
                    .any(|name| starts(name))
        })
        .collect()
}

/// The unit typed as `typed` is: a known code as typed (ADR 0028), the
/// known unit whose symbol in `system` it is, else the text itself, a
/// unit of the user's own; nothing for no unit.
pub fn unit_of_typed(typed: &str, system: UnitSystem) -> String {
    let text = typed.trim();
    if tally_units::by_code(text).is_some() {
        return text.to_string();
    }
    KNOWN
        .iter()
        .find(|known| tally_units::shown_unit(known.code, system) == text)
        .map_or_else(|| text.to_string(), |known| known.code.to_string())
}

/// The names of each known unit in `system` that [`matching_units`]
/// goes by, in every language the app speaks.
pub fn unit_names(system: UnitSystem) -> UnitNames {
    KNOWN
        .iter()
        .map(|known| {
            let names = LOCALES
                .iter()
                .flat_map(|locale| names_of(&L10n::new(locale), known.code, system).split(','))
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .collect();
            (known.code, names)
        })
        .collect()
}

/// The names of the known unit `code` in `system`, separated by commas.
fn names_of(t: &L10n, code: &str, system: UnitSystem) -> &'static str {
    let imperial = system == UnitSystem::Imperial;
    match code {
        "kg" if imperial => t.unit_names_pound(),
        "kg" => t.unit_names_kg(),
        "g" if imperial => t.unit_names_ounce(),
        "g" => t.unit_names_g(),
        "km" if imperial => t.unit_names_mile(),
        "km" => t.unit_names_km(),
        "m" if imperial => t.unit_names_foot(),
        "m" => t.unit_names_m(),
        "cm" if imperial => t.unit_names_inch(),
        "cm" => t.unit_names_cm(),
        "km/h" if imperial => t.unit_names_mph(),
        "km/h" => t.unit_names_kmh(),
        "°C" if imperial => t.unit_names_fahrenheit(),
        "°C" => t.unit_names_celsius(),
        "°" => t.unit_names_degree(),
        "h" => t.unit_names_hour(),
        "min" => t.unit_names_minute(),
        "s" => t.unit_names_second(),
        _ => "",
    }
}

#[cfg(test)]
mod tests;
