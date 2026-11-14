//! The tools an AI client calls through the AI port (#112, ADR 0026):
//! reading Notes with their Sections, Transcripts, Photo Texts, Capture
//! Context and Boxes, what kind each Note is with its parts, a photo
//! scaled down, the Areas and Topics, the Routines and Reminders, and
//! this Device's settings; and doing what the user can do with Notes,
//! Areas, Topics, Merges, the trash, Importance, the details of a Capture
//! Context, Routines and Reminders, and setting up Tallies, Sessions,
//! Checklists, Shopping Lists and Frogs (#168). Every change goes through
//! the screens' own calls inside [`Store::drafting`], so it is a Pending
//! Change of the AI Task until the user accepts it. The user's own data,
//! a Tally's entries, the ticks of Lines and Boxes, the check-offs, a
//! Session's picks and the eaten Frogs, is only read, each with its date.
//! Nothing reaches other people: no sending, sharing, Profile cards nor
//! blocking, and settings are only read.

use std::io::Cursor;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};

use crate::boxes::Boxes;
use crate::checklists::{
    Checklists, LineChecks, keys as checklist_keys, parse_line_in, parse_list_kind, split_lines,
};
use crate::entry::{dart_iso, parse_time};
use crate::frogs::Frogs;
use crate::note_history::NoteHistory;
use crate::note_kinds;
use crate::notes::{Attachment, CapturedDetail, Importance, Note, Notes};
use crate::people::PersonFilter;
use crate::person_settings::PersonSettings;
use crate::places::{NotesGo, Places};
use crate::routines::{Routine, Routines};
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule, WallClock};
use crate::search::Search;
use crate::sessions::{SessionMarks, SessionRule, SessionSetup, Sessions};
use crate::settings::{DeviceSettings, QuickNoteControl};
use crate::shopping_lists::{
    ListOnAgenda, Reappear, ReappearUnit, ShoppingLists, ShoppingOnAgenda, keys as shopping_keys,
};
use crate::store::Store;
use crate::tag_tree::{TagNode, TagTree};
use crate::tallies::{Tallies, Tally, TallyPeriod};
use crate::tally_units::{self, TallyRule, UnitSystem};
use crate::text_boxes::ShownBox;
use crate::text_format;
use crate::{Error, Result};

/// The longest side of a photo handed out, unless asked for less.
pub const PHOTO_EDGE: u32 = 1024;
/// The longest side a photo may be asked for at most.
pub const PHOTO_EDGE_MAX: u32 = 2048;

/// The most characters of an Attachment Text `read_note` hands out;
/// `read_attachment_text` reads the rest (ADR 0047).
pub const ATTACHMENT_TEXT_CHARS: usize = 4000;

/// The characters `read_attachment_text` hands out unless asked for
/// fewer, and the most it hands out at once.
pub const ATTACHMENT_READ_CHARS: usize = 20_000;
pub const ATTACHMENT_READ_MAX: usize = 50_000;
/// The largest photo read, by side.
const PHOTO_READ_MAX: u32 = 20_000;

/// The Capture Context fields of a Note, as the phone writes them.
const CONTEXT_FIELDS: [&str; 5] = [
    "location",
    "sourcePackage",
    "sourceLabel",
    "calendar",
    "bluetooth",
];

/// What a tool hands back: text, or a photo as JPEG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolContent {
    Text(String),
    Jpeg(Vec<u8>),
}

/// The answer to one tool call; `is_error` when the tool did nothing, its
/// text saying why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutput {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

impl ToolOutput {
    fn json(value: &Value) -> ToolOutput {
        ToolOutput {
            content: vec![ToolContent::Text(value.to_string())],
            is_error: false,
        }
    }

    fn failed(why: &str) -> ToolOutput {
        ToolOutput {
            content: vec![ToolContent::Text(why.to_string())],
            is_error: true,
        }
    }

    /// The MCP `CallToolResult`.
    pub fn to_json(&self) -> Value {
        let content: Vec<Value> = self
            .content
            .iter()
            .map(|c| match c {
                ToolContent::Text(text) => json!({"text": text, "type": "text"}),
                ToolContent::Jpeg(bytes) => json!({
                    "data": BASE64.encode(bytes),
                    "mimeType": "image/jpeg",
                    "type": "image",
                }),
            })
            .collect();
        json!({"content": content, "isError": self.is_error})
    }
}

/// A tool as `tools/list` names it.
fn tool(
    name: &str,
    description: &str,
    read_only: bool,
    properties: Value,
    required: &[&str],
) -> Value {
    json!({
        "annotations": {"readOnlyHint": read_only},
        "description": description,
        "inputSchema": {
            "properties": properties,
            "required": required,
            "type": "object",
        },
        "name": name,
    })
}

fn string(description: &str) -> Value {
    json!({"description": description, "type": "string"})
}

fn schedule_properties(extra: Value) -> Value {
    let mut properties = json!({
        "rule": string("How it repeats, as an RFC 5545 RRULE such as FREQ=DAILY or FREQ=WEEKLY;BYDAY=MO,TH; left out for a Reminder, which happens once"),
        "start": string("The first day, YYYY-MM-DD"),
        "times": {
            "description": "The times of day, HH:MM in 24 hours; empty for all day",
            "items": {"type": "string"},
            "type": "array",
        },
    });
    if let (Some(all), Value::Object(more)) = (properties.as_object_mut(), extra) {
        all.extend(more);
    }
    properties
}

/// Every tool, with its input schema.
pub fn tools() -> Vec<Value> {
    let id = |what: &str| string(&format!("The id of the {what}"));
    let mut all = vec![
        tool(
            "list_notes",
            "Lists Notes: the Unsorted ones, all, those in the trash or in one Area or Topic, or those whose text, Transcripts, Photo Texts, attached files' names or Attachment Texts hold a search. Each names its kind (plain, checklist, shoppingList, tally or session), the format of its text (textFormat: plain, markdown for formatted text, or code for a text shown as written in a fixed-width font), its open Frog and the Routines that run its Session.",
            true,
            json!({
                "place": string("Only the Notes in this Area or Topic (an Area's Topics included)"),
                "search": string("Only the Notes holding this text, ignoring case"),
                "which": {"enum": ["unsorted", "all", "trash"], "type": "string", "description": "Which Notes; all when left out"},
            }),
            &[],
        ),
        tool(
            "read_note",
            "Reads one Note in full: its text and its format (textFormat, as list_notes names it), Sections, Transcripts of its recordings, Photo Texts, photos, attached files (attachments: each file's hash, name, media type, size, pages when counted, and its Attachment Text, the text read out of it, cut after 4000 characters with its whole length in textLength; textState says read, empty when none was found, waiting while it is still to be read, fromComputer while the user's computer is still to read it, removed when the user removed it, or none for a kind of file without text; fileRemoved marks a file the user removed while its text stays), Capture Context, Boxes, Routines and Reminders with each Session's pool, count and picking rule, and its kind with its parts: for every kind but a plain Note its name (the text's first line, shown as its title) and description (the rest of the text, shown small under the name), a Tally's fields, a Session's setup (its pool, count, weekday counts and picking rule) with the Routines that run it, the Lines of a Checklist or Shopping List (on a Shopping List also those checked Once today, marked gone, as they stay struck through on the Agenda for the day), how a Shopping List or a Checklist without a Routine shows on the user's own Agenda (agenda: off, count or lines), and its Frogs. A Note copied from a Pack another person sent, kept with its \"From\" badge, names that person under from.",
            true,
            json!({"id": id("Note")}),
            &["id"],
        ),
        tool(
            "read_photo",
            "Reads one photo of a Note, scaled down, as a JPEG image.",
            true,
            json!({
                "note": id("Note"),
                "photo": string("The photo's hash, as read_note lists it"),
                "size": {"description": "The longest side in pixels, 1024 when left out, at most 2048", "type": "integer"},
            }),
            &["note", "photo"],
        ),
        tool(
            "read_attachment_text",
            "Reads the Attachment Text of one file attached to a Note, the text read out of it, from a character on: for a text read_note cuts. The file's bytes are never handed out.",
            true,
            json!({
                "file": string("The file's hash, as read_note lists it"),
                "from": {"description": "The character to start at, counted from 0; 0 when left out", "type": "integer"},
                "note": id("Note"),
                "size": {"description": "How many characters, 20000 when left out, at most 50000", "type": "integer"},
            }),
            &["note", "file"],
        ),
        tool(
            "list_places",
            "Lists the Areas in the person's own order, a setting of the person kept on all of their own Devices that cannot be changed here (Areas missing from it, such as new ones, come last by name), with their Topics, and whether each Area is shared and what this Device may do there.",
            true,
            json!({}),
            &[],
        ),
        tool(
            "list_tags",
            "Lists the tags written in the Notes, words with # in front such as #garten, nested by a / as in #life/health/back, by their levels: each with its name and whole tag as written most often, how many Notes hold it or a tag under it, and the tags under it. Only Notes neither in the trash nor merged count, with their Sections; a tag in code, in a link's address or in a Code text is none. list_notes with a tag as its search finds the Notes holding it or a tag under it.",
            true,
            json!({}),
            &[],
        ),
        tool(
            "find_notes",
            "Finds Notes by words, for asking about them (#267): each word is searched as list_notes searches (text, Sections, Transcripts, Photo Texts, attached files' names and Attachment Texts, Tally entries' lines and list Lines, ignoring case), and the Notes holding most of the words come first, then the newest. Each Note found names its id, title (its first line), capture time, Area or Topic, the words it holds and a short excerpt of its text; read_note reads it in full. Search again with other words, synonyms and spellings when little is found. Neither the trash nor merged Notes count.",
            true,
            json!({
                "limit": {"description": "How many Notes at most, 20 when left out, at most 50", "type": "integer"},
                "place": string("Only the Notes in this Area or Topic (an Area's Topics included)"),
                "words": {
                    "description": "The words or phrases to look for; a phrase is found as written",
                    "items": {"type": "string"},
                    "type": "array",
                },
            }),
            &["words"],
        ),
        tool(
            "list_new",
            "Lists what is new since a time (#267): the Notes captured since then and those given an Amend or a merged Note since then, newest first, each with its id, title (its first line), capture time, when it was last amended, its Area or Topic or that it is Unsorted, and a short excerpt of its text; read_note reads it in full. Neither the trash nor merged Notes count.",
            true,
            json!({
                "limit": {"description": "How many Notes at most, 50 when left out, at most 200", "type": "integer"},
                "since": string("From when: a day, YYYY-MM-DD, on this computer's clock, or a time, ISO 8601 such as 2026-11-07T09:30:00Z"),
            }),
            &["since"],
        ),
        tool(
            "list_routines",
            "Lists the Routines and Reminders, of one Note or all, each Session with its pool, count and picking rule; those in the trash on request.",
            true,
            json!({
                "note": string("Only those spawned from this Note"),
                "trash": {"description": "Those in the trash instead", "type": "boolean"},
            }),
            &[],
        ),
        tool(
            "read_settings",
            "Reads this Device's settings: the Re-nag interval, the Nudge's repeat, Quiet Hours, from when all-day Routines and Reminders remind (allDayFrom, HH:MM; before it they only show on the Agenda), whether alerts are on, whether the Agenda hides what is checked instead of striking it through (agendaHidesChecked), whether text, icons and tap targets show 1.3 times their size (fatFingers), whether Done on a one-time Reminder that is a plain Note's only Routine or Reminder moves the Note to the trash, a setting of the person kept on all of their own Devices (trashesDoneReminder), whether the person chose Simple or Advanced, a setting of the person too (features: simple or advanced; Simple only hides making Tallies, Sessions, Frogs and Topics, the format bar and Shared Area roles in the apps, while every set-up tool here stays available either way), what the quick note shows besides its text (quickNote: each control switched on, and whether the three Nudge buttons show instead of a single Save), the theme (system, light or dark), and who may connect to this port (aiReach: local for this computer only, docker for Docker containers on this computer). They cannot be changed here.",
            true,
            json!({}),
            &[],
        ),
        tool(
            "capture_note",
            "Captures a new Note, filed into an Area or Topic when one is given.",
            false,
            json!({"place": id("Area or Topic"), "text": string("The text")}),
            &["text"],
        ),
        tool(
            "edit_note",
            "Replaces the text of a Note.",
            false,
            json!({"id": id("Note"), "text": string("The new text")}),
            &["id", "text"],
        ),
        tool(
            "amend_note",
            "Adds a dated Section to a Note.",
            false,
            json!({"id": id("Note"), "text": string("The Section's text")}),
            &["id", "text"],
        ),
        tool(
            "edit_section",
            "Replaces the text of one Section of a Note, an Amend or a merged Note, by the id read_note lists for it. The Section keeps its place and its capture time; read_note shows when its text was last modified and by whom.",
            false,
            json!({"id": id("Section"), "text": string("The new text")}),
            &["id", "text"],
        ),
        tool(
            "file_note",
            "Files a Note into an Area or Topic, out of Unsorted or wherever it was.",
            false,
            json!({"id": id("Note"), "place": id("Area or Topic")}),
            &["id", "place"],
        ),
        tool(
            "merge_notes",
            "Merges one Note into another: it becomes a Section of the other and leaves every list.",
            false,
            json!({"id": id("Note to merge"), "into": id("Note it goes into")}),
            &["id", "into"],
        ),
        tool(
            "set_importance",
            "Sets how much a Note matters.",
            false,
            json!({
                "id": id("Note"),
                "importance": {"enum": ["unimportant", "normal", "important"], "type": "string"},
            }),
            &["id", "importance"],
        ),
        tool(
            "set_text_format",
            "Sets the format of a plain Note's text: plain, markdown (formatted text) or code (the whole text shown as written in a fixed-width font, unwrapped, nothing in it read as a format, a box or a tag). The text stays as written; a formatted text is never made plain, and a Note whose text is code becomes no other kind.",
            false,
            json!({
                "id": id("Note"),
                "format": {"enum": ["plain", "markdown", "code"], "type": "string"},
            }),
            &["id", "format"],
        ),
        tool(
            "remove_captured",
            "Removes one captured detail from the Capture Context of a Note or Section: where it was captured (place), the calendar events running (event), the Bluetooth devices connected (device) or the app it was shared in from (app). The capture time and the other details stay.",
            false,
            json!({
                "detail": {"enum": ["place", "event", "device", "app"], "type": "string"},
                "id": id("Note or Section"),
            }),
            &["id", "detail"],
        ),
        tool(
            "trash_note",
            "Moves a Note to the trash with the Routines and Reminders spawned from it; restoring it within 30 days brings them back.",
            false,
            json!({"id": id("Note")}),
            &["id"],
        ),
        tool(
            "restore_note",
            "Brings a Note back from the trash, with the Routines and Reminders that went with it.",
            false,
            json!({"id": id("Note")}),
            &["id"],
        ),
        tool(
            "add_area",
            "Adds an Area.",
            false,
            json!({"name": string("The name")}),
            &["name"],
        ),
        tool(
            "add_topic",
            "Adds a Topic inside an Area.",
            false,
            json!({"area": id("Area"), "name": string("The name")}),
            &["area", "name"],
        ),
        tool(
            "rename_place",
            "Renames an Area or Topic.",
            false,
            json!({"id": id("Area or Topic"), "name": string("The new name")}),
            &["id", "name"],
        ),
        tool(
            "set_area_colour",
            "Sets or clears the colour of an Area.",
            false,
            json!({"colour": {"description": "The colour as the app stores it, or null for none", "type": ["string", "null"]}, "id": id("Area")}),
            &["id", "colour"],
        ),
        tool(
            "set_area_icon",
            "Sets or clears the icon of an Area.",
            false,
            json!({"icon": {"description": "One of the app's Area icons, or null for none", "type": ["string", "null"]}, "id": id("Area")}),
            &["id", "icon"],
        ),
        tool(
            "delete_place",
            "Deletes an Area with its Topics, or a Topic; its Notes go back to Unsorted, into the trash, or (for a Topic) into its Area.",
            false,
            json!({
                "id": id("Area or Topic"),
                "notesGo": {"enum": ["unsorted", "trash", "area"], "type": "string"},
            }),
            &["id", "notesGo"],
        ),
        tool(
            "add_routine",
            "Adds a Routine, or a Reminder when it has no rule: spawned from a Note, or from a new Note with the text given.",
            false,
            schedule_properties(json!({
                "note": id("Note it is spawned from"),
                "text": string("The text of a new Note to spawn it from, when no note is given"),
            })),
            &["start", "times"],
        ),
        tool(
            "reschedule_routine",
            "Gives a Routine or Reminder a new schedule. What was Done stays Done.",
            false,
            schedule_properties(json!({"id": id("Routine")})),
            &["id", "start", "times"],
        ),
        tool(
            "set_renag",
            "Sets how often an ignored Occurrence of a Routine fires again, or back to the Device's default with null.",
            false,
            json!({"id": id("Routine"), "minutes": {"type": ["integer", "null"]}}),
            &["id", "minutes"],
        ),
        tool(
            "set_escalation",
            "Turns Escalation on after so many ignored Re-nags, or off with null.",
            false,
            json!({"after": {"type": ["integer", "null"]}, "id": id("Routine")}),
            &["id", "after"],
        ),
        tool(
            "trash_routine",
            "Moves a Routine or Reminder to the trash; its Note stays.",
            false,
            json!({"id": id("Routine")}),
            &["id"],
        ),
        tool(
            "restore_routine",
            "Brings a Routine or Reminder back from the trash.",
            false,
            json!({"id": id("Routine")}),
            &["id"],
        ),
    ];
    all.extend(set_up_tools());
    all.extend(data_tools());
    all
}

