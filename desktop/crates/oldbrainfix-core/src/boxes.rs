//! Ticking the boxes in the texts of Notes and Sections (ADR 0035), as
//! the Dart core's `boxes.dart` does: each tick is an entry of its own,
//! `box@<id>` = the state, on the entity whose text holds the box, the
//! Note, the Section, or the Note merged into another; the later one by
//! its date wins, and every one stays in the log. In a Shared Area
//! anyone with a role there ticks any box; anywhere else only ticks the
//! user's own Devices wrote count. A tick for an id the text no longer
//! holds counts for nothing.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::entry::{Entry, parse_time};
use crate::notes::Notes;
use crate::store::Store;
use crate::text_boxes::{self, BoxState, BoxTick, ShownBox};
use crate::{Error, Result};

/// The prefix of a tick's field, before the box id.
pub const PREFIX: &str = "box@";

/// The field a tick of the box `id` is written under.
pub fn tick_field(id: &str) -> String {
    format!("{PREFIX}{id}")
}

/// Reading and writing the ticks of boxes.
pub struct Boxes<'a> {
    store: &'a Store,
}

impl<'a> Boxes<'a> {
    pub fn new(store: &'a Store) -> Boxes<'a> {
        Boxes { store }
    }

    /// The latest tick of each box id of `entity` that counts; ids the
    /// text does not hold among them.
    pub fn ticks(&self, entity: &str) -> Result<BTreeMap<String, BoxTick>> {
        let mut latest: BTreeMap<String, (Entry, BoxTick)> = BTreeMap::new();
        for (e, id, tick) in self.counted(entity)? {
            let newer = latest
                .get(&id)
                .is_none_or(|(held, _)| Store::latest_wins(&e, held) == Ordering::Greater);
            if newer {
                latest.insert(id, (e, tick));
            }
        }
        Ok(latest
            .into_iter()
            .map(|(id, (_, tick))| (id, tick))
            .collect())
    }

    /// Every tick of the box `id` in the text of `entity` that counts,
    /// the oldest first.
    pub fn history(&self, entity: &str, id: &str) -> Result<Vec<BoxTick>> {
        let mut held: Vec<(Entry, BoxTick)> = self
            .counted(entity)?
            .into_iter()
            .filter(|(_, box_id, _)| box_id == id)
            .map(|(e, _, tick)| (e, tick))
            .collect();
        held.sort_by(|a, b| Store::latest_wins(&a.0, &b.0));
        Ok(held.into_iter().map(|(_, tick)| tick).collect())
    }

    /// The boxes of the text of `entity`, a Note's or a Section's id, as
    /// they show now; empty when there is no such text.
    pub fn of(&self, entity: &str) -> Result<Vec<ShownBox>> {
        Ok(self.boxes(entity)?.unwrap_or_default())
    }

    /// Whether this Device may tick the boxes of the text of `entity`:
    /// wherever it holds a role, as for checking a Line.
    pub fn may_tick(&self, entity: &str) -> Result<bool> {
        Ok(self.boxes(entity)?.is_some() && self.store.may_write_in(entity)?)
    }

    /// Ticks the box `id` in the text of `entity`, a Note's or a
    /// Section's id, to `state`. Nothing is written when it shows `state`
    /// already. An error when the text holds no such box, or this Device
    /// may not tick it.
    pub fn tick(&self, entity: &str, id: &str, state: BoxState) -> Result<()> {
        let Some(shown) = self.of(entity)?.into_iter().find(|b| b.id == id) else {
            return Err(Error::Invalid(format!("not a box of {entity}: {id}")));
        };
        if !self.store.may_write_in(entity)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {entity}"
            )));
        }
        if shown.state == state {
            return Ok(());
        }
        self.store
            .append(entity, &tick_field(id), Some(state.name()), None)
            .map(drop)
    }

    /// The boxes of the text of `entity`, or `None` when there is no such
    /// text.
    fn boxes(&self, entity: &str) -> Result<Option<Vec<ShownBox>>> {
        let notes = Notes::new(self.store);
        Ok(if entity.starts_with("note:") {
            notes.by_id(entity)?.map(|n| n.boxes)
        } else {
            notes.section_by_id(entity)?.map(|s| s.boxes)
        })
    }

    /// Every tick of `entity` in the shape the apps write that counts: in
    /// a Shared Area what the roles allow, anywhere else what the user's
    /// own Devices wrote, so a tick another person's modified app slips
    /// into the log changes nothing.
    fn counted(&self, entity: &str) -> Result<Vec<(Entry, String, BoxTick)>> {
        let shared = match self.store.area_of(entity)? {
            Some(area) => self.store.is_shared(&area)?,
            None => false,
        };
        let mut own = HashMap::new();
        let mut out = Vec::new();
        for e in self.store.history(entity)? {
            let Some(id) = e.field.strip_prefix(PREFIX) else {
                continue;
            };
            let Some(state) = BoxState::named(e.value.as_deref()) else {
                continue;
            };
            if !text_boxes::is_id(id) {
                continue;
            }
            let id = id.to_string();
            let mine = self.store.written_by_own_in(&e.device, &mut own)?;
            if !shared && !mine {
                continue;
            }
            let tick = BoxTick {
                state,
                at: parse_time(&e.date)?,
                device: e.device.clone(),
                own: mine,
            };
            out.push((e, id, tick));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{
        Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
    };
    use crate::shared::{AreaRole, Invite};
    use crate::signing::{ImportReport, SigningKey};
    use crate::store::tests::ticking;
    use crate::text_boxes::find;
    use crate::{OwnProfile, PairedDevice, Places, Search};
    use chrono::{Duration, TimeZone, Utc};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    /// One clock for every Device of a test, a second further at each
    /// reading, so what was written later wins everywhere.
    static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

    fn device(seed: u8, name: &str) -> Made<Store> {
        let start = ticking()();
        let clock: crate::Clock = Box::new(move || {
            let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            start + Duration::seconds(n)
        });
        let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }

    /// One LAN session, `joiner` asking `host`, both in a Shared Area.
    fn exchange(joiner: &Store, host: &Store) -> TestResult {
        let request = sync_request(
            joiner,
            &host.version_vector()?,
            "Phone",
            &host.device_id()?,
            "",
        )?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        apply_answer_as(joiner, &answer, true, None)?;
        Ok(())
    }

    fn remember(store: &Store, peer: &Store) -> TestResult {
        store.remember_device(&PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared: true,
        })?;
        Ok(())
    }

    /// Ann's Household, shared with Ben as a Member, and her Health Area
    /// beside it, shared with nobody.
    fn household() -> Made<(Store, Store, String, String)> {
        let ann = device(1, "Ann")?;
        let ben = device(2, "Ben")?;
        let house = Places::new(&ann).add_area("Household")?;
        let health = Places::new(&ann).add_area("Health")?;
        let area = ann.share(&house.id)?;
        let invite = Invite {
            area: area.clone(),
            role: AreaRole::Member,
        };
        let request = sync_request_as(
            &ben,
            &ann.version_vector()?,
            "Phone",
            &ann.device_id()?,
            "",
            Joining::Invited,
        )?;
        let (answer, _) =
            answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
        apply_answer_as(&ben, &answer, true, None)?;
        remember(&ann, &ben)?;
        remember(&ben, &ann)?;
        Ok((ann, ben, area, health.id))
    }

    fn shown(store: &Store, id: &str) -> Made<String> {
        Ok(Notes::new(store).by_id(id)?.ok_or("a Note")?.shown_text())
    }

    #[test]
    fn capture_edit_and_amend_make_boxes_and_the_text_written_comes_back() -> TestResult {
        let store = device(3, "Ann")?;
        let notes = Notes::new(&store);
        let note = notes.capture("☐ milk", None, None)?;
        let first = note.boxes.first().ok_or("a box")?.id.clone();
        assert_ne!(note.text, "☐ milk");
        assert_eq!(note.shown_text(), "☐ milk");
        notes.edit(&note.id, &format!("{}\n✅ eggs", note.text))?;
        let edited = notes.by_id(&note.id)?.ok_or("a Note")?;
        assert_eq!(edited.boxes.len(), 2);
        assert_eq!(edited.boxes[0].id, first);
        let section = notes.amend(&note.id, "❌ bread")?;
        assert_eq!(section.boxes[0].state, BoxState::Failed);
        let written = notes.take_over(&note.id, "☐ from a photo", true)?;
        assert_eq!(
            Some(&written),
            notes.by_id(&note.id)?.map(|n| n.text).as_ref()
        );
        assert_eq!(find(&written).len(), 3);
        Ok(())
    }

    #[test]
    fn a_tick_shows_the_later_one_wins_and_each_stays_in_the_history() -> TestResult {
        let store = device(3, "Ann")?;
        let notes = Notes::new(&store);
        let boxes = Boxes::new(&store);
        let note = notes.capture("☐ call Ann", None, None)?;
        let id = note.boxes[0].id.clone();
        assert!(boxes.may_tick(&note.id)?);
        boxes.tick(&note.id, &id, BoxState::Done)?;
        let before = store.all_entries()?.len();
        boxes.tick(&note.id, &id, BoxState::Done)?;
        assert_eq!(store.all_entries()?.len(), before);
        boxes.tick(&note.id, &id, BoxState::Failed)?;
        // Dated back, it loses to the ticks after it.
        let back = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single();
        store.append(&note.id, &tick_field(&id), Some("open"), back)?;
        let note = notes.by_id(&note.id)?.ok_or("a Note")?;
        assert_eq!(note.shown_text(), "❌ call Ann");
        assert!(note.boxes[0].last.as_ref().is_some_and(|t| t.own));
        assert_eq!(note.visible_text(), "❌ call Ann");
        let states: Vec<_> = boxes
            .history(&note.id, &id)?
            .into_iter()
            .map(|t| t.state)
            .collect();
        assert_eq!(states, [BoxState::Open, BoxState::Done, BoxState::Failed]);
        assert!(boxes.tick(&note.id, "zzzz", BoxState::Done).is_err());
        assert_eq!(Search::new(&store).notes("Ann", None)?.len(), 1);
        let share = crate::share::note_share(&note, &[], |d| d.to_string());
        assert_eq!(share.text, "❌ call Ann");
        Ok(())
    }

    #[test]
    fn a_tick_of_a_box_no_longer_in_the_text_counts_again_once_it_is_back() -> TestResult {
        let store = device(3, "Ann")?;
        let notes = Notes::new(&store);
        let note = notes.capture("☐ a\n☐ b", None, None)?;
        let (a, b) = (note.boxes[0].clone(), note.boxes[1].clone());
        Boxes::new(&store).tick(&note.id, &a.id, BoxState::Done)?;
        notes.edit(&note.id, &note.text[b.source_start..])?;
        assert_eq!(shown(&store, &note.id)?, "☐ b");
        assert!(Boxes::new(&store).ticks(&note.id)?.contains_key(&a.id));
        notes.edit(&note.id, &note.text)?;
        assert_eq!(shown(&store, &note.id)?, "✅ a\n☐ b");
        // A Section's boxes are ticked while its text stays.
        let section = notes.amend(&note.id, "☐ passport")?;
        Boxes::new(&store).tick(&section.id, &section.boxes[0].id, BoxState::Done)?;
        let held = notes.sections(&note.id)?;
        assert_eq!(held[0].shown_text(), "✅ passport");
        assert_eq!(held[0].text, section.text);
        Ok(())
    }

    #[test]
    fn ticks_of_another_shape_are_skipped() -> TestResult {
        let store = device(3, "Ann")?;
        let note = Notes::new(&store).capture("☐ pills", None, None)?;
        let id = note.boxes[0].id.clone();
        for (field, value) in [
            ("box@".to_string(), Some("done")),
            (format!("box@{id}x"), Some("done")),
            (format!("box@{}", id.to_uppercase()), Some("done")),
            (format!("box@@{id}"), Some("done")),
            (format!("box@{id}"), Some("Done")),
            (format!("box@{id}"), None),
            (format!("box@{id}"), Some("")),
        ] {
            store.append(&note.id, &field, value, None)?;
        }
        let note = Notes::new(&store).by_id(&note.id)?.ok_or("a Note")?;
        assert_eq!(note.shown_text(), "☐ pills");
        assert_eq!(note.boxes[0].last, None);
        Ok(())
    }

    #[test]
    fn a_member_ticks_the_boxes_of_any_note_and_section() -> TestResult {
        let (ann, ben, area, _) = household()?;
        let milk = Notes::new(&ann).capture("Buy milk\n☐ oat", Some(&area), None)?;
        let eggs = Notes::new(&ann).amend(&milk.id, "☐ ask about eggs")?;
        exchange(&ben, &ann)?;
        assert!(!ben.may_change(&milk.id)?);
        assert!(Boxes::new(&ben).may_tick(&milk.id)?);
        assert!(Boxes::new(&ben).may_tick(&eggs.id)?);
        Boxes::new(&ben).tick(&milk.id, &milk.boxes[0].id, BoxState::Done)?;
        Boxes::new(&ben).tick(&eggs.id, &eggs.boxes[0].id, BoxState::Failed)?;
        exchange(&ben, &ann)?;
        for store in [&ann, &ben] {
            let note = Notes::new(store).by_id(&milk.id)?.ok_or("a Note")?;
            assert_eq!(note.shown_text(), "Buy milk\n✅ oat");
            assert_eq!(note.boxes[0].starting, BoxState::Open);
            let last = note.boxes[0].last.clone().ok_or("a tick")?;
            assert_eq!(last.device, ben.device_id()?);
            let section = Notes::new(store)
                .section_by_id(&eggs.id)?
                .ok_or("a Section")?;
            assert_eq!(section.shown_text(), "❌ ask about eggs");
        }
        let tick = Notes::new(&ann).by_id(&milk.id)?.ok_or("a Note")?.boxes[0]
            .last
            .clone()
            .ok_or("a tick")?;
        assert!(!tick.own);
        assert_eq!(
            ann.ticked_by(&milk.id, &tick)?.map(|p| p.name),
            Some("Ben".to_string())
        );
        assert_eq!(ben.ticked_by(&milk.id, &tick)?, None);
        Ok(())
    }

    #[test]
    fn a_tick_another_persons_device_wrote_on_a_note_in_no_shared_area_counts_for_nothing()
    -> TestResult {
        let (ann, ben, _, health) = household()?;
        let note = Notes::new(&ann).capture("☐ pills", Some(&health), None)?;
        let id = note.boxes[0].id.clone();
        // As a modified app would.
        let forged = ben.append(&note.id, &tick_field(&id), Some("done"), None)?;
        let mut report = ImportReport::default();
        ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
        assert!(ann.all_entries()?.iter().any(|e| e.id() == forged.id()));
        assert_eq!(shown(&ann, &note.id)?, "☐ pills");
        assert!(Boxes::new(&ann).history(&note.id, &id)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_sent_item_carries_each_box_as_it_shows_its_id_kept() -> TestResult {
        let store = device(3, "Ann")?;
        let notes = Notes::new(&store);
        let note = notes.capture("☐ a ☐ b", None, None)?;
        let section = notes.amend(&note.id, "☐ c")?;
        Boxes::new(&store).tick(&note.id, &note.boxes[1].id, BoxState::Done)?;
        Boxes::new(&store).tick(&section.id, &section.boxes[0].id, BoxState::Failed)?;
        let item = store.sent_item_of(&note.id, false)?;
        let found: Vec<_> = find(&item.text)
            .into_iter()
            .map(|b| (b.id, b.state))
            .collect();
        assert_eq!(
            found,
            [
                (note.boxes[0].id.clone(), BoxState::Open),
                (note.boxes[1].id.clone(), BoxState::Done)
            ]
        );
        assert_eq!(find(&item.sections[0].text)[0].state, BoxState::Failed);
        Ok(())
    }
}
