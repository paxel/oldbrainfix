//! Profile cards, as the Dart core's `profile_cards.dart` makes and
//! opens them (ADR 0017, #107, #144): a person sends their own Profile
//! as a small file through any messenger, or shows it as a QR, and
//! whoever opens it has them in People. The card is
//! `<payload>.<signature>`, both base64url without padding; the QR holds
//! it after the `#` of [`LINK`], which never reaches a server. The
//! payload is one JSON object, keys sorted and no spaces, the same bytes
//! from both cores:
//! `{"back":true,"format":1,"key":<key record>,"profile":<signed Profile>}`.
//! `key` is the key record of the Device that made the card (ADR 0015),
//! `profile` the own Profile as it goes to other people with no aliases,
//! its record signed again by that Device: the record as kept in a file,
//! without its picture in the QR; `back` is there when the card answers
//! one the user opened. The signature is that Device's, over
//! [`card_bytes`] of the payload as sent, and its key must be bound to
//! the Profile. The file ([`Store::profile_card_file`]) is
//! `{"card":<card>,"type":"oldbrainfix-card"}`, keys sorted and no
//! spaces. A card adds a person and nothing else: no Area, no grant, no
//! key but the public ones.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use serde_json::{Map, Value};

use crate::Result;
use crate::people::PersonTaken;
use crate::profile::{
    OwnProfile, Profile, SignedProfile, chain, check, record_bytes, sanitize_name, short_code,
};
use crate::signing::{ImportReport, KeyRecord, verify_signature};
use crate::store::Store;

/// The address a QR's card goes out under; the card follows the `#`.
pub const LINK: &str = "https://paxel.github.io/oldbrainfix/p#";

/// The most characters of a card's text that are read at all.
pub const MAX_CHARS: usize = 16 * 1024;

/// The file ending of a Profile card file.
pub const EXTENSION: &str = "obfcard";

/// The type a Profile card file is shared as.
pub const MIME_TYPE: &str = "application/vnd.oldbrainfix.card";

/// A file larger than this is no Profile card: a picture received is at
/// most 256 KB.
pub const MAX_FILE_BYTES: usize = 1 << 20;

const FORMAT: i64 = 1;

/// What a Profile card file says it is.
const FILE_TYPE: &str = "oldbrainfix-card";

/// The names Windows keeps for its devices, with any ending.
fn is_reserved_file_name(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper.len() == 4
            && matches!(upper.as_bytes().get(3), Some(b'1'..=b'9')))
}

/// The name the Profile card file of a person called `name` goes by,
/// `Ann.obfcard`, as the Dart core's `profileCardFileName`: the name
/// sanitised as a Profile's, each character a file name cannot hold made
/// `_`, without blanks and full stops at either end, `_` before a name
/// Windows keeps for a device, and `Profile` for a name with nothing
/// left.
pub fn file_name(name: &str) -> String {
    let replaced: String = sanitize_name(name)
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut base = replaced.trim_matches([' ', '.']).to_string();
    if base.is_empty() {
        base = "Profile".to_string();
    }
    let stem = base
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    if is_reserved_file_name(stem) {
        base = format!("_{base}");
    }
    format!("{base}.{EXTENSION}")
}

/// The JSON object of the Profile card file `bytes`: one that says it is
/// one, of any format; none for anything else or one too large.
fn card_file_json(bytes: &[u8]) -> Option<Map<String, Value>> {
    if bytes.len() > MAX_FILE_BYTES {
        return None;
    }
    match serde_json::from_slice(bytes).ok()? {
        Value::Object(map) if map.get("type").and_then(Value::as_str) == Some(FILE_TYPE) => {
            Some(map)
        }
        _ => None,
    }
}

/// Whether `bytes` are a Profile card file rather than a Sent Item or a
/// bundle, in a format this version reads or a later one, as the Dart
/// core's `looksLikeProfileCard`.
pub fn looks_like_profile_card(bytes: &[u8]) -> bool {
    card_file_json(bytes).is_some()
}