/// A Tally's field as a tool takes it.
fn field_properties() -> Value {
    json!({
        "check": {"description": "A Check field: a box per entry, a tick counting as 1, summed; it has no unit and no rule, and stays a Check field. A number field when left out", "type": "boolean"},
        "name": string("The field's name, such as Push-ups"),
        "rule": {"description": "How the sets of an entry and a bar with several entries count: their sum or the largest; by the kind of unit when left out", "enum": ["sum", "max"], "type": "string"},
        "target": {"description": "The target per bar of the Tally's charts (a day, week, month or year, as the Tally's bar says), in the field's unit; none when left out", "type": "number"},
        "unit": string("The unit: one the app knows (kg, g, km, m, cm, km/h, °C, °, h, min, s) or any word; none when left out"),
    })
}

/// A Tally's own settings (#264), as a tool takes them.
fn tally_properties() -> Value {
    json!({
        "bar": {"description": "How long a bar of its charts is; its targets count per bar. A day when never set", "enum": ["day", "week", "month", "year"], "type": "string"},
        "sets": {"description": "Whether it counts in sets: an entry then holds several sets, each a row across its number fields, such as 60 kg × 10 and then 65 kg × 8", "type": "boolean"},
    })
}

/// The day range of a tool that reads the user's own data.
fn range_properties(extra: Value) -> Value {
    let mut properties = json!({
        "from": string("Only from this day on, YYYY-MM-DD, on this computer's clock"),
        "until": string("Only up to this day, YYYY-MM-DD, that day included"),
    });
    if let (Some(all), Value::Object(more)) = (properties.as_object_mut(), extra) {
        all.extend(more);
    }
    properties
}

/// The tools that set up Tallies, Sessions, Checklists, Shopping Lists
/// and Frogs (#168). None writes an entry, a tick, a check-off, a pick or
/// an eaten Frog.
fn set_up_tools() -> Vec<Value> {
    let id = |what: &str| string(&format!("The id of the {what}"));
    let pool = json!({"description": "The Notes of the Tallies it picks from, in the order Rolling goes through them", "items": {"type": "string"}, "type": "array"});
    let picking = json!({"description": "How it picks: random, rolling (through the pool in its order) or least (the fewest entries first); rolling when left out", "enum": ["random", "rolling", "least"], "type": "string"});
    let count =
        json!({"description": "How many Tallies an Occurrence picks, 1 to 99", "type": "integer"});
    vec![
        tool(
            "make_tally",
            "Makes a Note a Tally with the fields given, which takes it out of Unsorted, with its bar length and whether it counts in sets when given. A Checklist or Shopping List is never a Tally.",
            false,
            {
                let mut properties = tally_properties();
                if let Some(all) = properties.as_object_mut() {
                    all.insert("fields".into(), json!({"description": "At least one field", "items": {"properties": field_properties(), "required": ["name"], "type": "object"}, "type": "array"}));
                    all.insert("id".into(), id("Note"));
                }
                properties
            },
            &["id", "fields"],
        ),
        tool(
            "set_tally",
            "Sets a Tally's bar length or whether it counts in sets; what is left out stays.",
            false,
            {
                let mut properties = tally_properties();
                if let Some(all) = properties.as_object_mut() {
                    all.insert("id".into(), id("Note"));
                }
                properties
            },
            &["id"],
        ),
        tool(
            "add_tally_field",
            "Adds a field to a Tally, a number or a Check field, or makes a Note a Tally with its first field.",
            false,
            {
                let mut properties = field_properties();
                if let Some(all) = properties.as_object_mut() {
                    all.insert("id".into(), id("Note"));
                }
                properties
            },
            &["id", "name"],
        ),
        tool(
            "rename_tally_field",
            "Renames a field of a Tally.",
            false,
            json!({"field": id("field"), "id": id("Note"), "name": string("The new name")}),
            &["id", "field", "name"],
        ),
        tool(
            "set_tally_field",
            "Sets the unit, the target or the rule of a field of a Tally; what is left out stays. A unit of another kind is refused while entries hold values for the field, and a Check field takes only a target.",
            false,
            json!({
                "field": id("field"),
                "id": id("Note"),
                "rule": {"enum": ["sum", "max"], "type": "string"},
                "target": {"description": "The target per bar of the Tally's charts in the field's unit, or null for none", "type": ["number", "null"]},
                "unit": string("The new unit"),
            }),
            &["id", "field"],
        ),
        tool(
            "remove_tally_field",
            "Removes a field from a Tally. The values its entries hold stay, and it can be brought back on the screen.",
            false,
            json!({"field": id("field"), "id": id("Note")}),
            &["id", "field"],
        ),
        tool(
            "make_session",
            "Sets up a Note as a Session, which picks Tallies from its pool for each Occurrence of every Routine and Reminder of the Note: a plain Note by its id, or a new Note with the text given. With a schedule (start and times) it also spawns a Routine that runs it; without one the Session runs on no Agenda until add_routine gives the Note one.",
            false,
            schedule_properties(json!({
                "count": count,
                "note": id("plain Note to set up"),
                "picking": picking,
                "pool": pool,
                "text": string("The text of a new Note to set up, when no note is given"),
            })),
            &["pool"],
        ),
        tool(
            "set_session",
            "Changes the setup of a Session's Note, for every Routine that runs it: its pool, count, picking rule or the count of single weekdays; what is left out stays. The schedules are its Routines', changed with reschedule_routine.",
            false,
            json!({
                "count": count,
                "id": id("Session's Note"),
                "picking": picking,
                "pool": pool,
                "weekdayCounts": {"additionalProperties": {"type": ["integer", "null"]}, "description": "The count of single weekdays, 1 for Monday to 7 for Sunday, such as {\"6\": 2}; null takes a weekday's own count away", "type": "object"},
            }),
            &["id"],
        ),
        tool(
            "make_list",
            "Makes a Note a Checklist or a Shopping List: its first line stays its text and every other line not blank becomes a Line, on a Checklist under the line above when it starts with a space. A text with a line marked \"✓ \" is refused, as that Line would be checked.",
            false,
            json!({"id": id("Note"), "list": {"enum": ["checklist", "shoppingList"], "type": "string"}}),
            &["id", "list"],
        ),
        tool(
            "list_to_plain",
            "Turns a Checklist or Shopping List back into a plain Note: every Line becomes a line of its text.",
            false,
            json!({"id": id("Note")}),
            &["id"],
        ),
        tool(
            "switch_list",
            "Makes a Checklist a Shopping List, or a Shopping List a Checklist: its Lines that show carry over in their order, at the top, each with the reappear time of a new Line; the text stays. A list with a checked or struck Line is refused, as that Line would be checked again.",
            false,
            json!({"id": id("Note")}),
            &["id"],
        ),
        tool(
            "add_line",
            "Adds a Line to a Checklist, at the end or at a place, or to the end of a Shopping List. On a Shopping List a name it holds already adds nothing; one that is checked is refused, as adding it would bring it back.",
            false,
            json!({
                "at": {"description": "Checklist only: its place among the Lines, from 0; at the end when left out", "type": "integer"},
                "id": id("Note"),
                "text": string("The Line's text"),
                "under": {"description": "Checklist only: in the group of the Line above", "type": "boolean"},
            }),
            &["id", "text"],
        ),
        tool(
            "rename_line",
            "Renames a Line of a Checklist or Shopping List.",
            false,
            json!({"id": id("Line"), "text": string("The new text")}),
            &["id", "text"],
        ),
        tool(
            "move_line",
            "Moves a Line of a Checklist, with its group, to just before another Line, or to the end. A single Line put before a Line of a group joins that group. On a Shopping List an open Line moves to just before another open Line, or after the open ones; a struck Line does not move.",
            false,
            json!({"before": string("The id of the Line it goes before; the end when left out"), "id": id("Line")}),
            &["id"],
        ),
        tool(
            "group_line",
            "Puts a Line of a Checklist into the group of the Line above it, or back to the top.",
            false,
            json!({"id": id("Line"), "under": {"description": "true for the group of the Line above, false for the top", "type": "boolean"}}),
            &["id", "under"],
        ),
        tool(
            "set_reappear",
            "Sets how long a Line of a Shopping List stays struck through once checked: none (it stays struck until a time is set; what a new Line has), once (gone once checked), or after a count of days, weeks, months or years (months and years by the calendar, to the last day of a shorter month).",
            false,
            json!({
                "id": id("Line"),
                "reappear": {"enum": ["none", "once", "after"], "type": "string"},
                "count": {"description": "for after: how many of the unit, 1 to 99", "maximum": Reappear::MAX_COUNT, "minimum": 1, "type": "integer"},
                "unit": {"description": "for after", "enum": REAPPEAR_UNITS, "type": "string"}
            }),
            &["id", "reappear"],
        ),
        tool(
            "remove_line",
            "Deletes a Line of a Checklist or Shopping List; on a Checklist its group comes up to the top. It can be restored on the screen.",
            false,
            json!({"id": id("Line")}),
            &["id"],
        ),
        tool(
            "set_shopping_agenda",
            "Sets how a Shopping List shows on the user's own Agenda, on their own Devices only: off (never), count (one row with the number of its open Lines) or lines (each open Line on a row of its own, and each Line checked today struck through on its row until the day ends, unless agendaHidesChecked). Otherwise a list with no open Line never shows.",
            false,
            json!({"agenda": {"enum": ["off", "count", "lines"], "type": "string"}, "id": id("Note")}),
            &["id", "agenda"],
        ),
        tool(
            "set_checklist_agenda",
            "Sets how a Checklist without a Routine shows on the user's own Agenda, on their own Devices only: off (never), count (one row with the number of its open Lines) or lines (its name with each open Line under it, the default, and each Line checked today struck through in its place until the day ends, unless agendaHidesChecked). Otherwise a list with no open Line never shows; a Checklist with a Routine shows as its Occurrence instead.",
            false,
            json!({"agenda": {"enum": ["off", "count", "lines"], "type": "string"}, "id": id("Note")}),
            &["id", "agenda"],
        ),
        tool(
            "make_frog",
            "Makes a Frog of a Note: a job with no time, done once, offered on the Agenda to eat one a day. A Note has one open Frog at a time.",
            false,
            json!({"id": id("Note")}),
            &["id"],
        ),
        tool(
            "frog_to_plain",
            "Turns a Note's open Frog back into a plain Note: the Frog goes to the trash, the Note stays.",
            false,
            json!({"id": id("Note")}),
            &["id"],
        ),
    ]
}

