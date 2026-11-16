use super::*;

#[test]
fn the_menu_names_each_state_after_its_character() {
    let en = L10n::new("en");
    assert_eq!(menu_entry(&en, BoxState::Done), "✅ Check");
    assert_eq!(menu_entry(&en, BoxState::Failed), "❌ Fail");
    assert_eq!(menu_entry(&en, BoxState::Open), "☐ Open");
    assert_eq!(box_label(&en, BoxState::Done), "Checked");
    let de = L10n::new("de");
    assert_eq!(menu_entry(&de, BoxState::Open), "☐ Wieder öffnen");
    assert_eq!(box_label(&de, BoxState::Failed), "Gescheitert");
}

#[test]
fn a_tag_is_read_as_a_search_for_it_as_written() {
    assert_eq!(
        L10n::new("en").search_for_tag("#Garten"),
        "Search for #Garten"
    );
    assert_eq!(
        L10n::new("de").search_for_tag("#Garten"),
        "Nach #Garten suchen"
    );
}

#[test]
fn a_tags_job_colours_the_tag_alone() {
    let text = "Seeds #Garten- und #1 now";
    let font = FontId::proportional(14.0);
    let job = tag_job(text, (0, text.len()), font, (Color32::WHITE, Color32::RED));
    assert_eq!(job.text, text);
    let red: Vec<_> = job
        .sections
        .iter()
        .filter(|s| s.format.color == Color32::RED)
        .filter_map(|s| job.text.get(s.byte_range.start.0..s.byte_range.end.0))
        .collect();
    assert_eq!(red, ["#Garten"]);
    let part = tag_job(
        text,
        (6, 13),
        FontId::proportional(14.0),
        (Color32::WHITE, Color32::RED),
    );
    assert_eq!(part.text, "#Garten");
}

/// "Pack ☐" with the box of the id `a7k2`, and the box.
fn boxed() -> (String, FoundBox) {
    let text = format!("Pack \u{2610}{}", text_boxes::encode_id("a7k2"));
    let found = text_boxes::find(&text).remove(0);
    (text, found)
}

#[test]
fn an_edit_fields_job_shows_the_state_and_hides_the_id() {
    let (text, found) = boxed();
    let font = FontId::proportional(14.0);
    let job = tag_job(&text, (0, text.len()), font, (Color32::WHITE, Color32::RED));
    let mut look = BoxLook::default();
    look.states.insert("a7k2".into(), BoxState::Done);
    look.after.insert("a7k2".into(), "You, Mon 11:00".into());
    let laid = field_job(job, &look, &|_| 40.0);
    // As many characters, the glyph of its state.
    assert_eq!(laid.text.chars().count(), text.chars().count());
    assert_eq!(laid.text.get(found.start..found.id_start), Some("\u{2705}"));
    let hidden: Vec<_> = laid
        .sections
        .iter()
        .filter(|s| s.format.color == Color32::TRANSPARENT)
        .collect();
    assert_eq!(
        hidden.first().map(|s| s.byte_range.start.0),
        Some(found.id_start)
    );
    assert_eq!(hidden.last().map(|s| s.byte_range.end.0), Some(found.end));
    assert_eq!(
        hidden.last().map(|s| s.format.extra_letter_spacing),
        Some(40.0)
    );
}

#[test]
fn the_cursor_steps_over_a_box_and_follows_a_save() {
    let (text, _) = boxed();
    let chars = box_chars(&text);
    assert_eq!(chars, [(5, 11)]);
    // Into the id from before it: to its end; from after it: before it.
    assert_eq!(kept_out(&chars, 6, 5), 11);
    assert_eq!(kept_out(&chars, 10, 11), 5);
    assert_eq!(kept_out(&chars, 4, 11), 4);
    // A save gave the box its id: the cursor after it stays after it.
    assert_eq!(followed("Pack \u{2610}!", &format!("{text}!"), 7), 12);
    assert_eq!(followed("Pack \u{2610}!", &format!("{text}!"), 2), 2);
}

#[test]
fn a_cut_through_an_id_leaves_none_of_it() {
    let (text, _) = boxed();
    let cut: String = text.chars().take(8).collect();
    assert_eq!(without_stray_ids(&cut).as_deref(), Some("Pack \u{2610}"));
    assert_eq!(without_stray_ids(&text), None);
    // A subdivision's flag keeps its tags.
    let flag = "\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}\u{E0067}\u{E007F}";
    assert_eq!(without_stray_ids(flag), None);
}
