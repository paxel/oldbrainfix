use super::*;

fn adoc(text: &str) -> String {
    read(ImportFormat::AsciiDoc, text).text
}

fn org(text: &str) -> String {
    read(ImportFormat::Org, text).text
}

fn rst(text: &str) -> String {
    read(ImportFormat::RestructuredText, text).text
}

fn html(text: &str) -> String {
    read(ImportFormat::Html, text).text
}

#[test]
fn takes_the_extensions_of_the_four_formats_only() {
    use ImportFormat::{AsciiDoc, Html, Org, RestructuredText};
    let read: Vec<_> = [
        "adoc", "asciidoc", "asc", "org", "rst", "html", "htm", "md", "txt", "xhtml",
    ]
    .iter()
    .map(|e| ImportFormat::of_extension(e))
    .collect();
    assert_eq!(
        read,
        [
            Some(AsciiDoc),
            Some(AsciiDoc),
            Some(AsciiDoc),
            Some(Org),
            Some(RestructuredText),
            Some(Html),
            Some(Html),
            None,
            None,
            None,
        ]
    );
}

#[test]
fn ascii_doc_becomes_what_the_subset_has() {
    assert_eq!(
        adoc(
            "= Garden\n== Planting\nPlant *tomatoes*, _basil_ and `bed 2`, **every**day +\nand [.line-through]#never# at noon, see https://example.org/plan[the plan^].\n\n* Seeds\n** Carrots\n. First\n. Second\n\n[source,python]\n----\nprint(\"**x**\")\n----\n____\nA *quote*.\n____\n|===\n|Plant |Month\n\n|Tomato\n|May\n|==="
        ),
        "# Garden\n## Planting\nPlant **tomatoes**, *basil* and `bed 2`, **every**day\nand ~~never~~ at noon, see [the plan](https://example.org/plan).\n\n- Seeds\n  - Carrots\n1. First\n2. Second\n\n```python\nprint(\"**x**\")\n```\n> A **quote**.\n\n| Plant | Month |\n| --- | --- |\n| Tomato | May |"
    );
}

#[test]
fn ascii_doc_keeps_what_the_subset_lacks_as_it_is_written() {
    assert_eq!(
        adoc(
            ":author: Ada\n=== Deeper\n// a comment\nNOTE: Keep the *hose* coiled.\n[quote, Ada]\n.A title\nimage::photo.png[A photo]\n<<ref,see there>> and #marked#"
        ),
        ":author: Ada\n=== Deeper\n// a comment\nNOTE: Keep the **hose** coiled.\n\\[quote, Ada]\n.A title\nimage::photo.png\\[A photo]\n<<ref,see there>> and #marked#"
    );
}

#[test]
fn ascii_doc_reads_an_include_as_the_line_it_is_and_nothing_of_its_file() {
    assert_eq!(
        adoc("include::secret.adoc[]\ninclude::https://x.org/a.adoc[]"),
        "include::secret.adoc\\[]\ninclude::https://x.org/a.adoc\\[]"
    );
}

#[test]
fn org_becomes_what_the_subset_has() {
    assert_eq!(
        org(
            "#+TITLE: Week\n* Shopping\n  - milk and *butter*\n    - rye\n  1. first\n  2) second\n** Notes\nCall =Bob= about the +old+ ~car~ and /that/, see [[https://example.org/a][the page]].\n#+BEGIN_SRC shell\n  echo \"hi\"\n  ,* not a heading\n#+END_SRC\n#+begin_quote\nBe kind.\n#+end_quote\n| Name | Age |\n|------+-----|\n| Ann  | 3   |\nLine one\\\\"
        ),
        "# Week\n# Shopping\n- milk and **butter**\n  - rye\n1. first\n2. second\n## Notes\nCall `Bob` about the ~~old~~ `car` and *that*, see [the page](https://example.org/a).\n```shell\necho \"hi\"\n* not a heading\n```\n> Be kind.\n\n| Name | Age |\n| --- | --- |\n| Ann | 3 |\n\nLine one"
    );
}

#[test]
fn org_keeps_what_the_subset_lacks_as_it_is_written() {
    assert_eq!(
        org(
            "*** Deeper\n:PROPERTIES:\n:ID: 1234\n:END:\n#+AUTHOR: Ada\n# a comment\n_underlined_ and [[file:other.org][other]]"
        ),
        "\\*\\*\\* Deeper\n:PROPERTIES:\n:ID: 1234\n:END:\n#+AUTHOR: Ada\n\\# a comment\n\\_underlined\\_ and other"
    );
}

#[test]
fn org_reads_an_include_as_the_line_it_is_and_nothing_of_its_file() {
    assert_eq!(
        org("#+INCLUDE: \"secret.org\""),
        "#+INCLUDE: \"secret.org\""
    );
}

