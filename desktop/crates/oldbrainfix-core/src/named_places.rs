//! Named Places as the Dart core's `NamedPlaces` reads them, and which
//! one a point lies in, for the Capture Context of Notes and Sections.
//! Only the reading: the desktop does not make or change Named Places.

use std::f64::consts::PI;

use crate::Result;
use crate::store::{Store, utf16_cmp};
use crate::transcript_prompts::{dart_trim, lower};

/// Field keys of a Named Place.
pub mod keys {
    pub const TYPE: &str = "$type";
    pub const TYPE_NAMED_PLACE: &str = "namedPlace";
    pub const NAME: &str = "name";
    /// The centre, `<latitude>,<longitude>`.
    pub const CENTRE: &str = "centre";
    /// How far around the centre still counts, in metres.
    pub const RADIUS: &str = "radius";
}

/// A point on Earth in degrees, as the platform reports it: always
/// finite, the latitude within ±90° and the longitude within ±180°.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coordinates {
    latitude: f64,
    longitude: f64,
}

// Never NaN: `new` takes finite degrees only, so `==` is an equivalence.
impl Eq for Coordinates {}

impl Coordinates {
    /// The point at `latitude` and `longitude`; `None` when either is not
    /// finite or out of range.
    pub fn new(latitude: f64, longitude: f64) -> Option<Coordinates> {
        let within =
            |degrees: f64, max: f64| degrees.is_finite() && (-max..=max).contains(&degrees);
        (within(latitude, 90.0) && within(longitude, 180.0)).then_some(Coordinates {
            latitude,
            longitude,
        })
    }

    pub fn latitude(self) -> f64 {
        self.latitude
    }

    pub fn longitude(self) -> f64 {
        self.longitude
    }

    /// The point stored as `<latitude>,<longitude>`, as the Dart core's
    /// `Coordinates.decode` reads it; `None` for anything else.
    pub fn decode(value: Option<&str>) -> Option<Coordinates> {
        let mut parts = value?.split(',');
        let (Some(latitude), Some(longitude), None) = (parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        Coordinates::new(dart_double(latitude)?, dart_double(longitude)?)
    }
}

/// The great-circle distance between `a` and `b` in metres (haversine,
/// on a sphere of the Earth's mean radius), worked out as the Dart core
/// does it.
pub fn distance_in_metres(a: Coordinates, b: Coordinates) -> f64 {
    const EARTH_RADIUS: f64 = 6371008.8;
    let rad = |degrees: f64| degrees * PI / 180.0;
    let d_lat = rad(b.latitude - a.latitude);
    let d_lon = rad(b.longitude - a.longitude);
    let h = (d_lat / 2.0).sin().powf(2.0)
        + rad(a.latitude).cos() * rad(b.latitude).cos() * (d_lon / 2.0).sin().powf(2.0);
    // Finite coordinates give a finite `h`, where Rust's and Dart's `min`
    // agree.
    2.0 * EARTH_RADIUS * h.min(1.0).sqrt().asin()
}

/// A spot the user gave a name (CONTEXT.md).
#[derive(Debug, Clone)]
pub struct NamedPlace {
    /// Entity id, `namedplace:<id>`.
    pub id: String,
    pub name: String,
    pub centre: Coordinates,
    /// How far around `centre` still counts, in metres; read as stored,
    /// so a garbled one may be NaN or infinite, as in the Dart core.
    pub radius: f64,
}

// The radius compares by its bits, so a NaN one equals itself.
impl PartialEq for NamedPlace {
    fn eq(&self, other: &NamedPlace) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.centre == other.centre
            && self.radius.to_bits() == other.radius.to_bits()
    }
}

impl Eq for NamedPlace {}

/// The Named Place among `places` that `point` lies in, or `None` when it
/// lies in none. Where Named Places overlap, the smallest one wins, as
/// the most precise ("Kitchen" inside "Home"); between equal ones, the
/// nearest centre, then the first.
pub fn named_place_at<'p>(
    places: impl IntoIterator<Item = &'p NamedPlace>,
    point: Coordinates,
) -> Option<&'p NamedPlace> {
    let mut best: Option<(&NamedPlace, f64)> = None;
    for place in places {
        let distance = distance_in_metres(place.centre, point);
        // Written as the Dart core's, so a NaN radius counts the same.
        if distance > place.radius {
            continue;
        }
        let better = match best {
            None => true,
            Some((held, at)) => {
                place.radius < held.radius || (place.radius == held.radius && distance < at)
            }
        };
        if better {
            best = Some((place, distance));
        }
    }
    best.map(|(place, _)| place)
}

/// Reading Named Places.
pub struct NamedPlaces<'a> {
    store: &'a Store,
}

impl<'a> NamedPlaces<'a> {
    pub fn new(store: &'a Store) -> NamedPlaces<'a> {
        NamedPlaces { store }
    }

    /// The Named Place `id`, or `None` when there is none or its centre
    /// or radius cannot be read.
    pub fn by_id(&self, id: &str) -> Result<Option<NamedPlace>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_NAMED_PLACE) {
            return Ok(None);
        }
        let centre = Coordinates::decode(fields.get(keys::CENTRE));
        let radius = dart_double(fields.get(keys::RADIUS).unwrap_or(""));
        let (Some(centre), Some(radius)) = (centre, radius) else {
            return Ok(None);
        };
        Ok(Some(NamedPlace {
            id: id.to_string(),
            name: fields.get(keys::NAME).unwrap_or("").to_string(),
            centre,
            radius,
        }))
    }

    /// Every Named Place, by name, ignoring case.
    pub fn all(&self) -> Result<Vec<NamedPlace>> {
        let mut places = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NAMED_PLACE)? {
            places.extend(self.by_id(&id)?);
        }
        places.sort_by(|a, b| utf16_cmp(&lower(&a.name), &lower(&b.name)));
        Ok(places)
    }

    /// The Named Place `point` lies in, or `None` (see
    /// [`named_place_at`]).
    pub fn at(&self, point: Coordinates) -> Result<Option<NamedPlace>> {
        Ok(named_place_at(&self.all()?, point).cloned())
    }
}

/// A number as Dart's `double.tryParse` reads it: around the whitespace
/// Dart's `trim` takes off, an optional sign, then `NaN`, `Infinity`, or
/// digits with an optional point and fraction (one of them not empty)
/// and an optional exponent. Rust's own parser takes `inf` and `nan` in
/// any case besides, which Dart does not.
pub(crate) fn dart_double(text: &str) -> Option<f64> {
    let text = dart_trim(text);
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let negative = text.starts_with('-');
    match unsigned {
        "NaN" => return Some(f64::NAN),
        "Infinity" if negative => return Some(f64::NEG_INFINITY),
        "Infinity" => return Some(f64::INFINITY),
        _ => {}
    }
    let digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mantissa_ok =
        digits(whole) && digits(fraction) && !(whole.is_empty() && fraction.is_empty());
    let exponent_ok = exponent.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && digits(e)
    });
    if !mantissa_ok || !exponent_ok {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests;
