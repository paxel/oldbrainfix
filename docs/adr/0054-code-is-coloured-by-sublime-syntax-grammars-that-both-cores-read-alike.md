# Code is coloured by Sublime syntax grammars that both cores read alike

Code showed in one colour. The user chose (obf-1036, #346, Q13, Q16, Q17, Q23) that a Code text and every fenced code block of a Formatted text whose fence names a language are coloured by that language's syntax, on the phone and the desktop; that the colours come from grammar files, standard Sublime syntax definitions (`.sublime-syntax`), so that no code of a plugin ever runs; that a Code text's language is picked in the text format badge beside "Code", stored with the Note and synced, the guessed one pre-picked when a paste is offered as Code text; and that about fifteen languages ship built in: shell, Python, JavaScript/TypeScript, Rust, Dart, Kotlin, Java, C/C++, Go, SQL, JSON, YAML, XML/HTML, CSS and Markdown. Adding grammars of one's own is a later ticket (obf-1037, #347), which this one is made for.

## Both cores read the same grammars into the same spans

The Dart core (`highlight.dart`) and the Rust core (`highlight.rs`) each read a grammar and colour a text with it, and must give the same coloured parts for the same text, as the rest of the cores do; the cross-core fixture `desktop/fixtures/highlight.json` checks it. No library does this in both languages: syntect (MIT) reads Sublime grammars in Rust with Oniguruma's or fancy-regex's semantics, and a search of pub.dev (November 2026) found no package that reads them in Dart; those that read TextMate grammars hand Oniguruma patterns to Dart's `RegExp` as they are or bind Oniguruma natively. Sublime grammars are written for Oniguruma, whose regular expressions Dart's `RegExp` (ECMAScript) cannot all run: possessive quantifiers, atomic groups, `\G`, `\h`, POSIX classes, inline flags. The grammars of Sublime's own Packages repository use these throughout (C++ alone has over 500 possessive quantifiers), and also `branch_point`, `extends`, `embed` and `with_prototype`, which a line-by-line reader cannot follow alike without far more machinery.

So both cores carry a small interpreter of a **subset of Sublime syntax definitions**, and refuse a grammar that goes beyond it:

- **Keys**: `name`, `scope` (the grammar's identity), `file_extensions` (also the names a fence may use), `first_line_match` (the guess), `variables` and `contexts`; `hidden` and `version` are ignored. In a context: `match` with `scope`, `captures` and one of `push`, `set` or `pop: true`; `include` of a context of the same grammar; `meta_scope`, `meta_content_scope` and `meta_include_prototype` first; the `prototype` context. Push and set take a context's name, a list of names or an anonymous context. Refused: `embed`, `escape`, `branch_point`, `fail`, `extends`, `with_prototype`, `clear_scopes`, an `include` of another grammar, `pop` of more than one.
- **Regular expressions**: literals, `.`, `^`, `$`, `|`, groups, lookahead and lookbehind, greedy and lazy quantifiers, classes, `\w \W \d \D \s \S \b \B`, the character escapes, and a leading `(?x)` or `(?i)`. Refused: possessive quantifiers, atomic and named groups, backreferences, any other or later flag, `\G \A \z \Z \h \K \p{…}`, POSIX classes, `&&`, a capture group inside a quantified group or a lookaround, a quantifier on an assertion.
- **One meaning in both**: each core reads a regular expression into the same tree and writes it out for its own engine, Dart's `RegExp` in unicode mode and the Rust crate fancy-regex, both backtracking engines with lookaround. Where the engines differ, the tree says what is meant: `\w`, `\d`, `\s` and `\b` are ASCII (`[A-Za-z0-9_]`, `[0-9]`, the six ASCII blanks); `.` is any character but `\n`; `$` is "before the line's `\n` or at its end"; `(?i)` turns each ASCII letter into both its cases, and nothing else folds; every other character is written as a code point escape. A differential run of both engines over the corner cases showed exactly these differences before the rewriting and none after it. Captures inside quantified groups and lookarounds are refused because there ECMAScript resets them each round and Oniguruma-like engines keep the last; a backreference to a group that took no part matches the empty text in one and fails in the other.
- **Matching as Sublime does it**: line by line, each line with its `\n`; at a position every pattern of the top context is searched to the line's end, the earliest match wins, the earlier pattern on a tie; an empty match that changes nothing is passed over, and after 1000 empty matches in a row the rest of the line stays as it is. The text between matches has the meta scopes of every context on the stack; a match also its own scope and its captures' scopes, with `push` and `set` the new contexts' `meta_scope`, with `pop` the popped one's. The stack holds at most 64 contexts; `pop` on `main` does nothing. A line over 5000 characters and a text over 200000 are not coloured. Offsets are UTF-16 code units in Dart and UTF-8 bytes in Rust; the fixture counts code points.

A part's colour comes from its scopes, innermost first, by the longest dotted prefix in a fixed table (`comment`, `string`, `constant.numeric`, `keyword`, `keyword.operator`, `storage.type`, `entity.name.function` and so on) mapped onto eleven colours: comment, string, number, constant, keyword, operator, type, function, tag, attribute and variable. Each look gives each of them a colour that reads on its code background, the same in both apps (light and dark, GitHub's palette).

## The fifteen grammars are the app's own

The Packages repository's grammars are far outside the subset, so the fifteen built-in grammars (`assets/grammars/*.sublime-syntax`) are written for this app, small, in the standard format and under the app's licence (MIT OR Apache-2.0), with nothing of Sublime's copied. They colour keywords, types, strings with their escapes and interpolation, comments, numbers, function names and the like; JavaScript and TypeScript share one, as do C and C++ and XML and HTML. The Rust core embeds them with `include_str!`, the Dart core through `lib/src/built_in_grammars.dart`, written by `tool/gen_grammars.dart` and checked by a test against the files.

## Where colour shows

- A **Code text**'s language is a field of its Note, `language`, holding the grammar's scope (`source.rust`): the newest write wins and it syncs as every field does. Empty or absent means none. It stays when the text is made plain or formatted and counts again once the text is code again. A scope no grammar here has, such as one of a user's grammar on another Device, is kept and colours nothing.
- A **fenced code block** names its language by its info string's first word; a grammar whose file extensions or name, compared in lower case, hold that word colours it. A fence naming none or one not known stays as before.
- **Guess**: when the editor offers a pasted text as Code text, the language picked is the first grammar, in the built-in order, whose `first_line_match` the text's first line that is not blank matches; none matches, none is picked.
- Colour is drawn only: Search, Share, Copy text, the vault and the AI port give the text as written.
- **AI port** (ADR 0026): `read_note` and `list_notes` report `codeLanguage`; `list_code_languages` lists the grammars; `set_code_language` sets a Code text's language as a Pending Change, only to a grammar known here.
- **Vault** (ADR 0050): a Note's file holds `language` when it has one, and a rebuild brings it back.

## User grammars later

A grammar a user adds (obf-1037) is read by the same code and refused with its reason when it goes beyond the subset; both cores refuse the same grammars, which the fixture checks too. Its scope is its identity, so a Note coloured by it on one Device stays uncoloured, not wrong, on a Device without it.

## Considered Options

- syntect in Rust and a Dart port of it (rejected: no Dart port exists, and syntect follows Oniguruma's semantics, which Dart's `RegExp` cannot reproduce span for span).
- The Packages repository's grammars, trimmed to the subset (rejected: they lean on `branch_point`, `extends`, `embed`, possessive quantifiers and POSIX classes throughout; Kotlin and Dart are not among them at all).
- TextMate grammars or highlight.js definitions (rejected: the user chose Sublime syntax definitions, Q13).
- An own regular expression engine in both cores (rejected for now: the two engines, rewritten to one meaning, agree, and an own engine is far more code to keep the same).
- Highlighting by each platform's own library (rejected: the cores must colour alike).

## Consequences

- Grammars from elsewhere run here only when they stay inside the subset; obf-1037 must say why one is refused.
- fancy-regex has a backtracking limit Dart's `RegExp` has not; a pattern that runs out of it in Rust finds nothing there, so a grammar that backtracks wildly could colour one line differently. The built-in grammars avoid nested quantifiers.
- New dependencies: the Rust core takes fancy-regex (MIT) and yaml-rust2 (MIT OR Apache-2.0), the Dart core `yaml` (MIT), already in its lock through `test`.
