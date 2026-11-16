use super::*;

fn boxed(id: &str, state: BoxState) -> String {
    format!("{}{}", state.glyph(), encode_id(id))
}

fn counted() -> impl FnMut() -> Result<String> {
    let mut n = 0;
    move || {
        n += 1;
        Ok(n.to_string())
    }
}

#[test]
fn no_glyph_of_a_code_text_becomes_a_box() -> Result<()> {
    let mut drawn = 0;
    let text = convert("☐ x ✅", text_format::CODE, || {
        drawn += 1;
        Ok(drawn.to_string())
    })?;
    assert_eq!(text, "☐ x ✅");
    assert_eq!(drawn, 0);
    Ok(())
}

#[test]
fn a_glyph_with_an_id_is_a_box_and_a_bare_one_becomes_one() -> Result<()> {
    let text = format!(
        "a{}b ✅\u{FE0F}{} ✅\u{200D}🔥 {}",
        boxed("a7k2", BoxState::Open),
        encode_id("0000"),
        boxed("a7k2", BoxState::Done)
    );
    let found = find(&text);
    assert_eq!(found.len(), 2);
    assert_eq!(
        (found[0].start, found[0].id_start, found[0].end),
        (1, 4, 24)
    );
    assert_eq!(found[1].id_start - found[1].start, 6);
    let saved = convert(&text, text_format::PLAIN, counted())?;
    assert_eq!(find(&saved).len(), 3);
    assert_eq!(strip(&saved), "a☐b ✅ ✅\u{200D}🔥 ✅");
    assert_eq!(
        convert(&saved, text_format::PLAIN, || Err(crate::Error::Invalid(
            "no id wanted".into()
        )))?,
        saved
    );
    for id in ["A7K2", "a7k", "a7k22", "a-k2", ""] {
        assert!(!is_id(id), "{id}");
    }
    Ok(())
}

#[test]
fn an_id_the_text_holds_already_is_drawn_again() -> Result<()> {
    let mut ids = ["x", "x", "y"].into_iter();
    let mut drawn = 0;
    let saved = convert("☐ ☐", text_format::PLAIN, || {
        drawn += 1;
        Ok(ids.next().unwrap_or("z").to_string())
    })?;
    assert_eq!(drawn, 3);
    assert_eq!(
        find(&saved)
            .into_iter()
            .map(|b| b.id)
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
    Ok(())
}

#[test]
fn formatted_text_keeps_glyphs_in_code_and_link_addresses() -> Result<()> {
    let text = "- ☐ a `☐` [☐ b](https://x.org/☐)\n\n| ☐ c | d |\n| --- | --- |";
    let saved = convert(text, text_format::MARKDOWN, counted())?;
    assert_eq!(find(&saved).len(), 3);
    assert_eq!(strip(&saved), text);
    // As plain text only the glyph in the web address stays bare.
    assert_eq!(
        find(&convert(text, text_format::PLAIN, counted())?).len(),
        4
    );
    Ok(())
}

#[test]
fn a_glyph_inside_a_web_address_gets_no_id() -> Result<()> {
    for format in [text_format::PLAIN, text_format::MARKDOWN] {
        let saved = convert("https://x.org/☐ ☐", format, counted())?;
        assert!(saved.starts_with("https://x.org/☐ ☐"), "{saved:?}");
        assert_eq!(find(&saved).len(), 1);
    }
    Ok(())
}

#[test]
fn each_box_shows_its_latest_tick_without_its_id() {
    let text = format!(
        "Pack {} {} x",
        boxed("aaaa", BoxState::Open),
        boxed("bbbb", BoxState::Done)
    );
    let tick = BoxTick {
        state: BoxState::Failed,
        at: DateTime::<Utc>::UNIX_EPOCH,
        device: "d".into(),
        own: true,
    };
    let ticks = BTreeMap::from([("aaaa".to_string(), tick.clone())]);
    let shown = shown(&text, &ticks);
    assert_eq!(shown.text, "Pack ❌ ✅ x");
    for b in &shown.boxes {
        assert_eq!(&shown.text[b.start..b.end], b.state.glyph().to_string());
        assert_eq!(&text[b.id_start..b.source_end], encode_id(&b.id));
    }
    assert_eq!(shown.boxes[0].last.as_ref(), Some(&tick));
    assert_eq!(
        baked(&text, &ticks),
        format!(
            "Pack {} {} x",
            boxed("aaaa", BoxState::Failed),
            boxed("bbbb", BoxState::Done)
        )
    );
    assert_eq!(
        text_format::visible(text_format::PLAIN, &text),
        "Pack ☐ ✅ x"
    );
}
