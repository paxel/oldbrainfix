# Guide Writing

How AI coding agents update the user guide (`site/en/guide/`, `site/de/guide/`) when a feature changes. Its readers forget things and have short attention spans, ADHD included: they read the box, rarely the details.

## The box

On a page of boxes, every section (`<h2>`) starts with one box (#171). The build (`tool/site/build.js`) refuses a box that breaks a rule; `node --test "tool/site/*.test.js"` runs it.

```html
<guide-box name="start-new">
<h2>Start new</h2>
<p>Start with an empty app and the Areas you pick.</p>
<box-tab device="phone" alt="Pick your Areas on the phone">
<li>Tap <strong>New to Old Brain Fix</strong>.</li>
</box-tab>
<box-tab device="desktop" alt="Pick your Areas in the desktop app">
<li>Click <strong>New to Old Brain Fix</strong>.</li>
</box-tab>
<box-more>
<p>Reasons, exceptions and everything else.</p>
</box-more>
</guide-box>
```

- **Heading**: the task in everyday words, as the reader would ask for it ("Import notes from Google Keep"). The guide's start page lists every box by its heading.
- **Purpose**: one line under the heading, what the box is for.
- **Tabs**: one `phone` and one `desktop`, each with at most 5 steps. A step is one action, starts with a verb, has at most 12 words and gives no reason: "Tap + ▸ Checklist." A word holds a letter or a digit, so `+` and `▸` are none. Name buttons as the app does, in `<strong>`.
- **More details**: `<box-more>`, folded by default, holds reasons, exceptions, limits and what the screen says.
- **Frames**: each tab shows its frame beside the steps, from `site/assets/frames/<device>/<lang>/<theme>/<page>/<name>.png`, in English and German, light and dark. Add the scene with `boxFrame('<page>', '<name>', …)` in `test/screenshots/screenshots_test.dart` and `box_frame("<page>", "<name>")` in `desktop/crates/oldbrainfix-gui/tests/screenshots.rs`, then write the frames with `tool/guide-frames.sh` (see its header).
- The `name` is the same in both languages; the German page has the same boxes in the same order.

## Updating with a feature

- A feature that changes what the reader does changes the steps of its box, on both tabs, in both languages; a changed screen regenerates its frames.
- Keep the box short: when a step needs a reason or an exception, the reason goes into the details.
- Facts, serious, no sales voice. The German uses the app's terms (`lib/l10n/app_de.arb`, `site/glossary.de.md`).