/// The tools that read the user's own data, each with its date: a
/// Tally's entries, the ticks of Lines and Boxes, the check-offs, the
/// eaten Frogs and a Session's picks.
fn data_tools() -> Vec<Value> {
    let id = |what: &str| string(&format!("The id of the {what}"));
    vec![
        tool(
            "read_tally_entries",
            "Reads the entries of a Tally, newest first, everyone's: when each counts for, what it counts for each field in the field's unit (a number field its rule over the entry's sets, a Check field 1 when ticked), its sets, each a row across the number fields, and its line of text.",
            true,
            range_properties(json!({"note": id("Note of the Tally")})),
            &["note"],
        ),
        tool(
            "read_ticks",
            "Reads the ticks of a Note, newest first: each time a Line of its Checklist or Shopping List was checked, for the list itself or for an Occurrence of a Routine spawned from it, and each tick of a Box in its text or its Sections' texts, with its state.",
            true,
            range_properties(json!({"note": id("Note")})),
            &["note"],
        ),
        tool(
            "read_check_offs",
            "Reads the check-offs of Routines and Reminders, of one, of those of a Note, or all: each Occurrence checked off, with its day and when it was checked off. The range goes by the Occurrence's day.",
            true,
            range_properties(
                json!({"note": string("Only those spawned from this Note"), "routine": string("Only this Routine")}),
            ),
            &[],
        ),
        tool(
            "read_skips",
            "Reads the skipped Occurrences of Routines and Reminders, of one, of those of a Note, or all: each Occurrence the user skipped (Not today), with its day, when it was skipped and, when they chose Tomorrow, the one-time Occurrence added the day after. Skipping is the user's own; it is read only. The range goes by the Occurrence's day.",
            true,
            range_properties(
                json!({"note": string("Only those spawned from this Note"), "routine": string("Only this Routine")}),
            ),
            &[],
        ),
        tool(
            "read_eaten_frogs",
            "Reads the Frogs eaten (checked off), most recently first, with when each was eaten and its Note.",
            true,
            range_properties(json!({})),
            &[],
        ),
        tool(
            "read_note_history",
            "Reads a Note's History, newest first: each change of its text, its settings (Importance, Area or Topic, kind, text format, trash), its photos and recordings, its Amends and the Notes merged into it, and its Lines, with what it was before and after, who made it and when.",
            true,
            range_properties(json!({"note": id("Note")})),
            &["note"],
        ),
        tool(
            "read_session_picks",
            "Reads a Session's picks, newest first: for each Occurrence the Tallies picked, the entries recorded for them and whether it was checked off. The range goes by the Occurrence's day.",
            true,
            range_properties(json!({"id": id("Session")})),
            &["id"],
        ),
    ]
}

/// The tools the Ask panel offers its model (#267, ADR 0051): finding
/// and reading Notes, what is new, the Areas, Topics, tags and Routines
/// with their check-offs, and filing what is new, which an AI Task
/// drafts. Not `list_notes`, which hands out every Note's text at once,
/// nor `read_photo`: only what the model finds and reads is sent.
pub const ASK_TOOLS: [&str; 12] = [
    "find_notes",
    "list_new",
    "read_note",
    "read_attachment_text",
    "list_places",
    "list_tags",
    "list_routines",
    "read_check_offs",
    "file_note",
    "add_area",
    "add_topic",
    "edit_note",
];

/// The tools of [`ASK_TOOLS`], with their input schemas, as [`tools`]
/// names them.
pub fn ask_tools() -> Vec<Value> {
    tools()
        .into_iter()
        .filter(|t| {
            t.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| ASK_TOOLS.contains(&n))
        })
        .collect()
}

/// Whether `name` is a tool of [`tools`].
pub fn is_tool(name: &str) -> bool {
    tools()
        .iter()
        .any(|t| t.get("name").and_then(Value::as_str) == Some(name))
}

/// Whether the tool `name` only reads.
fn reads_only(name: &str) -> bool {
    tools().iter().any(|t| {
        t.get("name").and_then(Value::as_str) == Some(name)
            && t.pointer("/annotations/readOnlyHint") == Some(&Value::Bool(true))
    })
}

fn text_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Invalid(format!("{key} is missing")))
}

fn optional_text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

/// A number or null under `key`, which must be given.
fn optional_number(args: &Value, key: &str) -> Result<Option<i64>> {
    match args.get(key) {
        Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| Error::Invalid(format!("{key} is no whole number"))),
        None => Err(Error::Invalid(format!("{key} is missing"))),
    }
}

fn schedule_arg(args: &Value) -> Result<Schedule> {
    let start = Day::parse(text_arg(args, "start")?)
        .ok_or_else(|| Error::Invalid("start is no day YYYY-MM-DD".into()))?;
    let times = args
        .get("times")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Invalid("times is missing".into()))?
        .iter()
        .map(|t| {
            t.as_str()
                .and_then(ClockTime::parse)
                .ok_or_else(|| Error::Invalid(format!("not a time HH:MM: {t}")))
        })
        .collect::<Result<Vec<_>>>()?;
    let rule = match optional_text(args, "rule") {
        Some(rule) => {
            Some(Recurrence::parse(rule).map_err(|e| Error::Invalid(format!("rule: {e}")))?)
        }
        None => None,
    };
    Schedule::new(start, &times, rule)
        .ok_or_else(|| Error::Invalid("that schedule never happens".into()))
}

/// The setup of a Session as a tool hands it out, with the Routines
/// that run it when `routines` holds them.
fn setup_json(setup: &SessionSetup, routines: Option<Vec<String>>) -> Value {
    let weekdays: BTreeMap<String, i64> = setup
        .counts
        .iter()
        .map(|(day, count)| (day.to_string(), *count))
        .collect();
    let mut out = json!({
        "count": setup.count,
        "picking": setup.rule.name(),
        "pool": setup.pool,
        "weekdayCounts": weekdays,
    });
    if let (Some(routines), Some(all)) = (routines, out.as_object_mut()) {
        all.insert("routines".into(), json!(routines));
    }
    out
}

/// An error when the text of the Note `note` is code (#213): it becomes
/// no other kind, as the editors offer none.
fn refuse_code(notes: &Notes, note: &str) -> Result<()> {
    match notes.by_id(note)? {
        Some(n) if n.text_format == text_format::CODE => Err(Error::Invalid(format!(
            "the text of {note} is code: set_text_format makes it plain first"
        ))),
        _ => Ok(()),
    }
}

fn note_summary(note: &Note, kind: &NoteKind) -> Value {
    json!({
        "captured": dart_iso(note.captured),
        "frog": kind.frog,
        "id": note.id,
        "importance": note.importance.name(),
        "kind": kind.kind,
        "mergedInto": note.merged_into,
        "place": note.place,
        "sessions": kind.sessions,
        "text": note.text,
        "textFormat": note.text_format,
        "trashed": note.is_trashed(),
        "unsorted": note.is_unsorted(),
    })
}

/// How many Notes `find_notes` and `list_new` give when not told, and at
/// most (#267).
const FIND_LIMIT: usize = 20;
const FIND_MAX: usize = 50;
const NEW_LIMIT: usize = 50;
const NEW_MAX: usize = 200;
/// The characters of a title and an excerpt the asking tools give.
const TITLE_CHARS: usize = 100;
const EXCERPT_CHARS: usize = 240;

/// The `limit` asked for, `default` when left out, from 1 to `most`.
fn limit_arg(args: &Value, default: usize, most: usize) -> usize {
    args.get("limit")
        .and_then(Value::as_u64)
        .map_or(default, |n| usize::try_from(n).unwrap_or(most))
        .clamp(1, most)
}

/// The text of `note` as it reads: a formatted text without its marks.
fn reads_as(note: &Note) -> String {
    if note.text_format == text_format::MARKDOWN {
        crate::markdown::plain(&note.text)
    } else {
        note.text.clone()
    }
}

/// The first `most` characters of `text`, "…" after them when cut.
fn cut_to(text: &str, most: usize) -> String {
    let mut out: String = text.chars().take(most).collect();
    if text.chars().nth(most).is_some() {
        out.push('…');
    }
    out
}

