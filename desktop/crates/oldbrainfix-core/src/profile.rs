//! Profiles (ADR 0017), as the Dart core's `profiles.dart` keeps them:
//! every person has one, with an id, a name and a picture. This Device
//! keeps the user's own as local settings; nothing about it leaves the
//! Device yet (#48 syncs it, signed).

use std::io::Cursor;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::{DynamicImage, ImageFormat, ImageReader, Limits, RgbImage};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use unicode_segmentation::UnicodeSegmentation;

use crate::photos::strip_jpeg_metadata;
use crate::store::Store;
use crate::{Error, Result};

/// The most characters (Unicode code points) a Profile name keeps.
pub const NAME_MAX: usize = 40;

/// The edge of a Profile picture, in pixels; it is square.
pub const PICTURE_EDGE: u32 = 256;

/// The largest picture taken from someone else, in bytes.
pub const PICTURE_MAX_BYTES: usize = 256 * 1024;

/// The longest edge of a picture this core decodes at all, so a small
/// file declaring a huge canvas cannot fill the memory.
pub const DECODE_MAX_EDGE: u32 = 8192;

/// The longest edge of a picture taken from someone else: theirs were
/// made at [`PICTURE_EDGE`].
pub const RECEIVED_MAX_EDGE: u32 = 1024;

const QUALITY: u8 = 85;

/// The colours of the placeholder, ARGB, each dark enough for white
/// initials on it; the Dart core's `profileColors`.
pub const COLORS: [u32; 12] = [
    0xFF00796B, // teal
    0xFF1565C0, // blue
    0xFF6A1B9A, // purple
    0xFFAD1457, // pink
    0xFFC62828, // red
    0xFFBF360C, // deep orange
    0xFF2E7D32, // green
    0xFF4E342E, // brown
    0xFF37474F, // blue grey
    0xFF283593, // indigo
    0xFF006064, // cyan
    0xFF4527A0, // deep purple
];

/// One Profile record: its id, name, picture (a JPEG of
/// [`PICTURE_EDGE`] pixels, or none for the placeholder) and its
/// version, counted up with every change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub picture: Option<Vec<u8>>,
    pub version: i64,
}

impl Profile {
    /// The record as it is kept and, from #48 on, signed: keys sorted, no
    /// spaces, as the Dart core writes it; no picture, no key.
    pub fn encode(&self) -> String {
        let mut map = Map::new();
        map.insert("id".into(), json!(self.id));
        map.insert("name".into(), json!(self.name));
        if let Some(picture) = &self.picture {
            map.insert("picture".into(), json!(BASE64.encode(picture)));
        }
        map.insert("version".into(), json!(self.version));
        Value::Object(map).to_string()
    }

    /// The record `text` holds, its name sanitised again; none when it is
    /// none.
    pub fn parse(text: &str) -> Option<Profile> {
        let json: Value = serde_json::from_str(text).ok()?;
        let map = json.as_object()?;
        let id = map.get("id")?.as_str().filter(|id| !id.is_empty())?;
        let name = map.get("name")?.as_str()?;
        let version = map.get("version")?.as_i64().filter(|v| *v >= 1)?;
        // A whole number written as a double is no version.
        if !map.get("version")?.is_i64() && !map.get("version")?.is_u64() {
            return None;
        }
        let picture = match map.get("picture") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => Some(BASE64.decode(text).ok()?),
            Some(_) => return None,
        };
        Some(Profile {
            id: id.to_string(),
            name: sanitize_name(name),
            picture,
            version,
        })
    }

    fn next(&self, name: Option<String>, picture: Option<Option<Vec<u8>>>) -> Profile {
        Profile {
            id: self.id.clone(),
            name: name.unwrap_or_else(|| self.name.clone()),
            picture: picture.unwrap_or_else(|| self.picture.clone()),
            version: self.version + 1,
        }
    }
}

// ------------------------------------------------------------- names

fn is_whitespace(c: u32) -> bool {
    c == 0x20
        || c == 0xA0
        || c == 0x1680
        || (0x2000..=0x200A).contains(&c)
        || c == 0x202F
        || c == 0x205F
        || c == 0x3000
}

