//! Profiles (ADR 0017), as the Dart core's `profiles.dart` keeps them:
//! every person has one, with an id, a name and a picture. This Device
//! keeps the user's own as local settings, signed and bound to the keys
//! of the user's own Devices, and syncs it with them (#48).

use std::collections::HashSet;
use std::io::Cursor;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::{DynamicImage, ImageFormat, ImageReader, Limits, RgbImage};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use unicode_segmentation::UnicodeSegmentation;

use crate::photos::strip_jpeg_metadata;
use crate::signing::{SigningKey, verify_signature};
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
    /// The record as it is kept and signed: keys sorted, no spaces, as
    /// the Dart core writes it; no picture, no key.
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
        #[cfg(test)]
        tests::PARSED.with(|n| n.set(n.get() + 1));
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
pub(crate) fn decode(bytes: &[u8], max_edge: u32) -> Option<DynamicImage> {
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

// ------------------------------------------------ signing and binding

/// The bytes a Profile record's signature covers: the record as sent.
pub fn record_bytes(record: &str) -> Vec<u8> {
    format!("oldbrainfix:profile:{record}").into_bytes()
}

/// The bytes that bind the key `key` (base64) to the Profile `id`.
pub fn key_bytes(id: &str, key: &str) -> Vec<u8> {
    format!("oldbrainfix:profile-key:{id}:{key}").into_bytes()
}

/// The bytes that make the Profile id `alias` an alias of `to`.
pub fn alias_bytes(alias: &str, to: &str) -> Vec<u8> {
    format!("oldbrainfix:profile-alias:{alias}:{to}").into_bytes()
}

/// Whether `text` is a public key in standard base64, as written.
pub(crate) fn is_key(text: &str) -> bool {
    BASE64
        .decode(text)
        .is_ok_and(|bytes| bytes.len() == 32 && BASE64.encode(&bytes) == text)
}

fn verifies(by: &str, message: &[u8], sig: &str) -> bool {
    BASE64
        .decode(by)
        .is_ok_and(|key| verify_signature(&key, message, Some(sig)))
}

/// A key joined to a Profile's binding: `key`, and `sig` over
/// [`key_bytes`] by `by`, a key bound before it (both base64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileKey {
    pub key: String,
    pub by: String,
    pub sig: String,
}

impl ProfileKey {
    pub fn to_json(&self) -> Value {
        json!({"by": self.by, "key": self.key, "sig": self.sig})
    }

    pub fn from_json(json: &Value) -> Option<ProfileKey> {
        let by = json.get("by")?.as_str()?;
        let key = json.get("key")?.as_str()?;
        let sig = json.get("sig")?.as_str()?;
        (is_key(by) && is_key(key)).then(|| ProfileKey {
            key: key.to_string(),
            by: by.to_string(),
            sig: sig.to_string(),
        })
    }
}

/// The Profile id `alias` given up for `to`: `sig` over
/// [`alias_bytes`] by `by`, a key bound to the Profile given up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileAlias {
    pub alias: String,
    pub to: String,
    pub by: String,
    pub sig: String,
}

impl ProfileAlias {
    pub fn to_json(&self) -> Value {
        json!({"alias": self.alias, "by": self.by, "sig": self.sig, "to": self.to})
    }

    pub fn from_json(json: &Value) -> Option<ProfileAlias> {
        let alias = json.get("alias")?.as_str()?;
        let by = json.get("by")?.as_str()?;
        let sig = json.get("sig")?.as_str()?;
        let to = json.get("to")?.as_str()?;
        (!alias.is_empty() && !to.is_empty() && is_key(by)).then(|| ProfileAlias {
            alias: alias.to_string(),
            to: to.to_string(),
            by: by.to_string(),
            sig: sig.to_string(),
        })
    }

    /// Whether `by` made `sig`.
    pub fn signed(&self) -> bool {
        verifies(&self.by, &alias_bytes(&self.alias, &self.to), &self.sig)
    }
}

/// A Profile as it travels between Devices (ADR 0017), the Dart core's
/// `SignedProfile`: the record as its signer sent it, signed by
/// `signer`; its binding, the `first` key and the `keys` joined since;
/// the ids it replaced; and, between one's own Devices only, whether it
/// still has the defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedProfile {
    pub record: String,
    pub signer: String,
    pub sig: String,
    pub first: String,
    pub keys: Vec<ProfileKey>,
    pub aliases: Vec<ProfileAlias>,
    pub fresh: bool,
}

impl SignedProfile {
    /// Keys sorted, no spaces, as the Dart core writes it.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "aliases".into(),
            Value::Array(self.aliases.iter().map(ProfileAlias::to_json).collect()),
        );
        map.insert("first".into(), json!(self.first));
        if self.fresh {
            map.insert("fresh".into(), Value::Bool(true));
        }
        map.insert(
            "keys".into(),
            Value::Array(self.keys.iter().map(ProfileKey::to_json).collect()),
        );
        map.insert("record".into(), json!(self.record));
        map.insert("sig".into(), json!(self.sig));
        map.insert("signer".into(), json!(self.signer));
        Value::Object(map)
    }

    /// The signed Profile `json` holds, or none. Keys and aliases that are
    /// no such things are left out.
    pub fn from_json(json: &Value) -> Option<SignedProfile> {
        let record = json.get("record")?.as_str()?;
        let signer = json.get("signer")?.as_str().filter(|s| is_key(s))?;
        let sig = json.get("sig")?.as_str()?;
        let first = json.get("first")?.as_str().filter(|s| is_key(s))?;
        let keys = json.get("keys")?.as_array()?;
        let aliases = json.get("aliases")?.as_array()?;
        let fresh = match json.get("fresh") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return None,
        };
        Some(SignedProfile {
            record: record.to_string(),
            signer: signer.to_string(),
            sig: sig.to_string(),
            first: first.to_string(),
            keys: keys.iter().filter_map(ProfileKey::from_json).collect(),
            aliases: aliases.iter().filter_map(ProfileAlias::from_json).collect(),
            fresh,
        })
    }
}

