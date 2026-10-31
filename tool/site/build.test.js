// The site build (#44), as Catlog tests its own: front matter, headings, the
// two trees and their twins, the language switch, the alias, the search index
// per language, the link check, and the glossary and What's new generated
// from CONTEXT.md and the changelogs. Run with `node --test "tool/site/*.test.js"`.
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import {
  REPO_ROOT,
  buildSite,
  checkLinks,
  collectPages,
  parseFrontMatter,
  processHeadings,
  readVersion,
  switchHref,
} from "./build.js";
import { generatedPages, glossaryPages, parseChangelog, parseGlossary, whatsNewPages } from "./generated.js";

const PAGE = (title, body = "<p>Text.</p>", extra = "") =>
  `<!-- page: title="${title}" ${extra} -->\n${body}\n`;

/** A tiny site in a temp dir: both trees with an index and the pages given as {path: {en, de}}. */
function tinySite(pages = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "oldbrainfix-site-"));
  const src = path.join(dir, "site");
  fs.mkdirSync(path.join(src, "assets"), { recursive: true });
  fs.writeFileSync(path.join(src, "assets", "site.css"), "body{}");
  fs.writeFileSync(path.join(src, "assets", "site.js"), "");
  fs.writeFileSync(path.join(src, "assets", "icon.png"), "");
  for (const lang of ["en", "de"]) {
    fs.mkdirSync(path.join(src, lang), { recursive: true });
    fs.writeFileSync(path.join(src, lang, "index.html"), PAGE(lang === "en" ? "Home" : "Start"));
  }
  for (const [rel, bodies] of Object.entries(pages)) {
    for (const [lang, source] of Object.entries(bodies)) {
      const file = path.join(src, lang, rel);
      fs.mkdirSync(path.dirname(file), { recursive: true });
      fs.writeFileSync(file, source);
    }
  }
  return { dir, src, out: path.join(dir, "site-dist") };
}

/** A repository root in a temp dir with the given files, for the generated pages. */
function tinyRepo(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "oldbrainfix-repo-"));
  for (const [rel, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true });
    fs.writeFileSync(path.join(dir, rel), text);
  }
  return { root: dir, src: path.join(dir, "site") };
}

test("front matter: keys, order, unknown keys and tracks are refused", () => {
  const { meta, body } = parseFrontMatter(
    '<!-- page: title="A page" track="reference" order=3 summary="Short." alias="a.html" -->\n<p>x</p>',
  );
  assert.deepEqual(meta, { title: "A page", track: "reference", order: 3, summary: "Short.", alias: "a.html" });
  assert.equal(body, "<p>x</p>");
  assert.throws(() => parseFrontMatter("<p>no front matter</p>", "x.html"), /x\.html: missing front matter/);
  assert.throws(() => parseFrontMatter('<!-- page: title="A" track="nope" -->', "x.html"), /unknown track "nope"/);
  assert.throws(() => parseFrontMatter('<!-- page: title="A" colour="red" -->', "x.html"), /unknown front-matter key/);
  assert.throws(() => parseFrontMatter('<!-- page: track="reference" -->', "x.html"), /no title/);
});

test("headings get ids, copy anchors and a table of contents; umlauts slug to ascii", () => {
  const { html, headings, intro, sections } = processHeadings(
    "<p>Intro.</p><h2>Die Notiz über alles</h2><p>Body one.</p><h3>Same</h3><h3>Same</h3>",
  );
  assert.deepEqual(
    headings.map((h) => [h.level, h.id, h.text]),
    [
      [2, "die-notiz-ueber-alles", "Die Notiz über alles"],
      [3, "same", "Same"],
      [3, "same-2", "Same"],
    ],
  );
  assert.match(html, /<h2 id="die-notiz-ueber-alles">Die Notiz über alles<a class="anchor"/);
  assert.equal(intro, "Intro.");
  assert.equal(sections[0], "Body one.");
});

