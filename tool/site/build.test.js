// The site build (#44), as Catlog tests its own: front matter, headings, the
// two trees and their twins, the language switch, the alias, the search index
// per language, the link check, and the glossary and What's new generated
// from CONTEXT.md and the changelogs. Run with `node --test "tool/site/*.test.js"`.
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { after, test } from "node:test";
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

/** Every temp dir the tests make, removed when they end: /tmp is a tmpfs. */
const temps = [];
function tempDir(prefix) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  temps.push(dir);
  return dir;
}
after(() => {
  for (const dir of temps) fs.rmSync(dir, { recursive: true, force: true });
});

const PAGE = (title, body = "<p>Text.</p>", extra = "") =>
  `<!-- page: title="${title}" ${extra} -->\n${body}\n`;

/** A tiny site in a temp dir: both trees with an index and the pages given as {path: {en, de}}. */
function tinySite(pages = {}) {
  const dir = tempDir("oldbrainfix-site-");
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
  const dir = tempDir("oldbrainfix-repo-");
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
  const out = tempDir("oldbrainfix-site-dist-");
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
  const out = tempDir("oldbrainfix-site-dist-");
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
    // A box's frames lie in a folder of their page (#171): walked too.
    const names = (lang, theme) => {
      const dir = path.join(frames, device, lang, theme);
      if (!fs.existsSync(dir)) return [];
      return fs
        .readdirSync(dir, { recursive: true })
        .map((f) => String(f).split(path.sep).join("/"))
        .filter((f) => f.endsWith(".png"))
        .sort();
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

// ---------------------------------------------------------------------------
// The guide's boxes (#171): a short box per section, Phone and Desktop tabs,
// a frame per tab, the details folded; the start page lists every box.

/** One `<li>` per step. */
const steps = (items) => items.map((i) => `<li>${i}</li>`).join("\n");

/** A guide box in the authored form. */
function BOX({
  name = "start-new",
  heading = "Start new",
  purpose = "Pick your Areas.",
  phone = ["Tap <strong>New</strong>.", "Tap <strong>Start</strong>."],
  desktop = ["Click <strong>New</strong>."],
  devices = ["phone", "desktop"],
  more = "<p>Why it is so.</p>",
} = {}) {
  const tab = (device) =>
    `<box-tab device="${device}" alt="The ${device}">\n${steps(device === "phone" ? phone : desktop)}\n</box-tab>`;
  return (
    `<guide-box name="${name}">\n<h2>${heading}</h2>\n<p>${purpose}</p>\n` +
    devices.map(tab).join("\n") +
    (more ? `\n<box-more>\n${more}\n</box-more>` : "") +
    `\n</guide-box>`
  );
}

/** A guide page of boxes in both languages, the start page listing them, and every frame they show. */
function guideSite({ en = BOX(), de = en, frames = true, index = "<guide-questions></guide-questions>" } = {}) {
  const page = (title, body) => PAGE(title, body, 'track="guide" order=1');
  const site = tinySite({
    "guide/index.html": {
      en: PAGE("Guide", index, 'track="guide" order=0'),
      de: PAGE("Anleitung", index, 'track="guide" order=0'),
    },
    "guide/first.html": { en: page("First", en), de: page("Erstes", de) },
  });
  if (frames) {
    for (const device of ["phone", "desktop"]) {
      for (const lang of ["en", "de"]) {
        for (const theme of ["light", "dark"]) {
          for (const m of en.matchAll(/<guide-box name="([^"]+)"/g)) writeFrame(site.src, device, lang, theme, m[1]);
        }
      }
    }
  }
  return site;
}

/** A PNG header, enough for the build to read the frame's size. */
function writeFrame(src, device, lang, theme, box, page = "first") {
  const file = path.join(src, "assets", "frames", device, lang, theme, page, `${box}.png`);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  const png = Buffer.alloc(24);
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]).copy(png);
  png.writeUInt32BE(device === "phone" ? 648 : 1280, 16);
  png.writeUInt32BE(device === "phone" ? 1152 : 800, 20);
  fs.writeFileSync(file, png);
}