/// Control, bidi and invisible characters a name never keeps.
fn is_dropped(c: u32) -> bool {
    c <= 0x1F
        || (0x7F..=0x9F).contains(&c)
        || c == 0xAD
        || c == 0x34F
        || c == 0x61C
        || c == 0x115F
        || c == 0x1160
        || c == 0x17B4
        || c == 0x17B5
        || c == 0x180E
        || c == 0x200B
        || c == 0x200E
        || c == 0x200F
        || (0x2028..=0x202E).contains(&c)
        || (0x2060..=0x206F).contains(&c)
        || c == 0x3164
        || c == 0xFEFF
        || c == 0xFFA0
        || (0xFFF9..=0xFFFB).contains(&c)
        || (0x1D173..=0x1D17A).contains(&c)
        || (0xE0000..=0xE001F).contains(&c)
}

fn is_joiner(c: u32) -> bool {
    c == 0x200C || c == 0x200D
}

fn is_selector(c: u32) -> bool {
    (0xFE00..=0xFE0F).contains(&c) || (0xE0100..=0xE01EF).contains(&c)
}

fn is_tag(c: u32) -> bool {
    (0xE0020..=0xE007F).contains(&c)
}

const BLACK_FLAG: u32 = 0x1F3F4;

fn visible(c: Option<u32>) -> bool {
    c.is_some_and(|c| !is_whitespace(c) && !is_joiner(c) && !is_selector(c))
}

/// `raw` as a Profile name, as the Dart core's `sanitizeProfileName`
/// makes it: control, bidi and invisible characters removed, trimmed,
/// cut to [`NAME_MAX`] code points and trimmed again; empty when nothing
/// is left, which every screen shows as "No name". A zero-width joiner
/// or non-joiner stays only between two visible characters, a variation
/// selector only right after one, and tag characters only in a flag
/// after the black flag.
pub fn sanitize_name(raw: &str) -> String {
    let mut kept: Vec<u32> = Vec::new();
    for c in raw.chars().map(u32::from) {
        if is_dropped(c) {
            continue;
        }
        if is_tag(c) {
            let last = kept.last().copied();
            let in_flag =
                last == Some(BLACK_FLAG) || last.is_some_and(|l| is_tag(l) && l != 0xE007F);
            if !in_flag {
                continue;
            }
        }
        kept.push(c);
    }
    let mut out: Vec<u32> = Vec::new();
    for (i, &c) in kept.iter().enumerate() {
        let visible_before = visible(out.last().copied());
        if is_joiner(c) {
            if !visible_before || !visible(kept.get(i + 1).copied()) {
                continue;
            }
        } else if is_selector(c) && !visible_before {
            continue;
        }
        out.push(c);
    }
    let mut cut = trim(&out).to_vec();
    if cut.len() > NAME_MAX {
        cut.truncate(NAME_MAX);
        // No joiner or half a flag left hanging at the end.
        while cut.last().is_some_and(|&c| is_joiner(c) || is_tag(c)) {
            cut.pop();
        }
        cut = trim(&cut).to_vec();
    }
    cut.into_iter().filter_map(char::from_u32).collect()
}

fn trim(runes: &[u32]) -> &[u32] {
    let start = runes
        .iter()
        .position(|&c| !is_whitespace(c))
        .unwrap_or(runes.len());
    let end = runes
        .iter()
        .rposition(|&c| !is_whitespace(c))
        .map_or(start, |e| e + 1);
    &runes[start..end]
}

/// The placeholder's initials for `name`: the first character of its
/// first and of its last word, upper case; one for a single word, none
/// for no name.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| is_whitespace(u32::from(c)))
        .filter(|w| !w.is_empty())
        .collect();
    let first = |word: &str| {
        word.graphemes(true)
            .next()
            .map(|g| g.to_uppercase())
            .unwrap_or_default()
    };
    match words.as_slice() {
        [] => String::new(),
        [only] => first(only),
        [head, .., tail] => format!("{}{}", first(head), first(tail)),
    }
}

/// The placeholder's colour for the Profile `id`, ARGB, from [`COLORS`].
pub fn color(id: &str) -> u32 {
    let hash = Sha256::digest(format!("oldbrainfix:profile-colour:{id}").as_bytes());
    COLORS[usize::from(hash[0]) % COLORS.len()]
}

const CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The short code people compare in person, `4F2K-9QX7`: the first 40
/// bits of SHA-256 over `oldbrainfix:profile-code:<id>:` and the first
/// key's 32 public bytes, in Crockford base32.
pub fn short_code(id: &str, first_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("oldbrainfix:profile-code:{id}:").as_bytes());
    hasher.update(first_key);
    let hash = hasher.finalize();
    let bits = hash[..5].iter().fold(0u64, |a, b| (a << 8) | u64::from(*b));
    let mut out = String::new();
    for i in 0..8 {
        if i == 4 {
            out.push('-');
        }
        out.push(char::from(
            CODE_ALPHABET[((bits >> (35 - 5 * i)) & 31) as usize],
        ));
    }
    out
}

// ----------------------------------------------------------- pictures

/// `bytes` decoded and turned upright; none when they are no JPEG, PNG
/// or WebP, are broken, or declare a canvas wider or higher than
/// `max_edge`.
fn decode(bytes: &[u8], max_edge: u32) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP)
    ) {
        return None;
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(max_edge);
    limits.max_image_height = Some(max_edge);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().ok()?;
    let orientation = image::ImageDecoder::orientation(&mut decoder).ok();
    let mut image = DynamicImage::from_decoder(decoder).ok()?;
    if let Some(orientation) = orientation {
        image.apply_orientation(orientation);
    }
    Some(image)
}

/// The width and height of the upright picture `bytes`, or none when it
/// is none this core reads.
pub fn picture_size(bytes: &[u8]) -> Option<(u32, u32)> {
    decode(bytes, DECODE_MAX_EDGE).map(|image| (image.width(), image.height()))
}

/// The square at `left`, `top` with the edge `size` (pixels of the
/// upright picture `bytes`, kept inside it) as a Profile picture: JPEG,
/// [`PICTURE_EDGE`] pixels, transparent parts white, no metadata. None
/// when `bytes` are no picture.
pub fn make_picture(bytes: &[u8], left: i64, top: i64, size: i64) -> Option<Vec<u8>> {
    let image = decode(bytes, DECODE_MAX_EDGE)?;
    square(&image, left, top, size)
}

/// The middle square of the picture `bytes` as a Profile picture, for a
/// picture nobody cropped, such as the login's account picture.
pub fn square_picture(bytes: &[u8]) -> Option<Vec<u8>> {
    middle_square(bytes, DECODE_MAX_EDGE)
}

/// A Profile picture someone else sent, decoded and made again as
/// [`square_picture`] makes it; none when it is larger than
/// [`PICTURE_MAX_BYTES`], wider or higher than [`RECEIVED_MAX_EDGE`],
/// or no picture.
pub fn received_picture(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.len() > PICTURE_MAX_BYTES {
        return None;
    }
    middle_square(bytes, RECEIVED_MAX_EDGE)
}

fn middle_square(bytes: &[u8], max_edge: u32) -> Option<Vec<u8>> {
    let image = decode(bytes, max_edge)?;
    let size = image.width().min(image.height());
    square(
        &image,
        i64::from((image.width() - size) / 2),
        i64::from((image.height() - size) / 2),
        i64::from(size),
    )
}

fn square(image: &DynamicImage, left: i64, top: i64, size: i64) -> Option<Vec<u8>> {
    let (width, height) = (i64::from(image.width()), i64::from(image.height()));
    let edge = size.clamp(1, width.min(height));
    let x = left.clamp(0, width - edge);
    let y = top.clamp(0, height - edge);
    let edge = u32::try_from(edge).ok()?;
    let cropped = image.crop_imm(u32::try_from(x).ok()?, u32::try_from(y).ok()?, edge, edge);
    let resized = cropped.resize_exact(
        PICTURE_EDGE,
        PICTURE_EDGE,
        image::imageops::FilterType::Triangle,
    );
    let rgba = resized.to_rgba8();
    // A JPEG has no transparency: what shows through goes on white.
    let rgb = RgbImage::from_fn(PICTURE_EDGE, PICTURE_EDGE, |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let over = |c: u8| ((u16::from(c) * u16::from(a) + 255 * (255 - u16::from(a))) / 255) as u8;
        image::Rgb([over(r), over(g), over(b)])
    });
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, QUALITY)
        .encode_image(&rgb)
        .ok()?;
    Some(strip_jpeg_metadata(&out))
}

// ------------------------------------------------------- own Profile

const RECORD: &str = "profile";
const FIRST: &str = "profile:first";
const NAME_CHOSEN: &str = "profile:nameChosen";
const PICTURE_CHOSEN: &str = "profile:pictureChosen";

