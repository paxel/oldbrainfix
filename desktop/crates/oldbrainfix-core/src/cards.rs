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
mod tests {
    use super::*;
    use crate::sent::{OpenError, SenderTrust};
    use crate::store::tests::ticking;
    use crate::{Notes, SigningKey};
    use serde_json::json;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn device(seed: u8, name: &str) -> std::result::Result<Store, Box<dyn std::error::Error>> {
        let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        OwnProfile::new(&store).rename(name)?;
        Ok(store)
    }

    fn id_of(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
        Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
    }

    fn card(store: &Store, back: bool) -> std::result::Result<String, Box<dyn std::error::Error>> {
        Ok(store.profile_card(back)?.ok_or("a card")?)
    }

    fn payload_of(link: &str) -> std::result::Result<Value, Box<dyn std::error::Error>> {
        let (payload, _) = card_parts(link).ok_or("a card")?;
        Ok(serde_json::from_slice(&BASE64_URL.decode(payload)?)?)
    }

    /// A card made by hand: `payload` signed by `key`.
    fn card_of(key: &SigningKey, payload: &Value) -> String {
        let text = payload.to_string();
        let sig = BASE64
            .decode(key.sign(&card_bytes(&text)))
            .unwrap_or_default();
        format!(
            "{LINK}{}.{}",
            BASE64_URL.encode(text.as_bytes()),
            BASE64_URL.encode(sig)
        )
    }

    /// A Profile made by hand with `key` as its first key.
    fn forged(key: &SigningKey, record: &Value) -> Value {
        let text = record.to_string();
        SignedProfile {
            sig: key.sign(&record_bytes(&text)),
            record: text,
            signer: BASE64.encode(key.public_key()),
            first: BASE64.encode(key.public_key()),
            keys: Vec::new(),
            aliases: Vec::new(),
            fresh: false,
        }
        .to_json()
    }

