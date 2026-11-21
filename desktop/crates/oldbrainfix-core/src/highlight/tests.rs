use super::*;

/// The coloured parts of `text` in `g` as `text:colour`, for reading.
fn parts(g: &Grammar, text: &str) -> Vec<String> {
    g.colour(text)
        .iter()
        .map(|s| format!("{}:{}", &text[s.start..s.end], s.colour.name()))
        .collect()
}

fn grammar(contexts: &str) -> Grammar {
    Grammar::parse(&format!("name: T\nscope: source.t\ncontexts:\n{contexts}")).expect("a grammar")
}

#[test]
fn every_built_in_grammar_is_read() {
    for source in BUILT_IN_SOURCES {
        assert!(
            Grammar::parse(source).is_ok(),
            "{:?}",
            Grammar::parse(source).err()
        );
    }
    let all = &Grammars::built_in().all;
    assert_eq!(all.len(), 15);
    let scopes: HashSet<&str> = all.iter().map(|g| g.scope.as_str()).collect();
    assert_eq!(scopes.len(), 15);
}

#[test]
fn a_capture_shows_over_its_match() {
    let g = grammar(
        "  main:\n    - match: '(let)\\s+(\\w+)'\n      scope: string.x\n      captures:\n        1: keyword.let\n        2: meta.name\n",
    );
    assert_eq!(parts(&g, "let ab"), ["let:keyword", " ab:string"]);
}

#[test]
fn a_pushed_context_colours_until_it_pops() {
    let g = grammar(
        "  main:\n    - match: '\"'\n      push: str\n  str:\n    - meta_scope: string.quoted\n    - match: '\\\\.'\n      scope: constant.character.escape\n    - match: '\"'\n      pop: true\n",
    );
    assert_eq!(
        parts(&g, "a \"b\\\"c\nd\" e"),
        ["\"b:string", "\\\":constant", "c\nd\":string"]
    );
}

#[test]
fn word_classes_and_dot_are_ascii_and_a_character_is_one() {
    let g = grammar(
        "  main:\n    - match: '\\b\\w+\\b'\n      scope: keyword\n    - match: '<.>'\n      scope: string\n",
    );
    assert_eq!(
        parts(&g, "éab <😀> <é>"),
        ["ab:keyword", "<😀>:string", "<é>:string"]
    );
}

#[test]
fn case_folds_ascii_only_and_the_end_is_before_the_break() {
    let g = grammar(
        "  main:\n    - match: '(?i)select'\n      scope: keyword\n    - match: '\\d+$'\n      scope: constant.numeric\n",
    );
    assert_eq!(
        parts(&g, "SeLeCt 1 2\r\n3\n"),
        ["SeLeCt:keyword", "3:number"]
    );
}

#[test]
fn offsets_are_bytes() {
    let rust = Grammars::built_in()
        .by_scope(Some("source.rust"))
        .expect("Rust");
    let spans = rust.colour("/* 😀 */ fn");
    assert_eq!(
        spans.first(),
        Some(&CodeSpan {
            start: 0,
            end: 10,
            colour: CodeColour::Comment
        })
    );
    assert_eq!(
        spans.last(),
        Some(&CodeSpan {
            start: 11,
            end: 13,
            colour: CodeColour::Keyword
        })
    );
}

#[test]
fn what_goes_beyond_the_subset_is_refused() {
    for item in [
        "    - match: a\n      embed: scope:source.y\n      escape: b\n",
        "    - match: a\n      branch_point: p\n",
        "    - include: scope:source.y\n",
        "    - match: a\n      pop: 2\n",
        "    - match: 'a*+'\n",
        "    - match: '(?>a)'\n",
        "    - match: '(a)\\1'\n",
        "    - match: '[[:alpha:]]'\n",
        "    - match: '(?:(a))+'\n",
        "    - match: '\\Ga'\n",
    ] {
        let source = format!("name: T\nscope: source.t\ncontexts:\n  main:\n{item}");
        assert!(Grammar::parse(&source).is_err(), "{item}");
    }
}

#[test]
fn a_lookbehind_of_one_fixed_width_is_read_and_of_none_refused() {
    for read in [
        r"(?<=a)b",
        r"(?<![\w$])b",
        r"(?<=ab|cd)e",
        r"(?<=a{3}(?:bc){2})d",
        r"(?<=\b(?=x)[a-z]$)y",
        r"(?<=(?<!q)😀)z",
        r"(?<=x{1000})y",
    ] {
        assert!(CodeRegex::parse(read).is_ok(), "{read}");
    }
    for refused in [
        r"(?<=a+)b",
        r"(?<=a*)b",
        r"(?<=a?)b",
        r"(?<=a{1,2})b",
        r"(?<=ab|c)d",
        r"(?<!x+)y",
        r"(?<=(?:a|bc))d",
        r"(?<=x{1001})y",
        r"(?<=(?:x{10}){101})y",
    ] {
        let why = CodeRegex::parse(refused).map(|r| r.groups);
        assert!(
            matches!(&why, Err(GrammarRefused(w)) if w.contains("no fixed width")),
            "{refused}: {why:?}"
        );
    }
}

#[test]
fn fences_and_guesses_find_their_grammar() {
    let all = Grammars::built_in();
    assert_eq!(all.for_fence("RS").map(|g| g.name.as_str()), Some("Rust"));
    assert!(all.for_fence("").is_none());
    assert!(all.for_fence("cobol").is_none());
    assert!(all.by_scope(Some("")).is_none());
    assert_eq!(
        all.guess("\n#!/bin/bash\necho").map(|g| g.scope.as_str()),
        Some("source.shell")
    );
    assert!(all.guess("plain words").is_none());
}
