use super::*;
use crate::notes::Notes;
use crate::signing::SigningKey;
use chrono::TimeZone;

fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(y, m, d, h, 0, 0)
        .single()
        .unwrap_or_default()
}

fn now() -> DateTime<Utc> {
    at(2026, 10, 31, 12)
}

fn store() -> Store {
    match Store::in_memory(Box::new(now), || Ok(SigningKey::from_seed([1; 32]))) {
        Ok(s) => s,
        Err(e) => panic!("store: {e}"),
    }
}

fn file(name: &str, text: &str, modified: Option<DateTime<Utc>>) -> ImportFile {
    raw(name, text.as_bytes().to_vec(), modified)
}

fn raw(name: &str, bytes: Vec<u8>, modified: Option<DateTime<Utc>>) -> ImportFile {
    ImportFile {
        name: name.into(),
        modified,
        bytes,
    }
}

fn march() -> DateTime<Utc> {
    at(2025, 3, 1, 8)
}

fn may() -> DateTime<Utc> {
    at(2025, 5, 2, 9) + chrono::Duration::milliseconds(123)
}

/// The Notes inside the Area `summary` made, newest first.
fn inside(store: &Store, summary: &ImportSummary) -> Result<Vec<crate::notes::Note>> {
    let id = summary
        .area_id
        .as_deref()
        .ok_or(Error::Invalid("no Area".into()))?;
    Notes::new(store).inside(id)
}

#[test]
fn a_markdown_files_task_list_items_become_boxes() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            file("Shop.md", "- [ ] milk\n- [x] bread", Some(march())),
            file("Plain.txt", "- [ ] milk", Some(may())),
        ],
    )?;
    let notes = inside(&store, &summary)?;
    let shown: Vec<_> = notes.iter().map(|n| n.shown_text()).collect();
    assert_eq!(shown, ["Plain\n- [ ] milk", "# Shop\n- ☐ milk\n- ✅ bread"]);
    assert_eq!(notes[1].boxes.len(), 2);
    Ok(())
}

#[test]
fn a_folder_becomes_notes_in_a_new_area_dated_by_their_files() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            file("Later.txt", "Later\nsecond line", Some(may())),
            file("Earlier.md", "# Earlier\n**bold**", Some(march())),
            file("photo.png", "not text", Some(march())),
        ],
    )?;
    assert_eq!(
        (summary.notes, summary.skipped(), summary.not_text),
        (2, 1, 1)
    );
    let id = summary.area_id.clone().unwrap_or_default();
    let area = Places::new(&store).area_by_id(&id)?;
    assert_eq!(area.map(|a| a.name), Some("From Notes".into()));
    let notes = inside(&store, &summary)?;
    let texts: Vec<_> = notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(texts, ["Later\nsecond line", "# Earlier\n**bold**"]);
    let dates: Vec<_> = notes.iter().map(|n| n.captured).collect();
    assert_eq!(dates, [may(), march()]);
    let formats: Vec<_> = notes.iter().map(|n| n.text_format.as_str()).collect();
    assert_eq!(formats, [PLAIN, MARKDOWN]);
    // Written oldest first, every entry of a Note dated by its file.
    let earlier = &notes[1].id;
    let first = store
        .all_entries()?
        .into_iter()
        .find(|e| e.entity.starts_with("note:"));
    assert_eq!(first.map(|e| e.entity), Some(earlier.clone()));
    for entry in store.history(earlier)? {
        assert_eq!(entry.date_at, march());
    }
    Ok(())
}

#[test]
fn nothing_imported_is_unsorted_or_nudged() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            file("a.txt", "a", Some(march())),
            file("b.md", "b", Some(may())),
        ],
    )?;
    assert!(Notes::new(&store).unsorted()?.is_empty());
    let notes = inside(&store, &summary)?;
    assert_eq!(notes.len(), 2);
    for note in notes {
        assert_eq!(note.place, summary.area_id);
        assert_eq!(store.current(&note.id, keys::NUDGE)?, None);
    }
    Ok(())
}