    #[test]
    fn a_link_adds_the_person_not_verified_and_the_answer_offers_none() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let opened = ben.open_profile_card(&card(&ann, false)?, false)?;
        assert_eq!(opened.taken, CardTaken::Added);
        assert_eq!(opened.id, Some(id_of(&ann)?));
        assert!(opened.offer_back);
        assert_eq!(ben.person_shown(&id_of(&ann)?)?.name, "Ann");
        assert_eq!(
            ben.person_code(&id_of(&ann)?)?,
            OwnProfile::new(&ann).code()?
        );
        assert!(!ben.person_verified(&id_of(&ann)?)?);
        let back = ann.open_profile_card(&card(&ben, true)?, false)?;
        assert_eq!(back.taken, CardTaken::Added);
        assert!(!back.offer_back);
        let again = ben.open_profile_card(&card(&ann, false)?, true)?;
        assert_eq!(again.taken, CardTaken::Unchanged);
        assert!(ben.person_verified(&id_of(&ann)?)?);
        Ok(())
    }

    #[test]
    fn the_picture_stays_out_and_comes_with_their_profile_at_the_same_version() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        // A Profile picture the Dart core made.
        let picture = include_bytes!("../../../fixtures/shared/override.jpg");
        OwnProfile::new(&ann).set_picture(Some(picture.to_vec()))?;
        let link = card(&ann, false)?;
        assert!(!payload_of(&link)?.to_string().contains("picture"));
        ben.open_profile_card(&link, false)?;
        assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_none());
        let full = ann.own_profile_for_others()?.ok_or("her Profile")?;
        assert_eq!(ben.take_person(&full, None)?, PersonTaken::Updated);
        assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_some());
        assert_eq!(
            ben.open_profile_card(&link, false)?.taken,
            CardTaken::Unchanged
        );
        assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_some());
        Ok(())
    }

    #[test]
    fn a_card_changed_after_it_was_signed_is_unreadable() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let link = card(&ann, false)?;
        let (_, sig) = card_parts(&link).ok_or("a card")?;
        let mut flagged = payload_of(&link)?;
        flagged
            .as_object_mut()
            .ok_or("an object")?
            .insert("back".into(), Value::Bool(true));
        let changed = format!(
            "{LINK}{}.{sig}",
            BASE64_URL.encode(flagged.to_string().as_bytes())
        );
        assert_eq!(
            ben.open_profile_card(&changed, false)?,
            CardOpened::only(CardTaken::Unreadable, None)
        );
        assert_eq!(
            ben.open_profile_card("hello", false)?.taken,
            CardTaken::Unreadable
        );
        assert!(ben.kept_people()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_card_signed_by_a_key_its_profile_does_not_bind_is_unreadable() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let eve = SigningKey::from_seed([3; 32]);
        let mut payload = payload_of(&card(&ann, false)?)?;
        payload
            .as_object_mut()
            .ok_or("an object")?
            .insert("key".into(), KeyRecord::make(&eve).to_json());
        assert_eq!(
            ben.open_profile_card(&card_of(&eve, &payload), false)?
                .taken,
            CardTaken::Unreadable
        );
        assert!(ben.kept_people()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_copied_id_is_somebody_else_and_verifies_nobody() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        ben.open_profile_card(&card(&ann, false)?, false)?;
        let eve = SigningKey::from_seed([3; 32]);
        let copied = card_of(
            &eve,
            &json!({
                "format": 1,
                "key": KeyRecord::make(&eve).to_json(),
                "profile": forged(&eve, &json!({"id": id_of(&ann)?, "name": "Ann", "version": 9})),
            }),
        );
        assert_eq!(
            ben.open_profile_card(&copied, true)?.taken,
            CardTaken::Refused
        );
        assert!(!ben.person_verified(&id_of(&ann)?)?);
        Ok(())
    }

    #[test]
    fn a_blocked_persons_card_is_refused() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        ben.open_profile_card(&card(&ann, false)?, false)?;
        ben.block_person(&id_of(&ann)?, false)?;
        assert_eq!(
            ben.open_profile_card(&card(&ann, false)?, false)?.taken,
            CardTaken::Blocked
        );
        assert!(!ben.person_verified(&id_of(&ann)?)?);
        Ok(())
    }

    fn file(store: &Store, back: bool) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
        Ok(store.profile_card_file(back)?.ok_or("a card file")?)
    }

    /// The JSON of the card file `bytes`.
    fn file_json(bytes: &[u8]) -> std::result::Result<Value, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(bytes)?)
    }

    #[test]
    fn a_card_file_adds_the_person_with_the_picture() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let picture = include_bytes!("../../../fixtures/shared/override.jpg");
        OwnProfile::new(&ann).set_picture(Some(picture.to_vec()))?;
        let bytes = file(&ann, false)?;
        let json = file_json(&bytes)?;
        assert_eq!(
            json.as_object()
                .ok_or("an object")?
                .keys()
                .collect::<Vec<_>>(),
            ["card", "type"]
        );
        assert_eq!(json["type"], "oldbrainfix-card");
        assert!(looks_like_profile_card(&bytes));
        let opened = ben.open_profile_card_file(&bytes)?;
        assert_eq!(opened.taken, CardTaken::Added);
        assert_eq!(opened.id, Some(id_of(&ann)?));
        assert!(opened.offer_back);
        assert!(!ben.person_verified(&id_of(&ann)?)?);
        assert!(ben.person_shown(&id_of(&ann)?)?.picture.is_some());
        // The QR's card after it changes nothing; the full Profile neither.
        assert_eq!(
            ben.open_profile_card(&card(&ann, false)?, false)?.taken,
            CardTaken::Unchanged
        );
        let full = ann.own_profile_for_others()?.ok_or("her Profile")?;
        assert_eq!(ben.take_person(&full, None)?, PersonTaken::Unchanged);
        // Opened after the QR's card, the file brings the picture.
        let cara = device(3, "Cara")?;
        cara.open_profile_card(&card(&ann, false)?, false)?;
        assert!(cara.person_shown(&id_of(&ann)?)?.picture.is_none());
        assert_eq!(
            cara.open_profile_card_file(&bytes)?.taken,
            CardTaken::Updated
        );
        assert!(cara.person_shown(&id_of(&ann)?)?.picture.is_some());
        // The answer offers none; one's own and a blocked person's keep
        // nothing.
        let back = ann.open_profile_card_file(&file(&ben, true)?)?;
        assert_eq!(back.taken, CardTaken::Added);
        assert!(!back.offer_back);
        assert_eq!(ann.open_profile_card_file(&bytes)?.taken, CardTaken::Own);
        ben.block_person(&id_of(&ann)?, false)?;
        assert_eq!(
            ben.open_profile_card_file(&bytes)?.taken,
            CardTaken::Blocked
        );
        Ok(())
    }

    #[test]
    fn a_file_that_is_no_card_file_or_holds_a_changed_card_is_unreadable() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let bytes = file(&ann, false)?;
        let json = file_json(&bytes)?;
        let card_text = json["card"].as_str().ok_or("a card")?;
        let (payload, sig) = card_parts(card_text).ok_or("parts")?;
        let mut flagged: Value = serde_json::from_slice(&BASE64_URL.decode(&payload)?)?;
        flagged
            .as_object_mut()
            .ok_or("an object")?
            .insert("back".into(), Value::Bool(true));
        let changed = format!(
            "{}.{sig}",
            BASE64_URL.encode(flagged.to_string().as_bytes())
        );
        let mut padded = bytes.clone();
        padded.resize(MAX_FILE_BYTES + 1, b' ');
        let mut fits = bytes.clone();
        fits.resize(MAX_FILE_BYTES, b' ');
        let unreadable = [
            b"hello".to_vec(),
            Vec::new(),
            json!({"card": card_text}).to_string().into_bytes(),
            json!({"card": card_text, "type": "oldbrainfix-backup"})
                .to_string()
                .into_bytes(),
            json!({"card": changed, "type": "oldbrainfix-card"})
                .to_string()
                .into_bytes(),
            json!({"card": format!("{LINK}{card_text}"), "type": "oldbrainfix-card"})
                .to_string()
                .into_bytes(),
            json!({"card": 1, "type": "oldbrainfix-card"})
                .to_string()
                .into_bytes(),
            padded,
        ];
        for wrong in &unreadable {
            assert_eq!(
                ben.open_profile_card_file(wrong)?,
                CardOpened::only(CardTaken::Unreadable, None)
            );
        }
        assert!(!looks_like_profile_card(&unreadable[0]));
        assert!(!looks_like_profile_card(&unreadable[2]));
        assert!(!looks_like_profile_card(&unreadable[7]));
        assert!(ben.kept_people()?.is_empty());
        assert_eq!(ben.open_profile_card_file(&fits)?.taken, CardTaken::Added);
        Ok(())
    }

    #[test]
    fn a_card_file_of_a_later_format_needs_an_update() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let json = file_json(&file(&ann, false)?)?;
        for format in [json!(2), json!("1"), json!(1.0), Value::Null, json!(0)] {
            let mut later = json.clone();
            later
                .as_object_mut()
                .ok_or("an object")?
                .insert("format".into(), format.clone());
            let bytes = later.to_string().into_bytes();
            assert!(looks_like_profile_card(&bytes));
            assert_eq!(
                ben.open_profile_card_file(&bytes)?,
                CardOpened::only(CardTaken::NeedsUpdate, None),
                "{format}"
            );
        }
        assert!(ben.kept_people()?.is_empty());
        let mut one = json.clone();
        one.as_object_mut()
            .ok_or("an object")?
            .insert("format".into(), json!(1));
        assert_eq!(
            ben.open_profile_card_file(one.to_string().as_bytes())?
                .taken,
            CardTaken::Added
        );
        Ok(())
    }

    #[test]
    fn a_card_file_is_named_for_the_person() {
        for (name, file) in [
            ("Ann", "Ann.obfcard"),
            ("Ann Berg", "Ann Berg.obfcard"),
            ("a/b\\c:d*e?f\"g<h>i|j", "a_b_c_d_e_f_g_h_i_j.obfcard"),
            ("  ..Ann..  ", "Ann.obfcard"),
            ("...", "Profile.obfcard"),
            ("", "Profile.obfcard"),
            ("\u{200B}", "Profile.obfcard"),
            ("con", "_con.obfcard"),
            ("COM1", "_COM1.obfcard"),
            ("Nul .x", "_Nul .x.obfcard"),
            ("COM0", "COM0.obfcard"),
            ("Connie", "Connie.obfcard"),
            ("Zoë 😀", "Zoë 😀.obfcard"),
        ] {
            assert_eq!(file_name(name), file, "{name:?}");
        }
    }

    #[test]
    fn a_person_added_from_a_card_is_sent_to_sealed_and_theirs_preview_as_known() -> TestResult {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        assert!(ben.sent_item_recipients()?.is_empty());
        ben.open_profile_card(&card(&ann, false)?, false)?;
        let recipients = ben.sent_item_recipients()?;
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].name, "Ann");
        assert!(ben.paired_devices()?.is_empty());
        let note = Notes::new(&ben).capture("For Ann", None, None)?;
        let file = ben.send_note(&note.id, Some(&recipients[0]), false)?;
        assert!(
            ann.open_sent_item(&file)?
                .map(|o| o.encrypted)
                .unwrap_or(false)
        );
        let mine = Notes::new(&ann).capture("For Ben", None, None)?;
        let from_ann = ann.send_note(&mine.id, None, false)?;
        let opened = ben
            .open_sent_item(&from_ann)?
            .map_err(|e: OpenError| format!("{e:?}"))?;
        assert_eq!(opened.trust, SenderTrust::Known);
        ben.block_person(&id_of(&ann)?, false)?;
        assert!(ben.sent_item_recipients()?.is_empty());
        Ok(())
    }
}