/// The keys bound to the Profile `id` from `first` on: every key of
/// `held`, then of `offered`, signed by one bound before it, until no
/// more join; with the statements that bound them, sorted by key.
pub(crate) fn chain(
    id: &str,
    first: &str,
    held: &[ProfileKey],
    offered: &[ProfileKey],
) -> (HashSet<String>, Vec<ProfileKey>) {
    let mut bound = HashSet::from([first.to_string()]);
    let mut kept: Vec<ProfileKey> = Vec::new();
    let mut grew = true;
    while grew {
        grew = false;
        for k in held.iter().chain(offered) {
            if bound.contains(&k.key) || !bound.contains(&k.by) {
                continue;
            }
            if !verifies(&k.by, &key_bytes(id, &k.key), &k.sig) {
                continue;
            }
            bound.insert(k.key.clone());
            kept.push(k.clone());
            grew = true;
        }
    }
    kept.sort_by(|a, b| a.key.cmp(&b.key));
    (bound, kept)
}

/// A received Profile whose signatures held: its record read, the keys
/// joined to its binding, and its picture made again for the screen.
pub(crate) struct Checked {
    pub(crate) profile: Profile,
    pub(crate) keys: Vec<ProfileKey>,
    pub(crate) shown: Option<Vec<u8>>,
}

/// `sent` checked: the signature over its record's bytes as sent before
/// anything of the record is read, then the record, its binding from the
/// first key, its signer bound, and its picture made again.
pub(crate) fn check(sent: &SignedProfile) -> Option<Checked> {
    if !verifies(&sent.signer, &record_bytes(&sent.record), &sent.sig) {
        return None;
    }
    let profile = Profile::parse(&sent.record)?;
    let (bound, keys) = chain(&profile.id, &sent.first, &[], &sent.keys);
    if !bound.contains(&sent.signer) {
        return None;
    }
    let shown = match &profile.picture {
        Some(picture) => Some(received_picture(picture)?),
        None => None,
    };
    Some(Checked {
        profile,
        keys,
        shown,
    })
}

/// Whether the record `a` replaces `b`: a higher version; at the same
/// version one with a picture, so a card's record, which leaves the
/// picture out (#107), gives way to the one it was made of; else the
/// larger bytes, so two Devices that changed it apart end up with the
/// same one.
pub(crate) fn replaces(a: &str, pa: &Profile, b: &str, pb: &Profile) -> bool {
    if pa.version != pb.version {
        pa.version > pb.version
    } else if pa.picture.is_some() != pb.picture.is_some() {
        pa.picture.is_some()
    } else {
        a > b
    }
}

/// Whether the alias chain `aliases` leads from `to` to `id`.
fn reaches(to: &str, id: &str, aliases: &[ProfileAlias]) -> bool {
    let mut at = to;
    for _ in 0..=aliases.len() {
        if at == id {
            return true;
        }
        match aliases.iter().find(|a| a.alias == at) {
            Some(next) => at = &next.to,
            None => return false,
        }
    }
    false
}

/// What became of a Profile another of the user's Devices sent, the Dart
/// core's `ProfileTaken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfileTaken {
    /// Nothing new, or none sent.
    #[default]
    Unchanged,
    /// Its signatures did not hold, or a copied id: nothing was kept.
    Refused,
    /// A newer record, keys or aliases of the own Profile were kept.
    Updated,
    /// This Device took the other Profile over.
    Adopted,
    /// Both Profiles were changed by the user: "Which one is you?" waits.
    Asked,
}

impl ProfileTaken {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            ProfileTaken::Unchanged => "unchanged",
            ProfileTaken::Refused => "refused",
            ProfileTaken::Updated => "updated",
            ProfileTaken::Adopted => "adopted",
            ProfileTaken::Asked => "asked",
        }
    }

    /// Whether what shows of the own Profile, or a question about it,
    /// changed.
    pub fn changed(self) -> bool {
        matches!(
            self,
            ProfileTaken::Updated | ProfileTaken::Adopted | ProfileTaken::Asked
        )
    }
}

/// The Profile "Which one is you?" asks about: its record with the
/// picture made again, and its short code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileChoice {
    pub profile: Profile,
    pub code: String,
}

/// The sender's Profile a Sent Item carries, as its preview shows it,
/// the Dart core's `SentProfile`: the record with the picture made
/// again, its short code, whether it is kept here already, and whether
/// its code was compared in person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentProfile {
    pub profile: Profile,
    pub code: String,
    pub kept: bool,
    pub verified: bool,
}

// ------------------------------------------------------- own Profile

pub(crate) const RECORD: &str = "profile";
pub(crate) const FIRST: &str = "profile:first";
const NAME_CHOSEN: &str = "profile:nameChosen";
const PICTURE_CHOSEN: &str = "profile:pictureChosen";
const SIGNED: &str = "profile:signed";
pub(crate) const KEYS: &str = "profile:keys";
pub(crate) const ALIASES: &str = "profile:aliases";
const SHOWN: &str = "profile:shown";
const ASKED: &str = "profile:asked";

