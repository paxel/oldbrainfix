// Pages the build writes from the repository's own files (#44), as
// Catlog's: the glossary from CONTEXT.md and site/glossary.de.md, and
// What's new from the changelogs. Each is a page like an authored one, in
// both languages, and the build fails when the sources disagree with each
// other.
import fs from "node:fs";
import path from "node:path";
import { REPO_ROOT, escapeHtml, slugify } from "./common.js";

/** Markdown inline: `code`, **bold**, _em_ and [text](url). */
export function inline(text) {
  return escapeHtml(text)
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/\[([^\]]+)\]\((https?:[^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/(^|\s)_([^_]+)_(?=[\s.,;:]|$)/g, "$1<em>$2</em>");
}

// ---------------------------------------------------------------------------
// Glossary

/** The `**Term**:\n description\n _Avoid_: …` entries of a text; `[English]` after a term names its English twin. */
export function parseGlossary(text, file = "glossary") {
  const body = text.includes("## Language") ? text.slice(text.indexOf("## Language")) : text;
  const entries = [];
  for (const block of body.split(/\n\s*\n/)) {
    const lines = block.trim().split("\n");
    const head = /^\*\*([^*]+)\*\*(?:\s*\[([^\]]+)\])?:\s*$/.exec(lines[0] || "");
    if (!head) continue;
    const rest = lines.slice(1);
    const avoid = rest.filter((l) => l.startsWith("_Avoid_:")).map((l) => l.replace("_Avoid_:", "").trim());
    const description = rest.filter((l) => !l.startsWith("_Avoid_:")).join(" ").trim();
    entries.push({ term: head[1].trim(), english: (head[2] || head[1]).trim(), description, avoid: avoid[0] || "" });
  }
  if (!entries.length) throw new Error(`${file}: no glossary entries found`);
  return entries;
}

const GLOSSARY_TEXT = {
  en: {
    title: "Glossary",
    summary: "Every word Old Brain Fix uses, in one or two sentences: what it means, and the words it avoids.",
    lede: "Every term the app uses, in one or two sentences, grouped as the app's own model lists them.",
    avoid: "Not called",
  },
  de: {
    title: "Glossar",
    summary: "Jedes Wort, das Old Brain Fix benutzt, in ein oder zwei Sätzen: was es bedeutet, und die Wörter, die es meidet.",
    lede: "Jeder Begriff, den die App benutzt, in ein oder zwei Sätzen, in der Reihenfolge des Modells der App.",
    avoid: "Heißt nicht",
  },
};

export function glossaryPages(repoRoot = REPO_ROOT, srcDir = path.join(REPO_ROOT, "site")) {
  const en = parseGlossary(fs.readFileSync(path.join(repoRoot, "CONTEXT.md"), "utf8"), "CONTEXT.md");
  const deFile = path.join(srcDir, "glossary.de.md");
  const de = parseGlossary(fs.readFileSync(deFile, "utf8"), "site/glossary.de.md");
  const deByEnglish = new Map(de.map((e) => [e.english, e]));
  for (const entry of en) {
    if (!deByEnglish.has(entry.term)) {
      throw new Error(`site/glossary.de.md: no German entry for "${entry.term}" (write **Deutsch** [${entry.term}]:)`);
    }
  }
  const render = (lang, entries) => {
    const t = GLOSSARY_TEXT[lang];
    const items = entries
      .map(
        (e) =>
          `<dt id="${slugify(e.english)}">${escapeHtml(e.term)}</dt>\n<dd>${inline(e.description)}` +
          (e.avoid ? ` <span class="avoid">${escapeHtml(t.avoid)}: ${escapeHtml(e.avoid)}.</span>` : "") +
          `</dd>`,
      )
      .join("\n");
    return `<p class="lede">${escapeHtml(t.lede)}</p>\n<dl class="glossary">\n${items}\n</dl>`;
  };
  // The German list in the English order, so the two pages read alike.
  const deOrdered = en.map((e) => deByEnglish.get(e.term));
  return [
    { lang: "en", path: "reference/glossary.html", url: "en/reference/glossary.html", track: "reference", order: 1, title: GLOSSARY_TEXT.en.title, summary: GLOSSARY_TEXT.en.summary, alias: "", body: render("en", en) },
    { lang: "de", path: "reference/glossary.html", url: "de/reference/glossary.html", track: "reference", order: 1, title: GLOSSARY_TEXT.de.title, summary: GLOSSARY_TEXT.de.summary, alias: "", body: render("de", deOrdered) },
  ];
}

// ---------------------------------------------------------------------------
// What's new

