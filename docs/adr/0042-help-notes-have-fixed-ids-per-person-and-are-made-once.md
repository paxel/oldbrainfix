# Help Notes have ids fixed per person and version, and are made once

The app explains itself with its own Notes (#180, Q493–Q499): "Quick start", a Checklist of steps to try, "What the icons mean", with a picture of each group of icons, and one "What's new in X.Y.Z" per release, holding that release's Play notes. They are filed into their own Area "Old Brain Fix", never Unsorted and never Nudged, and the user edits, moves and deletes them like any Note. Both cores make them the same way (`help_notes.dart`, `help_notes.rs`); the apps hand in the words, pictures and What's new.

## The ids are fixed by the person and the version

Each help Note has an id made from the own Profile's id and, for a What's new, the version: `note:help-<profile>-quick-start`, `note:help-<profile>-icons`, `note:help-<profile>-whats-new-<version>`; the Area is `area:help-<profile>`, the Quick start's making `help-<profile>-quick-start` and its Lines `line:help-<profile>-quick-start-<n>`. So a second Device of the same person finds them made, and two of one's own Devices that make one apart, before they meet, write the same entities, and after a sync hold one Note with one list of Lines, not two.

The id names the person, not only the version as the Starter Set's do (ADR 0008's `area:starter-<slug>`): a help Note can be filed into a Shared Area, where a Note under the same id of another person would mix with it. The own Profile id is the one token every Device of the person holds once it joined them; the ids the Profile replaced (its aliases) count as the person too. Nothing is made before the own Profile exists, which on both apps is right after the first start's answer.

The Area has no `starter` field, so sharing it does not move it to new ids, and it is no Starter Set Area.

## Made once per person, by the person's own Devices

A help Note counts as made when one of the user's own Devices ever wrote an entry under its id (this one, one paired as its own, one whose key is bound to the own Profile), as for a setting of the person (ADR 0040). An entry another person's Device slipped in under the id counts for nothing. A Note in the trash and one purged keep their entries in the log, so a deleted help Note stays deleted: no Device makes it again.

## Fresh install or update

Each Device keeps, as a local setting (`helpVersion`, ADR 0008), the version it last made the help Notes for. On every start after the first-start flow both apps call the core with the version they run:

- The same version recorded: nothing, without reading anything.
- A first start just answered (new person, a Device added, a Backup restored) is fresh: Quick start and What the icons mean when never made for the person, no What's new.
- Otherwise an update: the two help Notes when never made, and the What's new of every version after the recorded one, up to the version run, that has Play notes bundled, oldest first, each once per person. A Device that recorded none came from 0.3.1 or earlier, the last version without help Notes: every bundled version after 0.3.1 counts as passed.

Then the version is recorded.

## Language

The Notes are written in the language of the Device that makes them; a What's new without that language is written in English. A second Device makes none, so no copy in another language appears. Two Devices in different languages that make the same Note apart, before they meet, end with the later Device's words on every field; the Quick start has as many Lines in each language (a test holds it), so its list is one language.

## The Area