/// The bytes a card's signature covers: the payload as sent.
pub fn card_bytes(payload: &str) -> Vec<u8> {
    format!("oldbrainfix:profile-card:{payload}").into_bytes()
}

/// What opening a card did, the Dart core's `CardTaken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardTaken {
    /// The person was not kept here before and is now, not verified.
    Added,
    /// The person was kept here; the card brought something new.
    Updated,
    /// The person was kept here as the card has them.
    Unchanged,
    /// The card is the user's own Profile: nothing was kept.
    Own,
    /// The person is blocked, or a Device of theirs made the card.
    Blocked,
    /// The card names a Profile kept here under another first key, or
    /// one its signer is not bound to: somebody else.
    Refused,
    /// No card, or its signatures do not hold.
    Unreadable,
    /// A Profile card file of a format a later version writes (ADR 0034):
    /// this app needs an update to open it.
    NeedsUpdate,
}

impl CardTaken {
    /// The name the Dart core gives it.
    pub fn name(self) -> &'static str {
        match self {
            CardTaken::Added => "added",
            CardTaken::Updated => "updated",
            CardTaken::Unchanged => "unchanged",
            CardTaken::Own => "own",
            CardTaken::Blocked => "blocked",
            CardTaken::Refused => "refused",
            CardTaken::Unreadable => "unreadable",
            CardTaken::NeedsUpdate => "needsUpdate",
        }
    }
}

/// What opening a card did, the Profile id it names (none when
/// unreadable), and whether to offer "Send yours back": the person was
/// new here and the card answers none of the user's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardOpened {
    pub taken: CardTaken,
    pub id: Option<String>,
    pub offer_back: bool,
}

impl CardOpened {
    fn only(taken: CardTaken, id: Option<String>) -> CardOpened {
        CardOpened {
            taken,
            id,
            offer_back: false,
        }
    }
}

/// A card whose signatures held.
struct Card {
    profile: SignedProfile,
    key: KeyRecord,
    back: bool,
    id: String,
}

fn is_card_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// The bytes of `text`, base64url without padding as written; none for
/// anything else.
fn from_base64_url(text: &str) -> Option<Vec<u8>> {
    if text.is_empty()
        || !text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let bytes = BASE64_URL.decode(text).ok()?;
    (BASE64_URL.encode(&bytes) == text).then_some(bytes)
}

/// The payload and signature of the card in `text`: the link, or the
/// card alone. Everything but printable ASCII is left out first, so the
/// blanks and line breaks a messenger puts in do no harm; then the card
/// is what follows the last `#` up to the first character no card has,
/// without a full stop after it.
fn card_parts(text: &str) -> Option<(String, String)> {
    let ascii: String = text.chars().filter(|c| ('!'..='~').contains(c)).collect();
    let after = match ascii.rfind('#') {
        Some(at) => &ascii[at + 1..],
        None => ascii.as_str(),
    };
    let end = after.find(|c| !is_card_char(c)).unwrap_or(after.len());
    let run = after[..end].trim_end_matches('.');
    if run.len() > MAX_CHARS {
        return None;
    }
    let mut parts = run.split('.');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(payload), Some(sig), None) => Some((payload.to_string(), sig.to_string())),
        _ => None,
    }
}

/// A Profile card whose signatures held, as the Dart core's
/// `CheckedProfileCard`: the person's signed Profile, the key record of
/// the Device that made it, bound to that Profile, and the Profile's id.
/// A Pack (#164) carries one to say who it is from.
#[derive(Debug, Clone)]
pub struct CheckedCard {
    pub profile: SignedProfile,
    pub key: KeyRecord,
    pub id: String,
}

/// The card `text` holds, a link or the card alone, checked as opening
/// one checks it, as the Dart core's `checkProfileCard`; none when it is
/// none or its signatures do not hold. Nothing is kept.
pub fn check_profile_card(text: &str) -> Option<CheckedCard> {
    read_card(text).map(|card| CheckedCard {
        profile: card.profile,
        key: card.key,
        id: card.id,
    })
}

