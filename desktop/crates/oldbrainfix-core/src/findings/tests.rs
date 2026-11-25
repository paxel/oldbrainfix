use super::*;
use crate::Result;
use crate::notes::{NewFile, Notes};
use crate::search::Search;
use crate::text_format::{MARKDOWN, PLAIN};

#[test]
fn folding_ignores_case_accents_and_umlauts_and_writes_ss_for_sharp_s() {
    assert_eq!(fold("Müller"), "muller");
    assert_eq!(fold("STRAßE"), "strasse");
    assert_eq!(fold("Mu\u{308}ller"), "muller");
    assert_eq!(fold("Ærø Œuvre ẞ"), "aero oeuvre ss");
    assert_eq!(fold("Ελλάδα Łódź Đorđe İzmir"), "ελλαδα lodz dorde izmir");
    assert_eq!(fold("plain ASCII 123"), "plain ascii 123");
    assert_eq!(fold("🧠 日記"), "🧠 日記");
    assert!(!fold("Müller").contains(&fold("mueller")));
    assert!(!fold("Müller").contains(&fold("muler")));
}

#[test]
fn a_text_folds_as_its_characters_fold_one_by_one() {
    let text = "Grüße aus Österreich, ΟΔΌΣ, Ǆ, \u{130}, Σ, 𝔸 ﬃ";
    let each: String = text.chars().map(|c| fold(&c.to_string())).collect();
    assert_eq!(fold(text), each);
}

#[test]
fn every_place_has_its_line_and_its_context() {
    let text = "Call Herr Müller\nthen muller again, MULLER";
    let found = in_text(text, PLAIN, " muller ");
    let places: Vec<_> = found.iter().map(|f| (f.start, f.end, f.line)).collect();
    assert_eq!(places, vec![(10, 17, 0), (23, 29, 1), (37, 43, 1)]);
    assert_eq!(found[0].context, "Call Herr Müller");
    assert_eq!(
        &found[0].context[found[0].context_start..found[0].context_end],
        "Müller"
    );
    assert_eq!(found[2].cell, None);
    assert_eq!(found[2].part, FindingPart::Text);
}

#[test]
fn a_blank_query_or_one_folding_to_nothing_finds_nothing() {
    assert!(in_text("a b", PLAIN, "  ").is_empty());
    assert!(in_text("e\u{301}", PLAIN, "\u{301}").is_empty());
}

#[test]
fn half_of_a_folded_letter_takes_the_whole_letter_once() {
    let found = in_text("Straße", PLAIN, "s");
    let places: Vec<_> = found.iter().map(|f| (f.start, f.end)).collect();
    assert_eq!(places, vec![(0, 1), (4, 6)]);
    assert_eq!(in_text("Mu\u{308}ller", PLAIN, "mu")[0].end, 4);
}

#[test]
fn a_formatted_text_is_found_as_it_reads_its_marks_inside_a_place() {
    let text = "# Title\n**Mül**ler and \\*star\\* and `co de`";
    let found = in_text(text, MARKDOWN, "muller");
    assert_eq!(found.len(), 1);
    assert_eq!(&text[found[0].start..found[0].end], "Mül**ler");
    assert_eq!(found[0].line, 1);
    assert_eq!(found[0].context, "Müller and *star* and co de");
    let star = in_text(text, MARKDOWN, "*star*");
    assert_eq!(&text[star[0].start..star[0].end], "\\*star\\*");
    let code = in_text(text, MARKDOWN, "co de");
    assert_eq!(&text[code[0].start..code[0].end], "co de");
    assert!(in_text(text, MARKDOWN, "#").is_empty());
}

#[test]
fn a_place_in_a_table_names_its_table_row_and_column() {
    let text = "Before\n\n| a | b |\n| --- | --- |\n| x | y |\n\n\
                | Name | City |\n| --- | --- |\n| Ann | Köln |\n| Bo | Graz `\\|` |";
    let found = in_text(text, MARKDOWN, "koln");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].cell,
        Some(FindingCell {
            table: 1,
            table_start: text.find("| Name").unwrap_or_default(),
            row: 1,
            column: 1,
        })
    );
    assert_eq!(found[0].context, "Ann Köln");
    assert_eq!(&text[found[0].start..found[0].end], "Köln");
    let across = in_text(text, MARKDOWN, "ann kö");
    assert_eq!(across[0].cell.map(|c| c.column), Some(0));
    assert_eq!(&text[across[0].start..across[0].end], "Ann | Kö");
    let pipe = in_text(text, MARKDOWN, "graz |");
    assert_eq!(&text[pipe[0].start..pipe[0].end], "Graz `\\|");
    assert_eq!(pipe[0].cell.map(|c| c.row), Some(2));
    assert!(in_text(text, MARKDOWN, "---").is_empty());
}

