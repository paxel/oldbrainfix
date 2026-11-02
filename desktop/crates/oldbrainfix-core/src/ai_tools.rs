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
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};

use crate::boxes::Boxes;
use crate::checklists::{
    Checklists, LineChecks, keys as checklist_keys, parse_line_in, parse_list_kind, split_lines,
};
use crate::entry::{dart_iso, parse_time};
use crate::frogs::Frogs;
use crate::note_kinds;
use crate::notes::{CapturedDetail, Importance, Note, Notes};
use crate::people::PersonFilter;
use crate::places::{NotesGo, Places};
use crate::routines::{Routine, Routines};
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule, WallClock};
use crate::search::Search;
use crate::sessions::{SessionMarks, SessionRule, SessionSetup, Sessions};
use crate::settings::{DeviceSettings, QuickNoteControl};
use crate::shopping_lists::{
    ListOnAgenda, Reappear, ShoppingLists, ShoppingOnAgenda, keys as shopping_keys,
};
use crate::store::Store;
use crate::tallies::{Tallies, Tally};
use crate::tally_units::{self, TallyRule, UnitSystem};
use crate::text_boxes::ShownBox;
use crate::text_format;
use crate::{Error, Result};

/// The longest side of a photo handed out, unless asked for less.
pub const PHOTO_EDGE: u32 = 1024;
/// The longest side a photo may be asked for at most.
pub const PHOTO_EDGE_MAX: u32 = 2048;
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
            "Lists Notes: the Unsorted ones, all, those in the trash or in one Area or Topic, or those whose text, Transcripts or Photo Texts hold a search. Each names its kind (plain, checklist, shoppingList, tally or session), its open Frog and the Routines that run its Session.",
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
            "Reads one Note in full: its text, Sections, Transcripts of its recordings, Photo Texts, photos, Capture Context, Boxes, Routines and Reminders with each Session's pool, count and picking rule, and its kind with its parts: for every kind but a plain Note its name (the text's first line, shown as its title) and description (the rest of the text, shown small under the name), a Tally's fields, a Session's setup (its pool, count, weekday counts and picking rule) with the Routines that run it, the Lines of a Checklist or Shopping List, how a Shopping List or a Checklist without a Routine shows on the user's own Agenda (agenda: off, count or lines), and its Frogs.",
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
            "list_places",
            "Lists the Areas with their Topics, and whether each Area is shared and what this Device may do there.",
            true,
            json!({}),
            &[],
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
            "Reads this Device's settings: the Re-nag interval, the Nudge's repeat, Quiet Hours, from when all-day Routines and Reminders remind (allDayFrom, HH:MM; before it they only show on the Agenda), whether alerts are on, what the quick note shows besides its text (quickNote: each control switched on, and whether the three Nudge buttons show instead of a single Save), the theme (system, light or dark), and who may connect to this port (aiReach: local for this computer only, docker for Docker containers on this computer). They cannot be changed here.",
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
            "Moves a Note to the trash, with the Routines and Reminders spawned from it unless told otherwise; it can be restored for 30 days.",
            false,
            json!({
                "id": id("Note"),
                "withRoutines": {"description": "Its Routines and Reminders too; true when left out", "type": "boolean"},
            }),
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
        "name": string("The field's name, such as Push-ups"),
        "rule": {"description": "How a day with several entries counts: their sum or the largest; by the kind of unit when left out", "enum": ["sum", "max"], "type": "string"},
        "target": {"description": "The target per day, in the field's unit; none when left out", "type": "number"},
        "unit": string("The unit: one the app knows (kg, g, km, m, cm, km/h, °C, °, h, min, s) or any word; none when left out"),
    })
}