/// The user's own Profile on this Device (ADR 0017), under the local
/// settings the Dart core's `OwnProfile` uses: `profile` holds the
/// record, `profile:first` the public key of the Device that made it
/// (base64), `profile:nameChosen` and `profile:pictureChosen` say the
/// user replaced a default, `profile:signed` holds the record's
/// signature and signer, `profile:keys` the keys joined to the binding,
/// `profile:aliases` the ids it replaced, `profile:shown` a received
/// picture made again, and `profile:asked` the other Profile "Which one
/// is you?" asks about.
pub struct OwnProfile<'a> {
    store: &'a Store,
}

impl<'a> OwnProfile<'a> {
    pub fn new(store: &'a Store) -> OwnProfile<'a> {
        OwnProfile { store }
    }

    /// The user's own Profile; none before [`Self::apply_defaults`] made
    /// it. A picture taken from another Device shows as made again here.
    pub fn get(&self) -> Result<Option<Profile>> {
        let Some(mut profile) = self
            .store
            .local_setting(RECORD)?
            .and_then(|t| Profile::parse(&t))
        else {
            return Ok(None);
        };
        if profile.picture.is_some()
            && let Some(shown) = self
                .store
                .local_setting(SHOWN)?
                .and_then(|s| BASE64.decode(s).ok())
        {
            profile.picture = Some(shown);
        }
        Ok(Some(profile))
    }

    /// The first key of the own Profile: the signing key of the Device
    /// that made it.
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

    /// The ids the own Profile replaced, the kept Profile showing for
    /// each.
    pub fn aliases(&self) -> Result<Vec<String>> {
        let mut ids: Vec<String> = self.held_aliases()?.into_iter().map(|a| a.alias).collect();
        ids.dedup();
        Ok(ids)
    }

    fn chosen(&self, key: &str) -> Result<bool> {
        Ok(self.store.local_setting(key)?.as_deref() == Some("true"))
    }

    fn fresh(&self) -> Result<bool> {
        Ok(!self.chosen(NAME_CHOSEN)? && !self.chosen(PICTURE_CHOSEN)?)
    }

    fn me(&self) -> Result<String> {
        Ok(BASE64.encode(self.store.signing_key()?.public_key()))
    }

    /// Whether only this Device holds the own Profile: it made it, and no
    /// other key or id joined it.
    fn alone(&self) -> Result<bool> {
        Ok(
            self.store.local_setting(FIRST)?.as_deref() == Some(self.me()?.as_str())
                && self.held_keys()?.is_empty()
                && self.held_aliases()?.is_empty(),
        )
    }

    /// Keeps `profile` as made here, signed with this Device's key.
    fn keep(&self, profile: &Profile) -> Result<()> {
        let text = profile.encode();
        self.store.set_local_setting(RECORD, &text)?;
        self.sign(&text)?;
        self.store.remove_local_setting(SHOWN)
    }

    fn sign(&self, record: &str) -> Result<()> {
        let key = self.store.signing_key()?;
        let signed = json!({
            "sig": key.sign(&record_bytes(record)),
            "signer": BASE64.encode(key.public_key()),
        });
        self.store.set_local_setting(SIGNED, &signed.to_string())
    }

    fn list<T>(&self, key: &str, read: fn(&Value) -> Option<T>) -> Result<Vec<T>> {
        Ok(self
            .store
            .local_setting(key)?
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.as_array().map(|a| a.iter().filter_map(read).collect()))
            .unwrap_or_default())
    }

    pub(crate) fn held_keys(&self) -> Result<Vec<ProfileKey>> {
        self.list(KEYS, ProfileKey::from_json)
    }

    fn held_aliases(&self) -> Result<Vec<ProfileAlias>> {
        self.list(ALIASES, ProfileAlias::from_json)
    }

    fn keep_keys(&self, keys: &[ProfileKey]) -> Result<()> {
        if keys.is_empty() {
            return self.store.remove_local_setting(KEYS);
        }
        let json = Value::Array(keys.iter().map(ProfileKey::to_json).collect());
        self.store.set_local_setting(KEYS, &json.to_string())
    }

    fn keep_aliases(&self, mut aliases: Vec<ProfileAlias>) -> Result<()> {
        if aliases.is_empty() {
            return self.store.remove_local_setting(ALIASES);
        }
        aliases.sort_by(|a, b| a.alias.cmp(&b.alias).then_with(|| a.by.cmp(&b.by)));
        let json = Value::Array(aliases.iter().map(ProfileAlias::to_json).collect());
        self.store.set_local_setting(ALIASES, &json.to_string())
    }

