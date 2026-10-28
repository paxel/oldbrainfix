//! The tools an AI client calls through the AI port (#112, ADR 0026):
//! reading Notes with their Sections, Transcripts, Photo Texts and
//! Capture Context, a photo scaled down, the Areas and Topics, the
//! Routines and Reminders, and this Device's settings; and doing what the
//! user can do with Notes, Areas, Topics, Merges, the trash, Importance,
//! Routines and Reminders. Every change goes through the screens' own
//! calls inside [`Store::drafting`], so it is a Pending Change of the AI
//! Task until the user accepts it. Nothing reaches other people: no
//! sending, sharing, Profile cards nor blocking, and settings are only
//! read.

use std::io::Cursor;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Duration;
use serde_json::{Value, json};

use crate::entry::dart_iso;
use crate::notes::{Importance, Note, Notes};
use crate::places::{NotesGo, Places};
use crate::routines::{Routine, Routines};
use crate::schedule::{ClockTime, Day, Recurrence, Schedule};
use crate::search::Search;
use crate::settings::DeviceSettings;
use crate::store::Store;
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
    vec![
        tool(
            "list_notes",
            "Lists Notes: the Unsorted ones, all, those in the trash or in one Area or Topic, or those whose text, Transcripts or Photo Texts hold a search.",
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
            "Reads one Note in full: its text, Sections, Transcripts of its recordings, Photo Texts, photos, Capture Context, Routines and Reminders.",
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
            "Lists the Routines and Reminders, of one Note or all; those in the trash on request.",
            true,
            json!({
                "note": string("Only those spawned from this Note"),
                "trash": {"description": "Those in the trash instead", "type": "boolean"},
            }),
            &[],
        ),
        tool(
            "read_settings",
            "Reads this Device's settings: the Re-nag interval, the Nudge's repeat, Quiet Hours and whether alerts are on. They cannot be changed here.",
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

fn note_summary(note: &Note) -> Value {
    json!({
        "captured": dart_iso(note.captured),
        "id": note.id,
        "importance": note.importance.name(),
        "mergedInto": note.merged_into,
        "place": note.place,
        "text": note.text,
        "trashed": note.is_trashed(),
        "unsorted": note.is_unsorted(),
    })
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
    pub fn ai_tool(&self, task: &str, name: &str, args: &Value) -> Result<ToolOutput> {
        if !is_tool(name) {
            return Err(Error::Invalid(format!("no tool {name}")));
        }
        let done = if reads_only(name) {
            self.ai_read(name, args)
        } else {
            self.drafting(task, || self.ai_change(name, args))
                .map(|value| ToolOutput::json(&value))
        };
        Ok(done.unwrap_or_else(|e| ToolOutput::failed(&e.to_string())))
    }

    fn ai_read(&self, name: &str, args: &Value) -> Result<ToolOutput> {
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
                ToolOutput::json(&Value::Array(found.iter().map(note_summary).collect()))
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
                        json!({
                            "captured": dart_iso(s.captured),
                            "id": s.id,
                            "photoTexts": texts_of(&s.photo_texts, "photo"),
                            "photos": s.photos,
                            "text": s.text,
                            "transcripts": texts_of(&s.transcripts, "recording"),
                        })
                    })
                    .collect();
                let routines: Vec<Value> = Routines::new(self)
                    .of_note(id)?
                    .iter()
                    .map(routine_json)
                    .collect();
                let mut out = note_summary(&note);
                if let Some(all) = out.as_object_mut() {
                    all.insert("captureContext".into(), Value::Object(context));
                    all.insert("mayChange".into(), json!(self.may_change(id)?));
                    all.insert(
                        "photoTexts".into(),
                        json!(texts_of(&note.photo_texts, "photo")),
                    );
                    all.insert("photos".into(), json!(note.photos));
                    all.insert("routines".into(), json!(routines));
                    all.insert("sections".into(), json!(sections));
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
                ToolOutput::json(&Value::Array(found.iter().map(routine_json).collect()))
            }
            "read_settings" => {
                let settings = DeviceSettings::new(self);
                let quiet = settings.quiet_hours()?;
                ToolOutput::json(&json!({
                    "alertsOn": settings.alerts_on()?,
                    "nudgeRepeatMinutes": settings.nudge_repeat()?.num_minutes(),
                    "quietHours": {
                        "from": quiet.start.to_string(),
                        "on": settings.quiet_hours_on()?,
                        "until": quiet.end.to_string(),
                    },
                    "renagMinutes": settings.renag_interval()?.num_minutes(),
                }))
            }
            _ => return Err(Error::Invalid(format!("no tool {name}"))),
        })
    }

    /// One change, inside [`Store::drafting`]; what it made, by id.
    fn ai_change(&self, name: &str, args: &Value) -> Result<Value> {
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
            _ => return Err(Error::Invalid(format!("no tool {name}"))),
        };
        Ok(json!({"id": made, "pending": "kept once the user accepts the AI Task"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::store;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

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

        let out = store.ai_tool(
            &task,
            "read_photo",
            &json!({"note": receipt.id, "photo": hash, "size": 100}),
        )?;
        let [ToolContent::Jpeg(jpeg)] = &out.content[..] else {
            return Err(format!("no JPEG: {out:?}").into());
        };
        let image = image::load_from_memory(jpeg)?;
        assert_eq!((image.width(), image.height()), (100, 50));

        let elsewhere = store.ai_tool(
            &task,
            "read_photo",
            &json!({"note": other.id, "photo": hash}),
        )?;
        assert!(elsewhere.is_error);
        assert!(store.ai_tool(&task, "send_note", &json!({})).is_err());
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

        let out = store.ai_tool(
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
    fn a_change_waits_and_a_read_shows_it_as_the_user_sees_it() -> TestResult {
        let store = store();
        let milk = Notes::new(&store).capture("Milk", None, None)?;
        let task = store.open_ai_task()?;
        let logged = store.all_entries()?.len();
        let schedule = json!({"note": milk.id, "rule": "FREQ=DAILY", "start": "2026-01-02", "times": ["08:00"]});

        let added = store.ai_tool(&task, "add_routine", &schedule)?;
        assert!(!added.is_error, "{added:?}");
        let read = store.ai_tool(&task, "list_routines", &json!({"note": milk.id}))?;
        let [ToolContent::Text(text)] = &read.content[..] else {
            return Err("a text".into());
        };
        let routines: Value = serde_json::from_str(text)?;
        assert_eq!(routines.pointer("/0/times/0"), Some(&json!("08:00")));
        assert_eq!(routines.pointer("/0/rule"), Some(&json!("FREQ=DAILY")));
        assert_eq!(store.all_entries()?.len(), logged);
        let settings = store.ai_tool(&task, "read_settings", &json!({}))?;
        assert!(!settings.is_error);
        Ok(())
    }
}