/// What a Tally's field multiplies (#200), as a tool takes it.
fn multiplies_property() -> Value {
    json!({"description": "The id of another field of the same Tally this field multiplies, such as Sets multiplying Reps, or null for none: not itself, not a field that multiplies, and not while another field multiplies this one", "type": ["string", "null"]})
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
            "Makes a Note a Tally with the fields given, which takes it out of Unsorted. A Checklist or Shopping List is never a Tally.",
            false,
            json!({
                "fields": {"description": "At least one field", "items": {"properties": field_properties(), "required": ["name"], "type": "object"}, "type": "array"},
                "id": id("Note"),
            }),
            &["id", "fields"],
        ),
        tool(
            "add_tally_field",
            "Adds a field to a Tally, or makes a Note a Tally with its first field. With multiplies it is a multiplier of that other field, such as Sets of Reps: an entry then counts that field as the multiplier's value times its own, and the multiplier counts as no value of its own.",
            false,
            {
                let mut properties = field_properties();
                if let Some(all) = properties.as_object_mut() {
                    all.insert("id".into(), id("Note"));
                    all.insert("multiplies".into(), multiplies_property());
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
            "Sets the unit, the target, the rule or what a field of a Tally multiplies; what is left out stays. A unit of another kind is refused while entries hold values for the field.",
            false,
            json!({
                "field": id("field"),
                "id": id("Note"),
                "multiplies": multiplies_property(),
                "rule": {"enum": ["sum", "max"], "type": "string"},
                "target": {"description": "The target per day in the field's unit, or null for none", "type": ["number", "null"]},
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
            "Moves a Line of a Checklist, with its group, to just before another Line, or to the end. A single Line put before a Line of a group joins that group.",
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
            "Sets how long a Line of a Shopping List stays struck through once checked: once (gone), or 1 to 4 weeks.",
            false,
            json!({"id": id("Line"), "reappear": {"enum": ["once", "1w", "2w", "3w", "4w"], "type": "string"}}),
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
            "Sets how a Shopping List shows on the user's own Agenda, on their own Devices only: off (never), count (one row with the number of its open Lines) or lines (each open Line on a row of its own). A list with no open Line never shows.",
            false,
            json!({"agenda": {"enum": ["off", "count", "lines"], "type": "string"}, "id": id("Note")}),
            &["id", "agenda"],
        ),
        tool(
            "set_checklist_agenda",
            "Sets how a Checklist without a Routine shows on the user's own Agenda, on their own Devices only: off (never), count (one row with the number of its open Lines) or lines (its name with each open Line under it, the default). A list with no open Line never shows; a Checklist with a Routine shows as its Occurrence instead.",
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
            "Reads the entries of a Tally, newest first, everyone's: when each counts for, its values in each field's unit and its line of text. A field another one multiplies shows what it counts, the product, with the multipliers' values as its factors; a multiplier shows no value of its own.",
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
            "read_eaten_frogs",
            "Reads the Frogs eaten (checked off), most recently first, with when each was eaten and its Note.",
            true,
            range_properties(json!({})),
            &[],
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
        "trashed": note.is_trashed(),
        "unsorted": note.is_unsorted(),
    })
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

/// The field `multiplies` names, or `None` for null.
fn multiplies_arg(other: &Value) -> Result<Option<&str>> {
    match other {
        Value::Null => Ok(None),
        Value::String(id) => Ok(Some(id.as_str())),
        _ => Err(Error::Invalid("multiplies is no field id".into())),
    }
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
                "hasEntries": f.has_entries,
                "id": f.id,
                "multiplies": f.multiplies,
                "name": f.name,
                "removed": f.hidden,
                "rule": f.rule.name(),
                "target": f.target.map(|t| tally_units::from_base(t, &f.unit, UnitSystem::Metric)),
                "unit": f.unit,
            })
        })
        .collect();
    json!({"fields": fields})
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
    /// and for a Shopping List's struck Lines.
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
        let done = if reads_only(name) {
            self.ai_read(name, args, wall)
        } else {
            self.drafting(task, || self.ai_change(name, args, wall))
                .map(|value| ToolOutput::json(&value))
        };
        Ok(done.unwrap_or_else(|e| ToolOutput::failed(&e.to_string())))
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
            all.insert("session".into(), setup_json(&session.setup, None));
        }
        Ok(out)
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
                Some(list) => Value::Array(
                    list.lines
                        .iter()
                        .map(|l| {
                            json!({
                                "backOn": l.back_on.map(|d| d.to_string()),
                                "checkedAt": l.checked_at.map(dart_iso),
                                "id": l.id,
                                "reappear": l.reappear.code(),
                                "struck": l.struck,
                                "text": l.text,
                            })
                        })
                        .collect(),
                ),
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
                        setup_json(&setup, Some(runs))
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
                    all.insert("name".into(), json!(name));
                    all.insert("description".into(), json!(description));
                    all.insert("boxes".into(), json!(boxes_json(&note.boxes)));
                    all.insert("captureContext".into(), Value::Object(context));
                    all.insert("frogs".into(), json!(frogs));
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
            "list_places" => {
                let places = Places::new(self);
                let mut out = Vec::new();
                for area in places.areas()? {
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
                    "aiReach": self.ai_reach()?.name(),
                    "alertsOn": settings.alerts_on()?,
                    "allDayFrom": settings.all_day_from()?.to_string(),
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
                            let mut value = json!({
                                "field": f.id,
                                "name": f.name,
                                "unit": f.unit,
                                "value": shown(base, &f.unit),
                            });
                            let factors: Vec<Value> = tally
                                .multipliers_of(&f.id)
                                .into_iter()
                                .filter_map(|m| {
                                    let factor = *entry.values.get(&m.id)?;
                                    Some(json!({
                                        "field": m.id,
                                        "name": m.name,
                                        "value": shown(factor, &m.unit),
                                    }))
                                })
                                .collect();
                            if let (false, Some(all)) = (factors.is_empty(), value.as_object_mut())
                            {
                                all.insert("factors".into(), json!(factors));
                            }
                            Some(value)
                        })
                        .collect();
                    let value = json!({
                        "at": dart_iso(entry.at),
                        "day": day(entry.at).to_string(),
                        "id": entry.id,
                        "line": entry.line,
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
            "remove_captured" => {
                let detail = text_arg(args, "detail")?;
                let detail = CapturedDetail::parse(detail)
                    .ok_or_else(|| Error::Invalid(format!("no captured detail {detail}")))?;
                notes.remove_captured(id()?, detail)?;
                id()?.to_string()
            }
            "trash_note" => {
                let with = args.get("withRoutines").and_then(Value::as_bool) != Some(false);
                routines.trash_note(id()?, with)?;
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
                if places.topic_by_id(place)?.is_some() {
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
                if let Some(other) = args.get("multiplies") {
                    tallies.set_multiplies(note, field, multiplies_arg(other)?)?;
                    set = true;
                }
                if !set {
                    return Err(Error::Invalid(
                        "nothing to set: give a unit, a target, a rule or what it multiplies"
                            .into(),
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
                        let made = sessions.spawn(note, schedule, &pool, count, rule)?;
                        (made.setup.note_id, Some(made.routine.id))
                    }
                    (Some(note), _, None) => {
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
                    sessions.set_pool(note, &strings_arg(args, "pool")?)?;
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
                let line = self.checklist_line(id()?)?;
                checklists.move_line(line, optional_text(args, "before"))?;
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
                let code = text_arg(args, "reappear")?;
                let reappear = Reappear::ALL
                    .into_iter()
                    .find(|r| r.code() == code)
                    .ok_or_else(|| Error::Invalid(format!("no reappear time {code}")))?;
                shopping.set_reappear(line, reappear)?;
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
        let multiplies = match field.get("multiplies") {
            Some(other) => multiplies_arg(other)?,
            None => None,
        };
        Tallies::new(self).add_field_multiplying(
            note,
            name,
            unit,
            target,
            rule_arg(field)?,
            multiplies,
        )
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
mod tests {
    use super::*;
    use crate::store::tests::store;
    use chrono::TimeZone;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    /// The tool `name` called with `args` for `task`, on a clock in UTC.
    fn call(store: &Store, task: &str, name: &str, args: &Value) -> Result<ToolOutput> {
        store.ai_tool(task, name, args, &|t: DateTime<Utc>| t.naive_utc())
    }

    fn png(width: u32, height: u32) -> std::result::Result<Vec<u8>, image::ImageError> {
        let mut out = Cursor::new(Vec::new());
        image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 40]))
            .write_to(&mut out, image::ImageFormat::Png)?;
        Ok(out.into_inner())
    }

    #[test]
    fn a_photo_is_handed_out_scaled_down_and_only_from_its_note() -> TestResult {
        let store = store();
        let notes = Notes::new(&store);
        let receipt = notes.capture_with("Receipt", None, None, &[png(400, 200)?], &[])?;
        let other = notes.capture("Other", None, None)?;
        let task = store.open_ai_task()?;
        let hash = receipt.photos.first().ok_or("a photo")?;

        let out = call(
            &store,
            &task,
            "read_photo",
            &json!({"note": receipt.id, "photo": hash, "size": 100}),
        )?;
        let [ToolContent::Jpeg(jpeg)] = &out.content[..] else {
            return Err(format!("no JPEG: {out:?}").into());
        };
        let image = image::load_from_memory(jpeg)?;
        assert_eq!((image.width(), image.height()), (100, 50));

        let elsewhere = call(
            &store,
            &task,
            "read_photo",
            &json!({"note": other.id, "photo": hash}),
        )?;
        assert!(elsewhere.is_error);
        assert!(call(&store, &task, "send_note", &json!({})).is_err());
        Ok(())
    }

    #[test]
    fn an_area_is_deleted_with_its_topics_and_its_notes_go_back_to_unsorted() -> TestResult {
        let store = store();
        let places = Places::new(&store);
        let house = places.add_area("Household")?;
        let kitchen = places.add_topic(&house.id, "Kitchen")?;
        let milk = Notes::new(&store).capture("Milk", Some(&kitchen.id), None)?;
        let task = store.open_ai_task()?;

        let out = call(
            &store,
            &task,
            "delete_place",
            &json!({"id": house.id, "notesGo": "unsorted"}),
        )?;

        assert!(!out.is_error, "{out:?}");
        assert!(places.areas()?.is_empty());
        assert!(
            Notes::new(&store)
                .by_id(&milk.id)?
                .is_some_and(|n| n.is_unsorted())
        );
        assert_eq!(store.log_only(|| places.areas())?.len(), 1);
        let review = store.ai_review(&task)?;
        let gone: Vec<(String, String, Option<String>)> = review
            .groups
            .iter()
            .filter(|(kind, _)| *kind == crate::ai_task::AiChangeKind::Place)
            .flat_map(|(_, items)| items.iter())
            .flat_map(|i| {
                i.changes
                    .iter()
                    .map(|c| (i.entity.clone(), c.field.clone(), c.after.clone()))
            })
            .collect();
        assert_eq!(
            gone,
            [
                (kitchen.id.clone(), "$type".to_string(), None),
                (house.id.clone(), "$type".to_string(), None),
            ]
        );
        assert!(review.groups.iter().any(|(kind, items)| {
            *kind == crate::ai_task::AiChangeKind::NoteFiled && items[0].entity == milk.id
        }));
        Ok(())
    }

    #[test]
    fn a_captured_detail_is_removed_as_a_pending_change_and_no_longer_read() -> TestResult {
        let store = store();
        let toner = Notes::new(&store).capture("Order toner", None, None)?.id;
        for (field, value) in [
            ("location", "52.52,13.405"),
            ("sourcePackage", "org.example.chat"),
            ("sourceLabel", "Chat"),
            ("calendar", r#"["Katzenklo"]"#),
        ] {
            store.append(&toner, field, Some(value), None)?;
        }
        let task = store.open_ai_task()?;
        let logged = store.all_entries()?.len();
        for detail in ["place", "app"] {
            let out = call(
                &store,
                &task,
                "remove_captured",
                &json!({"detail": detail, "id": toner}),
            )?;
            assert!(!out.is_error, "{out:?}");
        }
        let unknown = call(
            &store,
            &task,
            "remove_captured",
            &json!({"detail": "time", "id": toner}),
        )?;
        assert!(unknown.is_error);
        let again = call(
            &store,
            &task,
            "remove_captured",
            &json!({"detail": "place", "id": toner}),
        )?;
        assert!(again.is_error, "nothing left to remove");
        assert_eq!(store.all_entries()?.len(), logged);
        let read = call(&store, &task, "read_note", &json!({"id": toner}))?;
        let [ToolContent::Text(text)] = &read.content[..] else {
            return Err("a text".into());
        };
        let note: Value = serde_json::from_str(text)?;
        assert_eq!(
            note.get("captureContext"),
            Some(&json!({"calendar": r#"["Katzenklo"]"#}))
        );
        Ok(())
    }

    #[test]
    fn a_sections_text_is_edited_as_a_pending_change() -> TestResult {
        let store = store();
        let notes = Notes::new(&store);
        let note = notes.capture("Dentist", None, None)?;
        let amend = notes.amend(&note.id, "Moved to Wednesday")?;
        let later = notes.amend(&note.id, "Later")?;
        store.append(
            &later.id,
            crate::notes::keys::TEXT,
            Some(r##"{"format":"x-future","text":"# Later"}"##),
            None,
        )?;
        let task = store.open_ai_task()?;
        let logged = store.all_entries()?.len();
        let out = call(
            &store,
            &task,
            "edit_section",
            &json!({"id": amend.id, "text": "Moved to Thursday"}),
        )?;
        assert!(!out.is_error, "{out:?}");
        // A text in a format not known here, and no Section at all.
        for id in [&later.id, &note.id] {
            let refused = call(
                &store,
                &task,
                "edit_section",
                &json!({"id": id, "text": "Overwritten"}),
            )?;
            assert!(refused.is_error, "{id}");
        }
        assert_eq!(store.all_entries()?.len(), logged);
        let read = call(&store, &task, "read_note", &json!({"id": note.id}))?;
        let [ToolContent::Text(text)] = &read.content[..] else {
            return Err("a text".into());
        };
        let read: Value = serde_json::from_str(text)?;
        assert_eq!(
            read.pointer("/sections/0/text"),
            Some(&json!("Moved to Thursday"))
        );
        assert_eq!(read.pointer("/sections/0/mayEdit"), Some(&json!(true)));
        assert!(
            read.pointer("/sections/0/modified")
                .is_some_and(Value::is_string)
        );
        assert_eq!(read.pointer("/sections/1/mayEdit"), Some(&json!(false)));
        let review = store.ai_review(&task)?;
        assert!(review.groups.iter().any(|(kind, items)| {
            *kind == crate::ai_task::AiChangeKind::NoteText && items[0].entity == amend.id
        }));
        Ok(())
    }

    #[test]
    fn a_change_waits_and_a_read_shows_it_as_the_user_sees_it() -> TestResult {
        let store = store();
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        let task = store.open_ai_task()?;
        let logged = store.all_entries()?.len();
        let schedule = json!({"note": milk.id, "rule": "FREQ=DAILY", "start": "2026-01-02", "times": ["08:00"]});

        let added = call(&store, &task, "add_routine", &schedule)?;
        assert!(!added.is_error, "{added:?}");
        let read = call(&store, &task, "list_routines", &json!({"note": milk.id}))?;
        let [ToolContent::Text(text)] = &read.content[..] else {
            return Err("a text".into());
        };
        let routines: Value = serde_json::from_str(text)?;
        assert_eq!(routines.pointer("/0/times/0"), Some(&json!("08:00")));
        assert_eq!(routines.pointer("/0/rule"), Some(&json!("FREQ=DAILY")));
        assert_eq!(store.all_entries()?.len(), logged);
        DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
        let settings = call(&store, &task, "read_settings", &json!({}))?;
        assert!(!settings.is_error);
        let [ToolContent::Text(text)] = &settings.content[..] else {
            return Err("a text".into());
        };
        let read: Value = serde_json::from_str(text)?;
        assert_eq!(read.pointer("/quickNote/fileInto"), Some(&json!(true)));
        assert_eq!(read.pointer("/quickNote/formatBar"), Some(&json!(false)));
        assert_eq!(read.pointer("/quickNote/nudgeButtons"), Some(&json!(true)));
        assert_eq!(read.pointer("/theme"), Some(&json!("system")));
        assert_eq!(read.pointer("/aiReach"), Some(&json!("local")));
        assert_eq!(read.pointer("/allDayFrom"), Some(&json!("15:00")));
        Ok(())
    }

    type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn utc(t: DateTime<Utc>) -> chrono::NaiveDateTime {
        t.naive_utc()
    }

    /// The text a tool handed back, read as JSON.
    fn json_of(out: &ToolOutput) -> Made<Value> {
        let [ToolContent::Text(text)] = &out.content[..] else {
            return Err(format!("no text: {out:?}").into());
        };
        Ok(serde_json::from_str(text)?)
    }

    fn daily() -> Value {
        json!({"start": "2026-01-01", "times": ["08:00"], "rule": "FREQ=DAILY"})
    }

    fn schedule() -> Made<Schedule> {
        let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
        let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e}"))?;
        Ok(Schedule::new(Day::new(2026, 1, 1), &eight, Some(rule)).ok_or("a schedule")?)
    }

    /// A Note made a Tally with one field: its id and the field's.
    fn tally(store: &Store, text: &str) -> Made<(String, String)> {
        let note = Notes::new(store).capture(text, None, None)?.id;
        let field = Tallies::new(store).add_field(&note, text, "", None, None)?;
        Ok((note, field))
    }

    /// A Note made a Checklist of `text`.
    fn checklist(store: &Store, text: &str) -> Made<String> {
        let note = Notes::new(store).capture(text, None, None)?.id;
        Checklists::new(store).make(&note)?;
        Ok(note)
    }

    fn shopping_list(store: &Store, text: &str) -> Made<String> {
        let note = Notes::new(store).capture(text, None, None)?.id;
        ShoppingLists::new(store).make(&note)?;
        Ok(note)
    }

    fn lines(store: &Store, note: &str) -> Made<Vec<crate::checklists::Line>> {
        Ok(Checklists::new(store)
            .by_note(note)?
            .ok_or("a Checklist")?
            .lines)
    }

    fn tally_field(store: &Store, note: &str) -> Made<Option<crate::tallies::TallyField>> {
        Ok(Tallies::new(store)
            .by_note(note)?
            .and_then(|t| t.fields.into_iter().next()))
    }

    /// Seeds a store, calls `name` with the args the seed gives and checks
    /// with `shows` that the change waits, seen by the reads and not in
    /// the log; that Accept keeps it; and, on another store, that Reject
    /// leaves nothing.
    fn lands_as_pending(
        name: &str,
        seed: impl Fn(&Store) -> Made<Value>,
        shows: impl Fn(&Store, &Value) -> Made<bool>,
    ) -> TestResult {
        for accept in [true, false] {
            let store = store();
            let args = seed(&store)?;
            let task = store.open_ai_task()?;
            let logged = store.all_entries()?.len();
            let out = call(&store, &task, name, &args)?;
            assert!(!out.is_error, "{name}: {out:?}");
            assert_eq!(store.all_entries()?.len(), logged, "{name} wrote the log");
            assert!(shows(&store, &args)?, "{name}: the reads show it");
            assert!(
                !store.log_only(|| shows(&store, &args))?,
                "{name}: the log holds it already"
            );
            assert!(!store.ai_review(&task)?.is_empty(), "{name}: reviewed");
            if accept {
                store.accept_pending(&task)?;
                assert!(shows(&store, &args)?, "{name}: kept on Accept");
            } else {
                store.decline_pending(&task)?;
                assert!(!shows(&store, &args)?, "{name}: gone on Reject");
                assert_eq!(store.all_entries()?.len(), logged, "{name}: nothing left");
            }
        }
        Ok(())
    }

    fn arg(args: &Value, key: &str) -> Made<String> {
        Ok(args
            .get(key)
            .and_then(Value::as_str)
            .ok_or("an id")?
            .to_string())
    }

    #[test]
    fn every_tally_set_up_waits_for_the_review() -> TestResult {
        lands_as_pending(
            "make_tally",
            |s| {
                Ok(
                    json!({"fields": [{"name": "Run", "target": 5, "unit": "km"}], "id": Notes::new(s).capture("Run", None, None)?.id}),
                )
            },
            |s, a| {
                let field = tally_field(s, &arg(a, "id")?)?;
                Ok(field.is_some_and(|f| f.unit == "km" && f.target == Some(5000.0)))
            },
        )?;
        lands_as_pending(
            "add_tally_field",
            |s| Ok(json!({"id": tally(s, "Run")?.0, "name": "Time", "rule": "max", "unit": "min"})),
            |s, a| {
                let tally = Tallies::new(s).by_note(&arg(a, "id")?)?.ok_or("a Tally")?;
                Ok(tally.fields.len() == 2 && tally.fields[1].rule == TallyRule::Max)
            },
        )?;
        lands_as_pending(
            "add_tally_field",
            |s| {
                let (id, reps) = tally(s, "Push-ups")?;
                Ok(json!({"id": id, "multiplies": reps, "name": "Sets"}))
            },
            |s, a| {
                let tally = Tallies::new(s).by_note(&arg(a, "id")?)?.ok_or("a Tally")?;
                Ok(tally.fields.len() == 2
                    && tally.fields[1].multiplies.as_ref() == Some(&tally.fields[0].id))
            },
        )?;
        lands_as_pending(
            "set_tally_field",
            |s| {
                let (id, reps) = tally(s, "Squats")?;
                let sets = Tallies::new(s).add_field(&id, "Sets", "", None, None)?;
                Ok(json!({"field": sets, "id": id, "multiplies": reps}))
            },
            |s, a| {
                let tally = Tallies::new(s).by_note(&arg(a, "id")?)?.ok_or("a Tally")?;
                Ok(tally.fields[1].multiplies.as_ref() == Some(&tally.fields[0].id))
            },
        )?;
        lands_as_pending(
            "rename_tally_field",
            |s| {
                let (id, field) = tally(s, "Pushups")?;
                Ok(json!({"field": field, "id": id, "name": "Push-ups"}))
            },
            |s, a| Ok(tally_field(s, &arg(a, "id")?)?.is_some_and(|f| f.name == "Push-ups")),
        )?;
        lands_as_pending(
            "set_tally_field",
            |s| {
                let (id, field) = tally(s, "Plank")?;
                Ok(json!({"field": field, "id": id, "rule": "max", "target": 2, "unit": "min"}))
            },
            |s, a| {
                let field = tally_field(s, &arg(a, "id")?)?.ok_or("a field")?;
                Ok(field.unit == "min"
                    && field.target == Some(120.0)
                    && field.rule == TallyRule::Max)
            },
        )?;
        lands_as_pending(
            "remove_tally_field",
            |s| {
                let (id, field) = tally(s, "Squats")?;
                Ok(json!({"field": field, "id": id}))
            },
            |s, a| Ok(tally_field(s, &arg(a, "id")?)?.is_some_and(|f| f.hidden)),
        )
    }

    #[test]
    fn every_session_set_up_waits_for_the_review() -> TestResult {
        lands_as_pending(
            "make_session",
            |s| {
                let mut args = daily();
                let pool = vec![tally(s, "Push-ups")?.0, tally(s, "Squats")?.0];
                if let Some(all) = args.as_object_mut() {
                    all.insert("count".into(), json!(1));
                    all.insert("picking".into(), json!("random"));
                    all.insert("pool".into(), json!(pool));
                    all.insert("text".into(), json!("Workout"));
                }
                Ok(args)
            },
            |s, _| {
                let all = Sessions::new(s).all()?;
                Ok(all.len() == 1
                    && all[0].setup.pool.len() == 2
                    && all[0].setup.rule == SessionRule::Random)
            },
        )?;
        // Without a schedule the Note alone is set up.
        lands_as_pending(
            "make_session",
            |s| {
                let note = Notes::new(s).capture("Sport", None, None)?.id;
                Ok(json!({"note": note, "picking": "least", "pool": [tally(s, "Push-ups")?.0]}))
            },
            |s, a| {
                let note = arg(a, "note")?;
                let setup = Sessions::new(s).setup_of(&note)?;
                Ok(setup.is_some_and(|s| s.rule == SessionRule::Least)
                    && Routines::new(s).of_note(&note)?.is_empty())
            },
        )?;
        lands_as_pending(
            "set_session",
            |s| {
                let (one, two) = (tally(s, "Push-ups")?.0, tally(s, "Squats")?.0);
                let session = Sessions::new(s).create(
                    "Workout",
                    &schedule()?,
                    std::slice::from_ref(&one),
                    1,
                    SessionRule::Rolling,
                )?;
                Ok(
                    json!({"count": 2, "id": session.note_id(), "picking": "least", "pool": [one, two], "weekdayCounts": {"6": 1}}),
                )
            },
            |s, a| {
                let session = Sessions::new(s)
                    .setup_of(&arg(a, "id")?)?
                    .ok_or("a Session")?;
                Ok(session.pool.len() == 2
                    && session.count == 2
                    && session.rule == SessionRule::Least
                    && session.counts.get(&6) == Some(&1))
            },
        )
    }

    #[test]
    fn every_list_set_up_waits_for_the_review() -> TestResult {
        for (list, is) in [("checklist", true), ("shoppingList", false)] {
            lands_as_pending(
                "make_list",
                |s| {
                    Ok(
                        json!({"id": Notes::new(s).capture("Trip\nSocks\nShirt", None, None)?.id, "list": list}),
                    )
                },
                |s, a| {
                    let id = arg(a, "id")?;
                    Ok(match is {
                        true => Checklists::new(s).is_checklist(&id)?,
                        false => ShoppingLists::new(s).is_shopping_list(&id)?,
                    })
                },
            )?;
        }
        lands_as_pending(
            "list_to_plain",
            |s| Ok(json!({"id": checklist(s, "Trip\nSocks")?})),
            |s, a| Ok(!Checklists::new(s).is_checklist(&arg(a, "id")?)?),
        )?;
        lands_as_pending(
            "switch_list",
            |s| Ok(json!({"id": checklist(s, "Trip\nSocks")?})),
            |s, a| {
                ShoppingLists::new(s)
                    .is_shopping_list(&arg(a, "id")?)
                    .map_err(Into::into)
            },
        )?;
        lands_as_pending(
            "add_line",
            |s| Ok(json!({"at": 0, "id": checklist(s, "Trip\nSocks")?, "text": "Passport"})),
            |s, a| Ok(lines(s, &arg(a, "id")?)?[0].text == "Passport"),
        )?;
        lands_as_pending(
            "add_line",
            |s| Ok(json!({"id": shopping_list(s, "Shop\nMilk")?, "text": "Bread"})),
            |s, a| {
                let list = ShoppingLists::new(s).by_note(&arg(a, "id")?, &utc)?;
                Ok(list.is_some_and(|l| l.count() == 2))
            },
        )?;
        lands_as_pending(
            "rename_line",
            |s| Ok(json!({"id": lines(s, &checklist(s, "Trip\nSoks")?)?[0].id, "text": "Socks"})),
            |s, a| {
                let line = arg(a, "id")?;
                Ok(s.current(&line, checklist_keys::TEXT)?
                    .is_some_and(|t| crate::StoredText::read(Some(&t)).text == "Socks"))
            },
        )?;
        lands_as_pending(
            "move_line",
            |s| {
                let note = checklist(s, "Trip\nSocks\nPassport")?;
                let all = lines(s, &note)?;
                Ok(json!({"before": all[0].id, "id": all[1].id, "note": note}))
            },
            |s, a| Ok(lines(s, &arg(a, "note")?)?[0].text == "Passport"),
        )?;
        lands_as_pending(
            "group_line",
            |s| {
                let note = checklist(s, "Trip\nBag\nSocks")?;
                Ok(json!({"id": lines(s, &note)?[1].id, "note": note, "under": true}))
            },
            |s, a| Ok(lines(s, &arg(a, "note")?)?[1].in_group()),
        )?;
        lands_as_pending(
            "set_reappear",
            |s| {
                let note = shopping_list(s, "Shop\nMilk")?;
                let list = ShoppingLists::new(s)
                    .by_note(&note, &utc)?
                    .ok_or("a list")?;
                Ok(json!({"id": list.lines[0].id, "note": note, "reappear": "2w"}))
            },
            |s, a| {
                let list = ShoppingLists::new(s).by_note(&arg(a, "note")?, &utc)?;
                Ok(list.is_some_and(|l| l.lines[0].reappear == Reappear::Week2))
            },
        )?;
        lands_as_pending(
            "remove_line",
            |s| {
                let note = checklist(s, "Trip\nSocks\nShirt")?;
                Ok(json!({"id": lines(s, &note)?[0].id, "note": note}))
            },
            |s, a| Ok(lines(s, &arg(a, "note")?)?.len() == 1),
        )?;
        // Read from the log alone, the choice shows once accepted; the
        // Pending Change holds it until then.
        lands_as_pending(
            "set_shopping_agenda",
            |s| Ok(json!({"agenda": "lines", "id": shopping_list(s, "Shop\nMilk")?})),
            |s, a| {
                let entity = format!("{}{}", checklist_keys::AGENDA_PREFIX, arg(a, "id")?);
                Ok(s.current(&entity, shopping_keys::AGENDA)?.as_deref() == Some("lines"))
            },
        )?;
        lands_as_pending(
            "set_checklist_agenda",
            |s| Ok(json!({"agenda": "count", "id": checklist(s, "Trip\nSocks")?})),
            |s, a| {
                let entity = format!("{}{}", checklist_keys::AGENDA_PREFIX, arg(a, "id")?);
                Ok(s.current(&entity, checklist_keys::AGENDA)?.as_deref() == Some("count"))
            },
        )
    }

    #[test]
    fn a_checklists_agenda_choice_is_set_once_accepted() -> TestResult {
        let store = store();
        let trip = checklist(&store, "Trip\nSocks")?;
        let shop = shopping_list(&store, "Shop\nMilk")?;
        let gym = checklist(&store, "Gym\nTowel")?;
        Routines::new(&store).spawn(
            &gym,
            &Schedule::new(Day::new(2026, 3, 1), &[], None).ok_or("a schedule")?,
        )?;
        let task = store.open_ai_task()?;
        let set = |id: &str, agenda: &str| {
            call(
                &store,
                &task,
                "set_checklist_agenda",
                &json!({"agenda": agenda, "id": id}),
            )
        };
        assert!(set(&shop, "off")?.is_error, "a Shopping List");
        assert!(set(&gym, "off")?.is_error, "a Checklist with a Routine");
        assert!(set(&trip, "sideways")?.is_error, "no such way");
        assert!(!set(&trip, "count")?.is_error);
        let review = store.ai_review(&task)?;
        let items: Vec<&crate::ai_task::AiReviewItem> =
            review.groups.iter().flat_map(|(_, i)| i).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, crate::ai_task::AiChangeKind::List);
        assert_eq!(items[0].note.as_deref(), Some(trip.as_str()));
        let lists = Checklists::new(&store);
        assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Lines);
        store.accept_pending(&task)?;
        assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Count);
        let rows = lists.agenda_rows()?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].shows, ListOnAgenda::Count);
        Ok(())
    }

    #[test]
    fn a_shopping_lists_agenda_choice_is_set_once_accepted() -> TestResult {
        let store = store();
        let shop = shopping_list(&store, "Shop\nMilk")?;
        let trip = checklist(&store, "Trip\nSocks")?;
        let task = store.open_ai_task()?;
        let set = |id: &str, agenda: &str| {
            call(
                &store,
                &task,
                "set_shopping_agenda",
                &json!({"agenda": agenda, "id": id}),
            )
        };
        assert!(set(&trip, "off")?.is_error, "a Checklist");
        assert!(set(&shop, "sideways")?.is_error, "no such way");
        assert!(!set(&shop, "off")?.is_error);
        let review = store.ai_review(&task)?;
        let items: Vec<&crate::ai_task::AiReviewItem> =
            review.groups.iter().flat_map(|(_, i)| i).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, crate::ai_task::AiChangeKind::List);
        assert_eq!(items[0].note.as_deref(), Some(shop.as_str()));
        let lists = ShoppingLists::new(&store);
        assert_eq!(lists.on_agenda(&shop)?, ShoppingOnAgenda::Count);
        store.accept_pending(&task)?;
        assert_eq!(lists.on_agenda(&shop)?, ShoppingOnAgenda::Off);
        assert!(lists.agenda_rows(&utc)?.is_empty());
        Ok(())
    }

    #[test]
    fn every_frog_set_up_waits_for_the_review() -> TestResult {
        lands_as_pending(
            "make_frog",
            |s| Ok(json!({"id": Notes::new(s).capture("Call the bank", None, None)?.id})),
            |s, a| Ok(!Frogs::new(s).of_note(&arg(a, "id")?)?.is_empty()),
        )?;
        lands_as_pending(
            "frog_to_plain",
            |s| {
                let note = Notes::new(s).capture("Call the bank", None, None)?.id;
                Frogs::new(s).spawn(&note)?;
                Ok(json!({"id": note}))
            },
            |s, a| Ok(Frogs::new(s).of_note(&arg(a, "id")?)?.is_empty()),
        )
    }

    #[test]
    fn no_tool_writes_the_users_own_data() {
        let names = |read_only: bool| -> Vec<String> {
            tools()
                .iter()
                .filter(|t| t.pointer("/annotations/readOnlyHint") == Some(&Value::Bool(read_only)))
                .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
                .collect()
        };
        let writers = [
            "capture_note",
            "edit_note",
            "amend_note",
            "edit_section",
            "file_note",
            "merge_notes",
            "set_importance",
            "remove_captured",
            "trash_note",
            "restore_note",
            "add_area",
            "add_topic",
            "rename_place",
            "set_area_colour",
            "set_area_icon",
            "delete_place",
            "add_routine",
            "reschedule_routine",
            "set_renag",
            "set_escalation",
            "trash_routine",
            "restore_routine",
            "make_tally",
            "add_tally_field",
            "rename_tally_field",
            "set_tally_field",
            "remove_tally_field",
            "make_session",
            "set_session",
            "make_list",
            "list_to_plain",
            "switch_list",
            "add_line",
            "rename_line",
            "move_line",
            "group_line",
            "set_reappear",
            "remove_line",
            "set_shopping_agenda",
            "set_checklist_agenda",
            "make_frog",
            "frog_to_plain",
        ];
        assert_eq!(
            names(false),
            writers,
            "only these write, as Pending Changes"
        );
        let readers = names(true);
        for data in [
            "read_tally_entries",
            "read_ticks",
            "read_check_offs",
            "read_eaten_frogs",
            "read_session_picks",
        ] {
            assert!(readers.iter().any(|r| r == data), "{data} reads");
        }
        assert_eq!(readers.len() + writers.len(), tools().len());
    }

    #[test]
    fn a_list_with_checked_lines_or_a_checked_name_is_refused_with_words() -> TestResult {
        let store = store();
        let trip = Notes::new(&store).capture("Trip\n✓ Socks\nShirt", None, None)?;
        let shop = shopping_list(&store, "Shop\nMilk")?;
        let milk = ShoppingLists::new(&store)
            .by_note(&shop, &utc)?
            .ok_or("a list")?;
        ShoppingLists::new(&store).check(&milk.lines[0].id, &utc)?;
        let task = store.open_ai_task()?;

        let made = call(
            &store,
            &task,
            "make_list",
            &json!({"id": trip.id, "list": "checklist"}),
        )?;
        assert!(made.is_error);
        assert!(format!("{made:?}").contains("✓"), "{made:?}");
        let added = call(
            &store,
            &task,
            "add_line",
            &json!({"id": shop, "text": " milk "}),
        )?;
        assert!(added.is_error, "{added:?}");
        let ticked = call(&store, &task, "move_line", &json!({"id": milk.lines[0].id}))?;
        assert!(ticked.is_error, "a Shopping List's Lines do not move");
        assert!(
            store
                .pending_set(&task)?
                .ok_or("the task")?
                .drafts()
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn notes_name_their_kind_and_parts() -> TestResult {
        let store = store();
        let notes = Notes::new(&store);
        let plain = notes.capture("Plain ☐ box", None, None)?.id;
        let (run, field) = tally(&store, "Run")?;
        Tallies::new(&store).set_unit(&run, &field, "km")?;
        Tallies::new(&store).set_target(&run, &field, Some(5000.0))?;
        let trip = checklist(&store, "Trip\nBag\n Socks")?;
        let shop = shopping_list(&store, "Shop\nMilk")?;
        let bank = notes.capture("Call the bank", None, None)?.id;
        let frog = Frogs::new(&store).spawn(&bank)?;
        let workout = Sessions::new(&store).spawn(
            &bank,
            &schedule()?,
            std::slice::from_ref(&run),
            1,
            SessionRule::Least,
        )?;
        let task = store.open_ai_task()?;

        let listed = json_of(&call(&store, &task, "list_notes", &json!({}))?)?;
        let kind = |id: &str| -> Option<Value> {
            listed
                .as_array()?
                .iter()
                .find(|n| n.get("id") == Some(&json!(id)))
                .cloned()
        };
        let kind_of = |id: &str| kind(id).and_then(|n| n.get("kind").cloned());
        assert_eq!(kind_of(&plain), Some(json!("plain")));
        assert_eq!(kind_of(&run), Some(json!("tally")));
        assert_eq!(kind_of(&trip), Some(json!("checklist")));
        assert_eq!(kind_of(&shop), Some(json!("shoppingList")));
        assert_eq!(kind_of(&bank), Some(json!("session")));
        let bank_listed = kind(&bank).ok_or("the bank's Note")?;
        assert_eq!(bank_listed.get("frog"), Some(&json!(frog.id)));
        assert_eq!(bank_listed.get("sessions"), Some(&json!([workout.id()])));

        let read = |id: &str| json_of(&call(&store, &task, "read_note", &json!({"id": id}))?);
        let tallied = read(&run)?;
        assert_eq!(tallied.pointer("/tally/fields/0/unit"), Some(&json!("km")));
        assert_eq!(tallied.pointer("/tally/fields/0/target"), Some(&json!(5.0)));
        let packed = read(&trip)?;
        assert_eq!(packed.pointer("/lines/1/text"), Some(&json!("Socks")));
        assert!(
            packed
                .pointer("/lines/1/group")
                .is_some_and(|g| g.is_string())
        );
        assert_eq!(
            read(&shop)?.pointer("/lines/0/reappear"),
            Some(&json!("1w"))
        );
        assert_eq!(read(&shop)?.pointer("/agenda"), Some(&json!("count")));
        assert_eq!(read(&trip)?.pointer("/name"), Some(&json!("Trip")));
        assert_eq!(read(&trip)?.pointer("/description"), Some(&json!("")));
        assert_eq!(read(&plain)?.pointer("/name"), Some(&Value::Null));
        let gym = tally(&store, "Gym\nMondays and Thursdays")?.0;
        let gymmed = read(&gym)?;
        assert_eq!(gymmed.pointer("/name"), Some(&json!("Gym")));
        assert_eq!(
            gymmed.pointer("/description"),
            Some(&json!("Mondays and Thursdays"))
        );
        assert_eq!(read(&trip)?.pointer("/agenda"), Some(&json!("lines")));
        assert_eq!(read(&plain)?.pointer("/agenda"), Some(&Value::Null));
        let banked = read(&bank)?;
        assert_eq!(banked.pointer("/frogs/0/id"), Some(&json!(frog.id)));
        assert_eq!(
            banked.pointer("/routines/0/session/picking"),
            Some(&json!("least"))
        );
        assert_eq!(
            banked.pointer("/routines/0/session/pool"),
            Some(&json!([run]))
        );
        assert_eq!(banked.pointer("/session/pool"), Some(&json!([run])));
        assert_eq!(banked.pointer("/session/picking"), Some(&json!("least")));
        assert_eq!(
            banked.pointer("/session/routines"),
            Some(&json!([workout.id()]))
        );
        assert_eq!(read(&trip)?.pointer("/session"), Some(&Value::Null));
        assert_eq!(
            read(&plain)?.pointer("/boxes/0/state"),
            Some(&json!("open"))
        );
        Ok(())
    }

    #[test]
    fn a_multiplied_field_reads_as_the_product_with_its_factors() -> TestResult {
        let store = store();
        let (note, reps) = tally(&store, "Push-ups")?;
        let tallies = Tallies::new(&store);
        let sets = tallies.add_field(&note, "Sets", "", None, None)?;
        tallies.add(
            &note,
            &BTreeMap::from([(reps.clone(), 10.0), (sets.clone(), 3.0)]),
            "",
            None,
        )?;
        tallies.set_multiplies(&note, &sets, Some(&reps))?;
        let task = store.open_ai_task()?;
        let read = |name: &str, args: Value| json_of(&call(&store, &task, name, &args)?);
        let entries = read("read_tally_entries", json!({"note": note}))?;
        assert_eq!(entries.pointer("/0/values/0/field"), Some(&json!(reps)));
        assert_eq!(entries.pointer("/0/values/0/value"), Some(&json!(30.0)));
        assert_eq!(
            entries.pointer("/0/values/0/factors/0"),
            Some(&json!({"field": sets, "name": "Sets", "value": 3.0}))
        );
        assert_eq!(entries.pointer("/0/values/1"), None, "no value of its own");
        let shown = read("read_note", json!({"id": note}))?;
        assert_eq!(
            shown.pointer("/tally/fields/1/multiplies"),
            Some(&json!(reps))
        );
        assert_eq!(
            shown.pointer("/tally/fields/0/multiplies"),
            Some(&Value::Null)
        );
        Ok(())
    }

    #[test]
    fn the_users_own_data_is_read_with_its_dates_and_by_range() -> TestResult {
        let store = store();
        let at = |day: u32| {
            Utc.with_ymd_and_hms(2025, 10, day, 18, 0, 0)
                .single()
                .ok_or("a time")
        };
        let (run, field) = tally(&store, "Run")?;
        let tallies = Tallies::new(&store);
        for day in [3, 17] {
            tallies.add(
                &run,
                &BTreeMap::from([(field.clone(), 4.0)]),
                "park",
                Some(at(day)?),
            )?;
        }
        let trip = checklist(&store, "Trip\nSocks\nShirt")?;
        Checklists::new(&store).check(&lines(&store, &trip)?[0].id)?;
        let bank = Notes::new(&store).capture("Call the bank", None, None)?.id;
        let frog = Frogs::new(&store).spawn(&bank)?;
        Frogs::new(&store).mark_done(&frog.id, Some(at(20)?))?;
        let pills = Notes::new(&store).capture("Pills", None, None)?.id;
        let routine = Routines::new(&store).spawn(&pills, &schedule()?)?;
        let first = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
        Routines::new(&store).mark_done(&routine.id, &first, None)?;
        let session = Sessions::new(&store).spawn(
            &bank,
            &schedule()?,
            std::slice::from_ref(&run),
            1,
            SessionRule::Rolling,
        )?;
        Sessions::new(&store).pick_today(&utc)?;
        Sessions::new(&store).record(session.id(), &first, &run, &BTreeMap::new(), "done")?;
        let task = store.open_ai_task()?;
        let read = |name: &str, args: Value| json_of(&call(&store, &task, name, &args)?);

        let entries = read("read_tally_entries", json!({"note": run}))?;
        assert_eq!(
            entries.as_array().map(Vec::len),
            Some(3),
            "with the Session's"
        );
        let october = read(
            "read_tally_entries",
            json!({"from": "2025-10-01", "note": run, "until": "2025-10-31"}),
        )?;
        assert_eq!(october.pointer("/0/day"), Some(&json!("2025-10-17")));
        assert_eq!(october.pointer("/1/at"), Some(&json!(dart_iso(at(3)?))));
        assert_eq!(october.pointer("/1/values/0/value"), Some(&json!(4.0)));
        assert_eq!(october.as_array().map(Vec::len), Some(2));
        let ticks = read("read_ticks", json!({"note": trip}))?;
        assert_eq!(ticks.pointer("/0/text"), Some(&json!("Socks")));
        assert_eq!(ticks.pointer("/0/day"), Some(&json!("2026-01-01")));
        let none = read("read_ticks", json!({"note": trip, "until": "2025-12-31"}))?;
        assert_eq!(none, json!([]));
        let offs = read("read_check_offs", json!({"note": pills}))?;
        assert_eq!(offs.pointer("/0/occurrence"), Some(&json!(first.key())));
        assert!(offs.pointer("/0/at").is_some_and(Value::is_string));
        let eaten = read("read_eaten_frogs", json!({"from": "2025-10-20"}))?;
        assert_eq!(eaten.pointer("/0/at"), Some(&json!(dart_iso(at(20)?))));
        assert_eq!(
            read("read_eaten_frogs", json!({"from": "2025-10-21"}))?,
            json!([])
        );
        let picks = read("read_session_picks", json!({"id": session.id()}))?;
        assert_eq!(picks.pointer("/0/day"), Some(&json!("2026-01-01")));
        assert_eq!(picks.pointer("/0/picks/0/note"), Some(&json!(run)));
        assert!(
            picks
                .pointer("/0/picks/0/entries/0/at")
                .is_some_and(Value::is_string)
        );
        let bad = call(
            &store,
            &task,
            "read_eaten_frogs",
            &json!({"from": "October"}),
        )?;
        assert!(bad.is_error);
        Ok(())
    }
}