    /// Makes the own Profile the first time, with a new id, the default
    /// `name` and `picture` (already a Profile picture, or none for the
    /// placeholder) and this Device's key as its first key; later it
    /// takes the defaults the user has not replaced, while no other Device
    /// holds the Profile. Asks nothing; counts the version up only when
    /// something changed.
    pub fn apply_defaults(&self, name: &str, picture: Option<Vec<u8>>) -> Result<Profile> {
        let clean = sanitize_name(name);
        let Some(current) = self.get()? else {
            let made = Profile {
                id: self.store.new_id()?,
                name: clean,
                picture,
                version: 1,
            };
            self.store.set_local_setting(FIRST, &self.me()?)?;
            self.keep(&made)?;
            return Ok(made);
        };
        // A Profile held by several Devices keeps the name and picture it
        // had: each Device's own defaults would replace each other's.
        if !self.alone()? {
            return Ok(current);
        }
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

    fn signature(&self) -> Result<Option<(String, String)>> {
        let Some(text) = self.store.local_setting(SIGNED)? else {
            return Ok(None);
        };
        let Ok(json) = serde_json::from_str::<Value>(&text) else {
            return Ok(None);
        };
        Ok(
            match (
                json.get("sig").and_then(Value::as_str),
                json.get("signer").and_then(Value::as_str),
            ) {
                (Some(sig), Some(signer)) if is_key(signer) => {
                    Some((sig.to_string(), signer.to_string()))
                }
                _ => None,
            },
        )
    }

    /// The own Profile as it goes to the user's other Devices: the record
    /// with its signature (made here when the record has none that
    /// holds), its binding and aliases, and whether it still has the
    /// defaults; none before it exists.
    pub fn signed(&self) -> Result<Option<SignedProfile>> {
        let (Some(record), Some(first)) = (
            self.store.local_setting(RECORD)?,
            self.store.local_setting(FIRST)?,
        ) else {
            return Ok(None);
        };
        if !is_key(&first) {
            return Ok(None);
        }
        let mut signed = self.signature()?;
        if !signed
            .as_ref()
            .is_some_and(|(sig, signer)| verifies(signer, &record_bytes(&record), sig))
        {
            self.sign(&record)?;
            signed = self.signature()?;
        }
        let Some((sig, signer)) = signed else {
            return Ok(None);
        };
        Ok(Some(SignedProfile {
            record,
            signer,
            sig,
            first,
            keys: self.held_keys()?,
            aliases: self.held_aliases()?,
            fresh: self.fresh()?,
        }))
    }

    /// [`Self::signed`] as JSON, for the sync bodies and the keys file.
    pub fn to_send(&self) -> Result<Option<Value>> {
        Ok(self.signed()?.map(|s| s.to_json()))
    }

    /// Binds `key` (base64), the key of another of the user's own Devices
    /// met in a sync, to the own Profile, signed with this Device's key
    /// when that is bound.
    fn bind(&self, key: &str) -> Result<()> {
        let (Some(profile), Some(first)) = (
            self.store
                .local_setting(RECORD)?
                .and_then(|t| Profile::parse(&t)),
            self.store.local_setting(FIRST)?,
        ) else {
            return Ok(());
        };
        let me = self.me()?;
        if key == me || !is_key(key) {
            return Ok(());
        }
        let (bound, mut keys) = chain(&profile.id, &first, &self.held_keys()?, &[]);
        if !bound.contains(&me) || bound.contains(key) {
            return Ok(());
        }
        keys.push(ProfileKey {
            key: key.to_string(),
            by: me,
            sig: self.store.signing_key()?.sign(&key_bytes(&profile.id, key)),
        });
        keys.sort_by(|a, b| a.key.cmp(&b.key));
        self.keep_keys(&keys)
    }

    /// Takes in the Profile `sent` by another of the user's own Devices,
    /// whose signing key `from` the session or the keys file vouches for,
    /// as the Dart core's `takeOwnProfile`: that Device's key is bound to
    /// the own Profile first; then the same Profile, newer and validly
    /// signed, replaces the record kept, and an update signed by a key not
    /// bound to it is refused. Another Profile is taken over when this
    /// Device's still has the defaults and the other not, when both have
    /// them and the other's id is lower, or when it names this one's id as
    /// its alias; when both were changed by the user, "Which one is you?"
    /// waits.
    pub fn take(&self, sent: Option<&Value>, from: &[u8]) -> Result<ProfileTaken> {
        self.bind(&BASE64.encode(from))?;
        let Some(sent) = sent.filter(|v| !v.is_null()) else {
            return Ok(ProfileTaken::Unchanged);
        };
        let Some(theirs) = SignedProfile::from_json(sent) else {
            return Ok(ProfileTaken::Refused);
        };
        let Some(checked) = check(&theirs) else {
            return Ok(ProfileTaken::Refused);
        };
        self.take_checked(&theirs, &checked)
    }

    fn take_checked(&self, theirs: &SignedProfile, checked: &Checked) -> Result<ProfileTaken> {
        let record = self.store.local_setting(RECORD)?;
        let first = self.store.local_setting(FIRST)?;
        let mine = record.as_deref().and_then(Profile::parse);
        let (Some(record), Some(first), Some(mine)) = (record, first, mine) else {
            self.adopt(theirs, checked, None)?;
            return Ok(ProfileTaken::Adopted);
        };
        let id = &checked.profile.id;
        if *id == mine.id {
            return self.merge(theirs, checked, &mine, &record, &first);
        }
        let (my_bound, _) = chain(&mine.id, &first, &self.held_keys()?, &[]);
        let aliases_me = theirs.aliases.iter().any(|a| {
            a.alias == mine.id
                && my_bound.contains(&a.by)
                && a.signed()
                && reaches(&a.to, id, &theirs.aliases)
        });
        if self.aliases()?.contains(id) {
            // Given up here, and this one given up there: answered apart,
            // the lower id stays.
            if aliases_me && id.as_str() < mine.id.as_str() {
                self.adopt(theirs, checked, Some(&mine))?;
                return Ok(ProfileTaken::Adopted);
            }
            return Ok(ProfileTaken::Unchanged);
        }
        let fresh = self.fresh()?;
        if aliases_me
            || (fresh && !theirs.fresh)
            || (fresh && theirs.fresh && id.as_str() < mine.id.as_str())
        {
            self.adopt(theirs, checked, Some(&mine))?;
            return Ok(ProfileTaken::Adopted);
        }
        if fresh || theirs.fresh {
            return Ok(ProfileTaken::Unchanged);
        }
        let text = theirs.to_json().to_string();
        if self.store.local_setting(ASKED)?.as_deref() == Some(text.as_str()) {
            return Ok(ProfileTaken::Unchanged);
        }
        self.store.set_local_setting(ASKED, &text)?;
        Ok(ProfileTaken::Asked)
    }

    fn merge(
        &self,
        theirs: &SignedProfile,
        checked: &Checked,
        mine: &Profile,
        record: &str,
        first: &str,
    ) -> Result<ProfileTaken> {
        // A copied id with another first key is another person's Profile.
        if theirs.first != first {
            return Ok(ProfileTaken::Refused);
        }
        let held = self.held_keys()?;
        let (bound, keys) = chain(&mine.id, first, &held, &theirs.keys);
        if !bound.contains(&theirs.signer) {
            return Ok(ProfileTaken::Refused);
        }
        let mut taken = ProfileTaken::Unchanged;
        if keys != held {
            self.keep_keys(&keys)?;
        }
        let mut aliases = self.held_aliases()?;
        let mut more = false;
        for a in &theirs.aliases {
            if aliases.iter().any(|h| h.alias == a.alias && h.by == a.by) || a.alias == mine.id {
                continue;
            }
            if !bound.contains(&a.by) || !a.signed() {
                continue;
            }
            aliases.push(a.clone());
            more = true;
        }
        let ids: HashSet<String> = aliases.iter().map(|a| a.alias.clone()).collect();
        if more {
            self.keep_aliases(aliases)?;
            taken = ProfileTaken::Updated;
        }
        if let Some(asked) = self.asked()?
            && Profile::parse(&asked.record).is_some_and(|p| ids.contains(&p.id))
        {
            self.store.remove_local_setting(ASKED)?;
            taken = ProfileTaken::Updated;
        }
        if !replaces(&theirs.record, &checked.profile, record, mine) {
            return Ok(taken);
        }
        // A change made on another Device was the user's: the defaults no
        // longer apply to it here either.
        if !theirs.fresh {
            if checked.profile.name != mine.name {
                self.store.set_local_setting(NAME_CHOSEN, "true")?;
            }
            if checked.profile.picture != mine.picture {
                self.store.set_local_setting(PICTURE_CHOSEN, "true")?;
            }
        }
        self.keep_received(theirs, checked)?;
        Ok(ProfileTaken::Updated)
    }

    /// Keeps `theirs` record as received, with its signature and its
    /// picture made again.
    fn keep_received(&self, theirs: &SignedProfile, checked: &Checked) -> Result<()> {
        self.store.set_local_setting(RECORD, &theirs.record)?;
        let signed = json!({"sig": theirs.sig, "signer": theirs.signer});
        self.store.set_local_setting(SIGNED, &signed.to_string())?;
        match &checked.shown {
            Some(shown) => self.store.set_local_setting(SHOWN, &BASE64.encode(shown)),
            None => self.store.remove_local_setting(SHOWN),
        }
    }

    /// Takes `theirs` over as the own Profile; the id of `mine`, when
    /// there was one, becomes its alias, signed with this Device's key,
    /// which is bound to it: the entry other people count. The aliases it
    /// came with count only when a key bound to it made them.
    fn adopt(
        &self,
        theirs: &SignedProfile,
        checked: &Checked,
        mine: Option<&Profile>,
    ) -> Result<()> {
        let id = &checked.profile.id;
        let bound = chain(id, &theirs.first, &[], &theirs.keys).0;
        let mut aliases: Vec<ProfileAlias> = Vec::new();
        let received = theirs
            .aliases
            .iter()
            .filter(|a| bound.contains(&a.by))
            .cloned();
        for a in self.held_aliases()?.into_iter().chain(received) {
            if a.alias != *id
                && a.signed()
                && !aliases.iter().any(|h| h.alias == a.alias && h.by == a.by)
            {
                aliases.push(a);
            }
        }
        let me = self.me()?;
        if let Some(mine) = mine
            && mine.id != *id
            && !aliases.iter().any(|a| a.alias == mine.id && a.by == me)
        {
            aliases.push(ProfileAlias {
                alias: mine.id.clone(),
                to: id.clone(),
                by: me,
                sig: self.store.signing_key()?.sign(&alias_bytes(&mine.id, id)),
            });
        }
        self.keep_received(theirs, checked)?;
        self.store.set_local_setting(FIRST, &theirs.first)?;
        self.keep_keys(&checked.keys)?;
        self.keep_aliases(aliases)?;
        if theirs.fresh {
            self.store.remove_local_setting(NAME_CHOSEN)?;
            self.store.remove_local_setting(PICTURE_CHOSEN)?;
        } else {
            self.store.set_local_setting(NAME_CHOSEN, "true")?;
            self.store.set_local_setting(PICTURE_CHOSEN, "true")?;
        }
        self.store.remove_local_setting(ASKED)?;
        // The kept Profile's backup key wins over the one made for the
        // Profile given up; none held there, this one goes on (ADR 0016).
        self.store.yield_backup_key()
    }

    /// Takes the own Profile `sent` back from a Backup made on the Device
    /// `old`, as the Dart core's `restoreOwnProfile`: it replaces whatever
    /// Profile this Device made meanwhile, and this Device's key joins the
    /// binding, signed by the old Device's key. Whether a Profile was
    /// taken back.
    pub fn restore(&self, sent: Option<&Value>, old: &SigningKey) -> Result<bool> {
        let Some(theirs) = sent.and_then(SignedProfile::from_json) else {
            return Ok(false);
        };
        let Some(checked) = check(&theirs) else {
            return Ok(false);
        };
        let id = checked.profile.id.clone();
        let (bound, mut keys) = chain(&id, &theirs.first, &[], &checked.keys);
        let me = self.me()?;
        let by = BASE64.encode(old.public_key());
        if bound.contains(&by) && !bound.contains(&me) {
            keys.push(ProfileKey {
                sig: old.sign(&key_bytes(&id, &me)),
                key: me,
                by,
            });
            keys.sort_by(|a, b| a.key.cmp(&b.key));
        }
        self.keep_received(&theirs, &checked)?;
        self.store.set_local_setting(FIRST, &theirs.first)?;
        self.keep_keys(&keys)?;
        self.keep_aliases(
            theirs
                .aliases
                .iter()
                .filter(|a| a.alias != id && bound.contains(&a.by) && a.signed())
                .cloned()
                .collect(),
        )?;
        if theirs.fresh {
            self.store.remove_local_setting(NAME_CHOSEN)?;
            self.store.remove_local_setting(PICTURE_CHOSEN)?;
        } else {
            self.store.set_local_setting(NAME_CHOSEN, "true")?;
            self.store.set_local_setting(PICTURE_CHOSEN, "true")?;
        }
        self.store.remove_local_setting(ASKED)?;
        Ok(true)
    }

    fn asked(&self) -> Result<Option<SignedProfile>> {
        Ok(self
            .store
            .local_setting(ASKED)?
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| SignedProfile::from_json(&v)))
    }

