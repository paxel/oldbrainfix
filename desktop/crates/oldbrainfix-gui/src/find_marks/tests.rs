use super::*;
use oldbrainfix_core::text_boxes;
use oldbrainfix_core::text_format::{MARKDOWN, PLAIN};

/// A box written with the id `id`, as a saved text holds it.
fn boxed(glyph: &str, id: &str) -> String {
    format!("{glyph}{}", text_boxes::encode_id(id))
}

/// What of the source `text` each place found covers.
fn covered<'a>(text: &'a str, hits: &Hits) -> Vec<&'a str> {
    hits.found
        .iter()
        .filter_map(|h| text.get(h.source.0..h.source.1))
        .collect()
}

#[test]
fn a_text_without_ids_shows_as_it_is() {
    let (shown, cuts) = shown_of("Müller ☐ milk");
    assert_eq!(shown, "Müller ☐ milk");
    assert!(cuts.is_empty());
    assert_eq!(source_at(&cuts, 3), 3);
}

#[test]
fn a_place_after_a_box_with_an_id_lies_where_its_source_has_it() {
    let text = format!("{} buy milk at Müller", boxed("☐", "ab12"));
    let (shown, _) = shown_of(&text);
    assert_eq!(shown, "☐ buy milk at Müller");
    assert_eq!(shown, text_boxes::strip(&text));
    let hits = hits_in(&text, PLAIN, "muller");
    assert_eq!(covered(&text, &hits), ["Müller"]);
    assert_eq!(
        hits.found[0].shown,
        shown.find("Müller").unwrap_or(usize::MAX)
    );
}

#[test]
fn a_variation_selector_and_a_repeated_id_go_with_their_box() {
    let first = format!("\u{2705}\u{FE0F}{}", text_boxes::encode_id("zz99"));
    // The same id once more: no box of its own, and still no id shown.
    let again = boxed("❌", "zz99");
    let text = format!("{first} tea\n{again} Straße");
    let (shown, cuts) = shown_of(&text);
    assert_eq!(shown, "\u{2705} tea\n❌ Straße");
    assert_eq!(cuts.len(), 2);
    let hits = hits_in(&text, PLAIN, "strasse");
    assert_eq!(covered(&text, &hits), ["Straße"]);
    let hits = hits_in(&text, PLAIN, "tea");
    assert_eq!(covered(&text, &hits), ["tea"]);
}

#[test]
fn a_table_after_a_box_begins_where_its_source_has_it() -> Result<(), String> {
    let text = format!(
        "{} first\n\n| Name | City |\n| --- | --- |\n| Ann | Köln |\n",
        boxed("☐", "k3x0")
    );
    let (shown, cuts) = shown_of(&text);
    let found = findings::in_text(&shown, MARKDOWN, "koln");
    let cell = found
        .first()
        .and_then(|f| f.cell)
        .ok_or("no place in the table")?;
    assert_eq!(cell.row, 1);
    let start = source_at(&cuts, cell.table_start);
    assert!(text[start..].starts_with("| Name"));
    let hits = hits_in(&text, MARKDOWN, "koln");
    assert_eq!(covered(&text, &hits), ["Köln"]);
    Ok(())
}

#[test]
fn a_mark_cuts_a_job_where_the_places_lie() {
    let format = egui::TextFormat::simple(egui::FontId::proportional(14.0), Color32::WHITE);
    let mut job = LayoutJob::single_section("one two one".to_string(), format);
    mark_job(&mut job, &[(0, 3, false), (8, 11, true)], false);
    let tinted: Vec<(&str, Color32)> = job
        .sections
        .iter()
        .filter(|s| s.format.background != Color32::TRANSPARENT)
        .filter_map(|s| {
            let range = s.byte_range.start.0..s.byte_range.end.0;
            Some((job.text.get(range)?, s.format.background))
        })
        .collect();
    assert_eq!(
        tinted,
        [("one", tint(false, false)), ("one", tint(false, true))]
    );
    let all: usize = job
        .sections
        .iter()
        .map(|s| s.byte_range.end.0 - s.byte_range.start.0)
        .sum();
    assert_eq!(all, job.text.len());
}

#[test]
fn what_is_drawn_invisibly_stays_so() {
    let mut hidden = egui::TextFormat::simple(egui::FontId::proportional(1.0), Color32::WHITE);
    hidden.color = Color32::TRANSPARENT;
    let mut job = LayoutJob::single_section("abc".to_string(), hidden);
    mark_job(&mut job, &[(0, 3, true)], true);
    assert!(
        job.sections
            .iter()
            .all(|s| s.format.background == Color32::TRANSPARENT)
    );
}

#[test]
fn the_current_place_is_named_by_its_text_and_start() {
    let text = "milk, more milk";
    let hits = hits_in(text, PLAIN, "milk");
    let marks = FindMarks {
        bar: 1,
        query: "milk".into(),
        current: Some(Current {
            text: text_key(text),
            start: 11,
        }),
        reveal: true,
    };
    assert_eq!(hits.current(&marks), Some(1));
    let elsewhere = FindMarks {
        current: Some(Current {
            text: text_key("other"),
            start: 11,
        }),
        ..marks
    };
    assert_eq!(hits.current(&elsewhere), None);
}