/// A Note as the asking tools list it (#267): its id, title, capture
/// time and where it is filed, without its text.
fn found_json(note: &Note) -> Value {
    let text = reads_as(note);
    let title = text.lines().map(str::trim).find(|l| !l.is_empty());
    json!({
        "captured": dart_iso(note.captured),
        "id": note.id,
        "place": note.place,
        "title": cut_to(title.unwrap_or_default(), TITLE_CHARS),
        "unsorted": note.is_unsorted(),
    })
}

/// A short part of the text of `note`: around the first place it holds
/// `word`, ignoring case, else its start; on one line.
fn excerpt(note: &Note, word: Option<&str>) -> String {
    let text: String = reads_as(note)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let at = word
        .map(|w| w.to_lowercase().chars().collect::<Vec<char>>())
        .filter(|w| !w.is_empty())
        .and_then(|w| lower.windows(w.len()).position(|part| part == w.as_slice()))
        .unwrap_or(0);
    let start = at.saturating_sub(EXCERPT_CHARS / 4);
    let part: String = chars.iter().skip(start).take(EXCERPT_CHARS).collect();
    let mut out = if start > 0 {
        format!("…{part}")
    } else {
        part
    };
    if start + EXCERPT_CHARS < chars.len() {
        out.push('…');
    }
    out
}

/// What a Note is besides its text: `plain`, `checklist`,
/// `shoppingList`, `tally` or `session` (a Note is one of them), its
/// open Frog, and the Routines that run its Session.
struct NoteKind {
    kind: &'static str,
    frog: Option<String>,
    sessions: Vec<String>,
}

/// The Tallies, open Frogs and Sessions, read once for many Notes.
struct Kinds {
    tallies: BTreeSet<String>,
    frogs: BTreeMap<String, String>,
    sessions: BTreeMap<String, Vec<String>>,
}

/// The list the Note `note` is by its kind: `checklist`, `shoppingList`,
/// or `None` for none.
fn list_of(store: &Store, note: &str) -> Result<Option<&'static str>> {
    let kind = store.current(note, checklist_keys::KIND)?;
    Ok(
        match parse_list_kind(kind.as_deref()).map(|(list, _)| list) {
            Some(checklist_keys::CHECKLIST) => Some(CHECKLIST),
            Some(shopping_keys::SHOPPING) => Some(SHOPPING_LIST),
            _ => None,
        },
    )
}

const CHECKLIST: &str = "checklist";
const SHOPPING_LIST: &str = "shoppingList";
const SESSION: &str = "session";
const PENDING: &str = "kept once the user accepts the AI Task";

/// The days an answer is narrowed to, both included.
struct Range {
    from: Option<Day>,
    until: Option<Day>,
}

impl Range {
    fn of(args: &Value) -> Result<Range> {
        let day = |key: &str| -> Result<Option<Day>> {
            match optional_text(args, key) {
                None => Ok(None),
                Some(text) => Day::parse(text)
                    .map(Some)
                    .ok_or_else(|| Error::Invalid(format!("{key} is no day YYYY-MM-DD"))),
            }
        };
        Ok(Range {
            from: day("from")?,
            until: day("until")?,
        })
    }

    fn holds(&self, day: Day) -> bool {
        self.from.is_none_or(|from| day >= from) && self.until.is_none_or(|until| day <= until)
    }
}

/// The strings under `key`, which must be given.
fn strings_arg(args: &Value, key: &str) -> Result<Vec<String>> {
    args.get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Invalid(format!("{key} is missing")))?
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| Error::Invalid(format!("{key} holds no text: {v}")))
        })
        .collect()
}

fn rule_arg(args: &Value) -> Result<Option<TallyRule>> {
    match optional_text(args, "rule") {
        None => Ok(None),
        Some(rule) => TallyRule::parse(rule)
            .map(Some)
            .ok_or_else(|| Error::Invalid(format!("no rule {rule}: sum or max"))),
    }
}

fn picking_arg(args: &Value) -> Result<Option<SessionRule>> {
    match optional_text(args, "picking") {
        None => Ok(None),
        Some(rule) => SessionRule::parse(rule)
            .map(Some)
            .ok_or_else(|| Error::Invalid(format!("no picking rule {rule}"))),
    }
}

/// The names of the units of a reappear time in the tools, in the order
/// of [`ReappearUnit::ALL`].
const REAPPEAR_UNITS: [&str; 4] = ["day", "week", "month", "year"];

fn unit_name(unit: ReappearUnit) -> &'static str {
    match unit {
        ReappearUnit::Day => REAPPEAR_UNITS[0],
        ReappearUnit::Week => REAPPEAR_UNITS[1],
        ReappearUnit::Month => REAPPEAR_UNITS[2],
        ReappearUnit::Year => REAPPEAR_UNITS[3],
    }
}

/// A reappear time as the tools read it (#195): its kind, `none`,
/// `once` or `after`, and for `after` its count and unit.
fn reappear_json(reappear: Reappear) -> Value {
    match reappear {
        Reappear::Unset => json!({"kind": "none"}),
        Reappear::Once => json!({"kind": "once"}),
        Reappear::After(count, unit) => {
            json!({"count": count, "kind": "after", "unit": unit_name(unit)})
        }
    }
}

/// The reappear time `set_reappear` asks for: `reappear` none, once or
/// after, the last with `count` from 1 to 99 and `unit`.
fn reappear_arg(args: &Value) -> Result<Reappear> {
    match text_arg(args, "reappear")? {
        "none" => Ok(Reappear::Unset),
        "once" => Ok(Reappear::Once),
        "after" => {
            let name = text_arg(args, "unit")?;
            let unit = ReappearUnit::ALL
                .into_iter()
                .find(|u| unit_name(*u) == name)
                .ok_or_else(|| Error::Invalid(format!("no unit {name}")))?;
            let count =
                count_arg(args)?.ok_or_else(|| Error::Invalid("count is missing".into()))?;
            u32::try_from(count)
                .ok()
                .and_then(|count| Reappear::after(count, unit))
                .ok_or_else(|| Error::Invalid(format!("count {count} is not from 1 to 99")))
        }
        other => Err(Error::Invalid(format!("no reappear time {other}"))),
    }
}

fn count_arg(args: &Value) -> Result<Option<i64>> {
    match args.get("count") {
        None => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| Error::Invalid("count is no whole number".into())),
    }
}

/// A Tally's fields, each target in the field's unit.
fn tally_json(tally: &Tally) -> Value {
    let fields: Vec<Value> = tally
        .fields
        .iter()
        .map(|f| {
            json!({
                "check": f.check,
                "countsSets": f.multiplies.is_some(),
                "hasEntries": f.has_entries,
                "id": f.id,
                "name": f.name,
                "removed": f.hidden,
                "rule": f.rule.name(),
                "target": f.target.map(|t| tally_units::from_base(t, &f.unit, UnitSystem::Metric)),
                "unit": f.unit,
            })
        })
        .collect();
    json!({"bar": tally.bar.name(), "fields": fields, "sets": tally.counts_in_sets})
}

/// The Boxes of a text, each with its state and its latest tick.
fn boxes_json(boxes: &[ShownBox]) -> Vec<Value> {
    boxes
        .iter()
        .map(|b| {
            json!({
                "id": b.id,
                "state": b.state.name(),
                "ticked": b.last.as_ref().map(|t| dart_iso(t.at)),
            })
        })
        .collect()
}

/// `found` newest first, as the values to hand out.
fn newest_first(mut found: Vec<(DateTime<Utc>, Value)>) -> Value {
    found.sort_by_key(|a| std::cmp::Reverse(a.0));
    Value::Array(found.into_iter().map(|(_, v)| v).collect())
}

/// The attached `files` and those `removed` while their texts stay, with
/// their Attachment Texts cut after [`ATTACHMENT_TEXT_CHARS`], as
/// `read_note` hands them out: never their bytes.
fn attachments_json(
    files: &[Attachment],
    removed: &[Attachment],
    texts: &BTreeMap<String, String>,
    removed_texts: &[String],
) -> Vec<Value> {
    files
        .iter()
        .map(|a| (a, false))
        .chain(removed.iter().map(|a| (a, true)))
        .map(|(a, gone)| {
            let text = texts.get(&a.hash);
            let state = match text {
                _ if removed_texts.contains(&a.hash) => "removed",
                Some(t) if t.is_empty() => "empty",
                Some(_) => "read",
                None if !a.kind().has_text() => "none",
                None if a.computer => "fromComputer",
                None => "waiting",
            };
            json!({
                "file": a.hash,
                "fileRemoved": gone,
                "name": a.name,
                "pages": a.pages,
                "size": a.size,
                "text": text.map(|t| t.chars().take(ATTACHMENT_TEXT_CHARS).collect::<String>()),
                "textLength": text.map(|t| t.chars().count()),
                "textState": state,
                "type": a.media_type,
            })
        })
        .collect()
}

fn texts_of(map: &std::collections::BTreeMap<String, String>, key: &str) -> Vec<Value> {
    map.iter()
        .map(|(hash, text)| json!({key: hash, "text": text}))
        .collect()
}

fn routine_json(routine: &Routine) -> Value {
    let schedule = &routine.schedule;
    json!({
        "escalation": routine.escalation,
        "id": routine.id,
        "note": routine.note_id,
        "reminder": routine.is_reminder(),
        "renagMinutes": routine.renag.map(|r| r.num_minutes()),
        "rule": schedule.rule.as_ref().map(Recurrence::format),
        "start": schedule.start.to_string(),
        "times": schedule.times.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "trashed": routine.is_trashed(),
    })
}

/// `bytes` decoded and scaled down to `edge` on its longest side, as JPEG.
fn scaled_jpeg(bytes: &[u8], edge: u32) -> Option<Vec<u8>> {
    let image = crate::profile::decode(bytes, PHOTO_READ_MAX)?;
    let small = image.thumbnail(edge, edge).to_rgb8();
    let mut out = Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80)
        .encode_image(&small)
        .ok()?;
    Some(out.into_inner())
}

impl Store {
    /// Answers the call of the tool `name` with `args` for the AI Task
    /// `task`. A change is drafted into the task; one that fails leaves
    /// it as it was, and says why in an output marked as an error. An
    /// error only for a tool there is none of.
    /// `wall` is this computer's clock, for the days of the user's data
    /// and for a Shopping List's struck Lines. It runs as the AI sees the
    /// store ([`Store::for_ai`]): what is Hidden from AI does not exist.
    pub fn ai_tool(
        &self,
        task: &str,
        name: &str,
        args: &Value,
        wall: &WallClock,
    ) -> Result<ToolOutput> {
        if !is_tool(name) {
            return Err(Error::Invalid(format!("no tool {name}")));
        }
        // What is Hidden from AI does not exist for it (#267, ADR 0049).
        let done = self.for_ai(|| {
            if reads_only(name) {
                self.ai_read(name, args, wall)
            } else {
                self.drafting(task, || self.ai_change(name, args, wall))
                    .map(|value| ToolOutput::json(&value))
            }
        });
        Ok(done.unwrap_or_else(|e| ToolOutput::failed(&e.to_string())))
    }

    /// Answers the call of the Ask panel's tool `name` ([`ASK_TOOLS`])
    /// with `args` (#267, ADR 0051), as the AI sees the store. A tool that
    /// reads runs in no AI Task, so asking does not pause sync; the first
    /// change opens the AI Task ([`Store::open_ai_task`]) into `task`,
    /// where it and every later one are drafted as [`Store::ai_tool`]
    /// drafts them. A tool the panel does not offer, or one that fails,
    /// says why in an output marked as an error.
    pub fn ask_tool(
        &self,
        name: &str,
        args: &Value,
        wall: &WallClock,
        task: &mut Option<String>,
    ) -> Result<ToolOutput> {
        if !ASK_TOOLS.contains(&name) {
            return Ok(ToolOutput::failed(&format!("no tool {name}")));
        }
        if reads_only(name) {
            let done = self.for_ai(|| self.ai_read(name, args, wall));
            return Ok(done.unwrap_or_else(|e| ToolOutput::failed(&e.to_string())));
        }
        let id = match task {
            Some(id) => id.clone(),
            None => {
                let id = self.open_ai_task()?;
                *task = Some(id.clone());
                id
            }
        };
        self.ai_tool(&id, name, args, wall)
    }