test("the version comes from pubspec.yaml without the build number", () => {
  assert.equal(readVersion("name: oldbrainfix\nversion: 0.1.0+1\n"), "0.1.0");
  assert.throws(() => readVersion("name: oldbrainfix\n"), /no version/);
  assert.equal(readVersion(fs.readFileSync(path.join(REPO_ROOT, "pubspec.yaml"), "utf8")).split(".").length, 3);
});

test("an English page without its German twin fails the build", () => {
  const { src, out } = tinySite({ "reference/notes.html": { en: PAGE("Notes", "<p>x</p>", 'track="reference"') } });
  assert.throws(
    () => collectPages({ srcDir: src }),
    /en\/reference\/notes\.html: no de sibling \(expected de\/reference\/notes\.html\)/,
  );
  assert.throws(() => buildSite({ srcDir: src, outDir: out, version: "0.1.0" }), /no de sibling/);
  assert.ok(!fs.existsSync(out), "nothing is written");
});

test("a German page without its English twin fails the build too", () => {
  const { src } = tinySite({ "reference/notizen.html": { de: PAGE("Notizen", "<p>x</p>", 'track="reference"') } });
  assert.throws(() => collectPages({ srcDir: src }), /de\/reference\/notizen\.html: no en sibling/);
});

test("the switch leads to the same page in the other language, else to its start page", () => {
  const pages = [
    { lang: "en", path: "index.html", url: "en/index.html" },
    { lang: "de", path: "index.html", url: "de/index.html" },
    { lang: "en", path: "reference/glossary.html", url: "en/reference/glossary.html" },
    { lang: "de", path: "reference/glossary.html", url: "de/reference/glossary.html" },
    { lang: "en", path: "reference/only.html", url: "en/reference/only.html" },
  ];
  assert.equal(switchHref(pages[2], pages), "de/reference/glossary.html");
  assert.equal(switchHref(pages[3], pages), "en/reference/glossary.html");
  assert.equal(switchHref(pages[4], pages), "de/index.html");
});

test("a build writes both trees, a search index per language, the alias, the root and the 404", () => {
  const { src, out } = tinySite({
    "reference/privacy-policy.html": {
      en: PAGE("Privacy", '<p>See <a href="../index.html">home</a>.</p><h2>Data</h2><p>None leaves.</p>', 'track="reference" alias="privacy-policy.html"'),
      de: PAGE("Datenschutz", '<p>Siehe <a href="../index.html">Start</a>.</p><h2>Daten</h2><p>Nichts geht raus.</p>', 'track="reference"'),
    },
  });
  const result = buildSite({ srcDir: src, outDir: out, version: "9.9.9", base: "/oldbrainfix/" });
  assert.equal(result.pages, 4);
  const en = fs.readFileSync(path.join(out, "en", "reference", "privacy-policy.html"), "utf8");
  assert.match(en, /<html lang="en"/);
  assert.match(en, /class="lang-switch" href="\.\.\/\.\.\/de\/reference\/privacy-policy\.html"/);
  assert.match(en, /v9\.9\.9/);
  assert.match(en, /<p class="eyebrow track-eyebrow">Reference<\/p>/);
  assert.doesNotMatch(en, /fonts\.googleapis|fonts\.gstatic/, "the site loads nothing from elsewhere");
  const de = fs.readFileSync(path.join(out, "de", "reference", "privacy-policy.html"), "utf8");
  assert.match(de, /<html lang="de"/);
  assert.match(de, /Nachschlagen<\/p>/);
  assert.match(de, /Auf dieser Seite/);
  // The alias: the same page at the root, its links one level up.
  const alias = fs.readFileSync(path.join(out, "privacy-policy.html"), "utf8");
  assert.match(alias, /None leaves\./);
  assert.match(alias, /href="en\/index\.html">home</);
  assert.match(alias, /href="assets\/site\.css"/);
  const index = JSON.parse(fs.readFileSync(path.join(out, "en", "search-index.json"), "utf8"));
  assert.deepEqual(index.map((p) => p.url), ["index.html", "reference/privacy-policy.html"]);
  assert.equal(index[1].headings[0].body, "None leaves.");
  const deIndex = JSON.parse(fs.readFileSync(path.join(out, "de", "search-index.json"), "utf8"));
  assert.equal(deIndex[1].track, "Nachschlagen");
  assert.match(fs.readFileSync(path.join(out, "index.html"), "utf8"), /de\/index\.html/);
  assert.match(fs.readFileSync(path.join(out, "404.html"), "utf8"), /Page not found/);
  assert.ok(fs.existsSync(path.join(out, ".nojekyll")));
});