#[test]
fn a_long_line_is_cut_around_the_place_at_whole_words() {
    let words: Vec<String> = (0..40).map(|i| format!("word{i}")).collect();
    let found = in_text(&words.join(" "), PLAIN, "word20");
    let f = &found[0];
    assert!(f.context.starts_with('…') && f.context.ends_with('…'));
    assert_eq!(&f.context[f.context_start..f.context_end], "word20");
    assert!(f.context.chars().count() <= CONTEXT_LENGTH + 2);
    assert!(f.context.contains(" word19 word20 word21 "));
}

#[test]
fn a_note_has_places_in_its_text_media_files_and_sections() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture_with_files(
        "Müller called",
        None,
        None,
        &[],
        &[b"voice".to_vec()],
        &[NewFile {
            name: "muller.txt",
            media_type: "text/plain",
            bytes: b"Dear Mr Muller",
            pages: None,
        }],
    )?;
    notes.set_transcript(&note.id, &note.recordings[0], "Herr Müller says hello")?;
    notes.set_attachment_text(&note.id, &note.attachments[0].hash, "Dear Mr Muller")?;
    let section = notes.amend_as(&note.id, "and **Müller** again", MARKDOWN)?;
    let Some(now) = notes.by_id(&note.id)? else {
        panic!("no Note");
    };
    let found = of_note(&now, "MULLER", &notes.sections(&note.id)?);
    let parts: Vec<_> = found.iter().map(|f| (f.part, f.section.clone())).collect();
    assert_eq!(
        parts,
        vec![
            (FindingPart::Text, None),
            (FindingPart::Transcript, None),
            (FindingPart::FileName, None),
            (FindingPart::AttachmentText, None),
            (FindingPart::Text, Some(section.id)),
        ]
    );
    assert_eq!(found[1].key, note.recordings[0]);
    assert_eq!(found[1].context, "Herr Müller says hello");
    Ok(())
}

#[test]
fn search_folds_with_the_index_and_without_it() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Herr Müller, Hauptstraße 4", None, None)?;
    let amended = notes.capture("see below", None, None)?;
    notes.amend(&amended.id, "ÉCOLE")?;
    for search in [Search::new(&store), Search::scanning(&store)] {
        let ids = |q: &str| -> Result<Vec<String>> {
            Ok(search.notes(q, None)?.into_iter().map(|n| n.id).collect())
        };
        assert_eq!(ids("muller")?, vec![note.id.clone()]);
        assert_eq!(ids("HAUPTSTRASSE")?, vec![note.id.clone()]);
        assert_eq!(ids("ecole")?, vec![amended.id.clone()]);
        assert!(ids("mueller")?.is_empty());
        assert!(ids("\u{301}")?.is_empty());
        assert!(search.among("\u{301}", notes.all()?)?.is_empty());
    }
    Ok(())
}

#[test]
fn an_index_of_texts_only_lowered_is_built_anew_folded() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Herr Müller", None, None)?;
    let ids = || -> Result<Vec<String>> {
        Ok(Search::new(&store)
            .notes("muller", None)?
            .into_iter()
            .map(|n| n.id)
            .collect())
    };
    assert_eq!(ids()?, vec![note.id.clone()]);
    // As the index of version 1 held it, up to date.
    store.with_connection(|db| {
        db.execute("DELETE FROM search_texts", [])?;
        db.execute(
            "INSERT INTO search_texts (entity, text) VALUES (?, ?)",
            rusqlite::params![note.id, "herr müller"],
        )?;
        db.execute(
            "UPDATE search_state SET value = 1 WHERE key = 'version'",
            [],
        )?;
        Ok(())
    })?;
    assert_eq!(ids()?, vec![note.id.clone()]);
    Ok(())
}

#[test]
fn a_note_search_finds_by_its_own_texts_has_places_the_others_none() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    for (text, format) in [
        ("Müller", PLAIN),
        ("Straße 4", MARKDOWN),
        ("**Gr**üße", MARKDOWN),
        ("| a | b |\n| - | - |\n| Köln | x |", MARKDOWN),
        ("nothing here", PLAIN),
        ("Mu\u{308}nchen", PLAIN),
    ] {
        notes.capture_as(text, format, None, None)?;
    }
    let search = Search::new(&store);
    for query in [
        "muller", "strasse", "grusse", "koln x", "munchen", "e", "MÜLLER", "ss",
    ] {
        let found: HashSet<String> = search
            .notes(query, None)?
            .into_iter()
            .map(|n| n.id)
            .collect();
        for note in notes.all()? {
            assert_eq!(
                !of_note(&note, query, &[]).is_empty(),
                found.contains(&note.id),
                "{query} in {}",
                note.text
            );
        }
    }
    Ok(())
}