    /// The Tallies, open Frogs and Sessions, by their Notes.
    fn kinds(&self) -> Result<Kinds> {
        let tallies = Tallies::new(self)
            .all()?
            .into_iter()
            .map(|t| t.note_id)
            .collect();
        let frogs_of = Frogs::new(self);
        let mut frogs = BTreeMap::new();
        for frog in frogs_of.open()? {
            if let Some(note) = frogs_of.note(&frog)? {
                frogs.insert(note.id, frog.id);
            }
        }
        let mut sessions: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for session in Sessions::new(self).all()? {
            sessions
                .entry(session.routine.note_id.clone())
                .or_default()
                .push(session.routine.id);
        }
        Ok(Kinds {
            tallies,
            frogs,
            sessions,
        })
    }

    /// What the Note `note` is, by `kinds`.
    fn kind_of(&self, kinds: &Kinds, note: &Note) -> Result<NoteKind> {
        let tally = kinds.tallies.contains(&note.id)
            || (note.is_trashed() && Tallies::new(self).is_tally(&note.id)?);
        let kind = if tally {
            "tally"
        } else if Sessions::new(self).is_session_note(&note.id)? {
            SESSION
        } else {
            list_of(self, &note.id)?.unwrap_or("plain")
        };
        Ok(NoteKind {
            kind,
            frog: kinds.frogs.get(&note.id).cloned(),
            sessions: kinds.sessions.get(&note.id).cloned().unwrap_or_default(),
        })
    }

    /// A Routine or Reminder as a tool hands it out, a Session with its
    /// pool, count and picking rule.
    fn routine_value(&self, routine: &Routine) -> Result<Value> {
        let mut out = routine_json(routine);
        if let (Some(session), Some(all)) =
            (Sessions::new(self).by_id(&routine.id)?, out.as_object_mut())
        {
            let setup = self.seen_setup(&session.setup)?;
            all.insert("session".into(), setup_json(&setup, None));
        }
        Ok(out)
    }

    /// `setup` as the AI sees it: its pool without the Tallies Hidden
    /// from AI (#267).
    fn seen_setup(&self, setup: &SessionSetup) -> Result<SessionSetup> {
        let mut seen = setup.clone();
        seen.pool.clear();
        for id in &setup.pool {
            if !self.ai_hides(id)? {
                seen.pool.push(id.clone());
            }
        }
        Ok(seen)
    }

    fn routines_value(&self, routines: &[Routine]) -> Result<Value> {
        Ok(Value::Array(
            routines
                .iter()
                .map(|r| self.routine_value(r))
                .collect::<Result<_>>()?,
        ))
    }

    /// The Lines of the Note `note`, when it is a Checklist or Shopping
    /// List.
    fn lines_value(&self, note: &str, wall: &WallClock) -> Result<Value> {
        Ok(match list_of(self, note)? {
            Some(CHECKLIST) => match Checklists::new(self).by_note(note)? {
                Some(list) => Value::Array(
                    list.lines
                        .iter()
                        .map(|l| {
                            json!({
                                "checked": l.checked,
                                "checkedAt": l.checked_at.map(dart_iso),
                                "group": l.parent,
                                "id": l.id,
                                "text": l.text,
                            })
                        })
                        .collect(),
                ),
                None => Value::Null,
            },
            Some(_) => match ShoppingLists::new(self).by_note(note, wall)? {
                Some(list) => {
                    // A Line checked Once today stays struck on the Agenda
                    // for the day (#276), so it shows here too.
                    let today = Day::today(self.now(), wall);
                    let gone = list.gone().iter().filter(|l| l.checked_on(today, wall));
                    let shown = list.lines.iter().map(|l| (l, false));
                    Value::Array(
                        shown
                            .chain(gone.map(|l| (l, true)))
                            .map(|(l, gone)| {
                                json!({
                                    "backOn": l.back_on.map(|d| d.to_string()),
                                    "checkedAt": l.checked_at.map(dart_iso),
                                    "gone": gone,
                                    "id": l.id,
                                    "reappear": reappear_json(l.reappear),
                                    "struck": l.struck || gone,
                                    "text": l.text,
                                })
                            })
                            .collect(),
                    )
                }
                None => Value::Null,
            },
            None => Value::Null,
        })
    }

    fn ai_read(&self, name: &str, args: &Value, wall: &WallClock) -> Result<ToolOutput> {
        let notes = Notes::new(self);
        Ok(match name {
            "list_notes" => {
                let place = optional_text(args, "place");
                let found = match (optional_text(args, "search"), optional_text(args, "which")) {
                    (Some(query), _) => Search::new(self).notes(query, place)?,
                    (None, Some("trash")) => notes.trashed()?,
                    (None, Some("unsorted")) => notes.unsorted()?,
                    (None, _) => match place {
                        Some(place) => notes.inside(place)?,
                        None => notes.all()?,
                    },
                };
                let kinds = self.kinds()?;
                let mut out = Vec::new();
                for note in &found {
                    out.push(note_summary(note, &self.kind_of(&kinds, note)?));
                }
                ToolOutput::json(&Value::Array(out))
            }
            "read_note" => {
                let id = text_arg(args, "id")?;
                let note = notes
                    .by_id(id)?
                    .ok_or_else(|| Error::Invalid(format!("not a Note: {id}")))?;
                let fields = self.current_fields(id)?;
                let mut context = serde_json::Map::new();
                for key in CONTEXT_FIELDS {
                    if let Some(value) = fields.get(key) {
                        context.insert(key.to_string(), json!(value));
                    }
                }
                let sections: Vec<Value> = notes
                    .sections(id)?
                    .iter()
                    .map(|s| {
                        Ok(json!({
                            "captured": dart_iso(s.captured),
                            "editedBy": s.edited_by.as_ref().map(|p| p.name.clone()),
                            "id": s.id,
                            "boxes": boxes_json(&s.boxes),
                            "mayEdit": notes.may_edit_section(&s.id)?,
                            "modified": s.modified.map(dart_iso),
                            "attachments": attachments_json(
                                &s.attachments,
                                &s.removed_attachments,
                                &s.attachment_texts,
                                &s.removed_attachment_texts,
                            ),
                            "photoTexts": texts_of(&s.photo_texts, "photo"),
                            "photos": s.photos,
                            "text": s.text,
                            "transcripts": texts_of(&s.transcripts, "recording"),
                        }))
                    })
                    .collect::<Result<_>>()?;
                let routines = self.routines_value(&Routines::new(self).of_note(id)?)?;
                let frogs: Vec<Value> = Frogs::new(self)
                    .of_note(id)?
                    .iter()
                    .map(|f| {
                        json!({
                            "eaten": f.done.map(dart_iso),
                            "id": f.id,
                            "spawned": dart_iso(f.spawned),
                        })
                    })
                    .collect();
                let tally = Tallies::new(self).by_note(id)?.map(|t| tally_json(&t));
                let sessions = Sessions::new(self);
                let session = match sessions.setup_of(id)? {
                    Some(setup) => {
                        let runs = sessions.of_note(id)?;
                        let runs = runs.iter().map(|s| s.id().to_string()).collect();
                        setup_json(&self.seen_setup(&setup)?, Some(runs))
                    }
                    None => Value::Null,
                };
                let lines = self.lines_value(id, wall)?;
                let shopping = ShoppingLists::new(self);
                let checklists = Checklists::new(self);
                let agenda = if shopping.is_shopping_list(id)? {
                    Some(shopping.on_agenda(id)?.code())
                } else if checklists.may_show_on_agenda(id)? {
                    Some(checklists.on_agenda(id)?.code())
                } else {
                    None
                };
                let kind = self.kind_of(&self.kinds()?, &note)?;
                // The text of a Note of a kind is its name and its
                // description (#188); a plain Note's is its content.
                let (name, description) = match kind.kind {
                    "plain" => (None, None),
                    _ => (
                        Some(note_kinds::name(&note.text)),
                        Some(note_kinds::description(&note.text)),
                    ),
                };
                let mut out = note_summary(&note, &kind);
                if let Some(all) = out.as_object_mut() {
                    all.insert("agenda".into(), json!(agenda));
                    all.insert(
                        "attachments".into(),
                        json!(attachments_json(
                            &note.attachments,
                            &note.removed_attachments,
                            &note.attachment_texts,
                            &note.removed_attachment_texts,
                        )),
                    );
                    all.insert("name".into(), json!(name));
                    all.insert("description".into(), json!(description));
                    all.insert("boxes".into(), json!(boxes_json(&note.boxes)));
                    all.insert("captureContext".into(), Value::Object(context));
                    all.insert("frogs".into(), json!(frogs));
                    // The "From <name>" badge of a Note a Pack brought,
                    // only while it has one (#164).
                    if let Some(from) = self.pack_from(id)? {
                        all.insert("from".into(), json!(from.name));
                    }
                    all.insert("lines".into(), lines);
                    all.insert("mayChange".into(), json!(self.may_change(id)?));
                    all.insert(
                        "photoTexts".into(),
                        json!(texts_of(&note.photo_texts, "photo")),
                    );
                    all.insert("photos".into(), json!(note.photos));
                    all.insert("routines".into(), json!(routines));
                    all.insert("sections".into(), json!(sections));
                    all.insert("tally".into(), json!(tally));
                    all.insert("session".into(), session);
                    all.insert(
                        "transcripts".into(),
                        json!(texts_of(&note.transcripts, "recording")),
                    );
                }
                ToolOutput::json(&out)
            }
            "read_photo" => {
                let id = text_arg(args, "note")?;
                let hash = text_arg(args, "photo")?;
                let held = notes
                    .by_id(id)?
                    .is_some_and(|n| n.photos.iter().any(|p| p == hash))
                    || notes
                        .sections(id)?
                        .iter()
                        .any(|s| s.photos.iter().any(|p| p == hash));
                if !held {
                    return Err(Error::Invalid(format!("no photo {hash} on {id}")));
                }
                let edge = args
                    .get("size")
                    .and_then(Value::as_u64)
                    .map_or(PHOTO_EDGE, |s| u32::try_from(s).unwrap_or(PHOTO_EDGE_MAX))
                    .clamp(64, PHOTO_EDGE_MAX);
                let bytes = self
                    .photo(hash)
                    .ok_or_else(|| Error::Invalid(format!("the photo {hash} is not here")))?;
                let jpeg = scaled_jpeg(&bytes, edge)
                    .ok_or_else(|| Error::Invalid(format!("the photo {hash} cannot be read")))?;
                ToolOutput {
                    content: vec![ToolContent::Jpeg(jpeg)],
                    is_error: false,
                }
            }
            "read_attachment_text" => {
                let id = text_arg(args, "note")?;
                let hash = text_arg(args, "file")?;
                let mut text = notes
                    .by_id(id)?
                    .and_then(|n| n.attachment_texts.get(hash).cloned());
                if text.is_none() {
                    text = notes
                        .sections(id)?
                        .into_iter()
                        .find_map(|s| s.attachment_texts.get(hash).cloned());
                }
                let text = text
                    .ok_or_else(|| Error::Invalid(format!("no Attachment Text {hash} on {id}")))?;
                let count = |key: &str, default: usize, most: usize| {
                    args.get(key)
                        .and_then(Value::as_u64)
                        .map_or(default, |n| usize::try_from(n).unwrap_or(most))
                        .min(most)
                };
                let total = text.chars().count();
                let from = count("from", 0, total);
                let size = count("size", ATTACHMENT_READ_CHARS, ATTACHMENT_READ_MAX);
                let part: String = text.chars().skip(from).take(size).collect();
                ToolOutput::json(&json!({
                    "file": hash,
                    "from": from,
                    "text": part,
                    "total": total,
                }))
            }
            "list_places" => {
                let places = Places::new(self);
                let mut out = Vec::new();
                for area in PersonSettings::new(self).areas_in_order()? {
                    let topics: Vec<Value> = places
                        .topics(&area.id)?
                        .iter()
                        .map(|t| json!({"id": t.id, "name": t.name}))
                        .collect();
                    out.push(json!({
                        "access": self.access_in(&area.id, None)?.name(),
                        "colour": area.colour,
                        "icon": area.icon,
                        "id": area.id,
                        "name": area.name,
                        "shared": self.is_shared(&area.id)?,
                        "topics": topics,
                    }));
                }
                ToolOutput::json(&Value::Array(out))
            }
            "list_tags" => {
                fn node(n: &TagNode) -> Value {
                    json!({
                        "children": n.children.iter().map(node).collect::<Vec<_>>(),
                        "name": n.name,
                        "notes": n.notes,
                        "tag": n.tag,
                    })
                }
                // Area Locks are the phone's: nothing is closed here.
                let tree = TagTree::new(self).tree(&HashSet::new())?;
                ToolOutput::json(&Value::Array(tree.iter().map(node).collect()))
            }
            "find_notes" => {
                let words: Vec<String> = strings_arg(args, "words")?
                    .into_iter()
                    .map(|w| w.trim().to_string())
                    .filter(|w| !w.is_empty())
                    .collect();
                if words.is_empty() {
                    return Err(Error::Invalid("words holds no word".into()));
                }
                let place = optional_text(args, "place");
                let limit = limit_arg(args, FIND_LIMIT, FIND_MAX);
                let search = Search::new(self);
                // By id: the Note and the words it holds, in the order
                // asked.
                let mut found: BTreeMap<String, (Note, Vec<&str>)> = BTreeMap::new();
                for word in &words {
                    for note in search.notes(word, place)? {
                        found
                            .entry(note.id.clone())
                            .or_insert_with(|| (note, Vec::new()))
                            .1
                            .push(word);
                    }
                }
                let total = found.len();
                let mut hits: Vec<(Note, Vec<&str>)> = found.into_values().collect();
                hits.sort_by(|(a, wa), (b, wb)| {
                    wb.len()
                        .cmp(&wa.len())
                        .then_with(|| b.captured.cmp(&a.captured))
                        .then_with(|| a.id.cmp(&b.id))
                });
                let notes: Vec<Value> = hits
                    .iter()
                    .take(limit)
                    .map(|(note, held)| {
                        let mut out = found_json(note);
                        if let Some(all) = out.as_object_mut() {
                            all.insert(
                                "excerpt".into(),
                                json!(excerpt(note, held.first().copied())),
                            );
                            all.insert("words".into(), json!(held));
                        }
                        out
                    })
                    .collect();
                ToolOutput::json(&json!({"found": total, "notes": notes}))
            }
            "list_new" => {
                let since = text_arg(args, "since")?;
                let limit = limit_arg(args, NEW_LIMIT, NEW_MAX);
                // A day counts on this computer's clock, from its start.
                let after = |at: DateTime<Utc>| -> Result<bool> {
                    match Day::parse(since) {
                        Some(day) => Ok(Day::of(wall(at)) >= day),
                        None => Ok(at
                            >= parse_time(since).map_err(|_| {
                                Error::Invalid(
                                    "since is no day YYYY-MM-DD nor ISO 8601 time".into(),
                                )
                            })?),
                    }
                };
                let all = notes.all()?;
                let ids: Vec<String> = all.iter().map(|n| n.id.clone()).collect();
                let sections = notes.sections_of(&ids)?;
                let mut hits = Vec::new();
                for note in all {
                    let amended = sections
                        .get(&note.id)
                        .into_iter()
                        .flatten()
                        .map(|s| s.captured)
                        .max();
                    let amended = match amended {
                        Some(at) if after(at)? => Some(at),
                        _ => None,
                    };
                    if after(note.captured)? || amended.is_some() {
                        let newest = amended.map_or(note.captured, |at| at.max(note.captured));
                        hits.push((newest, note, amended));
                    }
                }
                hits.sort_by(|(a, na, _), (b, nb, _)| b.cmp(a).then_with(|| na.id.cmp(&nb.id)));
                let total = hits.len();
                let notes: Vec<Value> = hits
                    .iter()
                    .take(limit)
                    .map(|(_, note, amended)| {
                        let mut out = found_json(note);
                        if let Some(all) = out.as_object_mut() {
                            all.insert("amended".into(), json!(amended.map(dart_iso)));
                            all.insert("excerpt".into(), json!(excerpt(note, None)));
                        }
                        out
                    })
                    .collect();
                ToolOutput::json(&json!({"found": total, "notes": notes}))
            }
            "list_routines" => {
                let routines = Routines::new(self);
                let trash = args.get("trash").and_then(Value::as_bool) == Some(true);
                let found = match (optional_text(args, "note"), trash) {
                    (_, true) => routines.trashed()?,
                    (Some(note), false) => routines.of_note(note)?,
                    (None, false) => routines.all()?,
                };
                ToolOutput::json(&self.routines_value(&found)?)
            }
            "read_settings" => {
                let settings = DeviceSettings::new(self);
                let quiet = settings.quiet_hours()?;
                let shows = |control| settings.quick_note_shows(control);
                ToolOutput::json(&json!({
                    "agendaHidesChecked": settings.agenda_hides_checked()?,
                    "features": if PersonSettings::new(self).advanced()? {
                        "advanced"
                    } else {
                        "simple"
                    },
                    "aiReach": self.ai_reach()?.name(),
                    "alertsOn": settings.alerts_on()?,
                    "allDayFrom": settings.all_day_from()?.to_string(),
                    "fatFingers": settings.fat_fingers()?,
                    "nudgeRepeatMinutes": settings.nudge_repeat()?.num_minutes(),
                    "quickNote": {
                        "fileInto": shows(QuickNoteControl::FileInto)?,
                        "formatBar": shows(QuickNoteControl::FormatBar)?,
                        "importance": shows(QuickNoteControl::Importance)?,
                        "nudgeButtons": settings.quick_note_nudge_buttons_on()?,
                        "schedule": shows(QuickNoteControl::Schedule)?,
                        "shopping": shows(QuickNoteControl::Shopping)?,
                    },
                    "quietHours": {
                        "from": quiet.start.to_string(),
                        "on": settings.quiet_hours_on()?,
                        "until": quiet.end.to_string(),
                    },
                    "renagMinutes": settings.renag_interval()?.num_minutes(),
                    "theme": settings.theme()?.name(),
                    "trashesDoneReminder": PersonSettings::new(self).trashes_done_reminder()?,
                }))
            }
            other => ToolOutput::json(&self.ai_read_data(other, args, wall)?),
        })
    }

