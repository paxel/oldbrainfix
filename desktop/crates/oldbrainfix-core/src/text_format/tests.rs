use super::*;

const FUTURE: &str = r##"{"format":"x-future","text":"# Heading\n**bold**"}"##;
const LATER: &str = r#"{"format":"plain","more":1,"text":"Later milk","v":2}"#;

#[test]
fn plain_text_is_written_with_its_format_keys_sorted() {
    assert_eq!(
        StoredText::plain("Quote \" and \\ and \n").encode(),
        r#"{"format":"plain","text":"Quote \" and \\ and \n"}"#
    );
    assert_eq!(
        StoredText::read(Some(r#"{"format":"plain","text":"milk","v":1}"#)),
        StoredText::plain("milk")
    );
}

#[test]
fn version_1_is_the_default_and_never_written() {
    let unversioned = StoredText::read(Some(r#"{"format":"plain","text":"milk"}"#));
    assert_eq!(unversioned, StoredText::plain("milk"));
    // Written with its default, it is read the same and written without.
    assert_eq!(
        StoredText::read(Some(r#"{"format":"plain","text":"milk","v":1}"#)).encode(),
        r#"{"format":"plain","text":"milk"}"#
    );
    let future = StoredText::read(Some(FUTURE));
    assert_eq!(future.version, 1);
    assert!(!future.editable);
}

#[test]
fn a_value_written_before_the_format_was_recorded_reads_as_plain() {
    for value in [
        "milk",
        "",
        "{not json",
        r#"{"format":"plain"}"#,
        r#"{"format":3,"text":"x"}"#,
        r#"["format","text"]"#,
        r#" {"format":"x-future","text":"x"}"#,
        r#"{"format":"plain","text":"x","v":"1"}"#,
        r#"{"format":"plain","text":"x","v":1.5}"#,
        r#"{"format":"plain","text":"x","v":0}"#,
        r#"{"format":"plain","text":"x","v":-1}"#,
        r#"{"format":"plain","text":"x","v":true}"#,
        r#"{"format":"plain","text":"x","v":null}"#,
        r#"{"format":"plain","v":1}"#,
        r#"{"text":"x","v":1}"#,
    ] {
        assert_eq!(StoredText::read(Some(value)), StoredText::plain(value));
    }
    assert_eq!(StoredText::read(None), StoredText::plain(""));
}

#[test]
fn an_unknown_format_reads_as_its_source_text_not_editable() {
    let text = StoredText::read(Some(FUTURE));
    assert_eq!(text.format, "x-future");
    assert_eq!(text.text, "# Heading\n**bold**");
    assert!(!text.editable);
    for value in [
        r#"{"format":"plain","more":1,"text":"x"}"#,
        r#"{"format":"plain","more":1,"text":"x","v":1}"#,
    ] {
        let more = StoredText::read(Some(value));
        assert_eq!((more.format.as_str(), more.text.as_str()), (PLAIN, "x"));
        assert!(!more.editable);
    }
    let unknown = StoredText::read(Some(r#"{"format":"x-future","text":"x","v":1}"#));
    assert_eq!(unknown.format, "x-future");
    assert!(!unknown.editable);
}

#[test]
fn a_later_structure_shows_its_text_never_editable() {
    let later = StoredText::read(Some(LATER));
    assert_eq!(
        (later.format.as_str(), later.text.as_str(), later.version),
        (PLAIN, "Later milk", 2)
    );
    assert!(!later.editable);
    assert_ne!(later, StoredText::plain("Later milk"));
    // Without text, or keys of another kind, it shows nothing.
    for value in [
        r#"{"format":"plain","v":2}"#,
        r#"{"format":"plain","text":["x"],"v":2}"#,
        r#"{"text":7,"v":3}"#,
    ] {
        let text = StoredText::read(Some(value));
        assert_eq!(text.text, "", "{value}");
        assert!(!text.editable, "{value}");
    }
    assert_eq!(StoredText::read(Some(r#"{"text":7,"v":3}"#)).format, "");
}

#[test]
fn a_text_from_elsewhere_keeps_its_version() {
    let later = StoredText::of(PLAIN, "x", 2);
    assert!(!later.editable);
    assert_eq!(later.encode(), r#"{"format":"plain","text":"x","v":2}"#);
    assert_eq!(StoredText::read(Some(&later.encode())), later);
    assert!(StoredText::of(PLAIN, "x", VERSION).editable);
    assert!(!StoredText::of("x-future", "x", 1).editable);
}

#[test]
fn a_shebang_or_lines_mostly_of_code_punctuation_or_indented_look_like_code() {
    for text in [
        "#!/bin/bash\necho hi",
        "#!/usr/bin/env python3",
        "int main() {\n  return 0;\n}",
        "a = 1;\nb = 2;\nc = a + b;",
        "def f(x):\n    y = x\n    return y",
        "{\n  \"a\": 1,\n  \"b\": [1, 2]\n}\n",
        "for f in *; do\n\techo \"$f\"\n\tls \"$f\"\ndone",
        ".card {\r\n  color: red;\r\n}",
        "x;\n    y\nz",
    ] {
        assert!(looks_like_code(text), "{text:?}");
    }
    for text in [
        "",
        "#!",
        "echo hi;",
        "a;\nb;",
        " #!/bin/bash\necho hi",
        "Milk\nEggs\nBread",
        "- milk\n- eggs\n    - free range\n    - brown",
        "# Shopping\n- **milk**\n- eggs",
        "Dear Ann,\nsee you at 8 (the usual place).\nBen",
        "Recipe:\n    1 cup flour\n    2 eggs\nMix well.\nBake.",
        "one;\ntwo\nthree\nfour",
        "\n\n   \n\t\n",
        // A no-break space is no indentation.
        "x\n\u{a0}\u{a0}\u{a0}\u{a0}y\nz;",
    ] {
        assert!(!looks_like_code(text), "{text:?}");
    }
}

#[test]
fn a_code_text_reads_as_it_is_written() {
    assert!(KNOWN.contains(&CODE));
    let text = "# not a heading\n**x** ☐ #tag";
    assert_eq!(visible(CODE, text), text);
    assert_eq!(literal(CODE, "*x*"), "*x*");
    let stored = StoredText::of(CODE, text, VERSION);
    assert!(stored.editable);
    assert_eq!(
        stored.encode(),
        r##"{"format":"code","text":"# not a heading\n**x** ☐ #tag"}"##
    );
    assert_eq!(StoredText::read(Some(&stored.encode())), stored);
}