test("a box renders its purpose, steps, tabs, frames and folded details", () => {
  const { src, out } = guideSite({
    de: BOX({ heading: "Neu anfangen", purpose: "Wähle deine Bereiche.", phone: ["Tippe <strong>Neu</strong>."], desktop: ["Klicke <strong>Neu</strong>."] }),
  });
  buildSite({ srcDir: src, outDir: out, version: "1.0.0" });
  const en = fs.readFileSync(path.join(out, "en", "guide", "first.html"), "utf8");
  assert.match(en, /<section class="box" aria-labelledby="start-new">\n<h2 id="start-new">Start new<a class="anchor"/);
  assert.match(en, /<p class="box-purpose">Pick your Areas\.<\/p>/);
  assert.match(en, /<div class="box-tabs" role="tablist" aria-label="Device" hidden>/);
  assert.match(
    en,
    /<button type="button" role="tab" id="start-new-phone-tab" aria-controls="start-new-phone" aria-selected="true" tabindex="0" data-device="phone">Phone<\/button>/,
  );
  assert.match(
    en,
    /<button type="button" role="tab" id="start-new-desktop-tab" aria-controls="start-new-desktop" aria-selected="false" tabindex="-1" data-device="desktop">Desktop<\/button>/,
  );
  // Without script both panels show, each under its device's name.
  assert.match(en, /<div class="box-panel" role="tabpanel" id="start-new-phone" aria-labelledby="start-new-phone-tab" data-device="phone" tabindex="0">\n<p class="box-device">Phone<\/p>\n<ol class="box-steps">\n<li>Tap <strong>New<\/strong>\.<\/li>/);
  assert.match(en, /<div class="box-panel" role="tabpanel" id="start-new-desktop" aria-labelledby="start-new-desktop-tab" data-device="desktop" tabindex="0">/);
  assert.match(en, /<figure class="frame phone">\n<img class="shot-light" src="\.\.\/\.\.\/assets\/frames\/phone\/en\/light\/first\/start-new\.png" alt="The phone" width="648" height="1152" loading="lazy">/);
  assert.match(en, /<img class="shot-dark" src="\.\.\/\.\.\/assets\/frames\/desktop\/en\/dark\/first\/start-new\.png" alt="The desktop" width="1280" height="800" loading="lazy">/);
  assert.match(en, /<details class="box-more">\n<summary>More details<\/summary>\n<p>Why it is so\.<\/p>\n<\/details>/);
  assert.doesNotMatch(en, /<details[^>]* open/, "the details start folded");
  assert.doesNotMatch(en, /<\/?(guide-box|box-tab|box-more)\b/, "nothing of the authored form is left");
  const de = fs.readFileSync(path.join(out, "de", "guide", "first.html"), "utf8");
  assert.match(de, /aria-label="Gerät"/);
  assert.match(de, /data-device="phone">Handy<\/button>/);
  assert.match(de, /<summary>Mehr Details<\/summary>/);
  assert.match(de, /src="\.\.\/\.\.\/assets\/frames\/phone\/de\/light\/first\/start-new\.png"/);
});