test("a broken link or anchor fails the build, and a root-relative one too", () => {
  const files = new Map([
    ["en/index.html", '<a href="reference/x.html">x</a>'],
    ["en/reference/x.html", '<h2 id="a">A</h2><a href="../index.html#nope">n</a>'],
  ]);
  assert.throws(() => checkLinks(files), /missing anchor #nope/);
  files.set("en/reference/x.html", '<a href="gone.html">g</a>');
  assert.throws(() => checkLinks(files), /"gone\.html" does not resolve/);
  const { src } = tinySite({
    "reference/notes.html": {
      en: PAGE("Notes", '<a href="/en/index.html">root</a>', 'track="reference"'),
      de: PAGE("Notizen", "<p>x</p>", 'track="reference"'),
    },
  });
  assert.throws(() => collectPages({ srcDir: src }), /is root-relative/);
});

test("the repository's own site builds and links, generated pages included", () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), "oldbrainfix-site-dist-"));
  buildSite({ outDir: out, base: "/oldbrainfix/", generated: generatedPages() });
  for (const lang of ["en", "de"]) {
    for (const page of ["index", "glossary", "whats-new", "privacy-policy"]) {
      assert.ok(fs.existsSync(path.join(out, lang, "reference", `${page}.html`)), `${lang}/${page}`);
    }
  }
  // The privacy policy at the root, the address the Play Console holds.
  const policy = fs.readFileSync(path.join(out, "privacy-policy.html"), "utf8");
  assert.match(policy, /There is no server, no account and no analytics/);
  const de = fs.readFileSync(path.join(out, "de", "reference", "privacy-policy.html"), "utf8");
  assert.match(de, /Es gibt keinen Server, kein Konto und keine Analyse/);
  // Every term of CONTEXT.md is on both glossary pages, the German one in German.
  const glossary = fs.readFileSync(path.join(out, "en", "reference", "glossary.html"), "utf8");
  assert.match(glossary, /<dt id="recovery-sheet">Recovery Sheet<\/dt>/);
  const glossar = fs.readFileSync(path.join(out, "de", "reference", "glossary.html"), "utf8");
  assert.match(glossar, /<dt id="recovery-sheet">Wiederherstellungsblatt<\/dt>/);
  // What's new carries the current release from CHANGELOG.md in both languages.
  const version = readVersion(fs.readFileSync(path.join(REPO_ROOT, "pubspec.yaml"), "utf8"));
  const anchor = `id="v${version.replace(/\./g, "-")}"`;
  assert.ok(fs.readFileSync(path.join(out, "en", "reference", "whats-new.html"), "utf8").includes(anchor));
  assert.ok(fs.readFileSync(path.join(out, "de", "reference", "whats-new.html"), "utf8").includes(anchor));
});

test("the Guide track is in both languages' navigation and on both home pages", () => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), "oldbrainfix-site-dist-"));
  buildSite({ outDir: out, base: "/oldbrainfix/", generated: generatedPages() });
  for (const [lang, label] of [["en", "Guide"], ["de", "Anleitung"]]) {
    const home = fs.readFileSync(path.join(out, lang, "index.html"), "utf8");
    assert.match(home, new RegExp(`<section class="nav-track" aria-label="${label}"><p class="eyebrow">${label}</p><ul><li><a href="\\.\\./${lang}/guide/index\\.html">`));
    assert.match(home, /<a class="card" href="guide\/index\.html">/);
    const guide = fs.readFileSync(path.join(out, lang, "guide", "index.html"), "utf8");
    assert.match(guide, new RegExp(`<p class="eyebrow track-eyebrow">${label}</p>`));
  }
});