    /// The other Profile "Which one is you?" asks about, or none when
    /// nothing is asked.
    pub fn question(&self) -> Result<Option<ProfileChoice>> {
        let Some(asked) = self.asked()? else {
            return Ok(None);
        };
        let Some(checked) = check(&asked) else {
            return Ok(None);
        };
        let first = BASE64.decode(&asked.first).unwrap_or_default();
        let mut profile = checked.profile;
        profile.picture = checked.shown;
        Ok(Some(ProfileChoice {
            code: short_code(&profile.id, &first),
            profile,
        }))
    }

    /// Answers "Which one is you?": `keep_mine` keeps this Device's
    /// Profile and makes the other's id its alias, signed with this
    /// Device's key; otherwise this Device takes the other over. The
    /// answer reaches the other Devices with the next sync.
    pub fn answer(&self, keep_mine: bool) -> Result<()> {
        let asked = self.asked()?;
        let mine = self
            .store
            .local_setting(RECORD)?
            .and_then(|t| Profile::parse(&t));
        self.store.remove_local_setting(ASKED)?;
        let (Some(asked), Some(mine)) = (asked, mine) else {
            return Ok(());
        };
        let Some(checked) = check(&asked) else {
            return Ok(());
        };
        let id = checked.profile.id.clone();
        if !keep_mine {
            return self.adopt(&asked, &checked, Some(&mine));
        }
        if id == mine.id {
            return Ok(());
        }
        // This Device's key is bound to the Profile given up only on the
        // Devices that held it: the entry tells them which one is kept,
        // and the Device that gave it up signs it again on taking the kept
        // one over, for other people.
        let mut aliases = self.held_aliases()?;
        let me = self.me()?;
        if !aliases.iter().any(|a| a.alias == id && a.by == me) {
            aliases.push(ProfileAlias {
                alias: id.clone(),
                to: mine.id.clone(),
                by: me,
                sig: self.store.signing_key()?.sign(&alias_bytes(&id, &mine.id)),
            });
        }
        self.keep_aliases(aliases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        /// How many records [`Profile::parse`] read on this thread: a
        /// received record is not read before its signature holds.
        pub(super) static PARSED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }

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

    // ------------------------------------- between one's own Devices

    use crate::folder::{MemoryFolder, folder_sync, random_nonce};
    use crate::lan::{Session, answer_sync, apply_answer, sync_request};
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;

    /// A Device with its own Profile made from the default `name`.
    fn device(seed: u8, name: &str) -> std::result::Result<Store, Box<dyn std::error::Error>> {
        let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    /// One LAN session without the network: `joiner` joins `host`.
    fn exchange(
        joiner: &Store,
        host: &Store,
    ) -> std::result::Result<(Value, Value, Session, Session), Box<dyn std::error::Error>> {
        let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
        let (answer, served) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        let back = apply_answer(joiner, &answer)?;
        Ok((request, answer, served, back))
    }

    /// The Profile `store` holds, as both Devices must agree on it.
    fn held(store: &Store) -> Result<(Option<String>, Option<i64>, Option<String>)> {
        let own = OwnProfile::new(store);
        let profile = own.get()?;
        Ok((
            profile.as_ref().map(|p| p.id.clone()),
            profile.map(|p| p.version),
            own.code()?,
        ))
    }

    fn id(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
        Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
    }

    #[test]
    fn both_with_the_defaults_keep_the_lower_id() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Ada Lovelace")?;
        let mut ids = [id(&phone)?, id(&desk)?];
        ids.sort();
        let (request, answer, _, _) = exchange(&phone, &desk)?;
        assert!(request.get("profile").is_some());
        assert!(answer.get("profile").is_some());
        assert_eq!(id(&phone)?, ids[0]);
        assert_eq!(held(&phone)?, held(&desk)?);
        exchange(&phone, &desk)?;
        for store in [&phone, &desk] {
            assert_eq!(OwnProfile::new(store).aliases()?, vec![ids[1].clone()]);
            assert_eq!(OwnProfile::new(store).question()?, None);
        }
        Ok(())
    }

    #[test]
    fn the_defaults_take_a_changed_profile_over_without_a_question() -> TestResult {
        for joiner_chose in [true, false] {
            let phone = device(1, "Pixel 7")?;
            let desk = device(2, "Ada Lovelace")?;
            let chosen = if joiner_chose { &phone } else { &desk };
            OwnProfile::new(chosen).rename("Ada")?;
            let kept = id(chosen)?;
            let (_, _, served, back) = exchange(&phone, &desk)?;
            assert_eq!(id(&phone)?, kept);
            assert_eq!(held(&phone)?, held(&desk)?);
            let expected = if joiner_chose {
                (ProfileTaken::Adopted, ProfileTaken::Updated)
            } else {
                (ProfileTaken::Unchanged, ProfileTaken::Adopted)
            };
            assert_eq!((served.profile, back.profile), expected);
            assert_eq!(OwnProfile::new(&phone).question()?, None);
            assert_eq!(OwnProfile::new(&desk).question()?, None);
        }
        Ok(())
    }

    #[test]
    fn both_changed_ask_and_the_first_answer_settles_it() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        OwnProfile::new(&phone).rename("Ada L.")?;
        OwnProfile::new(&desk).rename("Ada")?;
        let (phone_id, desk_id) = (id(&phone)?, id(&desk)?);
        let (_, _, served, back) = exchange(&phone, &desk)?;
        assert_eq!(
            (served.profile, back.profile),
            (ProfileTaken::Asked, ProfileTaken::Asked)
        );
        let asked = OwnProfile::new(&phone).question()?.ok_or("a question")?;
        assert_eq!(asked.profile.name, "Ada");
        assert_eq!(Some(asked.code), OwnProfile::new(&desk).code()?);
        assert_eq!(id(&phone)?, phone_id);

        OwnProfile::new(&phone).answer(false)?;
        assert_eq!(OwnProfile::new(&phone).question()?, None);
        assert_eq!(id(&phone)?, desk_id);
        exchange(&phone, &desk)?;
        assert_eq!(OwnProfile::new(&desk).question()?, None);
        assert_eq!(held(&phone)?, held(&desk)?);
        assert_eq!(OwnProfile::new(&desk).aliases()?, vec![phone_id]);
        Ok(())
    }