#[test]
fn an_import_lands_beside_an_area_of_the_same_name() -> Result<()> {
    let store = store();
    let places = Places::new(&store);
    let mine = places.add_area("From Notes")?;
    let imports = Imports::new(&store);
    let first = imports.folder(" From Notes ", &[file("a.txt", "a", None)])?;
    let second = imports.folder("From Notes", &[file("b.txt", "b", None)])?;
    let name = |s: &ImportSummary| -> Result<Option<String>> {
        let id = s.area_id.clone().unwrap_or_default();
        Ok(places.area_by_id(&id)?.map(|a| a.name))
    };
    assert_eq!(name(&first)?, Some("From Notes (2)".into()));
    assert_eq!(name(&second)?, Some("From Notes (3)".into()));
    assert!(Notes::new(&store).inside(&mine.id)?.is_empty());
    assert!(imports.folder("  ", &[file("a.txt", "a", None)]).is_err());
    Ok(())
}

#[test]
fn nothing_to_import_makes_no_area_and_writes_nothing() -> Result<()> {
    let store = store();
    let imports = Imports::new(&store);
    let none = [
        file("photo.jpg", "x", None),
        file(".hidden.md", "x", None),
        file("README", "x", None),
    ];
    for files in [&none[..0], &none[..]] {
        let summary = imports.folder("From Notes", files)?;
        assert_eq!(summary.area_id, None);
        assert_eq!((summary.notes, summary.not_text), (0, files.len()));
    }
    assert!(store.all_entries()?.is_empty());
    Ok(())
}

#[test]
fn notes_of_one_date_go_by_name_as_code_points() -> Result<()> {
    let store = store();
    let files = [
        file("😀.txt", "emoji", Some(march())),
        file("～.txt", "tilde", Some(march())),
        file("b.txt", "b", Some(march())),
        file("a.txt", "a", Some(march())),
    ];
    Imports::new(&store).folder("From Notes", &files)?;
    let notes = Notes::new(&store);
    let mut lasts = Vec::new();
    for e in store.all_entries()? {
        if e.field == keys::TYPE && e.entity.starts_with("note:") {
            let text = notes.by_id(&e.entity)?.map(|n| n.text).unwrap_or_default();
            lasts.push(text.rsplit('\n').next().unwrap_or_default().to_string());
        }
    }
    assert_eq!(lasts, ["a", "b", "tilde", "emoji"]);
    Ok(())
}

#[test]
fn a_file_of_unknown_zero_or_later_time_is_dated_now() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            file("none.txt", "none", None),
            file("zero.txt", "zero", Some(DateTime::UNIX_EPOCH)),
            file("later.txt", "later", Some(at(2030, 1, 1, 0))),
        ],
    )?;
    for note in inside(&store, &summary)? {
        assert_eq!(note.captured, now());
    }
    Ok(())
}

/// The one Note `f` makes, as shown.
fn only(f: ImportFile) -> Result<String> {
    let store = store();
    let summary = Imports::new(&store).folder("From Notes", std::slice::from_ref(&f))?;
    assert_eq!(summary.notes, 1, "{}", f.name);
    let notes = inside(&store, &summary)?;
    Ok(notes.first().map(|n| n.shown_text()).unwrap_or_default())
}

#[test]
fn a_file_is_read_as_utf8_without_its_mark_or_as_windows_1252() -> Result<()> {
    let mut bom = vec![0xef, 0xbb, 0xbf];
    bom.extend(b"Shop\r\nmilk\reggs\r\n");
    assert_eq!(only(raw("Shop.txt", bom, None))?, "Shop\nmilk\neggs");
    let mut twice = vec![0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf];
    twice.extend(b"Two");
    assert_eq!(only(raw("Two.txt", twice, None))?, "\u{feff}Two");
    let mut bytes = b"Herr M".to_vec();
    bytes.extend([0xfc]);
    bytes.extend(b"ller ");
    bytes.extend([0x80, 0x81]);
    assert_eq!(
        only(raw("Bill.txt", bytes, None))?,
        "Bill\nHerr Müller €\u{81}"
    );
    Ok(())
}

#[test]
fn a_file_with_a_zero_byte_or_hidden_is_no_text() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            raw("wide.txt", vec![0xff, 0xfe, 0x41, 0x00], None),
            file("._mac.md", "x", None),
        ],
    )?;
    assert_eq!((summary.not_text, summary.area_id), (2, None));
    Ok(())
}