It is made with the first help Note that goes into it: "Old Brain Fix", the app's teal `00796B` and the icon `brain` (a new key of `areaIcons`, drawn as Material `psychology`, pickable for any Area). A renamed or recoloured one stays as the user left it. When the user deleted it and a help Note is due, it is made again under its id with the look it had, so no help Note is ever Unsorted. The Quick start is a Checklist without a Routine, so it shows on the Agenda. The user chose (Q549 (a)) one row with its count, "Quick start: 8 of 8 open", rather than a row per Line: making it also writes the person's own "Show on the Agenda" choice for it, `agenda:<quick start id>` = `count` (#219), in the same batch, so both cores write the same entries and the person may change it as for any Checklist.

## What's new is bundled from the release notes

The release's Play notes (`play_notes_en`, `play_notes_de` in `docs/release-notes-<version>.md`, written at release time by the store kit) are the text. `python3 tool/whats_new.py --bundle` writes every release after 0.3.1 into `assets/help/whats_new.json`, which the phone bundles as an asset and the desktop with `include_str!`. Flutter has no build step that could read the release notes itself, so the file is generated and committed:

- CI fails when the committed file is not what the release notes give (`test_whats_new.py`).
- The tag pipeline runs `--bundle --require <version>` and fails when the tagged version has no notes or the committed file lacks them.

So a release cannot ship without its What's new, while a branch whose release notes are not written yet (0.4.0 before its release) builds and stays green; there the app makes no What's new for it.

## The icon pictures

One picture per group of icons (the Note editor's badges, the Agenda, a Note card), in English and German, light, drawn from the app's icon font by the phone's screenshots generator (`test/screenshots/screenshots_test.dart`, in the guide frames, `UPDATE_FRAMES=1` writes them) into `assets/help/`. The desktop bundles the same files, so both apps attach the same bytes under the same hashes. The Note's text lists the same names and meanings, so search finds them; tests on both apps check that every icon named in the text has its picture and that the desktop's groups are the phone's.

## Tests see none

Both apps make help Notes only when handed what to make them from: the phone's `OldBrainFixApp.help`, the desktop's `App::set_help`, given by `main.dart` and the desktop binary only. Every other test and the screenshot generators start without them.

## Amendment: where and how (#260, Q589 (a))

The 0.4.0 Lines said what to do, never where or how ("Capture a thought with +."). From 0.5.0 each Line names the phone's place and gesture, such as "Tap the round + at the bottom right of the Agenda, then Text, type a thought and tap 1 h.", in up to about 20 words; the 12-word box rule (#171) does not hold for them. The Quick start's text is its name and a Description (#188), folded under it: "On the desktop", then the desktop's way for each Line, one line each, in the Lines' order (menus, keys, clicks, drag and right-click). Both cores take it as `HelpTexts.quickStartDescription` and write the Note's text as name and Description joined (`NoteKinds.join`). The phone's words are written whichever Device makes the Note first, as before; neither the Lines nor the Description name anything of Advanced, and tests on both apps hold it.

A Quick start already made stays as it was: it is made once per person (above), and its id stays `note:help-<profile>-quick-start`, so a person who has the 0.4.0 one gets no second one and keeps its words. The What's new of 0.5.0, written at release time from the release notes, names the change and where the Quick start is (the Area "Old Brain Fix"); that needs no code. A deleted Quick start stays deleted, as before.

## Amendment: links to the places (#261, Q599 (a))

The Quick start and What the icons mean are written as formatted texts from 0.5.0, so their app links open (ADR 0046): the Quick start's Description begins with a numbered link to each step's place, before the desktop's way, and What the icons mean ends with a link to the user guide. A Quick start or What the icons mean made before stays as it was, plain and without links, as above.

## Considered

- Ids by version only, as the Starter Set's (rejected: a help Note filed into a Shared Area would mix with another person's).
- A random seed kept in the settings of the person (rejected: two Devices that make one before they meet would write two seeds and two Notes).
- A marker per help Note in `settings:person` instead of the Note's own history (rejected: the fixed id already tells whether it was made).
- Dating the help Notes' entries back like a planting (rejected: their Note History would show a date the user never saw; the made-once check already keeps a second Device from writing them).
- Failing every build when the current version has no Play notes (rejected: the notes are written at release time, so every branch build would fail until then).
- Quick start Lines for Tallies, Sessions or Frogs marked Advanced (rejected: Simple is picked by default; the Quick start teaches what Simple offers).
- A new version of the Quick start for people who have the 0.4.0 one, under a new id such as `note:help-<profile>-quick-start-2`, the old one kept (rejected for #260: a second Checklist of the same steps in the same Area, and a second "Quick start" row on the Agenda; the What's new already reaches everyone who updates).

## Consequences

The cross-core fixture `help` writes the same log from both cores, and `help.json` holds what each step made. A Device that never learned that a help Note was made makes it itself; when the other Device had meanwhile deleted it, the Note comes back after they meet, as its writes there are later than the deletion. It takes two of one's own Devices updated apart and kept apart for that.