/// The card `text` holds ([`card_parts`]), checked ([`check_card`]).
fn read_card(text: &str) -> Option<Card> {
    let (payload, sig) = card_parts(text)?;
    check_card(&payload, &sig)
}

/// The card of `payload` and `sig`, checked: its key record valid and
/// its signature over the payload as sent, before anything else of it is
/// read; then its Profile's signatures, and its key bound to that
/// Profile. None when any of it fails.
fn check_card(payload: &str, sig: &str) -> Option<Card> {
    let payload = String::from_utf8(from_base64_url(payload)?).ok()?;
    let sig = from_base64_url(sig)?;
    let json: Value = serde_json::from_str(&payload).ok()?;
    let map = json.as_object()?;
    if map.get("format")?.as_i64() != Some(FORMAT) {
        return None;
    }
    let back = match map.get("back") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(true)) => true,
        Some(_) => return None,
    };
    let key_json = map.get("key").filter(|v| !v.is_null())?;
    let profile_json = map.get("profile").filter(|v| !v.is_null())?;
    let key = KeyRecord::from_json(key_json).filter(KeyRecord::valid)?;
    if !verify_signature(
        &key.public_key,
        &card_bytes(&payload),
        Some(&BASE64.encode(&sig)),
    ) {
        return None;
    }
    let profile = SignedProfile::from_json(profile_json)?;
    let checked = check(&profile)?;
    let id = checked.profile.id;
    if !chain(&id, &profile.first, &[], &profile.keys)
        .0
        .contains(&BASE64.encode(&key.public_key))
    {
        return None;
    }
    Some(Card {
        profile,
        key,
        back,
        id,
    })
}

impl Store {
    /// The own Profile as a card's link, for the QR, as the Dart core's
    /// `profileCard`: the record without its picture, signed by this
    /// Device, with the binding and this Device's key record, the whole
    /// signed by this Device; with `back` when it answers a card the
    /// user opened. None before the own Profile exists, or while this
    /// Device's key is not bound to it yet.
    pub fn profile_card(&self, back: bool) -> Result<Option<String>> {
        Ok(self
            .own_card(back, false)?
            .map(|card| format!("{LINK}{card}")))
    }

    /// The own Profile as a card file, `<Name>.obfcard` ([`file_name`]),
    /// as the Dart core's `profileCardFile`: the card as
    /// [`Store::profile_card`] makes it, but with the record as kept,
    /// picture and all, so the person shows with it at once. None when
    /// [`Store::profile_card`] is.
    pub fn profile_card_file(&self, back: bool) -> Result<Option<Vec<u8>>> {
        Ok(self.own_card(back, true)?.map(|card| {
            let mut map = Map::new();
            map.insert("card".into(), Value::String(card));
            map.insert("type".into(), FILE_TYPE.into());
            Value::Object(map).to_string().into_bytes()
        }))
    }

    /// The own card alone, `<payload>.<signature>`, as
    /// [`Store::profile_card_file`] holds it, picture and all, as the Dart
    /// core's `profileCardText`: what a Pack carries with "Add my
    /// Profile" (#164). None when [`Store::profile_card`] is.
    pub fn profile_card_text(&self) -> Result<Option<String>> {
        self.own_card(false, true)
    }

    /// The own card, `<payload>.<signature>`, with the record as kept
    /// when `picture`, else without its picture.
    fn own_card(&self, back: bool, picture: bool) -> Result<Option<String>> {
        let Some(own) = OwnProfile::new(self).signed()? else {
            return Ok(None);
        };
        let Some(record) = Profile::parse(&own.record) else {
            return Ok(None);
        };
        let key = self.signing_key()?;
        let me = BASE64.encode(key.public_key());
        if !chain(&record.id, &own.first, &own.keys, &[])
            .0
            .contains(&me)
        {
            return Ok(None);
        }
        let sent = if picture {
            own.record
        } else {
            Profile {
                picture: None,
                ..record
            }
            .encode()
        };
        let profile = SignedProfile {
            sig: key.sign(&record_bytes(&sent)),
            record: sent,
            signer: me,
            first: own.first,
            keys: own.keys,
            aliases: Vec::new(),
            fresh: false,
        };
        let mut map = Map::new();
        if back {
            map.insert("back".into(), Value::Bool(true));
        }
        map.insert("format".into(), FORMAT.into());
        map.insert("key".into(), KeyRecord::make(&key).to_json());
        map.insert("profile".into(), profile.to_json());
        let payload = Value::Object(map).to_string();
        let sig = BASE64
            .decode(key.sign(&card_bytes(&payload)))
            .map_err(|e| crate::Error::Invalid(format!("a signature: {e}")))?;
        Ok(Some(format!(
            "{}.{}",
            BASE64_URL.encode(payload.as_bytes()),
            BASE64_URL.encode(sig)
        )))
    }