#[test]
fn a_file_is_taken_by_its_extension_in_any_case() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            file("A.MD", "alpha", None),
            file("b.Markdown", "beta", None),
            file("sub/c.TXT", "gamma", None),
            file("win\\d.txt", "delta", None),
            file("e.text", "epsilon", None),
            file("f.mar\u{212a}down", "kelvin", None),
        ],
    )?;
    assert_eq!((summary.notes, summary.not_text), (4, 2));
    let mut texts: Vec<_> = inside(&store, &summary)?
        .iter()
        .map(|n| n.shown_text())
        .collect();
    texts.sort();
    assert_eq!(texts, ["# A\nalpha", "# b\nbeta", "c\ngamma", "d\ndelta"]);
    Ok(())
}

#[test]
fn a_file_is_titled_by_its_name_unless_its_first_line_says_so() -> Result<()> {
    let cases = [
        ("Dentist.txt", "Tuesday 9:00", "Dentist\nTuesday 9:00"),
        (
            "dentist.txt",
            "\n  \nDENTIST\nTuesday\n\n",
            "DENTIST\nTuesday",
        ),
        ("Shop.md", "## *Shop*\n- milk", "## *Shop*\n- milk"),
        ("2*3 [x].md", "- a", "# 2\\*3 \\[x]\n- a"),
        ("Call the bank.md", "", "# Call the bank"),
        ("  .txt", "  kept  ", "  kept"),
    ];
    for (name, text, shown) in cases {
        assert_eq!(only(file(name, text, None))?, shown, "{name}");
    }
    let store = store();
    let summary = Imports::new(&store).folder("From Notes", &[file("  .txt", " \n ", None)])?;
    assert_eq!((summary.not_text, summary.area_id), (1, None));
    Ok(())
}

#[test]
fn a_file_gets_its_boxes() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[file("Trip.md", "# Trip\n- ☐ passport", None)],
    )?;
    let notes = inside(&store, &summary)?;
    assert_eq!(notes.first().map(|n| n.boxes.len()), Some(1));
    assert_eq!(
        notes.first().map(|n| n.shown_text()),
        Some("# Trip\n- ☐ passport".into())
    );
    Ok(())
}

#[test]
fn a_file_or_text_too_large_is_skipped() -> Result<()> {
    let store = store();
    let summary = Imports::new(&store).folder(
        "From Notes",
        &[
            raw("big.txt", vec![b'a'; MAX_FILE_BYTES + 1], None),
            // Under the file's limit, but each quote stored as two bytes.
            file("quotes.txt", &"\"".repeat(MAX_TEXT_BYTES / 2 + 1), None),
            file("fits.txt", &"x".repeat(MAX_TEXT_BYTES - 100), None),
        ],
    )?;
    assert_eq!((summary.too_large, summary.notes), (2, 1));
    Ok(())
}

#[test]
fn beyond_the_total_the_oldest_files_are_skipped() -> Result<()> {
    let store = store();
    let text = "a".repeat(256_000);
    let files: Vec<_> = (0..132)
        .map(|i| {
            file(
                &format!("{i}.txt"),
                &text,
                Some(march() + chrono::Duration::minutes(i)),
            )
        })
        .collect();
    let summary = Imports::new(&store).folder("From Notes", &files)?;
    assert_eq!((summary.notes, summary.too_large), (131, 1));
    let dates: Vec<_> = inside(&store, &summary)?
        .iter()
        .map(|n| n.captured)
        .collect();
    assert!(!dates.contains(&march()));
    assert_eq!(
        dates.first(),
        Some(&(march() + chrono::Duration::minutes(131)))
    );
    Ok(())
}

mod keep {
    use super::*;
    use serde_json::json;

    fn made() -> DateTime<Utc> {
        at(2024, 3, 1, 8) + chrono::Duration::microseconds(250)
    }

    fn changed() -> DateTime<Utc> {
        at(2024, 6, 2, 9) + chrono::Duration::minutes(30)
    }