test("every guide frame comes in English and German, light and dark", () => {
  const frames = path.join(REPO_ROOT, "site", "assets", "frames");
  for (const device of fs.readdirSync(frames)) {
    const names = (lang, theme) => {
      const dir = path.join(frames, device, lang, theme);
      return fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith(".png")).sort() : [];
    };
    const all = new Set(["en", "de"].flatMap((l) => ["light", "dark"].flatMap((t) => names(l, t))));
    assert.ok(all.size > 0, `${device}: no frames`);
    for (const lang of ["en", "de"]) {
      for (const theme of ["light", "dark"]) {
        assert.deepEqual(names(lang, theme), [...all].sort(), `${device}/${lang}/${theme}`);
      }
    }
  }
});

test("the glossary is read from CONTEXT.md, and a term without its German entry fails", () => {
  const entries = parseGlossary("## Language\n\n### Capturing\n\n**Note**:\nA thought.\n_Avoid_: memo\n\n**Bereich** [Area]:\nA shelf.\n");
  assert.deepEqual(entries.map((e) => [e.term, e.english, e.avoid]), [["Note", "Note", "memo"], ["Bereich", "Area", ""]]);
  assert.throws(() => parseGlossary("nothing here", "x.md"), /x\.md: no glossary entries/);
  const { root, src } = tinyRepo({
    "CONTEXT.md": "## Language\n\n**Note**:\nA thought.\n\n**Area**:\nA shelf.\n",
    "site/glossary.de.md": "**Notiz** [Note]:\nEin Gedanke.\n",
  });
  assert.throws(() => glossaryPages(root, src), /no German entry for "Area"/);
  fs.appendFileSync(path.join(src, "glossary.de.md"), "\n**Bereich** [Area]:\nEin Regal.\n");
  const [en, de] = glossaryPages(root, src);
  assert.match(en.body, /<dt id="area">Area<\/dt>/);
  assert.match(de.body, /<dt id="area">Bereich<\/dt>/);
});

test("What's new is read from the changelogs, and a release without German notes fails", () => {
  const log = parseChangelog("## [0.2.0]\n\n### Added\n- One.\n  more\n- Two.\n\n## [0.1.0] - 2026-10-05\n\n### Fixed\n- Three.\n");
  assert.deepEqual(log.map((s) => [s.version, s.date, s.categories.map((c) => [c.name, c.items])]), [
    ["0.2.0", "", [["Added", ["One. more", "Two."]]]],
    ["0.1.0", "2026-10-05", [["Fixed", ["Three."]]]],
  ]);
  const { root, src } = tinyRepo({
    "CHANGELOG.md": "# Changelog\n\n## [0.2.0]\n\n### Added\n- Sites.\n",
    "OLDER_CHANGES.md": "# Older changes\n\nNothing moved here yet.\n",
    "site/changelog.de.md": "# Änderungen\n\n## [0.1.0]\n\n### Neu\n- Alt.\n",
  });
  assert.throws(() => whatsNewPages(root, src), /no German section for \[0\.2\.0\]/);
  fs.writeFileSync(path.join(src, "changelog.de.md"), "# Änderungen\n\n## [0.2.0]\n\n### Neu\n- Seiten.\n");
  const [en, de] = whatsNewPages(root, src);
  assert.match(en.body, /<h3>Added<\/h3>\n<ul>\n<li>Sites\.<\/li>/);
  assert.match(de.body, /<h3>Neu<\/h3>\n<ul>\n<li>Seiten\.<\/li>/);
  assert.match(de.body, /In Arbeit/);
});