    #[test]
    fn answered_apart_against_each_other_the_lower_id_stays() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        OwnProfile::new(&phone).rename("Ada L.")?;
        OwnProfile::new(&desk).rename("Ada")?;
        let lower = id(&phone)?.min(id(&desk)?);
        exchange(&phone, &desk)?;
        OwnProfile::new(&phone).answer(true)?;
        OwnProfile::new(&desk).answer(true)?;
        exchange(&phone, &desk)?;
        exchange(&phone, &desk)?;
        exchange(&phone, &desk)?;
        assert_eq!(id(&phone)?, lower);
        assert_eq!(held(&phone)?, held(&desk)?);
        Ok(())
    }

    #[test]
    fn a_shared_profile_keeps_its_name_from_either_devices_defaults() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        exchange(&phone, &desk)?;
        for store in [&phone, &desk] {
            let own = OwnProfile::new(store);
            let version = own.get()?.ok_or("a Profile")?.version;
            assert_eq!(own.apply_defaults("Renamed", None)?.version, version);
        }
        // A name the user typed on one shows on the other at once.
        OwnProfile::new(&desk).rename("Ada")?;
        let (_, _, _, back) = exchange(&phone, &desk)?;
        assert_eq!(back.profile, ProfileTaken::Updated);
        assert_eq!(
            OwnProfile::new(&phone).get()?.ok_or("a Profile")?.name,
            "Ada"
        );
        Ok(())
    }

    #[test]
    fn changed_apart_at_the_same_version_the_one_with_a_picture_stays() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        OwnProfile::new(&desk).rename("Ada")?;
        exchange(&phone, &desk)?;
        // "Bea" sorts after "Ada": the larger bytes alone would keep the
        // phone's.
        OwnProfile::new(&phone).rename("Bea")?;
        OwnProfile::new(&desk).set_picture(square_picture(&halves(64, 64, ImageFormat::Png)))?;
        exchange(&phone, &desk)?;
        let kept = OwnProfile::new(&phone).get()?.ok_or("a Profile")?;
        assert_eq!(kept.name, "Ada");
        assert!(kept.picture.is_some());
        assert_eq!(held(&phone)?, held(&desk)?);
        Ok(())
    }

    #[test]
    fn an_update_signed_by_a_key_not_bound_is_refused() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        OwnProfile::new(&desk).rename("Ada")?;
        exchange(&phone, &desk)?;
        let before = OwnProfile::new(&phone).get()?;
        let sent = OwnProfile::new(&desk).signed()?.ok_or("signed")?;
        let mine = OwnProfile::new(&desk).get()?.ok_or("a Profile")?;
        let stranger = SigningKey::from_seed([9; 32]);
        let stranger_b64 = BASE64.encode(stranger.public_key());
        let forged = Profile {
            id: mine.id.clone(),
            name: "Mallory".into(),
            picture: None,
            version: mine.version + 5,
        }
        .encode();
        let signed_by = |key: &SigningKey, keys: Vec<ProfileKey>| SignedProfile {
            record: forged.clone(),
            signer: BASE64.encode(key.public_key()),
            sig: key.sign(&record_bytes(&forged)),
            first: sent.first.clone(),
            keys,
            aliases: Vec::new(),
            fresh: false,
        };
        let desk_key = SigningKey::from_seed([2; 32]);
        let from = desk_key.public_key();
        let own = OwnProfile::new(&phone);
        let taken = own.take(
            Some(&signed_by(&stranger, sent.keys.clone()).to_json()),
            &from,
        )?;
        assert_eq!(taken, ProfileTaken::Refused);
        let mut self_bound = sent.keys.clone();
        self_bound.push(ProfileKey {
            key: stranger_b64.clone(),
            by: stranger_b64.clone(),
            sig: stranger.sign(&key_bytes(&mine.id, &stranger_b64)),
        });
        let taken = own.take(Some(&signed_by(&stranger, self_bound).to_json()), &from)?;
        assert_eq!(taken, ProfileTaken::Refused);
        let mut changed = sent.to_json();
        changed["record"] = json!(forged);
        assert_eq!(own.take(Some(&changed), &from)?, ProfileTaken::Refused);
        let copy = SignedProfile {
            first: stranger_b64.clone(),
            ..signed_by(&stranger, Vec::new())
        };
        assert_eq!(
            own.take(Some(&copy.to_json()), &from)?,
            ProfileTaken::Refused
        );
        assert_eq!(own.get()?, before);
        // Signed by a bound key, it counts.
        let good = signed_by(&desk_key, sent.keys.clone()).to_json();
        assert_eq!(own.take(Some(&good), &from)?, ProfileTaken::Updated);
        assert_eq!(own.get()?.ok_or("a Profile")?.name, "Mallory");
        Ok(())
    }

    #[test]
    fn a_record_whose_signature_fails_is_refused_before_it_is_read() -> TestResult {
        let key = SigningKey::from_seed([9; 32]);
        let key_b64 = BASE64.encode(key.public_key());
        let record = json!({
            "id": "e".repeat(32),
            "name": "Eve",
            "picture": format!("*not base64*{}", "A".repeat(1000)),
            "version": 1,
        })
        .to_string();
        let sent = |sig: String| SignedProfile {
            record: record.clone(),
            signer: key_b64.clone(),
            sig,
            first: key_b64.clone(),
            keys: Vec::new(),
            aliases: Vec::new(),
            fresh: false,
        };
        let before = PARSED.with(|n| n.get());
        assert!(check(&sent(key.sign(&record_bytes("{}")))).is_none());
        assert_eq!(PARSED.with(|n| n.get()), before, "nothing of it was read");
        // Signed, the same record is read, and refused for its picture.
        assert!(check(&sent(key.sign(&record_bytes(&record)))).is_none());
        assert!(PARSED.with(|n| n.get()) > before);
        Ok(())
    }

    #[test]
    fn taking_over_or_restoring_keeps_only_aliases_a_bound_key_made() -> TestResult {
        let desk = device(2, "Desk")?;
        OwnProfile::new(&desk).rename("Ada")?;
        let desk_id = id(&desk)?;
        let stranger = SigningKey::from_seed([9; 32]);
        let stranger_id = "a".repeat(32);
        let forged = ProfileAlias {
            alias: stranger_id.clone(),
            to: desk_id.clone(),
            by: BASE64.encode(stranger.public_key()),
            sig: stranger.sign(&alias_bytes(&stranger_id, &desk_id)),
        };
        assert!(forged.signed());
        desk.set_local_setting(ALIASES, &Value::Array(vec![forged.to_json()]).to_string())?;
        // A Device with the defaults takes the computer's over.
        let phone = device(1, "Pixel 7")?;
        exchange(&phone, &desk)?;
        assert_eq!(id(&phone)?, desk_id);
        assert!(!OwnProfile::new(&phone).aliases()?.contains(&stranger_id));
        // A Backup carrying it restores none either.
        let pad = device(3, "Pad")?;
        let sent = OwnProfile::new(&desk).signed()?.ok_or("signed")?.to_json();
        assert!(OwnProfile::new(&pad).restore(Some(&sent), &desk.signing_key()?)?);
        assert_eq!(id(&pad)?, desk_id);
        assert!(OwnProfile::new(&pad).aliases()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_received_picture_shows_made_again_and_none_is_refused() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        let picture = square_picture(&halves(300, 300, ImageFormat::Png));
        OwnProfile::new(&desk).set_picture(picture)?;
        exchange(&phone, &desk)?;
        assert_eq!(id(&phone)?, id(&desk)?);
        assert_profile_picture(
            OwnProfile::new(&phone)
                .get()?
                .and_then(|p| p.picture)
                .as_deref(),
        );
        let key = SigningKey::from_seed([2; 32]);
        let mine = OwnProfile::new(&desk).get()?.ok_or("a Profile")?;
        let broken = json!({
            "id": mine.id,
            "name": mine.name,
            "picture": BASE64.encode(b"no picture at all"),
            "version": mine.version + 1,
        })
        .to_string();
        let mut sent = OwnProfile::new(&desk).to_send()?.ok_or("sent")?;
        sent["record"] = json!(broken);
        sent["sig"] = json!(key.sign(&record_bytes(&broken)));
        let taken = OwnProfile::new(&phone).take(Some(&sent), &key.public_key())?;
        assert_eq!(taken, ProfileTaken::Refused);
        Ok(())
    }

    #[test]
    fn the_shared_folder_carries_it_sealed() -> TestResult {
        let phone = device(1, "Pixel 7")?;
        let desk = device(2, "Desk")?;
        OwnProfile::new(&desk).rename("Ada")?;
        exchange(&phone, &desk)?;
        OwnProfile::new(&desk).rename("Ada Lovelace")?;
        let mut folder = MemoryFolder::default();
        folder_sync(&desk, &mut folder, &mut random_nonce)?;
        let read = folder_sync(&phone, &mut folder, &mut random_nonce)?;
        assert!(read.profile_changed);
        assert_eq!(
            OwnProfile::new(&phone).get()?.ok_or("a Profile")?.name,
            "Ada Lovelace"
        );
        assert_eq!(held(&phone)?, held(&desk)?);
        Ok(())
    }
}