test("the box rules: at most 5 steps, each at most 12 words", () => {
  const six = ["One.", "Two.", "Three.", "Four.", "Five.", "Six."];
  assert.throws(() => collectPages({ srcDir: guideSite({ en: BOX({ phone: six }) }).src }), /en\/guide\/first\.html: box "start-new": the phone tab has 6 steps, more than 5/);
  assert.doesNotThrow(() => collectPages({ srcDir: guideSite({ en: BOX({ phone: six.slice(0, 5) }) }).src }));
  const thirteen = "Tap one two three four five six seven eight nine ten <strong>eleven twelve</strong>.";
  assert.throws(() => collectPages({ srcDir: guideSite({ en: BOX({ desktop: [thirteen] }) }).src }), /box "start-new": a desktop step has 13 words, more than 12: "Tap one two/);
  // Twelve words pass; a sign such as + or ▸ is no word.
  const twelve = "Tap + ▸ one two three four five six seven eight nine ten.";
  assert.doesNotThrow(() => collectPages({ srcDir: guideSite({ en: BOX({ desktop: [twelve] }) }).src }));
});

test("the box rules: both tabs, each with its frames in both languages, light and dark", () => {
  assert.throws(() => collectPages({ srcDir: guideSite({ en: BOX({ devices: ["phone"] }) }).src }), /box "start-new": no desktop tab/);
  assert.throws(() => collectPages({ srcDir: guideSite({ en: BOX({ devices: ["desktop"] }) }).src }), /box "start-new": no phone tab/);
  assert.throws(() => collectPages({ srcDir: guideSite({ en: BOX({ devices: ["phone", "phone", "desktop"] }) }).src }), /box "start-new": two phone tabs/);
  const { src } = guideSite();
  fs.rmSync(path.join(src, "assets", "frames", "desktop", "de", "dark", "first", "start-new.png"));
  assert.throws(() => collectPages({ srcDir: src }), /box "start-new": the desktop tab has no frame assets\/frames\/desktop\/de\/dark\/first\/start-new\.png/);
  assert.throws(() => collectPages({ srcDir: guideSite({ frames: false }).src }), /the phone tab has no frame assets\/frames\/phone\/en\/light/);
});

test("the box rules: a purpose, a heading, nothing else; every section of a box page is a box", () => {
  assert.throws(() => collectPages({ srcDir: guideSite({ en: BOX({ purpose: "" }).replace("<p></p>", "") }).src }), /box "start-new": no purpose/);
  assert.throws(
    () => collectPages({ srcDir: guideSite({ en: BOX().replace("</box-more>", "</box-more>\n<p>Loose.</p>") }).src }),
    /box "start-new": unexpected content "<p>Loose\.<\/p>"/,
  );
  assert.throws(() => collectPages({ srcDir: guideSite({ en: `${BOX()}\n<h2>Loose</h2>\n<p>x</p>` }).src }), /en\/guide\/first\.html: the section "Loose" is not a box/);
  assert.throws(() => collectPages({ srcDir: guideSite({ en: `${BOX()}\n${BOX({ heading: "Again" })}` }).src }), /two boxes named "start-new"/);
});

test("a box page's twin has the same boxes, which share their frames", () => {
  const en = `${BOX()}\n${BOX({ name: "restore", heading: "Restore" })}`;
  const de = `${BOX({ name: "restore", heading: "Wiederherstellen" })}\n${BOX({ heading: "Neu" })}`;
  assert.throws(() => collectPages({ srcDir: guideSite({ en, de }).src }), /de\/guide\/first\.html: boxes restore, start-new differ from en\/guide\/first\.html: start-new, restore/);
});

test("the start page lists every box as a question, linked to it", () => {
  const en = `${BOX()}\n${BOX({ name: "restore", heading: "Restore from a Backup" })}`;
  const de = `${BOX({ heading: "Neu anfangen" })}\n${BOX({ name: "restore", heading: "Aus einer Sicherung wiederherstellen" })}`;
  const { src, out } = guideSite({ en, de });
  buildSite({ srcDir: src, outDir: out, version: "1.0.0" });
  const index = fs.readFileSync(path.join(out, "en", "guide", "index.html"), "utf8");
  assert.match(
    index,
    /<div class="questions">\n<p class="questions-page">First<\/p>\n<ul>\n<li><a href="first\.html#start-new">Start new<\/a><\/li>\n<li><a href="first\.html#restore-from-a-backup">Restore from a Backup<\/a><\/li>\n<\/ul>\n<\/div>/,
  );
  const de2 = fs.readFileSync(path.join(out, "de", "guide", "index.html"), "utf8");
  assert.match(de2, /<li><a href="first\.html#aus-einer-sicherung-wiederherstellen">Aus einer Sicherung wiederherstellen<\/a><\/li>/);
  // Without the list on the start page, a box could be missed: the build refuses.
  assert.throws(() => collectPages({ srcDir: guideSite({ index: "<p>Hello.</p>" }).src }), /en\/guide\/index\.html: no <guide-questions> for the boxes of the guide/);
});

test("the repository's guide: getting started is all boxes, and each is a question on the start page", () => {
  const pages = collectPages({ generated: generatedPages() });
  for (const lang of ["en", "de"]) {
    const page = pages.find((p) => p.url === `${lang}/guide/getting-started.html`);
    const ids = [...page.body.matchAll(/<section class="box" aria-labelledby="([^"]+)">/g)].map((m) => m[1]);
    assert.ok(ids.length >= 5, `${lang}: boxes ${ids}`);
    const index = pages.find((p) => p.url === `${lang}/guide/index.html`);
    for (const id of ids) assert.ok(index.body.includes(`href="getting-started.html#${id}"`), `${lang}: ${id} on the start page`);
  }
});