const CATEGORY = {
  en: { Added: "Added", Changed: "Changed", Deprecated: "Deprecated", Removed: "Removed", Fixed: "Fixed", Security: "Security" },
  de: { Added: "Neu", Changed: "Geändert", Deprecated: "Veraltet", Removed: "Entfernt", Fixed: "Behoben", Security: "Sicherheit" },
};
const NEWS_TEXT = {
  en: { title: "What's new", summary: "Every release of Old Brain Fix, newest first, with what changed for you.", lede: "Every release, newest first: what you see, in the words of the changelog.", unreleased: "In the works" },
  de: { title: "Was ist neu", summary: "Jede Version von Old Brain Fix, neueste zuerst, mit dem, was sich für dich geändert hat.", lede: "Jede Version, neueste zuerst: was du siehst, in den Worten des Änderungsprotokolls.", unreleased: "In Arbeit" },
};

/** The `## [version] - date` sections of a changelog, each with its categories and bullet lines. */
export function parseChangelog(text, file = "changelog") {
  const sections = [];
  let current = null;
  let category = null;
  for (const line of text.split("\n")) {
    const head = /^## \[([^\]]+)\](?:\s*-\s*(\S+))?/.exec(line);
    if (head) {
      current = { version: head[1], date: head[2] || "", categories: [] };
      sections.push(current);
      category = null;
      continue;
    }
    if (!current) continue;
    const cat = /^### (\w+)/.exec(line);
    if (cat) {
      category = { name: cat[1], items: [] };
      current.categories.push(category);
      continue;
    }
    if (line.startsWith("- ") && category) category.items.push(line.slice(2).trim());
    else if (line.startsWith("  ") && category && category.items.length) category.items[category.items.length - 1] += " " + line.trim();
  }
  if (!sections.length) throw new Error(`${file}: no version sections found`);
  return sections;
}

/** The English changelog pair at the root, the German one beside the site's sources. */
function readChangelogs(repoRoot, srcDir) {
  const read = (file, name) => {
    if (!fs.existsSync(file)) return [];
    const text = fs.readFileSync(file, "utf8");
    // OLDER_CHANGES.md has no section until the first version bump moves one there.
    if (name === "OLDER_CHANGES.md" && !/^## \[/m.test(text)) return [];
    return parseChangelog(text, name);
  };
  return {
    en: [
      ...read(path.join(repoRoot, "CHANGELOG.md"), "CHANGELOG.md"),
      ...read(path.join(repoRoot, "OLDER_CHANGES.md"), "OLDER_CHANGES.md"),
    ],
    de: read(path.join(srcDir, "changelog.de.md"), "site/changelog.de.md"),
  };
}

/** What's new in both languages; every English section needs its German twin, dated alike. */
export function whatsNewPages(repoRoot = REPO_ROOT, srcDir = path.join(REPO_ROOT, "site")) {
  const { en, de } = readChangelogs(repoRoot, srcDir);
  if (!en.length) throw new Error("CHANGELOG.md: no version sections found");
  const deByVersion = new Map(de.map((s) => [s.version, s]));
  const enByVersion = new Map(en.map((s) => [s.version, s]));
  for (const s of en) {
    const twin = deByVersion.get(s.version);
    if (!twin) throw new Error(`site/changelog.de.md: no German section for [${s.version}]`);
    if (twin.date !== s.date) {
      throw new Error(`[${s.version}]: the German section is dated "${twin.date}", the English "${s.date}"`);
    }
  }
  for (const s of de) {
    if (!enByVersion.has(s.version)) throw new Error(`site/changelog.de.md: [${s.version}] has no English section`);
  }
  const render = (lang) => {
    const t = NEWS_TEXT[lang];
    const parts = en.map((section) => {
      const source = lang === "de" ? deByVersion.get(section.version) : section;
      const heading = `${section.version}${section.date ? ` <small>${escapeHtml(section.date)}</small>` : ` <small>${escapeHtml(t.unreleased)}</small>`}`;
      const cats = source.categories
        .filter((c) => c.items.length)
        .map((c) => `<h3>${escapeHtml(CATEGORY[lang][c.name] || c.name)}</h3>\n<ul>\n${c.items.map((i) => `<li>${inline(i)}</li>`).join("\n")}\n</ul>`)
        .join("\n");
      return `<h2 id="v${section.version.replace(/\./g, "-")}">${heading}</h2>\n${cats}`;
    });
    return `<p class="lede">${escapeHtml(t.lede)}</p>\n${parts.join("\n")}`;
  };
  return ["en", "de"].map((lang) => ({
    lang,
    path: "reference/whats-new.html",
    url: `${lang}/reference/whats-new.html`,
    track: "reference",
    order: 2,
    title: NEWS_TEXT[lang].title,
    summary: NEWS_TEXT[lang].summary,
    alias: "",
    body: render(lang),
  }));
}

export function generatedPages(srcDir = path.join(REPO_ROOT, "site"), repoRoot = REPO_ROOT) {
  return [...glossaryPages(repoRoot, srcDir), ...whatsNewPages(repoRoot, srcDir)];
}
