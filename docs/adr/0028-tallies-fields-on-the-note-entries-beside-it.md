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
