use super::*;

/// `spans` as a short string, as the Dart core's test writes them.
fn tree(spans: &[Span]) -> String {
    spans
        .iter()
        .map(|s| match s.kind {
            SpanKind::Text => format!("\"{}\"", s.text),
            SpanKind::Code => format!("code\"{}\"", s.text),
            SpanKind::Link => format!("link<{}>({})", s.url, tree(&s.children)),
            _ => format!("{}({})", s.kind.name(), tree(&s.children)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn line(text: &str) -> String {
    tree(&parse(text)[0].spans)
}

#[test]
fn the_line_of_an_offset_counts_every_line_break_once() {
    let text = "a\r\nb\rc\nd";
    assert_eq!(
        (0..=text.len())
            .map(|i| line_at(text, i))
            .collect::<Vec<_>>(),
        [0, 0, 0, 1, 1, 2, 2, 3, 3]
    );
    assert_eq!(line_at("a\nb", 99), 1);
    assert_eq!(line_at("", 0), 0);
    assert_eq!(line_at("a\n", 2), 1);
    // An offset inside a character is taken as it comes.
    assert_eq!(line_at("☐\n☐", 1), 0);
}

#[test]
fn each_format_is_a_part_of_its_line_its_source_where_it_lies() {
    let text = "**bold** *it* ~~gone~~ `co de` [site](https://x.org)";
    assert_eq!(
        line(text),
        "bold(\"bold\") \" \" italic(\"it\") \" \" strike(\"gone\") \" \" code\"co de\" \
         \" \" link<https://x.org>(\"site\")"
    );
    let spans = &parse(text)[0].spans;
    let link = &spans[spans.len() - 1];
    assert_eq!(&text[spans[0].start..spans[0].end], "**bold**");
    assert_eq!(&text[link.content_end..link.end], "](https://x.org)");
    assert_eq!(line("***x***"), "italic(bold(\"x\"))");
    assert_eq!(line("2 * 3 * 4"), "\"2 * 3 * 4\"");
    assert_eq!(line("snake_case_name"), "\"snake_case_name\"");
    assert_eq!(line("[a [b](c)](d)"), "link<d>(\"a [b](c)\")");
    assert_eq!(line("![photo](u)"), "\"![photo](u)\"");
}

#[test]
fn offsets_are_bytes_and_never_split_a_character() {
    let text = "ü **ö** 🧠";
    let bold = &parse(text)[0].spans[1];
    assert_eq!((bold.start, bold.end), (3, 9));
    assert_eq!(word_at(text, 1), (0, 2));
    // Inside a character, the offset goes back to its start.
    assert_eq!(word_at("🧠x", 2), (0, 0));
    assert_eq!(word_at("🧠x", 4), (4, 5));
    let change = toggle("ü", 1, 1, SpanKind::Bold).ok();
    assert_eq!(change.map(|c| c.text), Some("**ü**".to_string()));
    assert!(toggle("x", 0, 1, SpanKind::Link).is_err());
}

#[test]
fn a_plain_text_escaped_reads_as_itself() {
    for text in [
        "# x\n1. y\n* z\n> q",
        "**b** `c` [l](u) ~~s~~ snake_case #my_tag \\* \\",
        "ü_ü _ü 🧠~~",
        "| a | b |\n|---|:-:|\n| c | d |",
        "- | -\n|-|\nx",
    ] {
        let escaped = escape(text);
        assert_eq!(plain(&escaped), text, "{escaped}");
        assert!(!uses(&escaped), "{escaped}");
    }
}

#[test]
fn a_tag_in_code_or_in_a_links_destination_is_no_tag() {
    let text = "#a `#b` [#c](https://x.org/#d) #e\n# Heading #f";
    let found: Vec<String> = tags(text).into_iter().map(|t| t.tag).collect();
    assert_eq!(found, ["#a", "#c", "#e", "#f"]);
    // As plain text all count.
    assert_eq!(text_tags::find(text).len(), 5);
}

#[test]
fn a_tag_inside_a_web_address_is_no_tag() {
    let text = "see https://x.org/?#frag and #a";
    let tags_of =
        |found: Vec<FoundTag>| -> Vec<String> { found.into_iter().map(|t| t.tag).collect() };
    assert_eq!(tags_of(text_tags::find(text)), ["#frag", "#a"]);
    assert_eq!(tags_of(plain_tags(text)), ["#a"]);
    assert_eq!(tags_of(tags(text)), ["#a"]);
}

fn plain_links(text: &str) -> Vec<&str> {
    plain_autolinks(text)
        .into_iter()
        .map(|(s, e)| &text[s..e])
        .collect()
}

#[test]
fn a_bare_http_or_https_address_is_one_in_any_case() {
    assert_eq!(
        plain_links("see https://x.org/a?b=1 and HTTP://y.de now"),
        ["https://x.org/a?b=1", "HTTP://y.de"]
    );
    assert!(plain_links("www.x.org ftp://x.org mailto:a@x.org https:// x").is_empty());
    assert!(plain_links("xhttps://x.org 1https://x.org").is_empty());
    assert_eq!(plain_links("(https://x.org)"), ["https://x.org"]);
}

#[test]
fn a_web_address_ends_as_githubs_autolinks_end() {
    assert_eq!(
        plain_links("https://x.org. https://x.org/a, https://x.org/b:;!?"),
        ["https://x.org", "https://x.org/a", "https://x.org/b"]
    );
    assert_eq!(
        plain_links("(see https://x.org/a_(b)) and https://x.org/c)."),
        ["https://x.org/a_(b)", "https://x.org/c"]
    );
    assert_eq!(
        plain_links("<https://x.org>\nhttps://y.org\u{a0}z https://z.org\u{3000}"),
        ["https://x.org", "https://y.org", "https://z.org"]
    );
    assert_eq!(plain_links("https://x.org/a.b.c"), ["https://x.org/a.b.c"]);
    assert_eq!(
        plain_links("https://x.org\u{202f}https://y.org\u{2028}"),
        ["https://x.org", "https://y.org"]
    );
}

#[test]
fn a_formatted_text_has_no_web_address_in_code_a_code_block_or_a_link() {
    let text = "`https://a.org` [https://b.org](https://c.org) **https://d.org** \
                *https://e.org*, https://f.org\\_x\n```\nhttps://g.org\n```\n\
                | https://h.org | b |\n|---|---|";
    let found: Vec<&str> = autolinks(text)
        .into_iter()
        .map(|(s, e)| &text[s..e])
        .collect();
    assert_eq!(
        found,
        [
            "https://d.org",
            "https://e.org",
            "https://f.org",
            "https://h.org"
        ]
    );
    assert_eq!(plain_links(text).len(), 7);
}

#[test]
fn only_a_web_or_e_mail_address_or_an_app_link_is_opened() {
    assert!(may_open("HTTPS://x.org") && may_open("mailto:a@x.org"));
    assert!(may_open("oldbrainfix://settings/backups"));
    assert_eq!(
        html("[Backups](oldbrainfix://settings/backups)"),
        "<p>Backups</p>\n"
    );
    assert_eq!(
        link_labelled(
            "go ",
            3,
            3,
            "oldbrainfix://unsorted",
            Some("Unsorted [all]")
        )
        .text,
        "go [Unsorted \\[all\\]](oldbrainfix://unsorted)"
    );
    for url in [
        "oldbrainfix://x",
        "oldbrainfix://file/../a",
        "javascript:x",
        "file:///etc",
        "intent://x",
        "x.org",
        "ʜttps://x",
    ] {
        assert!(!may_open(url), "{url}");
    }
}

#[test]
fn html_groups_lines_escapes_and_links_only_what_may_open() {
    assert_eq!(
        html("one\ntwo\n\n  \nthree\n> a\n> **b**\n- a\n  - b\n  1. c\n- d\n\n3. f\n# H"),
        "<p>one<br>two</p>\n<p>three</p>\n<blockquote>a<br><strong>b</strong></blockquote>\n\
         <ul><li>a<ul><li>b</li></ul><ol><li>c</li></ol></li><li>d</li></ul>\n\
         <ol start=\"3\"><li>f</li></ol>\n<h1>H</h1>\n"
    );
    assert_eq!(
        html("<b>&</b> 'q' `<i>` [w](https://x.org/?a&b) [no](javascript:x)"),
        "<p>&lt;b&gt;&amp;&lt;/b&gt; &#39;q&#39; <code>&lt;i&gt;</code> \
         <a href=\"https://x.org/?a&amp;b\">w</a> no</p>\n"
    );
    assert_eq!(
        html("<!-- first column headings -->\n| A | B |\n|:-:|--:|\n| x | **y** |"),
        "<table><thead><tr><th align=\"center\">A</th><th align=\"right\">B</th></tr></thead>\
         <tbody><tr><th align=\"center\">x</th><td align=\"right\"><strong>y</strong></td></tr>\
         </tbody></table>\n"
    );
    assert_eq!(html(""), "");
}

// ------------------------------------------------------ Code blocks

/// The kinds of the lines of `text`, `c` for a code block's.
fn kinds(text: &str) -> String {
    parse(text)
        .iter()
        .map(|b| match b.kind {
            BlockKind::Code => 'c',
            kind => kind.name().chars().next().unwrap_or('?'),
        })
        .collect()
}

/// The content lines of the code blocks of `text`.
fn content(text: &str) -> Vec<&str> {
    parse(text)
        .iter()
        .filter(|b| b.code.as_ref().is_some_and(|c| !c.is_fence(b.line)))
        .map(|b| slice(text, b.start, b.end))
        .collect()
}

fn code_of(text: &str, line: usize) -> Option<Arc<CodeBlock>> {
    parse(text).get(line).and_then(|b| b.code.clone())
}

#[test]
fn a_fence_of_backticks_or_tildes_holds_lines_read_as_nothing() -> Result<()> {
    let text = "```\n# no *heading*\n- ☐ `x` [a](b)\n```\nafter";
    assert_eq!(kinds(text), "ccccp");
    let blocks = parse(text);
    let code = blocks[0]
        .code
        .clone()
        .ok_or(Error::Invalid("no block".into()))?;
    assert_eq!((code.first_line, code.last_line, code.closed), (0, 3, true));
    assert_eq!(
        (code.fence.as_str(), code.info.as_str(), code.language()),
        ("```", "", "")
    );
    assert_eq!(
        slice(text, code.start, code.end),
        "```\n# no *heading*\n- ☐ `x` [a](b)\n```"
    );
    assert_eq!(tree(&blocks[1].spans), "\"# no *heading*\"");
    assert_eq!(tree(&blocks[2].spans), "\"- ☐ `x` [a](b)\"");
    assert!(blocks[0].spans.is_empty() && blocks[3].spans.is_empty());
    assert!(blocks[4].code.is_none());
    assert_eq!(kinds("~~~\n**x**\n~~~"), "ccc");
    assert_eq!(
        code_of("~~~~ x", 0).map(|c| c.fence.clone()).as_deref(),
        Some("~~~~")
    );
    assert_eq!(kinds("``\nx\n``"), "ppp");
    assert_eq!(kinds("``` a`b\nx"), "pp");
    Ok(())
}

#[test]
fn the_closing_fence_is_of_the_same_character_and_at_least_as_long() {
    let four = "````\n```\nx\n````\ny";
    assert_eq!(kinds(four), "ccccp");
    assert_eq!(content(four), ["```", "x"]);
    assert_eq!(kinds("```\nx\n`````\ny"), "cccp");
    assert_eq!(code_of("```\nx\n`````\ny", 0).map(|c| c.last_line), Some(2));
    assert_eq!(content("~~~\n```\n~~~"), ["```"]);
    assert_eq!(content("```\n~~~\n```"), ["~~~"]);
    assert_eq!(kinds("```\n~~~\n```\n~~~"), "cccc");
    assert_eq!(
        code_of("```\n~~~\n```\n~~~", 3).map(|c| c.first_line),
        Some(3)
    );
    let nested = "`````\n````\n```\nx\n```\n````\n`````\nz";
    assert_eq!(kinds(nested), "cccccccp");
    assert_eq!(content(nested), ["````", "```", "x", "```", "````"]);
}

#[test]
fn a_closing_fence_is_up_to_three_spaces_in_with_only_blanks_after() {
    assert_eq!(kinds("```\nx\n   ```  \t\ny"), "cccp");
    assert_eq!(kinds("```\nx\n    ```\ny"), "cccc");
    assert_eq!(kinds("```\nx\n``` no\ny"), "cccc");
    assert_eq!(kinds("   ```\nx\n```"), "ccc");
    assert_eq!(kinds("    ```\nx\n```"), "ppc");
    assert_eq!(kinds("\t```\nx"), "pp");
}

#[test]
fn an_unclosed_fence_runs_to_the_end_of_the_text() -> Result<()> {
    let text = "a\n```js\nb\n| c | d |\n|---|---|\n# e";
    assert_eq!(kinds(text), "pccccc");
    let code = code_of(text, 1).ok_or(Error::Invalid("no block".into()))?;
    assert_eq!(
        (code.first_line, code.last_line, code.closed),
        (1, 5, false)
    );
    assert!(!code.is_fence(5));
    assert!(table::all(text).is_empty());
    assert_eq!(plain(text), "a\nb\n| c | d |\n|---|---|\n# e");
    assert_eq!(kinds("```"), "c");
    assert_eq!(plain("```"), "");
    assert_eq!(plain("```\n```"), "");
    Ok(())
}

#[test]
fn the_info_string_is_kept_its_first_word_the_language() -> Result<()> {
    let text = "```bash  -e  \necho hi\n```";
    let code = code_of(text, 0).ok_or(Error::Invalid("no block".into()))?;
    assert_eq!((code.info.as_str(), code.language()), ("bash  -e", "bash"));
    assert_eq!(plain(text), "echo hi");
    assert_eq!(
        code_of("~~~ py `x`\n", 0)
            .map(|c| c.language().to_string())
            .as_deref(),
        Some("py")
    );
    Ok(())
}

#[test]
fn a_table_a_tag_and_a_box_in_a_block_are_text() -> Result<()> {
    let text = "#a\n```\n| x | y |\n|---|---|\n#b ☐ c\n```\n#c";
    assert!(table::all(text).is_empty());
    let found: Vec<String> = tags(text).into_iter().map(|t| t.tag).collect();
    assert_eq!(found, ["#a", "#c"]);
    assert!(tags("``` #x\ny").is_empty());
    let mut n = 0;
    let saved = crate::text_boxes::convert(
        "☐ a\n```\n☐ b\n```",
        crate::text_format::MARKDOWN,
        || {
            n += 1;
            Ok(n.to_string())
        },
    )?;
    assert_eq!(crate::text_boxes::find(&saved).len(), 1);
    assert!(saved.contains("☐ b"));
    Ok(())
}

#[test]
fn a_fence_ends_a_table_and_a_table_may_follow_a_block() {
    let text = "a | b\n-|-\nc\n```\nx\n```\nd | e\n-|-";
    assert_eq!(kinds(text), "tttccctt");
    assert_eq!(table::all(text).len(), 2);
    assert_eq!(
        table::insert("```\nx\n```\ny", 5).text,
        "```\nx\n```\n\n|  |  |\n| --- | --- |\n|  |  |\n\ny"
    );
}

#[test]
fn lines_of_any_line_break_are_lines_of_the_block() {
    let text = "```\r\na\rb\r\n```\r\nc";
    assert_eq!(kinds(text), "ccccp");
    assert_eq!(plain(text), "a\rb\r\nc");
}

#[test]
fn html_of_a_block_is_a_pre_with_the_language_as_a_class() {
    assert_eq!(
        html("a\n```bash x\n<b> & \"q\"\n\n```\nb"),
        "<p>a</p>\n<pre><code class=\"language-bash\">&lt;b&gt; &amp; &quot;q&quot;\n\n</code></pre>\n<p>b</p>\n"
    );
    assert_eq!(html("~~~\n"), "<pre><code>\n</code></pre>\n");
    assert_eq!(html("```\nx"), "<pre><code>x\n</code></pre>\n");
}

#[test]
fn a_text_with_a_block_uses_a_format_an_escaped_one_has_no_block() {
    assert!(uses("```\nx\n```"));
    assert!(uses("~~~"));
    for text in ["```\nx\n```", "~~~ sh\n# y\n~~~", "````"] {
        let escaped = escape(text);
        assert!(!uses(&escaped), "{escaped}");
        assert_eq!(plain(&escaped), text);
    }
}

#[test]
fn formats_and_line_formats_leave_a_block_as_it_is() -> Result<()> {
    let text = "a\n```\nb c\n```";
    assert_eq!(toggle(text, 7, 7, SpanKind::Bold)?, unchanged(text, 7, 7));
    assert_eq!(
        toggle(text, 0, text.len(), SpanKind::Italic)?.text,
        "*a*\n```\nb c\n```"
    );
    assert_eq!(link(text, 6, 9, "https://x.org").text, text);
    assert_eq!(
        toggle_line(text, 0, text.len(), BlockKind::Quote).text,
        "> a\n```\nb c\n```"
    );
    assert_eq!(
        toggle_line(text, 0, 0, BlockKind::Code),
        unchanged(text, 0, 0)
    );
    Ok(())
}

#[test]
fn a_closing_fence_typed_is_a_shortcut_an_opening_one_is_not() {
    let text = "x\n```\na\n````";
    let at: Vec<usize> = (0..=text.len()).filter(|&i| shortcut_at(text, i)).collect();
    assert_eq!(at, [text.len()]);
    assert!(!shortcut_at("```\na\n```", 3));
}

fn change(text: &str, start: usize, end: usize) -> Change {
    Change {
        text: text.into(),
        start,
        end,
    }
}

#[test]
fn code_block_puts_the_lines_touched_into_a_fence() {
    assert_eq!(
        code_block("a\nb c\nd", 3, 4),
        change("a\n```\nb c\n```\nd", 7, 8)
    );
    assert_eq!(
        code_block("a\nb\nc", 0, 3),
        change("```\na\nb\n```\nc", 4, 7)
    );
    assert_eq!(code_block("a\n\nb", 2, 2), change("a\n```\n\n```\nb", 6, 6));
    assert_eq!(code_block("", 0, 0), change("```\n\n```", 4, 4));
    assert_eq!(
        code_block("x ```` y\r\nz", 0, 0).text,
        "`````\r\nx ```` y\r\n`````\r\nz"
    );
    assert_eq!(
        code_block("a\n```\nb\n```", 0, 3).text,
        "````\na\n```\nb\n```\n````"
    );
    assert_eq!(
        code_block("x | y\n-|-\nz", 0, 0).text,
        "```\nx | y\n-|-\nz\n```"
    );
}

#[test]
fn code_block_in_a_block_takes_its_fences_off() {
    assert_eq!(
        code_block("a\n```sh\nb\nc\n```\nd", 9, 9),
        change("a\nb\nc\nd", 3, 3)
    );
    assert_eq!(code_block("```\nb", 5, 5), change("b", 1, 1));
    assert_eq!(code_block("```\n```", 0, 0), change("", 0, 0));
    assert_eq!(code_block("x\n```", 3, 3), change("x\n", 2, 2));
    assert_eq!(
        code_block("```\na\n```\nb", 4, 11).text,
        "````\n```\na\n```\nb\n````"
    );
}

#[test]
fn a_task_list_item_becomes_an_item_with_a_box() {
    let boxes = |text: &str| task_boxes(text, 0, text.len()).text;
    assert_eq!(
        boxes("- [ ] milk\n* [x] bread\n+ [X] eggs\n1. [ ] call\n  - [x] in"),
        "- ☐ milk\n* ✅ bread\n+ ✅ eggs\n1. ☐ call\n  - ✅ in"
    );
    for text in [
        "- [ ]",
        "- [ ]milk",
        "- [y] milk",
        "- [  ] milk",
        "[ ] milk",
        "> - [ ] milk",
        "- \\[ ] milk",
        "- `[ ] milk`",
        "- [ ](https://x.org) milk",
        "```\n- [ ] milk\n```",
        "| a |\n|---|\n| - [ ] b |",
        "- ☐ milk",
    ] {
        assert_eq!(boxes(text), text, "{text:?}");
    }
}

#[test]
fn task_boxes_only_on_the_lines_touched_with_offsets_kept() {
    let text = "- [ ] a\n- [x] b\n- [ ] c";
    assert_eq!(
        task_boxes(text, 6, 6),
        change("- ☐ a\n- [x] b\n- [ ] c", 6, 6)
    );
    assert_eq!(
        task_boxes(text, 8, 16),
        change("- [ ] a\n- ✅ b\n- [ ] c", 8, 16)
    );
    assert_eq!(task_boxes("x\n- [ ] ", 8, 8), change("x\n- ☐ ", 8, 8));
    assert_eq!(task_boxes("- [ ]", 5, 5).text, "- [ ]");
}

#[test]
fn a_box_at_an_items_start_is_written_back_as_a_task_list_item() {
    assert_eq!(
        task_lists("- ☐ milk\n2. ✅ bread\n  * ❌ call Ann  \n- ❌ "),
        "- [ ] milk\n2. [x] bread\n  * [x] ~~call Ann~~  \n- [x] "
    );
    assert_eq!(task_lists("- ❌ ~~gone~~"), "- [x] ~~gone~~");
    assert_eq!(task_lists("- ❌ **a** b"), "- [x] ~~**a** b~~");
    for text in [
        "☐ milk",
        "buy ☐ milk",
        "- ☐",
        "- ☐milk",
        "- x ☐ milk",
        "> ☐ milk",
        "```\n- ☐ milk\n```",
        "| a |\n|---|\n| ☐ b |",
    ] {
        assert_eq!(task_lists(text), text, "{text:?}");
    }
    let shown = "# Shop\n- ☐ milk\n- ✅ bread";
    let back = task_lists(shown);
    assert_eq!(task_boxes(&back, 0, back.len()).text, shown);
}

#[test]
fn saving_a_formatted_text_gives_its_task_list_items_boxes() -> Result<()> {
    let mut n = 0;
    let mut new_id = || {
        n += 1;
        Ok(format!("id:{n}"))
    };
    let text = "- [ ] milk\n- [x] bread\n```\n- [ ] code\n```";
    let saved = crate::text_boxes::convert(text, crate::text_format::MARKDOWN, &mut new_id)?;
    assert_eq!(
        crate::text_boxes::strip(&saved),
        "- ☐ milk\n- ✅ bread\n```\n- [ ] code\n```"
    );
    assert_eq!(crate::text_boxes::find(&saved).len(), 2);
    for format in [crate::text_format::PLAIN, crate::text_format::CODE] {
        assert_eq!(crate::text_boxes::convert(text, format, &mut new_id)?, text);
    }
    Ok(())
}