    /// The user's own data, each with its date, narrowed to the days of
    /// `from` and `until`: read only.
    fn ai_read_data(&self, name: &str, args: &Value, wall: &WallClock) -> Result<Value> {
        let range = Range::of(args)?;
        let day = |at: DateTime<Utc>| Day::today(at, wall);
        let mut found: Vec<(DateTime<Utc>, Value)> = Vec::new();
        match name {
            "read_tally_entries" => {
                let note = text_arg(args, "note")?;
                let tallies = Tallies::new(self);
                let tally = tallies
                    .by_note(note)?
                    .ok_or_else(|| Error::Invalid(format!("not a Tally: {note}")))?;
                for entry in tallies.entries(note, &PersonFilter::default())? {
                    if !range.holds(day(entry.at)) {
                        continue;
                    }
                    let shown = |base: f64, unit: &str| {
                        tally_units::from_base(base, unit, UnitSystem::Metric)
                    };
                    let values: Vec<Value> = tally
                        .fields
                        .iter()
                        .filter_map(|f| {
                            let base = tally.counted(&entry, &f.id)?;
                            Some(json!({
                                "field": f.id,
                                "name": f.name,
                                "unit": f.unit,
                                "value": shown(base, &f.unit),
                            }))
                        })
                        .collect();
                    let sets: Vec<Value> = tally
                        .sets_of(&entry)
                        .iter()
                        .map(|set| {
                            let row: serde_json::Map<String, Value> = tally
                                .fields
                                .iter()
                                .filter_map(|f| {
                                    let base = *set.get(&f.id)?;
                                    Some((f.id.clone(), json!(shown(base, &f.unit))))
                                })
                                .collect();
                            Value::Object(row)
                        })
                        .collect();
                    let value = json!({
                        "at": dart_iso(entry.at),
                        "day": day(entry.at).to_string(),
                        "id": entry.id,
                        "line": entry.line,
                        "sets": sets,
                        "values": values,
                    });
                    found.push((entry.at, value));
                }
            }
            "read_ticks" => {
                let note = text_arg(args, "note")?;
                let lines: Vec<(String, String)> = match list_of(self, note)? {
                    Some(CHECKLIST) => Checklists::new(self)
                        .by_note(note)?
                        .map(|l| l.lines.into_iter().map(|l| (l.id, l.text)).collect())
                        .unwrap_or_default(),
                    Some(_) => ShoppingLists::new(self)
                        .by_note(note, wall)?
                        .map(|l| {
                            l.lines
                                .iter()
                                .chain(l.gone())
                                .map(|l| (l.id.clone(), l.text.clone()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    None => Vec::new(),
                };
                for (line, text) in &lines {
                    for e in self.history(line)? {
                        let at = e.value.as_deref().and_then(|v| parse_time(v).ok());
                        let Some(at) = at.filter(|_| e.field == checklist_keys::CHECKED) else {
                            continue;
                        };
                        if range.holds(day(at)) {
                            let value = json!({"at": dart_iso(at), "day": day(at).to_string(), "line": line, "text": text});
                            found.push((at, value));
                        }
                    }
                }
                for routine in Routines::new(self).of_note(note)? {
                    let checks = LineChecks::of(self, &routine.id)?;
                    for (key, by_line) in &checks.by_occurrence {
                        for (line, e) in by_line {
                            let Some(at) = e.value.as_deref().and_then(|v| parse_time(v).ok())
                            else {
                                continue;
                            };
                            if !range.holds(day(at)) {
                                continue;
                            }
                            let text = lines.iter().find(|(id, _)| id == line).map(|(_, t)| t);
                            let value = json!({
                                "at": dart_iso(at),
                                "day": day(at).to_string(),
                                "line": line,
                                "occurrence": key,
                                "routine": routine.id,
                                "text": text,
                            });
                            found.push((at, value));
                        }
                    }
                }
                let mut texts = vec![note.to_string()];
                texts.extend(Notes::new(self).sections(note)?.into_iter().map(|s| s.id));
                let boxes = Boxes::new(self);
                for entity in &texts {
                    for shown in boxes.of(entity)? {
                        for tick in boxes.history(entity, &shown.id)? {
                            if !range.holds(day(tick.at)) {
                                continue;
                            }
                            let value = json!({
                                "at": dart_iso(tick.at),
                                "box": shown.id,
                                "day": day(tick.at).to_string(),
                                "in": entity,
                                "state": tick.state.name(),
                            });
                            found.push((tick.at, value));
                        }
                    }
                }
            }
            "read_check_offs" => {
                let routines = Routines::new(self);
                let all = match (optional_text(args, "routine"), optional_text(args, "note")) {
                    (Some(id), _) => routines.by_id(id)?.into_iter().collect(),
                    (None, Some(note)) => routines.of_note(note)?,
                    (None, None) => routines.all()?,
                };
                for routine in all {
                    for (key, at) in &routine.done {
                        let Some(occurrence) = Occurrence::parse(key) else {
                            continue;
                        };
                        if !range.holds(occurrence.day) {
                            continue;
                        }
                        let value = json!({
                            "at": dart_iso(*at),
                            "day": occurrence.day.to_string(),
                            "note": routine.note_id,
                            "occurrence": key,
                            "routine": routine.id,
                        });
                        found.push((*at, value));
                    }
                }
            }
            "read_skips" => {
                let routines = Routines::new(self);
                let all = match (optional_text(args, "routine"), optional_text(args, "note")) {
                    (Some(id), _) => routines.by_id(id)?.into_iter().collect(),
                    (None, Some(note)) => routines.of_note(note)?,
                    (None, None) => routines.all()?,
                };
                for routine in all {
                    for (key, at) in &routine.skipped {
                        let Some(occurrence) = Occurrence::parse(key) else {
                            continue;
                        };
                        // Done wins over a skip (ADR 0006).
                        if !range.holds(occurrence.day) || routine.done_at(&occurrence).is_some() {
                            continue;
                        }
                        let tomorrow = Occurrence::new(occurrence.day.add_days(1), occurrence.time);
                        let value = json!({
                            "at": dart_iso(*at),
                            "day": occurrence.day.to_string(),
                            "note": routine.note_id,
                            "occurrence": key,
                            "routine": routine.id,
                            "tomorrow": routine.is_moved(&tomorrow).then(|| tomorrow.key()),
                        });
                        found.push((*at, value));
                    }
                }
            }
            "read_eaten_frogs" => {
                let frogs = Frogs::new(self);
                for frog in frogs.done()? {
                    let Some(at) = frog.done.filter(|at| range.holds(day(*at))) else {
                        continue;
                    };
                    let note = frogs.note(&frog)?;
                    let value = json!({
                        "at": dart_iso(at),
                        "day": day(at).to_string(),
                        "id": frog.id,
                        "note": note.as_ref().map(|n| n.id.clone()),
                        "text": note.map(|n| n.text),
                    });
                    found.push((at, value));
                }
            }
            "read_note_history" => {
                let note = text_arg(args, "note")?;
                if Notes::new(self).by_id(note)?.is_none() {
                    return Err(Error::Invalid(format!("not a Note: {note}")));
                }
                // An Area, Topic or Note Hidden from AI it was filed in
                // or merged into reads as none (#267).
                let seen = |value: Option<String>| -> Result<Option<String>> {
                    match value {
                        Some(v) if self.ai_hides(&v)? => Ok(None),
                        other => Ok(other),
                    }
                };
                for c in NoteHistory::new(self).of(note)? {
                    if !range.holds(day(c.at)) || self.ai_hides(&c.entity)? {
                        continue;
                    }
                    let value = json!({
                        "after": seen(c.after)?,
                        "at": dart_iso(c.at),
                        "before": seen(c.before)?,
                        "by": c.by.as_ref().map(|p| p.name.clone()),
                        "change": c.kind.name(),
                        "day": day(c.at).to_string(),
                        "in": c.entity,
                    });
                    found.push((c.at, value));
                }
            }
            "read_session_picks" => {
                let id = text_arg(args, "id")?;
                let session = Sessions::new(self)
                    .by_id(id)?
                    .ok_or_else(|| Error::Invalid(format!("not a Session: {id}")))?;
                let marks = SessionMarks::of(self, id)?;
                let mut picked: Vec<(String, Value)> = Vec::new();
                for (key, picks) in &marks.picks {
                    let Some(occurrence) = Occurrence::parse(key) else {
                        continue;
                    };
                    if !range.holds(occurrence.day) {
                        continue;
                    }
                    let mut tallies = Vec::new();
                    for note in picks {
                        // A pick of a Tally Hidden from AI is not there.
                        if self.ai_hides(note)? {
                            continue;
                        }
                        let entries: Vec<Value> = marks
                            .entries_for(self, key, note)?
                            .iter()
                            .map(|e| json!({"at": dart_iso(e.at), "id": e.id}))
                            .collect();
                        tallies.push(json!({"entries": entries, "note": note}));
                    }
                    let value = json!({
                        "checkedOff": session.routine.done.get(key).map(|at| dart_iso(*at)),
                        "day": occurrence.day.to_string(),
                        "occurrence": key,
                        "picks": tallies,
                    });
                    picked.push((key.clone(), value));
                }
                picked.sort_by(|a, b| b.0.cmp(&a.0));
                return Ok(Value::Array(picked.into_iter().map(|(_, v)| v).collect()));
            }
            _ => return Err(Error::Invalid(format!("no tool {name}"))),
        }
        Ok(newest_first(found))
    }

    /// One change, inside [`Store::drafting`]; what it made, by id.
    fn ai_change(&self, name: &str, args: &Value, wall: &WallClock) -> Result<Value> {
        let notes = Notes::new(self);
        let places = Places::new(self);
        let routines = Routines::new(self);
        let id = || text_arg(args, "id");
        let made = match name {
            "capture_note" => {
                notes
                    .capture(text_arg(args, "text")?, optional_text(args, "place"), None)?
                    .id
            }
            "edit_note" => {
                notes.edit(id()?, text_arg(args, "text")?)?;
                id()?.to_string()
            }
            "amend_note" => notes.amend(id()?, text_arg(args, "text")?)?.id,
            "edit_section" => {
                notes.edit_section(id()?, text_arg(args, "text")?)?;
                id()?.to_string()
            }
            "file_note" => {
                notes.file(id()?, text_arg(args, "place")?)?;
                id()?.to_string()
            }
            "merge_notes" => {
                notes.merge(id()?, text_arg(args, "into")?)?;
                id()?.to_string()
            }
            "set_importance" => {
                let importance = match text_arg(args, "importance")? {
                    "unimportant" => Importance::Unimportant,
                    "normal" => Importance::Normal,
                    "important" => Importance::Important,
                    other => return Err(Error::Invalid(format!("no Importance {other}"))),
                };
                notes.set_importance(id()?, importance)?;
                id()?.to_string()
            }
            "set_text_format" => {
                let format = text_arg(args, "format")?;
                let format = text_format::KNOWN
                    .into_iter()
                    .find(|f| *f == format)
                    .ok_or_else(|| Error::Invalid(format!("no text format {format}")))?;
                let note = notes
                    .by_id(id()?)?
                    .ok_or_else(|| Error::Invalid(format!("not a Note: {}", id().unwrap_or(""))))?;
                // As the editor's badge: a plain Note's text only (#213).
                if self.kind_of(&self.kinds()?, &note)?.kind != "plain" {
                    return Err(Error::Invalid(format!("not a plain Note: {}", note.id)));
                }
                notes.set_format(&note.id, format)?;
                note.id
            }
            "remove_captured" => {
                let detail = text_arg(args, "detail")?;
                let detail = CapturedDetail::parse(detail)
                    .ok_or_else(|| Error::Invalid(format!("no captured detail {detail}")))?;
                notes.remove_captured(id()?, detail)?;
                id()?.to_string()
            }
            "trash_note" => {
                routines.trash_note(id()?)?;
                id()?.to_string()
            }
            "restore_note" => {
                routines.restore_note(id()?)?;
                id()?.to_string()
            }
            "add_area" => places.add_area(text_arg(args, "name")?)?.id,
            "add_topic" => {
                places
                    .add_topic(text_arg(args, "area")?, text_arg(args, "name")?)?
                    .id
            }
            "rename_place" => {
                places.rename(id()?, text_arg(args, "name")?)?;
                id()?.to_string()
            }
            "set_area_colour" => {
                places.set_colour(id()?, optional_text(args, "colour"))?;
                id()?.to_string()
            }
            "set_area_icon" => {
                places.set_icon(id()?, optional_text(args, "icon"))?;
                id()?.to_string()
            }
            "delete_place" => {
                let go = match text_arg(args, "notesGo")? {
                    "unsorted" => NotesGo::Unsorted,
                    "trash" => NotesGo::Trash,
                    "area" => NotesGo::Area,
                    other => return Err(Error::Invalid(format!("notesGo cannot be {other}"))),
                };
                let place = id()?;
                let topic = places.topic_by_id(place)?.is_some();
                // A Note Hidden from AI would be left behind in a deleted
                // place or moved unseen (#267): the user deletes it.
                if (topic || places.area_by_id(place)?.is_some())
                    && self.holds_hidden_from_ai(place)?
                {
                    return Err(Error::Invalid(format!(
                        "{place} can only be deleted in the app"
                    )));
                }
                if topic {
                    places.delete_topic(place, go)?;
                } else {
                    places.delete_area(place, go)?;
                }
                place.to_string()
            }
            "add_routine" => {
                let schedule = schedule_arg(args)?;
                match (optional_text(args, "note"), optional_text(args, "text")) {
                    (Some(note), _) => routines.spawn(note, &schedule)?.id,
                    (None, Some(text)) => routines.create(text, &schedule)?.id,
                    (None, None) => {
                        return Err(Error::Invalid("a note or a text is needed".into()));
                    }
                }
            }
            "reschedule_routine" => {
                routines.reschedule(id()?, &schedule_arg(args)?)?;
                id()?.to_string()
            }
            "set_renag" => {
                let minutes = optional_number(args, "minutes")?;
                routines.set_renag(id()?, minutes.map(Duration::minutes))?;
                id()?.to_string()
            }
            "set_escalation" => {
                routines.set_escalation(id()?, optional_number(args, "after")?)?;
                id()?.to_string()
            }
            "trash_routine" => {
                routines.trash(id()?)?;
                id()?.to_string()
            }
            "restore_routine" => {
                routines.restore(id()?)?;
                id()?.to_string()
            }
            other => return self.ai_set_up(other, args, wall),
        };
        Ok(json!({"id": made, "pending": PENDING}))
    }

    /// One set-up of a Tally, Session, Checklist, Shopping List or Frog,
    /// inside [`Store::drafting`]; what it made, by id.
    fn ai_set_up(&self, name: &str, args: &Value, wall: &WallClock) -> Result<Value> {
        let tallies = Tallies::new(self);
        let sessions = Sessions::new(self);
        let checklists = Checklists::new(self);
        let shopping = ShoppingLists::new(self);
        let frogs = Frogs::new(self);
        let id = || text_arg(args, "id");
        let field = || text_arg(args, "field");
        let done = |id: &str| json!({"id": id, "pending": PENDING});
        Ok(match name {
            "make_tally" => {
                let note = id()?;
                refuse_code(&Notes::new(self), note)?;
                if tallies.is_tally(note)? {
                    return Err(Error::Invalid(format!(
                        "{note} is a Tally already: add_tally_field adds a field"
                    )));
                }
                let fields = args
                    .get("fields")
                    .and_then(Value::as_array)
                    .filter(|f| !f.is_empty())
                    .ok_or_else(|| Error::Invalid("fields needs at least one field".into()))?;
                let made = fields
                    .iter()
                    .map(|f| self.ai_add_field(note, f))
                    .collect::<Result<Vec<_>>>()?;
                self.ai_set_tally(note, args)?;
                json!({"fields": made, "id": note, "pending": PENDING})
            }
            "add_tally_field" => {
                let note = id()?;
                let made = self.ai_add_field(note, args)?;
                json!({"field": made, "id": note, "pending": PENDING})
            }
            "rename_tally_field" => {
                tallies.rename_field(id()?, field()?, text_arg(args, "name")?)?;
                done(id()?)
            }
            "set_tally_field" => {
                let (note, field) = (id()?, field()?);
                let mut set = false;
                if let Some(unit) = optional_text(args, "unit") {
                    tallies.set_unit(note, field, unit)?;
                    set = true;
                }
                if let Some(target) = args.get("target") {
                    let unit = tallies
                        .by_note(note)?
                        .and_then(|t| t.field(field).map(|f| f.unit.clone()))
                        .ok_or_else(|| Error::Invalid(format!("not a field of {note}: {field}")))?;
                    let base = match target {
                        Value::Null => None,
                        v => Some(tally_units::to_base(
                            v.as_f64()
                                .ok_or_else(|| Error::Invalid("target is no number".into()))?,
                            &unit,
                            UnitSystem::Metric,
                        )),
                    };
                    tallies.set_target(note, field, base)?;
                    set = true;
                }
                if let Some(rule) = rule_arg(args)? {
                    tallies.set_rule(note, field, rule)?;
                    set = true;
                }
                if !set {
                    return Err(Error::Invalid(
                        "nothing to set: give a unit, a target or a rule".into(),
                    ));
                }
                done(note)
            }
            "set_tally" => {
                let note = id()?;
                if !tallies.is_tally(note)? {
                    return Err(Error::Invalid(format!("not a Tally: {note}")));
                }
                if !self.ai_set_tally(note, args)? {
                    return Err(Error::Invalid(
                        "nothing to set: give a bar or whether it counts in sets".into(),
                    ));
                }
                done(note)
            }
            "remove_tally_field" => {
                tallies.hide_field(id()?, field()?)?;
                done(id()?)
            }
            "make_session" => {
                let pool = strings_arg(args, "pool")?;
                let count = count_arg(args)?.unwrap_or(1);
                let rule = picking_arg(args)?.unwrap_or(SessionRule::Rolling);
                let scheduled = ["start", "times", "rule"]
                    .iter()
                    .any(|key| args.get(key).is_some());
                let schedule = match scheduled {
                    true => Some(schedule_arg(args)?),
                    false => None,
                };
                let note = optional_text(args, "note");
                let text = optional_text(args, "text");
                let (note, routine) = match (note, text, &schedule) {
                    (Some(note), _, Some(schedule)) => {
                        refuse_code(&Notes::new(self), note)?;
                        let made = sessions.spawn(note, schedule, &pool, count, rule)?;
                        (made.setup.note_id, Some(made.routine.id))
                    }
                    (Some(note), _, None) => {
                        refuse_code(&Notes::new(self), note)?;
                        sessions.make(note, &pool, count, rule)?;
                        (note.to_string(), None)
                    }
                    (None, Some(text), Some(schedule)) => {
                        let made = sessions.create(text, schedule, &pool, count, rule)?;
                        (made.setup.note_id, Some(made.routine.id))
                    }
                    (None, Some(text), None) => {
                        let text = text.trim();
                        if text.is_empty() {
                            return Err(Error::Invalid("the text must not be empty".into()));
                        }
                        let note = Notes::new(self).capture(text, None, None)?.id;
                        sessions.make(&note, &pool, count, rule)?;
                        (note, None)
                    }
                    (None, None, _) => {
                        return Err(Error::Invalid("a note or a text is needed".into()));
                    }
                };
                json!({"id": note, "pending": PENDING, "routine": routine})
            }
            "set_session" => {
                let note = id()?;
                let mut set = false;
                if args.get("pool").is_some() {
                    let mut pool = strings_arg(args, "pool")?;
                    // The Tallies Hidden from AI it held stay, unseen (#267).
                    if let Some(setup) = sessions.setup_of(note)? {
                        for id in self.hidden_of(&setup.pool)? {
                            if !pool.contains(&id) {
                                pool.push(id);
                            }
                        }
                    }
                    sessions.set_pool(note, &pool)?;
                    set = true;
                }
                if let Some(count) = count_arg(args)? {
                    sessions.set_count(note, count)?;
                    set = true;
                }
                if let Some(rule) = picking_arg(args)? {
                    sessions.set_rule(note, rule)?;
                    set = true;
                }
                if let Some(days) = args.get("weekdayCounts") {
                    let days = days
                        .as_object()
                        .ok_or_else(|| Error::Invalid("weekdayCounts is no object".into()))?;
                    for (weekday, count) in days {
                        let weekday: u32 = weekday
                            .parse()
                            .map_err(|_| Error::Invalid(format!("no weekday {weekday}")))?;
                        let count = match count {
                            Value::Null => None,
                            v => Some(v.as_i64().ok_or_else(|| {
                                Error::Invalid(format!("the count of {weekday} is no whole number"))
                            })?),
                        };
                        sessions.set_count_on(note, weekday, count)?;
                        set = true;
                    }
                }
                if !set {
                    return Err(Error::Invalid(
                        "nothing to set: give a pool, a count, a picking rule or weekdayCounts"
                            .into(),
                    ));
                }
                done(note)
            }
            "make_list" => {
                let note = id()?;
                let text = Notes::new(self).by_id(note)?.map(|n| {
                    let markdown = n.text_format == text_format::MARKDOWN;
                    split_lines(&n.text)
                        .iter()
                        .skip(1)
                        .filter_map(|r| parse_line_in(r, markdown))
                        .any(|l| l.checked)
                });
                if text == Some(true) {
                    return Err(Error::Invalid(format!(
                        "the text of {note} has lines marked \"✓ \", which would be checked Lines, and an AI Task checks nothing: take the marks out with edit_note first, or leave this to the user"
                    )));
                }
                match text_arg(args, "list")? {
                    CHECKLIST => checklists.make(note)?,
                    SHOPPING_LIST => shopping.make(note)?,
                    other => return Err(Error::Invalid(format!("no list {other}"))),
                }
                done(note)
            }
            "list_to_plain" => {
                let note = id()?;
                match list_of(self, note)? {
                    Some(CHECKLIST) => checklists.to_plain(note)?,
                    Some(_) => shopping.to_plain(note, wall)?,
                    None => {
                        return Err(Error::Invalid(format!(
                            "neither a Checklist nor a Shopping List: {note}"
                        )));
                    }
                }
                done(note)
            }
            "switch_list" => {
                let note = id()?;
                let ticked = match list_of(self, note)? {
                    Some(CHECKLIST) => checklists
                        .by_note(note)?
                        .is_some_and(|l| l.lines.iter().any(|l| l.checked)),
                    Some(_) => shopping
                        .by_note(note, wall)?
                        .is_some_and(|l| l.lines.iter().any(|l| l.struck)),
                    None => {
                        return Err(Error::Invalid(format!(
                            "neither a Checklist nor a Shopping List: {note}"
                        )));
                    }
                };
                if ticked {
                    return Err(Error::Invalid(format!(
                        "{note} has checked Lines, which would be checked again in the other list, and an AI Task checks nothing: leave this to the user"
                    )));
                }
                shopping.switch_list(note, wall)?;
                done(note)
            }
            "add_line" => {
                let (note, text) = (id()?, text_arg(args, "text")?);
                let line = match list_of(self, note)? {
                    Some(CHECKLIST) => {
                        let at = args
                            .get("at")
                            .and_then(Value::as_u64)
                            .and_then(|at| usize::try_from(at).ok());
                        let under = args.get("under").and_then(Value::as_bool) == Some(true);
                        checklists.add(note, text, at, under)?
                    }
                    Some(_) => {
                        let wanted = ShoppingLists::name_key(text);
                        let checked = shopping.by_note(note, wall)?.is_some_and(|list| {
                            list.struck()
                                .into_iter()
                                .chain(list.gone())
                                .any(|l| ShoppingLists::name_key(&l.text) == wanted)
                        });
                        if checked {
                            return Err(Error::Invalid(format!(
                                "{note} holds a checked Line of that name, and adding it would bring it back, which only the user does"
                            )));
                        }
                        shopping.add(note, text, wall)?
                    }
                    None => {
                        return Err(Error::Invalid(format!(
                            "neither a Checklist nor a Shopping List: {note}"
                        )));
                    }
                };
                json!({"id": note, "line": line, "pending": PENDING})
            }
            "rename_line" => {
                let (line, text) = (id()?, text_arg(args, "text")?);
                match self.list_of_line(line)? {
                    CHECKLIST => checklists.rename(line, text)?,
                    _ => shopping.rename(line, text)?,
                }
                done(line)
            }
            "move_line" => {
                let line = id()?;
                let before = optional_text(args, "before");
                match self.list_of_line(line)? {
                    CHECKLIST => checklists.move_line(line, before)?,
                    _ => shopping.move_line(line, wall, before)?,
                }
                done(line)
            }
            "group_line" => {
                let line = self.checklist_line(id()?)?;
                let under = args
                    .get("under")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| Error::Invalid("under is missing".into()))?;
                checklists.indent(line, under)?;
                done(line)
            }
            "set_reappear" => {
                let line = id()?;
                if self.list_of_line(line)? != SHOPPING_LIST {
                    return Err(Error::Invalid(format!(
                        "not a Line of a Shopping List: {line}"
                    )));
                }
                shopping.set_reappear(line, reappear_arg(args)?)?;
                done(line)
            }
            "remove_line" => {
                let line = id()?;
                match self.list_of_line(line)? {
                    CHECKLIST => checklists.delete(line)?,
                    _ => shopping.delete(line)?,
                }
                done(line)
            }
            "set_shopping_agenda" => {
                let note = id()?;
                let code = text_arg(args, "agenda")?;
                let shows = ShoppingOnAgenda::parse(code)
                    .ok_or_else(|| Error::Invalid(format!("no way to show {code}")))?;
                shopping.set_on_agenda(note, shows)?;
                done(note)
            }
            "set_checklist_agenda" => {
                let note = id()?;
                let code = text_arg(args, "agenda")?;
                let shows = ListOnAgenda::parse(code)
                    .ok_or_else(|| Error::Invalid(format!("no way to show {code}")))?;
                if !checklists.may_show_on_agenda(note)? {
                    return Err(Error::Invalid(format!(
                        "not a Checklist without a Routine: {note}"
                    )));
                }
                checklists.set_on_agenda(note, shows)?;
                done(note)
            }
            "make_frog" => {
                let note = id()?;
                let frog = frogs.spawn(note)?;
                json!({"frog": frog.id, "id": note, "pending": PENDING})
            }
            "frog_to_plain" => {
                let note = id()?;
                let open: Vec<String> = frogs
                    .of_note(note)?
                    .into_iter()
                    .filter(|f| !f.is_done())
                    .map(|f| f.id)
                    .collect();
                if open.is_empty() {
                    return Err(Error::Invalid(format!("no open Frog of {note}")));
                }
                for frog in &open {
                    frogs.trash(frog)?;
                }
                done(note)
            }
            _ => return Err(Error::Invalid(format!("no tool {name}"))),
        })
    }

    /// Adds the field `field` asks for to the Note `note`, its target
    /// given in its unit; the field's id.
    fn ai_add_field(&self, note: &str, field: &Value) -> Result<String> {
        let name = text_arg(field, "name")?;
        let unit = optional_text(field, "unit").unwrap_or("").trim();
        let target = match field.get("target") {
            None | Some(Value::Null) => None,
            Some(v) => Some(tally_units::to_base(
                v.as_f64()
                    .ok_or_else(|| Error::Invalid("target is no number".into()))?,
                unit,
                UnitSystem::Metric,
            )),
        };
        let tallies = Tallies::new(self);
        match field.get("check") {
            None | Some(Value::Bool(false)) => {
                tallies.add_field(note, name, unit, target, rule_arg(field)?)
            }
            Some(Value::Bool(true)) => tallies.add_check(note, name, target),
            Some(_) => Err(Error::Invalid("check is no true or false".into())),
        }
    }

    /// Sets the bar length and whether it counts in sets of the Tally of
    /// `note` as `args` give them; whether they gave any.
    fn ai_set_tally(&self, note: &str, args: &Value) -> Result<bool> {
        let tallies = Tallies::new(self);
        let mut set = false;
        if let Some(bar) = args.get("bar") {
            let bar = bar
                .as_str()
                .and_then(TallyPeriod::parse)
                .ok_or_else(|| Error::Invalid("bar is no day, week, month or year".into()))?;
            tallies.set_bar(note, bar)?;
            set = true;
        }
        if let Some(sets) = args.get("sets") {
            let on = sets
                .as_bool()
                .ok_or_else(|| Error::Invalid("sets is no true or false".into()))?;
            tallies.set_counts_in_sets(note, on)?;
            set = true;
        }
        Ok(set)
    }

    /// The list the Line `line` belongs to: [`CHECKLIST`] or
    /// [`SHOPPING_LIST`].
    fn list_of_line(&self, line: &str) -> Result<&'static str> {
        let note = self
            .current(line, checklist_keys::NOTE)?
            .filter(|_| line.starts_with(checklist_keys::LINE_PREFIX))
            .ok_or_else(|| Error::Invalid(format!("not a Line: {line}")))?;
        list_of(self, &note)?.ok_or_else(|| Error::Invalid(format!("not a Line of a list: {line}")))
    }

    /// `line`, when it is a Line of a Checklist.
    fn checklist_line<'a>(&self, line: &'a str) -> Result<&'a str> {
        if self.list_of_line(line)? != CHECKLIST {
            return Err(Error::Invalid(format!("not a Line of a Checklist: {line}")));
        }
        Ok(line)
    }
}

#[cfg(test)]
mod tests;