#[test]
fn restructured_text_becomes_what_the_subset_has() {
    assert_eq!(
        rst(
            "=======\nRecipes\n=======\n\nSoup\n====\n\nCut **onions** and *carrots*, use ``salt``, see `the site <https://example.org/soup>`_.\n\n- one\n\n  - deep\n\n#. auto\n#. again\n\nExample::\n\n    print(\"x\")\n\n.. code-block:: python\n   :linenos:\n\n   def f():\n       return 1\n\n+------+-----+\n| Name | Qty |\n+======+=====+\n| Egg  | 2   |\n+------+-----+"
        ),
        "# Recipes\n\n## Soup\n\nCut **onions** and *carrots*, use `salt`, see [the site](https://example.org/soup).\n\n- one\n\n  - deep\n\n1. auto\n2. again\n\nExample:\n```\nprint(\"x\")\n```\n\n```python\ndef f():\n    return 1\n```\n\n| Name | Qty |\n| --- | --- |\n| Egg | 2 |"
    );
}

#[test]
fn restructured_text_keeps_what_the_subset_lacks_as_it_is_written() {
    assert_eq!(
        rst(
            "Top\n===\n\nMid\n---\n\nThird\n~~~~~\n\n.. note:: Keep warm.\n\n:Author: Ada\nEscaped \\*star\\* and :role:`text`."
        ),
        "# Top\n\n## Mid\n\nThird\n\\~\\~\\~\\~\\~\n\n.. note:: Keep warm.\n\n:Author: Ada\nEscaped \\*star\\* and :role:\\`text\\`."
    );
}

#[test]
fn restructured_text_reads_an_include_as_the_line_it_is_and_nothing_of_its_file() {
    assert_eq!(
        rst(".. include:: secret.rst\n   :literal:\n\nAfter"),
        ".. include:: secret.rst\n   :literal:\n\nAfter"
    );
}

#[test]
fn html_becomes_what_the_subset_has_its_title_apart() {
    let page = read(
        ImportFormat::Html,
        "<!DOCTYPE html><html><head><title>My &amp; page</title></head><body><h1>Hello   <em>world</em></h1>\n<p>Some <b>bold </b>text and <a href=\"https://example.org/?a=1&amp;b=2\">a link</a>.<br>Next &copy; &#x1F600; &bogus;</p><ul><li>One<ul><li>Inner</li></ul></li><li><p>Two</p></li></ul><ol start=\"3\"><li>Three</li><li>Four</li></ol><blockquote><p>Quoted *stars*</p></blockquote><pre><code class=\"language-js\">let a = 1;\n  b &lt; 2;\n</code></pre><table><tr><th>A</th><th>B|C</th></tr><tr><td>1</td><td><code>x|y</code></td></tr></table><h3>Small</h3><div>one</div><div>two <s>gone</s></div>",
    );
    assert_eq!(page.title, "My & page");
    assert_eq!(
        page.text,
        "# Hello *world*\n\nSome **bold** text and [a link](https://example.org/?a=1&b=2).\nNext © 😀 &bogus;\n\n- One\n  - Inner\n- Two\n\n3. Three\n4. Four\n\n> Quoted \\*stars\\*\n\n```js\nlet a = 1;\n  b < 2;\n```\n\n| A | B\\|C |\n| --- | --- |\n| 1 | `x\\|y` |\n\nSmall\n\none\ntwo ~~gone~~"
    );
}

#[test]
fn html_drops_scripts_styles_and_comments_and_fetches_no_image() {
    let text = html(
        "<style>p { color: red }</style><SCRIPT type=\"text/javascript\">steal(\"<p>\")</SCRIPT><p onclick=\"steal()\">Kept<!-- hidden --></p><img src=\"https://tracker.example/pixel.png\" alt=\"A cat\"><a href=\"javascript:steal()\">bad</a> <a href=\"file:///etc/passwd\">local</a><script>never ends",
    );
    assert_eq!(text, "Kept\n\nA cat bad local");
    for gone in ["steal", "tracker", "color"] {
        assert!(!text.contains(gone), "{gone}");
    }
}

#[test]
fn html_leaves_broken_references_and_markup_as_text() {
    assert_eq!(
        html("a &#0; &#xD800; &#x110000; &amp b < c <b>bold"),
        "a &#0; &#xD800; &#x110000; &amp b < c **bold**"
    );
}

#[test]
fn escapes_what_would_read_as_a_mark() {
    assert_eq!(
        adoc("# not a heading\n- [ ] a | b \\"),
        "\\# not a heading\n- \\[ ] a \\| b \\\\"
    );
    assert_eq!(html("<p>1. one</p><p>> two</p>"), "1\\. one\n\n\\> two");
}

#[test]
fn reads_a_long_line_of_marks_in_a_time_that_grows_with_its_length() {
    let marks = "*_`+[=/~".repeat(200_000);
    let started = std::time::Instant::now();
    for format in [
        ImportFormat::AsciiDoc,
        ImportFormat::Org,
        ImportFormat::RestructuredText,
        ImportFormat::Html,
    ] {
        read(format, &marks);
        read(format, &format!("<a{marks}"));
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(20));
}