    const PNG: [u8; 10] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2];

    /// A Keep note's `.json` at `Takeout/Keep/<name>.json`.
    fn note(name: &str, json: Value) -> ImportFile {
        file(
            &format!("Takeout/Keep/{name}.json"),
            &json.to_string(),
            None,
        )
    }

    /// `json` with the times Keep says a note was made and changed.
    fn timed(created: DateTime<Utc>, edited: DateTime<Utc>, json: Value) -> Value {
        let mut json = json;
        json["createdTimestampUsec"] = json!(created.timestamp_micros());
        json["userEditedTimestampUsec"] = json!(edited.timestamp_micros());
        json
    }

    fn times(json: Value) -> Value {
        timed(made(), changed(), json)
    }

    fn keep(store: &Store, entries: &[ImportFile]) -> Result<ImportSummary> {
        Imports::new(store).keep("From Google Keep", entries)
    }

    fn only(store: &Store, summary: &ImportSummary) -> Result<crate::notes::Note> {
        inside(store, summary)?
            .pop()
            .ok_or(Error::Invalid("no Note".into()))
    }

    #[test]
    fn a_note_keeps_its_title_text_and_date_its_labels_as_tags() -> Result<()> {
        let store = store();
        let summary = keep(
            &store,
            &[note(
                "Dentist",
                times(json!({
                    "title": "Dentist",
                    "textContent": "Tuesday 9:00\r\nbring the X-ray #errands",
                    "textContentHtml": "<p>ignored</p>",
                    "isPinned": true,
                    "color": "RED",
                    "labels": [
                        {"name": "Errands"},
                        {"name": "Home Garden"},
                        {"name": "home-garden"},
                        {"name": "2024"},
                        {"name": "🛒"},
                        {"nom": "x"},
                    ],
                })),
            )],
        )?;
        assert_eq!((summary.notes, summary.skipped()), (1, 0));
        let id = summary.area_id.clone().unwrap_or_default();
        let area = Places::new(&store).area_by_id(&id)?;
        assert_eq!(area.map(|a| a.name), Some("From Google Keep".into()));
        let dentist = only(&store, &summary)?;
        assert_eq!(
            dentist.text,
            "Dentist\nTuesday 9:00\nbring the X-ray #errands\n#Home-Garden #2024"
        );
        assert_eq!(dentist.text_format, PLAIN);
        assert_eq!(dentist.captured, made());
        for entry in store.history(&dentist.id)? {
            assert_eq!(entry.date_at, made());
        }
        assert!(Notes::new(&store).unsorted()?.is_empty());
        assert_eq!(store.current(&dentist.id, keys::NUDGE)?, None);
        Ok(())
    }

    #[test]
    fn a_list_becomes_a_checklist_its_lines_checked_when_the_note_last_changed() -> Result<()> {
        let store = store();
        let summary = keep(
            &store,
            &[note(
                "Groceries",
                times(json!({
                    "title": "Groceries",
                    "textContent": "",
                    "listContent": [
                        {"text": "Milk", "isChecked": true},
                        {"text": " Eggs\nlarge ", "isChecked": false},
                        {"text": "  ", "isChecked": true},
                        {"text": "Bread", "isChecked": "yes"},
                        {"isChecked": true},
                    ],
                    "labels": [{"name": "shopping"}],
                })),
            )],
        )?;
        assert_eq!(summary.checklists, 1);
        let groceries = only(&store, &summary)?;
        assert_eq!(groceries.text, "Groceries\n#shopping");
        let list = Checklists::new(&store)
            .by_note(&groceries.id)?
            .ok_or(Error::Invalid("no Checklist".into()))?;
        let lines: Vec<_> = list
            .lines
            .iter()
            .map(|l| (l.text.as_str(), l.checked))
            .collect();
        assert_eq!(
            lines,
            [("Milk", true), ("Eggs large", false), ("Bread", false)]
        );
        let milk = &list.lines[0];
        assert_eq!(milk.checked_at, Some(changed()));
        let dates: HashMap<String, DateTime<Utc>> = store
            .history(&milk.id)?
            .into_iter()
            .map(|e| (e.field, e.date_at))
            .collect();
        assert_eq!(dates.get(checklist_keys::CHECKED), Some(&changed()));
        assert_eq!(dates.get(checklist_keys::TYPE), Some(&made()));
        assert!(!groceries.spawned);
        Ok(())
    }

    #[test]
    fn a_list_starts_off_the_agenda_until_switched_on() -> Result<()> {
        let store = store();
        let summary = keep(
            &store,
            &[note(
                "Groceries",
                times(json!({
                    "title": "Groceries",
                    "listContent": [{"text": "Milk", "isChecked": false}],
                })),
            )],
        )?;
        let lists = Checklists::new(&store);
        let id = only(&store, &summary)?.id;
        assert_eq!(lists.on_agenda(&id)?, ListOnAgenda::Off);
        assert!(lists.agenda_rows()?.is_empty());
        lists.set_on_agenda(&id, ListOnAgenda::Lines)?;
        let shown: Vec<_> = lists
            .agenda_rows()?
            .into_iter()
            .map(|r| r.list.note_id)
            .collect();
        assert_eq!(shown, [id]);
        Ok(())
    }

    #[test]
    fn a_list_checked_before_it_was_made_is_checked_when_it_was_made() -> Result<()> {
        let store = store();
        let summary = keep(
            &store,
            &[note(
                "Odd",
                timed(
                    made(),
                    at(2020, 1, 1, 0),
                    json!({"title": "Odd", "listContent": [{"text": "Milk", "isChecked": true}]}),
                ),
            )],
        )?;
        let id = only(&store, &summary)?.id;
        let list = Checklists::new(&store).by_note(&id)?;
        let checked: Vec<_> = list
            .iter()
            .flat_map(|l| &l.lines)
            .map(|l| l.checked_at)
            .collect();
        assert_eq!(checked, [Some(made())]);
        Ok(())
    }

    #[test]
    fn a_note_in_the_trash_is_skipped_an_archived_one_is_made() -> Result<()> {
        let store = store();
        let summary = keep(
            &store,
            &[
                note("Old", times(json!({"title": "Old", "isTrashed": true}))),
                note("Kept", times(json!({"title": "Kept", "isArchived": true}))),
                note("Odd", times(json!({"title": "Odd", "isTrashed": "yes"}))),
            ],
        )?;
        assert_eq!(
            (summary.trashed, summary.skipped(), summary.notes),
            (1, 1, 2)
        );
        let mut texts: Vec<_> = inside(&store, &summary)?
            .into_iter()
            .map(|n| n.text)
            .collect();
        texts.sort();
        assert_eq!(texts, ["Kept", "Odd"]);
        Ok(())
    }

    #[test]
    fn photos_it_names_are_its_photos_missing_ones_other_kinds_and_large_ones_counted() -> Result<()>
    {
        let store = store();
        let jpeg = vec![0xff, 0xd8, 0xff, 0xd9];
        let summary = keep(
            &store,
            &[
                note(
                    "Garden",
                    times(json!({
                        "title": "Garden",
                        "attachments": [
                            {"filePath": "a1.png", "mimetype": "image/png"},
                            {"filePath": "b2.jpeg", "mimetype": "image/jpeg"},
                            {"filePath": "a1.png", "mimetype": "image/png"},
                            {"filePath": "gone.webp", "mimetype": "image/webp"},
                            {"filePath": "hum.3gp", "mimetype": "audio/3gpp"},
                            {"filePath": "big.GIF", "mimetype": "image/gif"},
                            {"mimetype": "image/png"},
                        ],
                    })),
                ),
                note(
                    "Shed",
                    times(json!({"attachments": [{"filePath": "a1.png"}]})),
                ),
                raw("Takeout/Keep/a1.png", PNG.to_vec(), None),
                raw("Takeout/Keep/b2.jpg", jpeg.clone(), None),
                raw("Takeout/Keep/hum.3gp", vec![1, 2, 3], None),
                raw("Takeout/Keep/big.GIF", vec![0; MAX_PHOTO_BYTES + 1], None),
            ],
        )?;
        assert_eq!((summary.notes, summary.photos, summary.missing), (2, 3, 2));
        assert_eq!(
            (summary.not_photo, summary.too_large, summary.skipped()),
            (1, 1, 4)
        );
        let mut both = inside(&store, &summary)?;
        both.sort_by(|a, b| a.text.cmp(&b.text));
        let [shed, garden] = &both[..] else {
            return Err(Error::Invalid("two Notes".into()));
        };
        assert_eq!(garden.photos.len(), 2);
        assert_eq!(store.photo(&garden.photos[0]), Some(PNG.to_vec()));
        assert_eq!(store.photo(&garden.photos[1]), Some(jpeg));
        // A Note of a photo alone has no text, as at Capture.
        assert_eq!(shed.text, "");
        assert_eq!(shed.photos, garden.photos[..1]);
        Ok(())
    }

    #[test]
    fn only_the_entries_of_a_keep_folder_count_when_there_is_one() -> Result<()> {
        let store = store();
        let dentist = times(json!({"title": "Dentist"})).to_string();
        let walk = times(json!({"title": "Walk"})).to_string();
        let summary = keep(
            &store,
            &[
                file("Takeout/Keep/Dentist.json", &dentist, None),
                file("Takeout/Keep/Dentist.html", "<html>Dentist</html>", None),
                file("Takeout/Keep/Labels.txt", "Errands\n", None),
                file("Takeout/Fit/Walk.json", &walk, None),
                file("Takeout/archive_browser.html", "<html></html>", None),
            ],
        )?;
        assert_eq!((summary.notes, summary.skipped()), (1, 0));
        assert_eq!(only(&store, &summary)?.text, "Dentist");
        let loose = keep(
            &store,
            &[
                file("Dentist.json", &dentist, None),
                file("sub\\Walk.json", &walk, None),
            ],
        )?;
        assert_eq!(loose.notes, 2);
        Ok(())
    }

    #[test]
    fn what_is_no_keep_note_or_holds_nothing_is_no_text() -> Result<()> {
        let store = store();
        let summary = keep(
            &store,
            &[
                file("Takeout/Keep/Broken.json", r#"{"title": "Broken""#, None),
                file("Takeout/Keep/List.json", "[1, 2]", None),
                note("Untimed", json!({"title": "Untimed"})),
                note(
                    "Float",
                    json!({
                        "title": "Float",
                        "createdTimestampUsec": 1.7e15,
                        "userEditedTimestampUsec": 1.7e15,
                    }),
                ),
                note("Empty", times(json!({"title": " ", "textContent": "\n"}))),
                note("Labels", times(json!({"labels": [{"name": "Errands"}]}))),
                note("No lines", times(json!({"listContent": []}))),
                raw("Takeout/Keep/Wide.json", vec![0xff, 0xfe, 0x7b, 0x00], None),
                raw(
                    "Takeout/Keep/Big.json",
                    vec![0x20; MAX_FILE_BYTES + 1],
                    None,
                ),
            ],
        )?;
        assert_eq!(summary.area_id, None);
        assert_eq!((summary.not_text, summary.too_large), (8, 1));
        assert!(store.all_entries()?.is_empty());
        Ok(())
    }

    #[test]
    fn a_note_is_captured_when_made_else_when_last_changed_else_now() -> Result<()> {
        let store = store();
        let later = now() + chrono::Duration::days(1);
        let summary = keep(
            &store,
            &[
                note(
                    "Zero",
                    timed(DateTime::UNIX_EPOCH, changed(), json!({"title": "Zero"})),
                ),
                note("Later", timed(later, later, json!({"title": "Later"}))),
                note(
                    "Text",
                    json!({
                        "createdTimestampUsec": made().timestamp_micros().to_string(),
                        "userEditedTimestampUsec": changed().timestamp_micros(),
                        "title": "Text",
                    }),
                ),
            ],
        )?;
        let dates: HashMap<String, DateTime<Utc>> = inside(&store, &summary)?
            .into_iter()
            .map(|n| (n.text, n.captured))
            .collect();
        let expected: HashMap<String, DateTime<Utc>> = [
            ("Zero".to_string(), changed()),
            ("Later".to_string(), now()),
            ("Text".to_string(), changed()),
        ]
        .into();
        assert_eq!(dates, expected);
        Ok(())
    }

    #[test]
    fn says_which_files_it_reads_and_how_much_of_them() {
        assert_eq!(keep_reads("Takeout/Keep/a.json"), Some(MAX_FILE_BYTES));
        assert_eq!(keep_reads("Takeout/Keep/A.JSON"), Some(MAX_FILE_BYTES));
        for photo in ["a.jpg", "a.JPEG", "a.png", "a.webp", "a.gif"] {
            assert_eq!(keep_reads(&format!("Keep/{photo}")), Some(MAX_PHOTO_BYTES));
        }
        for other in ["a.html", "Labels.txt", "a.3gp", "._a.json", "a"] {
            assert_eq!(keep_reads(&format!("Keep/{other}")), None, "{other}");
        }
    }
}

#[test]
fn only_formats_known_here_are_taken() {
    let store = store();
    let note = ImportedNote {
        name: "a".into(),
        text: "a".into(),
        format: "x-future".into(),
        captured: now(),
        lines: None,
        photos: Vec::new(),
    };
    assert!(
        Imports::new(&store)
            .notes("From Keep", &[note], 0, 0)
            .is_err()
    );
}
