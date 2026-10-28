//! Profile cards, as the Dart core's `profile_cards.dart` makes and
//! opens them (ADR 0017, #107): a person sends their own Profile as a
//! link through any messenger, or shows it as a QR, and whoever opens it
//! has them in People. The card follows the `#` of [`LINK`], which never
//! reaches a server: `<payload>.<signature>`, both base64url without
//! padding. The payload is one JSON object, keys sorted and no spaces,
//! the same bytes from both cores:
//! `{"back":true,"format":1,"key":<key record>,"profile":<signed Profile>}`.
//! `key` is the key record of the Device that made the card (ADR 0015),
//! `profile` the own Profile as it goes to other people with its picture
//! left out and no aliases, its record signed again by that Device;
//! `back` is there when the card answers one the user opened. The
//! signature is that Device's, over [`card_bytes`] of the payload as
//! sent, and its key must be bound to the Profile. A card adds a person
//! and nothing else: no Area, no grant, no key but the public ones.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL;
use serde_json::{Map, Value};

use crate::Result;
use crate::people::PersonTaken;
use crate::profile::{OwnProfile, Profile, SignedProfile, chain, check, record_bytes, short_code};
use crate::signing::{ImportReport, KeyRecord, verify_signature};
use crate::store::Store;

/// The address a card goes out under; the card follows the `#`.
pub const LINK: &str = "https://paxel.github.io/oldbrainfix/p#";

/// The most characters of a card that are read at all.
pub const MAX_CHARS: usize = 16 * 1024;

const FORMAT: i64 = 1;

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

/// The card `text` holds ([`card_parts`]), checked: its key record valid
/// and its signature over the payload as sent, before anything else of
/// it is read; then its Profile's signatures, and its key bound to that
/// Profile. None when any of it fails.
fn read_card(text: &str) -> Option<Card> {
    let (payload, sig) = card_parts(text)?;
    let payload = String::from_utf8(from_base64_url(&payload)?).ok()?;
    let sig = from_base64_url(&sig)?;
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
    /// The own Profile as a card's link, as the Dart core's
    /// `profileCard`: the record without its picture, signed by this
    /// Device, with the binding and this Device's key record, the whole
    /// signed by this Device; with `back` when it answers a card the
    /// user opened. None before the own Profile exists, or while this
    /// Device's key is not bound to it yet.
    pub fn profile_card(&self, back: bool) -> Result<Option<String>> {
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
        let bare = Profile {
            picture: None,
            ..record
        }
        .encode();
        let profile = SignedProfile {
            sig: key.sign(&record_bytes(&bare)),
            record: bare,
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
            "{LINK}{}.{}",
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
        let Some(card) = read_card(text) else {
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
