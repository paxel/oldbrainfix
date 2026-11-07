# Tallies: fields on the Note, entries beside it, numbers in millionths

A Tally (#103) is a Note that records how much was done: push-ups, pages read, minutes of video, lifted weight. It has fields, each a number with a unit, and a history of entries that the apps show as a table and as one chart per field. Both cores read and write it the same way (`tallies.dart`, `tally_units.dart`, `tallies.rs`, `tally_units.rs`); a Session (#104) picks Tallies and counts their entries.

## The fields lie on the Note

A Tally is no entity of its own: its fields are fields of its Note, each under `tally:<field id>:<key>` with the keys `name`, `unit`, `target`, `rule` (`sum` or `max`) and `hidden` (`true`, or null when shown). The field id is a fresh id; the fields show in the order they were added, by the earliest date of their `name`, then by id, so every Device orders them alike. A Note is a Tally while it has a field; the first one takes it out of Unsorted by writing the Note's `spawned`, as a Frog does (ADR 0027). Only a Note in the lists becomes a Tally.

Lying on the Note, the fields are changed as the Note is: by its owner, or by a Manager of its Area (ADR 0013). No new rule decides who may define a Tally, and its fields go wherever the Note goes: into a Shared Area with its filing, with an offer from afar, into a Backup. A field is never removed: hiding it keeps its values, which return when it is restored. Fields can be added and renamed at any time; a field's kind of unit is fixed once any entry, a removed one included, holds a value for it. Its unit may still change within the kind, from kilograms to grams, since the values are stored in the kind's base unit. Concurrent edits to a field go by latest wins; no Conflict is raised on them.

A Tally merged into another Note shows under that Note with its fields and entries, as Sections and Frogs follow a Merge; a field keeps lying on the Note it was added to, and is changed there.

## Entries are entities beside it

An entry is the entity `tallyEntry:<id>` with `$type` = `tallyEntry`, `note` (the Tally's Note), `at` (the instant it counts for, the store's now unless given), one `v:<field id>` per field it holds a value for, `line` (its free text line, absent when empty) and `trashed` (when removed; null when restored). A field added later has no value in older entries. Search finds a Note by the line of an entry not removed.

An entry lies in the Area of its Note: it is sealed under that Area's key, goes with the Note when the Note is filed into a Shared Area (`withFiled`) and with an offer from afar. It is judged like a Section: anyone with a role in the Area adds one, and only the person who wrote its first `$type`, or a Manager, changes or removes it. Its author is that Device, shown through `authorOf`; the charts and the table take a person filter (`PersonFilter`), the user's own by default in the apps, everyone's or one person's on a switch.

A blocked person's entries carry text and numbers, so they are refused and deleted like the rest of their writing (ADR 0021).

## Numbers and units

Every number in the log, a value or a target, is a whole count of millionths of its base unit, written as a plain decimal without trailing zeros (`8046.72`, `-3.5`, `20`), at most 10^9 either way. Reading accepts only that form. No floating-point text reaches the log, so both cores keep, sum and compare the same numbers; sums are taken over the millionths, held at the 64-bit limits.

A unit is a code from the known list or text as typed. The known units are stored in metric, in the base unit of their kind: mass `kg` and `g` in kilograms (shown in imperial as `lb`, `oz`), distance `km`, `m`, `cm` in metres (`mi`, `ft`, `in`), speed `km/h` (`mph`), temperature `°C` (`°F`), angle `°`, time `h`, `min`, `s` in seconds (the same in both systems). Text equal to a known code is that unit; any other is typed and is stored and shown as typed. Conversion uses the exact factors of the international yard and pound and the same order of operations in both cores. A new field's rule defaults by kind: the largest for mass, speed, temperature and angle, the sum for distance, time and typed units.

Which system a Device shows is a Device setting (`unitSystem`, ADR 0008): metric or imperial when chosen, else the one of the region the system reports, imperial for US, LR and MM. The cores also show a number (time as `m:ss` or `h:mm:ss`, anything else with up to two decimals and the decimal sign the app passes) and read one as typed (a decimal comma counts as a point; a time also as `m:ss` or `h:mm:ss`), so both apps agree.

## Charts

A chart is one bar per day, week (from Monday), month or year on the Device's wall clock, from the first bar with a value to today's, at most the latest 1000. A bar holds the field's rule applied to every value in it, the sum or the largest; bars without a value hold none. Entries after today stay out of the charts and show in the table.

Amended by #264: the length of a bar is the Tally's own, set in its setup (below), and a target counts per bar. The apps draw slim bars of a fixed width with gaps, the newest on the right, as many of the latest as fit the width, the target as a thin line; a single bar no longer fills the chart. Nothing scrolls sideways, as a horizontal swipe navigates (#258); older values are in the entries.

## Considered Options

- A `tally:<id>` entity as a child of its Note (rejected: as a Section-like child a Member could make another person's Note a Tally; on the Note, the Note's own rule decides).
- The fields as one JSON value (rejected: two Devices changing different keys of one field would lose one change, and JSON numbers are written differently by the two cores).
- Entries as Sections (rejected: the issue wants structured values with a date; Sections are texts with a Capture Context).
- Values stored as typed, with their unit (rejected: the issue asks for metric storage, and a sum over mixed units would need converting on every read).
- Values as floating-point text (rejected: the two cores write some doubles differently).
- Weeks from the locale's first day (rejected for now: one rule for both apps; Monday as ISO 8601).

## Consequences

- Tallies do not travel in Sent Items; a Note sent leaves its fields and entries behind.
- The AI port has no tool for Tallies, so an AI Task cannot change them (ADR 0026).
- An entry removed stays in the log and can be restored; it is never purged.

## Changes to earlier decisions

- ADR 0005: making a Note a Tally takes it out of Unsorted, as spawning does.
- ADR 0008: the unit system is a Device setting.
- ADR 0013: a Tally's fields are changed as its Note is; its entries are judged like Sections.

## Shown on the card (#190)

The charts and the entries show on the Tally's detailed card, with "Add entry"; the Note editor holds only the setup, the fields with their settings. The compact card's totals today take the same person filter the detailed view starts with: the user's own in a Shared Area. The length of a bar and whether the older entries show are kept per Note as local settings of the Device (`tallyPeriod:<note>`, `tallyEntriesShown:<note>`), the same keys in both apps; nothing is written to the log.

Amended by #264: the length of a bar lies on the Note (below); `tallyPeriod:<note>` is no longer read, and the compact card shows each field's total of the current bar, today's for a Tally of days.

## A Note is its kind (ADR 0038)

Amended by ADR 0038 (#188): a Tally's text shows as its name, the first line, heading its page and card, and its description, the rest, folded under it, above the Tally's content. A Tally stays a Tally: its kind badge offers no other kind.

## Multipliers (#200)

Replaced by Sets (#264, below): the apps no longer write `multiplies`, and what follows says how the cores still read it.

A field can multiply one other field of the same Tally, as Sets multiply Reps: the key `tally:<field id>:multiplies` holds the other field's id, null when it multiplies nothing. It is written as the other keys of a field are, by who may change the Note, latest wins.

It is read, never written into the entries: per entry the multiplied field counts as its value times the value of each of its multipliers the entry holds, a multiplier without a value counting as 1; totals, targets, the charts and the AI port count that product. The product is taken in millionths, each step rounded half away from zero and held at the 64-bit limits, in the same order in both cores. A multiplier counts no value of its own: it has no total, no chart and no target, its stored target and rule are kept for when it is a field of its own again, and the entry form still asks for its value. An entry shows a multiplied field's value after its multipliers' values, as bare numbers ("3 × 10 reps"). Making a field a multiplier later applies to the entries already there, and nothing is rewritten.

Lead's choices, as both cores read them: a field's setting multiplies when it names a field shown other than itself whose own setting names no field shown other than that field, and the field itself is shown. So no chain forms, two fields naming each other both stay plain, and hiding the field multiplied, which is what removing a field is (fields are never removed), leaves the multiplier a plain field until the field is shown again. Several fields may multiply one field; the product takes them all, in the fields' order. Setting one is refused for itself, a field hidden or not of the Tally, a field whose setting already multiplies, and on a field hidden or one another field multiplies. The fields of a Note merged into the Tally take part like its own.

- Considered: the product written into each entry (rejected: old entries would need rewriting, and a later change of the setting would leave them wrong); one multiplier per field only (rejected: the product of several reads plainly and needs no rule to pick one).

## Sets, Check fields and the bar length (#264)

The user found the chart one big block with few bars, the target blind to the multiplier, and several multiplied fields unclear; sports need sets with their own values (60 kg × 10, then 65 kg × 8), and cleaning Tallies (Bathroom, Kitchen) had no field that fits. Both cores read and write the following alike; the cross-core fixtures cover them.

- **The Tally's own settings** lie on its Note, beside its fields, written as they are, by who may change the Note, latest wins: `tally:bar`, the length of a bar of its charts (`day`, `week`, `month` or `year`; absent or anything else reads as a day), and `tally:sets`, whether it counts in sets (`true` or `false`). A Note merged into the Tally takes no part: its own settings stay unread. Only the Tally's own Note is read for them.
- **A Check field** is a field with `tally:<field id>:kind` = `check`; absent, or anything else, is a number. It has no unit and always sums, whatever its rule says; its kind is chosen when it is added and never changes. A tick is the value 1 under `v:<field id>`, nothing when unticked; another value is refused on write. It may hold a target, counted per bar like any.
- **Sets.** An entry's first set and its ticks stay under `v:<field id>`, so an entry without sets reads as before and an older version reads its first set. Each further set n, 2 to 99, lies under `v<n>:<field id>`, n written without a leading zero; any other such key is not read, and the sets read in the order of n, a gap closing up. A set that holds nothing is not written. Writing refuses a Check field, a field counting sets (below) or no field in a further set, and more than 99 sets. Changing an entry's sets writes each `v<n>:` that differs and null for those it no longer holds.
- **What an entry counts** for a number field is its rule over its sets (reps by their sum, a weight by the largest), in millionths, sums held at the 64-bit limits; for a Check field its tick. Bars, totals, targets and the AI port take that, then the rule over the entries of the bar. No computed field such as volume (weight × reps): the app is not a full sports tracker.
- **The switch** only decides the entry form: with it, "Add set" adds a set that starts with the values of the set before. Entries keep and count their sets either way. A Check field is ticked once per entry, above the sets.
- **Former multipliers.** A field the #200 rules read as multiplying another one now counts sets: it counts no value of its own, has no target, no chart and no place in the entry form, and its setup says so. An entry that holds no further sets but a value of such a field reads as that many sets of its first set: the first such field in the fields' order, its value rounded half away from zero, at least 1 and at most 99. So 3 Rounds × 20 Push-ups reads as three sets of 20, counting 60 as before, without rewriting the entry. A Tally with such a field counts in sets while `tally:sets` is absent. Changing such an entry's sets writes them out and takes the multiplier's value away, so it holds its sets itself.
- **Packs** carry a field's `check`, the Tally's `bar` when not a day and `sets` when on; `multiplies` still travels as before.
- **The AI port** (ADR 0026) reads a Tally's `bar` and `sets` and each field's `check` and whether it counts sets, and each entry's sets with what it counts per field; `make_tally` and the new `set_tally` set the bar length and the switch, `add_tally_field` takes `check`. No tool writes `multiplies` any more.

Considered: sets as one JSON value per entry (rejected: no JSON in the log, and two Devices changing different sets would lose one); the number of sets as a key of its own (rejected: the highest set held says it, and an older version would show it as a field); the bar length kept per Device (rejected by Q604: it syncs with the Note); a volume field (rejected by Q608); writing former multiplier entries out as sets (rejected: every Device would rewrite the same entries).

