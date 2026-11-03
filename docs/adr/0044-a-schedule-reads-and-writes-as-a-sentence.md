# A schedule reads and writes as a sentence; the sentence is never stored

The Routine editor's schedule has a line at its top that shows the schedule as a sentence and fills the chips and times as it is typed (#227, Q552 (a), Q553 (a)). Both cores write, read and suggest it the same way (`schedule_sentence.dart`, `schedule_sentence.rs`, fixture `schedule_sentences.json`), so both apps read each other's sentences. The sentence is a view: the schedule's fields are stored, as before (ADR 0006).

## The module

Three functions, in both cores (in Rust `schedule_sentence::{sentence, read, suggest}`):

- `scheduleSentence(schedule, locale, today)` writes a schedule of the simple editor (`SimpleSchedule`: its `SimpleRepeat` with the end, its start and times).
- `readSchedule(text, locale, today)` gives the schedule read, or none when the text names no repeat, and the ranges of the parts not understood.
- `suggestSchedule(text, cursor, locale)` gives the words that may come at the cursor, each replacing the word being typed.

The language is the app's: `de` reads and writes German, any other English. `today` is a parameter, so the same input gives the same output on both cores: a day of `today`'s year is written without its year and a day typed without one lies in it, and a start on `today` is left out and one not typed is `today`. So reading a written sentence gives the same schedule back, for every simple schedule (the round trip the fixture holds).

## The grammar

The sentence is the repeat, then the times, then the start and the end, each but the repeat left out when it is the default (all day, from today, never ending).

| Part | English | German |
|---|---|---|
| Once | `Once on 11/5` | `Einmal am 5.11.` |
| Every N | `Every day`, `Every 3 days` (week, month, year) | `Jeden Tag`, `Alle 3 Tage`; `Jede Woche`, `Jeden Monat`, `Jedes Jahr` |
| Weekdays | `Weekdays Mon, Thu` | `Wochentags Mo, Do` |
| Monthly on a day | `Every month on day 15`, `Every 2 months on day 15` | `Jeden Monat am 15.`, `Alle 2 Monate am 15.` |
| Monthly on a weekday | `Every month on the first Monday` (second, third, fourth, last) | `Jeden Monat am ersten Montag` (zweiten, dritten, vierten, letzten) |
| Times | ` at 08:00, 15:00` | ` um 08:00, 15:00` |
| Start | `, from 12/1` | `, ab 1.12.` |
| End on a day | `, until 12/31` | `, bis 31.12.` |
| End after N | `, 10 times`; `, once` | `, 10-mal`; `, einmal` |

A day is written in the numbers of the language's date, `12/31` and `31.12.`, with `/2027` and `2027` when it is not this year's. Weekday names are the app's: the phone's short ones, as on its weekday chips (`Mon`, `Mo`), and the long ones after an ordinal.

The reader takes more than the writer writes: any case; short or long weekday names, with or without a dot (the desktop's `Mo.`); `Every Mon` and `Immer Mo` for weekdays, `Jeden Montag` too; `and`/`und` between weekdays and times; the singular or plural of a unit after a number; any of `Jeden`, `Jede`, `Jedes`, `Alle`; `time` for one; `10mal` and `10 mal`; a time as `8:00`; a day with `/` as month and day and with `.` as day and month in either language, a year of two digits as 20xx; the parts after the repeat without their commas and in any order, a later one winning.

## Unknown parts change nothing

The reader goes token by token. A token that does not fit where it stands is marked and skipped. A part begun and not finished (`until` without a day, `on the` without a weekday, `at 25:00`) is marked as a whole and falls back to what came before it, so `Every month on day 32` reads as every month. While the repeat itself is not understood (`Every 2 wekks`) the reading is no schedule and the editor keeps what it has; once it is, the schedule read is the whole schedule: the times typed, all day for none. Once takes one time and no end; more are marked.

The line shows what the chips hold whenever it is not being typed in, and also when a chip changes while it is: leaving it drops the marked words. On an Advanced schedule it shows the Agenda's summary of the schedule (`schedule_text`), read-only, with "Switch to Simple to type".

## Suggestions

At the cursor the reader's state gives the words that may follow: the repeats at the start, the units, the weekday names not picked yet, the ordinals, `at`/`from`/`until` after the repeat (`at` only before times, no end for once), `times` after a number, and times: the hour typed on the hour and the half (`8` gives `08:00`, `08:30`), else 08:00, 12:00 and 18:00. A suggestion replaces the word being typed and adds a space.

## The AI port

`read_routine` and the other reads keep the schedule's fields (`rule`, `start`, `times`) and get no sentence. The AI reads and writes RRULEs well; a sentence would be a second, language-bound reading of the same fields, written for the day of the read, and could only add a way for the two to be read apart. The set-up tools take the fields as before.

## Considered

- Writing the start always, and days with their year (rejected: the issue's example leaves both out; `today` keeps it a pure function).
- Fields left out of the sentence keeping their value (rejected: deleting `at 08:00` could not make it all day; the sentence is the schedule).
- The desktop's dotted short weekdays (`Mo.`) in the sentence (rejected: the phone's chips and the issue's example have `Mo`; both are read).
- Month names in days (rejected: the sentence writes the date's numbers, as the app's days do).
- Replacing the Agenda's and All Routines' summary (`scheduleText`) with the sentence (rejected for now: the rows show the dates with weekdays and the times after a `·`; the editor's Advanced summary reuses it).