/// The user's own Profile on this Device (ADR 0017), under the local
/// settings the Dart core's `OwnProfile` uses: `profile` holds the
/// record, `profile:first` the public key of the Device that made it
/// (base64), and `profile:nameChosen` and `profile:pictureChosen` say the
/// user replaced a default.
pub struct OwnProfile<'a> {
    store: &'a Store,
}

impl<'a> OwnProfile<'a> {
    pub fn new(store: &'a Store) -> OwnProfile<'a> {
        OwnProfile { store }
    }

    /// The user's own Profile; none before [`Self::apply_defaults`] made
    /// it.
    pub fn get(&self) -> Result<Option<Profile>> {
        Ok(self
            .store
            .local_setting(RECORD)?
            .and_then(|t| Profile::parse(&t)))
    }

    /// The first key of the own Profile: this Device's signing key when
    /// it made the Profile.
    pub fn first_key(&self) -> Result<Option<Vec<u8>>> {
        Ok(self
            .store
            .local_setting(FIRST)?
            .and_then(|t| BASE64.decode(t).ok()))
    }

    /// The own Profile's short code, or none before it exists.
    pub fn code(&self) -> Result<Option<String>> {
        Ok(match (self.get()?, self.first_key()?) {
            (Some(profile), Some(key)) => Some(short_code(&profile.id, &key)),
            _ => None,
        })
    }

    fn chosen(&self, key: &str) -> Result<bool> {
        Ok(self.store.local_setting(key)?.as_deref() == Some("true"))
    }

    fn keep(&self, profile: &Profile) -> Result<()> {
        self.store.set_local_setting(RECORD, &profile.encode())
    }

    /// Makes the own Profile the first time, with a new id, the default
    /// `name` and `picture` (already a Profile picture, or none for the
    /// placeholder) and this Device's key as its first key; later it
    /// takes the defaults the user has not replaced. Asks nothing; counts
    /// the version up only when something changed.
    pub fn apply_defaults(&self, name: &str, picture: Option<Vec<u8>>) -> Result<Profile> {
        let clean = sanitize_name(name);
        let Some(current) = self.get()? else {
            let made = Profile {
                id: self.store.new_id()?,
                name: clean,
                picture,
                version: 1,
            };
            let key = self.store.signing_key()?.public_key();
            self.store.set_local_setting(FIRST, &BASE64.encode(key))?;
            self.keep(&made)?;
            return Ok(made);
        };
        let take_name = !self.chosen(NAME_CHOSEN)? && clean != current.name;
        let take_picture = !self.chosen(PICTURE_CHOSEN)? && picture != current.picture;
        if !take_name && !take_picture {
            return Ok(current);
        }
        let next = current.next(take_name.then_some(clean), take_picture.then_some(picture));
        self.keep(&next)?;
        Ok(next)
    }

    /// Gives the own Profile the `name` the user typed, sanitised.
    pub fn rename(&self, name: &str) -> Result<Profile> {
        let current = self.own()?;
        self.store.set_local_setting(NAME_CHOSEN, "true")?;
        let clean = sanitize_name(name);
        if clean == current.name {
            return Ok(current);
        }
        let next = current.next(Some(clean), None);
        self.keep(&next)?;
        Ok(next)
    }

    /// Gives the own Profile the `picture` the user chose (a Profile
    /// picture), or the placeholder for none.
    pub fn set_picture(&self, picture: Option<Vec<u8>>) -> Result<Profile> {
        let current = self.own()?;
        self.store.set_local_setting(PICTURE_CHOSEN, "true")?;
        if picture == current.picture {
            return Ok(current);
        }
        let next = current.next(None, Some(picture));
        self.keep(&next)?;
        Ok(next)
    }

    fn own(&self) -> Result<Profile> {
        self.get()?
            .ok_or_else(|| Error::Invalid("no own Profile yet: apply the defaults first".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    /// A `width` × `height` picture, red on the left half, blue on the
    /// right.
    fn halves(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let image = RgbImage::from_fn(width, height, |x, _| {
            if x < width / 2 {
                image::Rgb([255, 0, 0])
            } else {
                image::Rgb([0, 0, 255])
            }
        });
        let mut out = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(image)
            .write_to(&mut out, format)
            .expect("encodes");
        out.into_inner()
    }

    /// The marker bytes of `jpeg`'s segments before the scan.
    fn markers(jpeg: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 2;
        while i + 4 <= jpeg.len() && jpeg[i] == 0xFF {
            let marker = jpeg[i + 1];
            if marker == 0xDA {
                break;
            }
            out.push(marker);
            i += 2 + ((usize::from(jpeg[i + 2]) << 8) | usize::from(jpeg[i + 3]));
        }
        out
    }

    fn assert_profile_picture(picture: Option<&[u8]>) {
        let picture = picture.expect("a picture");
        assert_eq!(&picture[..2], &[0xFF, 0xD8]);
        let decoded = image::load_from_memory(picture).expect("decodes");
        assert_eq!((decoded.width(), decoded.height()), (256, 256));
        assert!(
            markers(picture)
                .iter()
                .all(|m| !(0xE1..=0xEF).contains(m) && *m != 0xFE)
        );
    }

    #[test]
    fn names_keep_every_script_and_emoji() {
        assert_eq!(sanitize_name("  Ada  "), "Ada");
        assert_eq!(sanitize_name("李小龍"), "李小龍");
        assert_eq!(
            sanitize_name("Mum \u{1F469}\u{200D}\u{1F467} \u{2764}\u{FE0F}"),
            "Mum \u{1F469}\u{200D}\u{1F467} \u{2764}\u{FE0F}"
        );
    }

    #[test]
    fn names_lose_control_bidi_and_zero_width_characters() {
        assert_eq!(sanitize_name("A\u{0}d\u{7}a\n\tB\u{7F}\u{85}"), "AdaB");
        assert_eq!(
            sanitize_name("\u{202E}Ada\u{202C} \u{2066}Bo\u{2069}\u{200E}\u{200F}\u{61C}"),
            "Ada Bo"
        );
        assert_eq!(sanitize_name("\u{200B} Ada\u{2060}\u{FEFF}\u{AD} "), "Ada");
        assert_eq!(sanitize_name("\u{200D}Ada\u{200D}"), "Ada");
        assert_eq!(sanitize_name("A\u{200D}\u{200D}d"), "A\u{200D}d");
        assert_eq!(sanitize_name("\u{FE0F}Ada"), "Ada");
        assert_eq!(sanitize_name("Ada\u{E0041}\u{E0042}"), "Ada");
    }

    #[test]
    fn names_are_cut_to_40_code_points_and_may_be_empty() {
        assert_eq!(sanitize_name(&"x".repeat(50)), "x".repeat(40));
        assert_eq!(sanitize_name(&"😀".repeat(45)), "😀".repeat(40));
        assert_eq!(
            sanitize_name(&format!("{} bc", "a".repeat(39))),
            "a".repeat(39)
        );
        assert_eq!(sanitize_name(" \u{200B}\u{202E}\n "), "");
    }

    #[test]
    fn the_placeholder_has_initials_and_a_colour_from_the_id() {
        assert_eq!(initials("ada lovelace"), "AL");
        assert_eq!(initials("Ada King Lovelace"), "AL");
        assert_eq!(initials("Ada"), "A");
        assert_eq!(initials(""), "");
        assert_eq!(
            initials("\u{1F469}\u{200D}\u{1F467} Mum"),
            "\u{1F469}\u{200D}\u{1F467}M"
        );
        assert_eq!(color("abc"), color("abc"));
        assert!(COLORS.contains(&color("abc")));
    }

    #[test]
    fn the_short_code_depends_on_id_and_key() {
        let key = crate::SigningKey::from_seed([0; 32]).public_key();
        let mut seed = [0; 32];
        seed[0] = 1;
        let other = crate::SigningKey::from_seed(seed).public_key();
        let code = short_code("p1", &key);
        assert_eq!(code.len(), 9);
        assert_eq!(&code[4..5], "-");
        assert_eq!(code, short_code("p1", &key));
        assert_ne!(code, short_code("p1", &other));
        assert_ne!(code, short_code("p2", &key));
    }

    #[test]
    fn a_chosen_square_becomes_a_256_jpeg_without_metadata() {
        let source = halves(600, 400, ImageFormat::Png);
        assert_eq!(picture_size(&source), Some((600, 400)));
        let picture = make_picture(&source, 300, 0, 400);
        assert_profile_picture(picture.as_deref());
        let pixel = image::load_from_memory(&picture.unwrap_or_default())
            .map(|i| i.to_rgb8().get_pixel(128, 128).0)
            .unwrap_or_default();
        assert!(pixel[2] > 200 && pixel[0] < 60, "{pixel:?}");
        assert_profile_picture(make_picture(&source, 900, -40, 2000).as_deref());
    }

    #[test]
    fn a_picture_received_is_made_again_and_refused_when_too_big_or_none() {
        let jpeg = halves(512, 512, ImageFormat::Jpeg);
        // An Exif segment that must not come through.
        let mut with_exif = jpeg[..2].to_vec();
        with_exif.extend_from_slice(&[0xFF, 0xE1, 0x00, 0x08, b'E', b'x', b'i', b'f', 0, 0]);
        with_exif.extend_from_slice(&jpeg[2..]);
        assert_profile_picture(received_picture(&with_exif).as_deref());
        let mut big = halves(64, 64, ImageFormat::Png);
        big.resize(PICTURE_MAX_BYTES + 1, 0);
        assert_eq!(received_picture(&big), None);
        assert_eq!(received_picture(b"hi"), None);
        assert_eq!(received_picture(&[0xFF, 0xD8, 0xFF, 0, 1, 2]), None);
        let png = halves(64, 64, ImageFormat::Png);
        assert_eq!(received_picture(&png[..png.len() / 2]), None);
        // A format left out, even when it would decode.
        assert_eq!(
            received_picture(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"),
            None
        );
    }

    #[test]
    fn a_canvas_wider_than_the_decoder_takes_is_refused() {
        let wide = |width| halves(width, 1, ImageFormat::Png);
        assert_eq!(picture_size(&wide(8192)), Some((8192, 1)));
        assert_eq!(picture_size(&wide(8193)), None);
        assert_eq!(square_picture(&wide(8193)), None);
        // Pictures from others are made at 256: anything above 1024 is
        // refused.
        assert_profile_picture(received_picture(&wide(1024)).as_deref());
        assert_eq!(received_picture(&wide(1025)), None);
    }

    #[test]
    fn records_are_written_sorted_and_malformed_ones_refused() {
        let profile = Profile {
            id: "p1".into(),
            name: "Ada".into(),
            picture: Some(vec![1, 2, 3]),
            version: 2,
        };
        assert_eq!(
            profile.encode(),
            r#"{"id":"p1","name":"Ada","picture":"AQID","version":2}"#
        );
        assert_eq!(Profile::parse(&profile.encode()), Some(profile));
        for bad in [
            "nope",
            r#"{"id":"","name":"A","version":1}"#,
            r#"{"id":"p","name":"A","version":1.5}"#,
            r#"{"id":"p","name":"A","version":0}"#,
            r#"{"id":"p","name":3,"version":1}"#,
            r#"{"id":"p","name":"A","picture":"%%","version":1}"#,
            r#"{"id":"p","name":"A","picture":"AQ-_","version":1}"#,
            r#"{"id":"p","name":"A","picture":"AQJ=","version":1}"#,
            r#"{"id":"p","name":"\ud800A","version":1}"#,
        ] {
            assert_eq!(Profile::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_own_profile_follows_the_defaults_until_the_user_chose() -> TestResult {
        let store = crate::store::tests::store();
        let own = OwnProfile::new(&store);
        assert_eq!(own.get()?, None);
        assert!(own.rename("Ada").is_err());
        let made = own.apply_defaults(" desk ", None)?;
        assert_eq!((made.name.as_str(), made.version), ("desk", 1));
        assert_eq!(
            own.first_key()?,
            Some(store.signing_key()?.public_key().to_vec())
        );
        assert_eq!(
            own.code()?,
            Some(short_code(&made.id, &store.signing_key()?.public_key()))
        );
        assert_eq!(own.apply_defaults("desk", None)?.version, 1);
        assert_eq!(own.apply_defaults("Ada's desk", None)?.version, 2);
        own.rename("Ada")?;
        assert_eq!(own.apply_defaults("other", None)?.name, "Ada");
        assert_eq!(own.rename("Ada\u{200B}")?.version, 3);
        own.set_picture(None)?;
        let picture = square_picture(&halves(64, 64, ImageFormat::Png));
        assert_eq!(own.apply_defaults("other", picture)?.picture, None);
        Ok(())
    }
}