    /// Opens the card `text` (a link tapped, scanned or pasted), as the
    /// Dart core's `openProfileCard`: the person it names is kept as
    /// another person's Profile is (ADR 0017), not verified, and the key
    /// of the Device that made it is pinned, so Sent Items to them are
    /// sealed for it. Scanned `in_person`, their short code is the one
    /// the card binds and the person is verified at once. The user's own
    /// card, a blocked person's, one a Device of a blocked person made,
    /// and one that names a Profile kept here under another first key
    /// keep nothing.
    pub fn open_profile_card(&self, text: &str, in_person: bool) -> Result<CardOpened> {
        self.take_card(read_card(text), in_person)
    }

    /// Opens the Profile card file `bytes` (a file tapped, opened or
    /// dropped), as the Dart core's `openProfileCardFile` and as
    /// [`Store::open_profile_card`] opens a link: a file that is none, is
    /// larger than [`MAX_FILE_BYTES`] or holds a card that is changed or
    /// not bound is unreadable, one of a later format
    /// ([`CardTaken::NeedsUpdate`]) is not read further. Only an absent
    /// `format` or the whole number 1 is this version's (ADR 0034). A
    /// file is never in person: the person stays not verified.
    pub fn open_profile_card_file(&self, bytes: &[u8]) -> Result<CardOpened> {
        let json = card_file_json(bytes);
        if json
            .as_ref()
            .and_then(|json| json.get("format"))
            .is_some_and(|format| format.as_i64() != Some(FORMAT))
        {
            return Ok(CardOpened::only(CardTaken::NeedsUpdate, None));
        }
        let card = json
            .as_ref()
            .and_then(|json| json.get("card"))
            .and_then(Value::as_str)
            .and_then(|card| {
                let mut parts = card.split('.');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some(payload), Some(sig), None) => check_card(payload, sig),
                    _ => None,
                }
            });
        self.take_card(card, false)
    }

    /// Keeps the person of `card`, checked, as
    /// [`Store::open_profile_card`] says.
    fn take_card(&self, card: Option<Card>, in_person: bool) -> Result<CardOpened> {
        let Some(card) = card else {
            return Ok(CardOpened::only(CardTaken::Unreadable, None));
        };
        let id = card.id.clone();
        if self.is_own_person(&id)? {
            return Ok(CardOpened::only(CardTaken::Own, Some(id)));
        }
        if self.is_blocked(&id)? || self.blocked_devices()?.contains(&card.key.device) {
            return Ok(CardOpened::only(CardTaken::Blocked, Some(id)));
        }
        let taken = match self.take_person(&card.profile.to_json(), None)? {
            PersonTaken::Added => CardTaken::Added,
            PersonTaken::Updated => CardTaken::Updated,
            PersonTaken::Unchanged => CardTaken::Unchanged,
            PersonTaken::Refused => return Ok(CardOpened::only(CardTaken::Refused, Some(id))),
        };
        self.learn_keys(&[card.key], None, &mut ImportReport::default())?;
        if in_person && let Ok(first) = BASE64.decode(&card.profile.first) {
            self.verify_person(&id, &short_code(&id, &first))?;
        }
        Ok(CardOpened {
            taken,
            id: Some(id),
            offer_back: taken == CardTaken::Added && !card.back,
        })
    }
}

#[cfg(test)]
mod tests;
