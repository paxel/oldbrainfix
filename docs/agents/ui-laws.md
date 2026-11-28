# UI laws

Rules every screen follows. Check a change against them before it lands;
a screen that breaks one is a bug, not a style choice. Terms are those of
`CONTEXT.md`. #n refers to issue n on https://github.com/paxel/oldbrainfix/issues.

The laws come in three parts (#196): **Both apps** binds the phone and the
desktop alike; **Phone** and **Desktop** hold what only one app does, and
replace a law of Both apps where they say so. A desktop law marked
"(draft, #196)" waits for the user's review.

## Design system

The building blocks every screen is made of, defined once for both apps
(obf-1081, C24). This section stands before the three parts and binds
the phone and the desktop alike. A screen uses these blocks and invents
no others; a block missing here is added here first. The page that
shows each one live was approved on 2026-11-24 (obf-1081), and each
app builds its screens from one shared widget set.

- **Everything tappable is recognisable as tappable** (obf-1081): a
  fill, an outline, or the accent's colour on its words and icon says
  "this does something". A label with an icon in the text's or the weak
  text's colour, without a frame, is never the only sign of an action;
  text in the weak colour reads as a hint, never as a button.

### Buttons

There are three kinds, and an action takes the first that fits.

- **Filled: the main action.** At most one on a dialog or page (Save,
  Create, Done, Use, Restore, Continue), on the accent with its words
  in `on_accent`. An action that deletes or removes (Delete, Remove,
  Move to trash) is filled on the error colour instead. Phone:
  `FilledButton` / `FilledButton.icon`, and `DestructiveButton` in
  `look.dart`. Desktop: `theme::primary`,
  `theme::main_action` (which Ctrl+Enter also runs) and
  `theme::destructive` in `theme.rs`. Do not use it for a second action
  beside the main one, nor as a page's only way to leave. A page's bar
  is the exception: Save there stays the bar's labelled button, as
  "The bar" says, and the filled kind is for the actions inside the
  page (obf-1105).
- **Outlined: a standing action that is not the main one.** It has a
  1 px outline and its words, an icon before them where one says more,
  for example the quick note's full-width "Open in the editor"
  (obf-1080) or the list band's "+ New Note", "View" and "Show Notes
  by…" (obf-1067). Phone: `OutlinedButton` / `OutlinedButton.icon`,
  which `lookTheme`'s `outlinedButtonTheme` in `look.dart` gives the
  tonal fill with the outline. Desktop: every button egui draws (`ui.button`, `Button::new`) wears
  the secondary look of "Buttons look like buttons" (Desktop ▸ Buttons
  and look, #218), a tonal fill with the outline, except Cancel and
  Close in a dialog's action row, which are text buttons without an
  icon (`design::row_button`, obf-1102); `band_button` in
  `app.rs` draws that frame at the band's height, and
  `capture_choices::open_in_editor` wears it across the width
  (obf-1081). Do not use it for the main action or in a menu.
- **Text with icon: a lesser action inside a card or a row** ("Copy
  text", the Frogs' "Another one"). It has no frame: the icon and the
  words are both in the accent, which is what makes it a button, and it
  gets a tinted ground under the finger or pointer. Phone:
  `TextButton.icon`. Desktop: `design::text_button` in `design.rs`
  (obf-1081), its words and icon in the accent's text colour. Do not draw it
  in the text's or the weak colour, and do not leave out its icon. A
  dialog's action row is the one place a text button goes without an
  icon (Cancel, Close, Not now beside the filled main action), because
  the row itself shows that these are its buttons; desktop:
  `design::row_button` (obf-1102).
- Icon-only buttons (⋮, the pencil, the view button, the format bar)
  get a rounded ground under the pointer and the ring while focused
  (desktop `theme::icon_button_back`, phone `IconButton`), and they
  always have a tooltip and a screen-reader label. On the desktop the
  count stepper's − and + wear the outlined kind's frame in place of
  that ground (`design::framed_icon_button`, obf-1112). Menu entries stay
  flat, links stay links, and badges keep their own look (#192).
- **The setting badge is a block of its own** (#160, obf-1117): a
  Note's setting in the Note editor and the Note dialog is a split box
  with radius 8, its icon in a tinted cell on the left and its value on
  the right, a drop-down mark after the value where a tap opens a
  choice. It is neither a button nor a chip; the buttons beside it in a
  group keep their own kind and shape. Phone: `setting_badges.dart`.
  Desktop: `setting_badges.rs`.

### The header band

- **One band carries a list's or a picker's search** (#351, obf-1041,
  obf-1064, obf-1088). It is filled with a tone set off from the
  ground, has a line along its bottom, and whatever scrolls goes under
  that line. On the desktop the band of a list of Notes has two rows:
  back, forward and the path with the place as its title (22), then
  the search field (its text 16), the view switch, "Show Notes by…"
  with its chips, and "+ New Note", all one height (36), wrapping under
  the field below 200 wide.
- Phone: `SearchBand`, and `SearchHeaderSliver` for a list that scrolls
  it away, followed by `SearchHeaderLine`, whose line stays under the
  bar once the band is gone (obf-1129), in `browser.dart`; its fill is
  `searchHeaderColour`, the
  marks' tone (`LookTones.mark` in `look.dart`). Desktop:
  `theme::search_band` with `theme::band_fill`, and `list_band` /
  `band_tools` with `BandSizes` in `app.rs`.
- Use it on every page or dialog that searches what it lists. Do not
  use it as a decoration on a page without a search, and never put a
  second band under it.

### Cards

- **A card is an embossed panel, radius 16**: the card fill, a shade
  below and right, a highlight above and left, and a stronger lift for
  important ones (see Cards). Phone: `Embossed` in `look.dart`. Desktop:
  `theme::embossed`, laid out by `embossed` / `agenda_card` in `app.rs`.
  An Area's colour runs down its left edge, 6 wide (`theme::area_band`).
  An Area's colour is drawn everywhere in its shade for the look, at
  least 3:1 to the ground, the question marks and the cards (phone
  `areaColour`, desktop `area_colour`, obf-1156); a Topic's icon is
  drawn in its Area's colour.
- Its kinds are a Note's card (`NoteCard`, padding 16, on the desktop
  14; the card text is 16), a Settings section's card with its title on top
  (obf-1085, phone `SettingsSection`), and the one card of a Checklist,
  Shopping List or Tally, its name, description and content together
  (obf-1071).
- Use a card for a thing (a Note or a section of settings) that stands
  over the question marks. Do not nest a card in a card, and do not
  give a Section, a Line or a field a card of its own.

### Chips

- **A chip is a short, tappable word with rounded ends**, a 1 px
  outline, and its fill tinted with the accent while picked (the phone
  gives Material's chip the pill and 32 inside the outline once, in
  `lookTheme`'s `chipTheme` in `look.dart`; the desktop
  `design::Chip` and `design::chip` in `design.rs`, a full pill 32
  high, in a list's band the band's 36, obf-1081). There are
  three uses:
  - **Filter chips** pick what a list shows and can be taken back
    (the Session pool's tags, #220; a Routine's weekdays; the person
    filter's kept people, whose ✕ takes them out, obf-1064). Phone:
    `FilterChip` / `InputChip`. Desktop: `design::chip`, a kept
    person's with `Chip::closing`.
  - **Tag chips** start a search for a tag, and the row ends in
    "All tags…" (obf-1074), as do the tags a typed "#…" suggests.
    Phone: `TagChipRow` (`ActionChip`) in `tag_tree_screen.dart`.
    Desktop: `tag_tree_view::most_used_row` and
    `tag_tree_view::suggestions`, of `design::Chip`.
  - **Choice chips** pick exactly one of a few short options (Theme,
    Text size). Phone: `ChoiceChips` in `look.dart`. Desktop:
    `chip_row` in `app.rs` (`Chip::radio`).
- Use a chip for a short word in a row of equals. Do not use one for
  an action with a consequence (use a button), nor for a choice with
  details (that is a `RadioColumn` or `radio_column`).

### Fields

- **Fields share one height** (obf-1128): on the desktop a framed
  single-line field is as tall as a button beside it, whatever its
  font and the text size, its text 8 inside the outline at the sides.
  Desktop: `theme::field`, and `theme::field_margin` for a field in
  another font. A list's search field is the band's 36 (`band_search`
  in `app.rs`); a field without a frame inside a card, a cell or a
  Line keeps its own measures.

### Spacing and type

- **Spacing comes in steps of 4, 8, 12, 16, 24 and 32** (logical pixels
  on the phone, points on the desktop) for the room between blocks and
  a card's padding. Use 4 between a control's parts, 8 between chips,
  the rows of a card or the controls of a row, 12 between groups, 16
  for a card's padding and the page's side gutter, and 24 or 32 between
  sections. Any other step there is a break. A block's own inner
  measures (a button's 10 × 4 padding, a chip's, the Area band's 6) are
  part of its look and stay as this section gives them.
- **Text sizes**: content (a card's text, Lines, typed text) is 16 and
  grows with Text size (#326; phone `bodyLarge` in `ContentText`,
  desktop `theme::content`). A page's or band's title is 22 (phone
  `titleLarge`, desktop egui's heading text style, `ui.heading`, and
  `BandSizes::title`, both `theme::TITLE`, obf-1121). A dialog's heading is
  Material's dialog title on the phone and 18 strong on the desktop
  (`design::dialog_title`, obf-1125). A card's or section's heading is 16
  medium (phone `titleMedium`, through `sectionHeading` in `look.dart`,
  obf-1124), on the desktop strong. Strong text on the desktop is set
  off by the theme's strong tone (`TextTones::strong`, darker than the
  text in light, brighter in dark) and carries no colour of its own,
  which would win over it (obf-1126). Labels and
  buttons are 14 (phone `labelLarge` / `bodyMedium`, desktop egui's
  body and button text styles, `theme::LABEL`). A caption, meta line
  or hint is 12 in the weak colour (phone `bodySmall`, desktop
  `.small()`, `theme::CAPTION`). The desktop sets these once in egui's
  text styles (`theme::install`), grown by Text size as content is
  (obf-1081). Fat fingers grows all of them (#224); on the phone Text
  size grows only content.

## Both apps

### Navigation

- Every browser level (Areas, Topics, Notes) has a "+" that adds the
  thing shown on that level. On the Areas list it adds an Area. **In an
  Area, a Topic and All Notes "+" offers Note, Checklist, Shopping
  List, Tally, Session and Frog** (#161), and in an Area New Topic
  (obf-1014);
  Session, Frog and Topic only for who may set them up, and Tally,
  Session, Frog and Topic only in Advanced (#197). What it makes is
  filed into the Area or Topic shown, from All Notes into none: Note
  opens Capture; Checklist and Shopping List ask their name, then open
  the editor with the cursor in the first Line or the field at the top;
  Frog asks its name only. **Tally is one page**: its name, a row per
  field, one at first, with "Add field" for the next, each field's rule,
  unit and target as badges whose tap opens their small editor, the unit
  typed with suggestions; rows left without a name are dropped on Save.
  **Session is one page**: its name, its pool as a list of checkboxes
  ending in "New Tally", which opens the Tally page and comes back with
  that Tally ticked, then its count, rule and schedule as badges, the
  schedule every day, all day, at first; it makes the Session's Note
  with its first Routine (#191). **Every Session pool list filters by
  tag** (#220), on this page, the Session Note's page and the Routine
  editor: the #tags of its Tallies' texts as filter chips above it,
  those most Tallies carry first; the Tallies carrying any chip picked,
  or a tag nested under it (`#sport/run` under `#sport`, #267), show,
  ticks the filter hides stay and a line counts them, "New Tally"
  starts its name with the tags picked, and Rolling's order moves only
  unfiltered. The filter is a view, kept while the page is open, never
  stored. Leaving a name dialog without Create makes nothing. **Leaving
  saves what is valid and asks only about what isn't** (#275, Q658):
  the Tally and Session pages, the Routine editor and a Tally entry,
  left by Back, the swipe back or the bar's arrow, on the desktop by
  Esc, a click beside the dialog, Alt+Left or the mouse's back button,
  save as their Save does once something changed; when it can't be saved, a schedule that
  never happens or an empty required value, they ask once "Discard
  changes?", Discard or Keep editing, which shows what to fix. Left as
  they opened they make nothing; a Tally made on the way stays. The
  desktop's Cancel leaves without saving.
- **Nothing slides in from the bottom**, anywhere: a bottom sheet is a
  bug. A menu opens where the finger is; choices that outgrow a menu
  are a page.
- **Simple or Advanced** (#197, ADR 0041) is one switch of the
  person, asked on the first start's Starter Set page above the Areas,
  two radio buttons with Simple picked, and changed in Settings after
  the settings of the Device. **Simple hides only making**:
  Tally, Session, Frog and Topic in "+", New Topic… in File into…
  (#272), Frog and Session in every spawn menu, New Session, Tally and
  Session in the kind badge, "Make it a Tally", the format bar, and
  roles when sharing and in People.
  What exists shows and works as in Advanced. Never chosen is Advanced.

### Clicks and taps

- A checkbox or switch is the only thing that toggles state; the row
  around it does not.
- **A tick shows at once** (#338, Q752): a box ticks, and its Line
  strikes through, in the frame of the tap or click; the save follows
  after that frame. A save that fails shows the box as stored again and
  says so at the top. On the phone, on the desktop, everywhere a box is.
- **A time budget** (#339, Q753): every tap or click shows its effect
  within 100 ms, every page change within 500 ms, counted from the tap to
  the frame that shows it. A notification tapped while the app runs
  opens what it is about within the 500 ms (obf-1039). On the phone and
  on the desktop.
- **A button tap shows a green "!"** (obf-1143), as cat(a)log's paw:
  every tap of a button of any kind, filled, outlined, text and icon
  buttons and the "+", shows a green "!" 56 high above the finger, on
  the desktop above the pointer, growing and fading out over 0.9 s,
  silent, drawn over dialogs too, taking no touch and saying nothing to
  a screen reader. With the system's "remove animations" it stands still
  and goes after 0.9 s, on the phone; the desktop reads no such setting.
  A switch in Settings ▸ Look (on the desktop Settings ▸ This computer)
  turns it off on that Device; it is on from the start. Cards, rows, chips, switches and
  menu entries are no buttons and show none.

### Announcements

- Everything the app has to say goes to the top of the screen, under
  the status bar: one announcement at a time, the page pushed down
  below it. Nothing ever appears at the bottom, where the gestures and
  the keyboard live: a snackbar is a bug.
- An announcement goes by itself or by a tap; it is never swiped away.
  A horizontal swipe stays navigation only.
- An announcement says only that a job finished, failed or waits; the
  tap opens the details, or, on an offer, does what its last word
  names: "Undo" undoes, and "Show", after a Note made that the person
  filter hides (obf-1070), clears the filter. Values never go into an
  announcement.
- A quick local success the screen already shows (copied, saved) gets
  no announcement. The one exception is the phone's offer to undo what
  was just deleted, skipped or moved to tomorrow (#268, #269; see Phone,
  "Announcements").
- A failure's tap opens a dialog with cause, fix and the full text,
  Close and Report; the page it belongs to is a line inside, not a
  third button.
- An answer to a form field (name taken, not a code) is that field's
  own error text, red, under the field.
- **The playing bar is no announcement** (obf-1155): while a recording
  plays, and its Note is left, every screen shows a slim bar at the
  top, under the status bar (on the desktop under the menu bar, and at
  the top of a dialog where its announcements go), pushing the page
  down; never at the bottom. It holds the Note's name, its first line
  as rows show it, whose tap opens the Note, and Stop. A recording of a
  closed locked Area, or any behind the App Lock, reads "Private
  recording", and the tap asks to unlock first. It goes once the
  recording ends or stops, and does not show on the page or dialog of
  the Note playing, where its own stop button is. It stacks: the bar
  stays on top and the one announcement shows under it, both pushing
  the page down. Phone: `PlayingBar` in `playing_bar.dart`. Desktop:
  `playing_bar` in `app.rs`.

### Capture

- **Capture never asks anything and never needs unlocking.** The App
  Lock and Area Locks guard browsing, not Capture.
- Voice recording starts at once and **ends by tapping one big stop
  button**. No hold-to-talk; a pause ends it only where "End a
  recording after a pause" in Settings ▸ Capture is set on that Device
  (obf-1053), as Stop ends it. After Stop, and after
  the photo or the pictures picked, the user is on the capture screen
  with the recording or photo on it and room to type. Leaving saves.
- **The quick note is the text, three Nudge buttons and Open in the
  editor, nothing else** (#352): the text field at the top, under it
  remind me in **10 min, 30 min, 1 h**, and under them Open in the
  editor, a full-width outlined button with the editor's pencil
  (obf-1080), which saves the Note as leaving does and opens it in the
  full editor in its place (on the desktop the Note dialog). On the phone they sit right under the text field and do
  not move when the keyboard opens (#396); a long text scrolls and they
  stay above the keyboard. Simple and Advanced alike, on the phone, in the desktop's
  quick note dialog and in the capture popup. No Save and no Back on
  top: Android's system Back saves, the iPhone keeps a back arrow top
  left that saves, the desktop's Esc and the dialog's cross save, each
  as the 1 h button does; an empty text saves nothing. Everything else,
  Photo, Voice, Gallery, Paste, File, the Kind, Add to Note… (there Merge
  into…), Remind me…, File into…, the Frog, the Importance and the format bar, lives
  in the full editor and the Note dialog. An entry point that starts
  with something, voice, the camera, a picker, the clipboard or a share,
  still opens the quick note with it: what it brought shows above the
  text, and Open in the editor takes it on; an Area's or a Topic's "+"
  opens the full editor instead (obf-1140).
- Leaving without tapping a Nudge button means 1 h. The three are
  actions, not a choice of value: a tap saves the Note with that Nudge
  time, so the single-choice law does not apply to them. Where the
  Device switched the Nudge buttons off (Settings ▸ Quick note), one
  Save takes their place, and it nudges in 1 h as leaving does.
- **The Kind is the full editor's** (#352): the quick note makes a Plain
  Note; its kind badge, in the Note editor and the Note dialog, makes it a
  Checklist, a Shopping List, a Tally or a Session, each line a Line
  unless the first is a heading, which is then its Name (#309).
- **Add to a Shopping List**, in the Note editor and the Note dialog
  (#352), makes each line of a Plain Note a Line of the list picked, and
  keeps no Note. It is offered only for a Note that holds no photo, file
  or Section and has nothing spawned from it. **A Note with a recording
  is offered it with a speech model or without** (#311, Q716): it stays
  on the list as a Section under a tear line, so a misheard item can be
  listened to, and its typed lines become Lines at once. The items said
  in the recording's Transcript become Lines too, split at commas, at
  "und" and "and" and at the pauses Whisper marks; at once when it is
  transcribed already, else as soon as it is, on this Device or another,
  also when the speech model is downloaded later. They are added once:
  transcribing again adds nothing.
- **Everything is saved together** when the user leaves the screen or
  taps a Nudge button, Save or Open in the editor. Nothing is half-saved: for an
  empty Note nothing is saved.
- **A sorted Note needs no Nudge.** From an Area or a Topic a new Note
  skips the quick note (obf-1140): who is there has taken the time to
  sort, and its "+" ▸ Note opens the full editor, the phone's Note editor
  and the desktop's Note dialog, on a new Note filed there without a
  Nudge. Left with nothing in it, it makes nothing. The widgets, the
  Agenda's "+", the lock screen, the tray, Quick note and All Notes keep
  the quick note, and so does a file dropped onto an Area or a Topic on
  the desktop, which files it there with one Save and no Nudge.

### Agenda

- Order: the quiet day's line while today is quiet (obf-1044), then
  the Missed Summary card, then the "x Unsorted" banner, then
  the "x Sent Items waiting" line while a stranger's Sent Item waits for
  "Show", then the "x shared Areas waiting" line while an offered Area
  waits for Accept or Decline, then today's all-day Occurrences, then the
  all-day appointments of the calendars picked, then the Shopping Lists'
  groups as the user chose for each list (today only, oldest capture
  first, none for a list without open Lines or Lines checked today), then the
  Checklists' groups (today only, oldest capture first), then today's timed
  Occurrences and timed appointments together by time (an Occurrence
  before an appointment at the same time), then the "Eat a frog" section
  while today's pick holds a Frog (never above what is due). Every later
  day orders its rows the same way.
- After a visible gap comes tomorrow, in dimmer colours, then the five
  days after it, each under its weekday and date, dimmed alike. Every
  day shows, an empty one with "Nothing planned"; only an empty today
  says how to capture instead ("Nothing due today. Tap + to capture a
  thought."). The later days'
  Occurrences can be checked off today.
- **Every group on the Agenda is a heading with its entries under it**
  (#208): a Shopping List, a Checklist, a Checklist Routine's Occurrence
  and a Session's Occurrence alike. The heading leads with its box, or
  with its kind's icon where it has none, then its time and words, its
  Area's colour and icon and, on an Occurrence, ⋮; the entries are
  indented so their boxes stand where the heading's words begin, a Line
  under another Line one step further, and every entry looks alike: its
  box and its words. A tap on a heading opens its list or Note (#270:
  an Occurrence's the editor of its Note, with its Due line once it
  is due), a tap on
  an entry's words what its heading opens (a pick its entry form; a
  list's Line the list as it is, without the new Line its heading
  starts, #276), a box checks its entry. One card holds a group. Which Lines a
  Checklist's group shows is decided in one place (`agendaLines`, on the
  desktop `agenda_lines`).
- **Every Occurrence row has ⋮**, which opens the same menu as a long
  press: **Snooze** (10 min, 30 min, 1 h) while it is due, started and
  not yet Missed, and **Not today** (Skip, and Tomorrow) while it is
  neither Done nor skipped and this Device may check it off (#269), each
  under its heading; then **Edit schedule**, which opens the Routine or
  Reminder editor (#270), as the Routine's badge in the Note editor
  does; and History for a Routine. The row shares no calendar file
  (obf-1014). Edit schedule and History ask a Masked row's Area to open
  first; Snooze and Not today answer it as its box does.
- **A Checklist without a Routine or Reminder is a group today** while
  it has open Lines or Lines checked today (#143, #208, #276): its name
  as the heading, without a box, and its open Lines under it, each with
  its box, which strikes it for the day, as an Occurrence: a Line
  checked today stays struck in its place until the day ends, and its
  box unchecks it; or, as the user chose under **Show on the Agenda** in
  the ⋮ of its card or in its badge in the editor (#219), the heading
  alone with the count, "Before the trip: 3 of 7 open", while a Line is
  open, or not at all: the three radio choices of a Shopping List,
  there only for such a Checklist. A tap on the heading opens the list
  with the cursor in a new empty Line after the open ones, the keyboard
  up (#276); a tap on a Line's words opens the list as it is. A Masked
  one is the heading alone, "Private Checklist: 3 of 7 open".
- **A Shopping List's group today** (#186, #208): its name as the
  heading, without a box, and its open Lines under it, each with its
  box, which strikes it for the day, as an Occurrence (#276): a Line
  checked today stays struck under the open ones until the day ends,
  also one checked Once, and its box brings it back; chosen to show
  "one row with the number of open Lines", or Masked, it is the heading
  alone, "Weekly: 4 open", while a Line is open. A tap on the heading
  opens the list with the cursor in a new empty Line after the open
  ones, the keyboard up (#276); a tap on a Line's words opens the list
  as it is.
- The Missed Summary stays until the user taps "seen"; opening the app
  never clears it.
- **A quiet day shows everything and rings nothing** (obf-1044, ADR
  0053): **Quiet today** in the Agenda's menu, offered while today is
  not quiet (on the phone a bar action with a bell, in ⋮ when the bar
  is full; on the desktop a button last in the Agenda's top row, which
  reads "End quiet day" while the day is quiet, obf-1066), makes today quiet on all of the person's own Devices,
  without an announcement. While it holds, the Agenda's first line reads
  "Quiet day until midnight" with **End**; End ends it at once and
  Quiet today is offered again; at midnight the line goes by itself.
  Today's rows stay as they are, each checked off on its own; nothing
  notifies, no ring, Re-nag, Heads-up, Escalation, Nudge or Changes
  Nudge, and what is left undone counts as skipped from midnight, never
  as Missed. After End what is still to come rings, not what passed.
- **Checked off today, there all day.** An Occurrence checked off today
  stays on the Agenda for the day as Done, box checked and title struck
  through; the box unchecks it. Checking off opens nothing. **A skipped
  Occurrence stays for the day struck as skipped** (#269), "Skipped"
  under its title, or "The day after instead" when Not today ▸ Tomorrow
  skipped it; its box still checks it off. The one-time Occurrence
  Tomorrow added shows the day after, at the same time, with "From the
  day before" under its title. **Unless the
  Device hides what is checked** (#217): with "Hide what's checked on
  the Agenda" in Settings ▸ Agenda, off by default, a Done Occurrence, a
  checked Line and a pick with its entry leave the Agenda and the widget
  at once instead of striking through, a skipped Occurrence with them,
  and a group with nothing left
  goes with them, a Session's heading once it is Done and "Eat a frog"
  once today's Frog is eaten.
- **Done can move its Note to the trash** (#211): with "Move a Note to
  the Trash when its one-time Reminder is done" in Settings ▸ Agenda,
  off by default and the person's, the same on all of their own Devices,
  Done on a one-time Reminder that is a Plain Note's only Routine or
  Reminder moves the Note with it to the trash wherever it is checked
  off: the Agenda and its Missed rows, the widget, the notification and
  the full-screen alarm, the Note editor and dialog, which close, and the
  card's box. Its row and card go at once. Lists, Tallies, Sessions and
  Notes with a Routine or a second Reminder stay, and so does a Note this
  Device may not change. Unchecking brings nothing back; the Note comes
  back from the trash, still Done.
- **A Session's group today lists its picks under its title** (#208),
  in the order picked, each an entry with its box, checked and struck
  through once it has an entry, each Tally's name with its targets
  ("Push-ups 20 times"), and "×3" after them for a Tally of three Sets
  (obf-1159). The pick's box and words open its entry form,
  prefilled as any new entry; saving records it for this Occurrence.
  Each pick without an entry offers "Another one" under it while the
  pool holds a Tally not picked. **The heading's box only shows Done**:
  it checks itself once every pick has an entry, removing an entry
  unchecks it, and a tap on it does nothing, on every day. Tomorrow and
  later days show only the title and how many it picks: the picks are
  made on the day.
- **The Agenda reads its rows beside the app** (#207): on the phone in
  an isolate, on the desktop on a thread, each on a connection of its
  own. While a read runs, the rows read last stay; nothing empties, no
  spinner shows. Before the first read is in, no day shows, so no
  "Nothing planned" that is not so. A read the log has changed since is
  dropped and read again. On the phone a box tapped shows its new state
  at once, and a second tap undoes the first, before the read is in.

### Cards

- A Note is drawn as a card with a soft embossed look, visibly above the
  background.
- **A card starts with its text** (#330, Q733): one top row holds, on
  the left, another person's picture and where the Note is filed (its
  Area's icon and "Household / Kitchen", or "Unsorted"), and on the
  right the card's buttons; no divider follows, and the text starts in
  the second row. On the desktop the buttons (Hidden from AI, the view
  button, ⋮, the pencil) stand in a column of their own on the right,
  the check mark of a card selected along at its head, its place kept
  while the card is not selected, so selecting moves nothing; on the
  phone the compact view's ⋮ and the Important and Hidden from AI marks
  stand there, the box of a list that selects leads the row, and the
  detailed view's pencil and ⋮ stay in its action row. With nothing on
  the left the text starts in the top row, beside the buttons. "From
  Ann" (#164) and today's boxes (#215) follow the top row, over the
  text. **The place is left out in the list of the Area or Topic the
  Note is in**: All Notes, Unsorted, the trash, a search across Areas,
  Merge into… and the Agenda's just captured card name it, and an
  Area's search names the Topic of a hit in one of its Topics.
- **A Note is read on its card; the editor is the last step.** Every
  kind has a **compact view** and a **detailed view** (#189). The
  detailed view shows the whole Note: every Section oldest first, the original
  and then each Amend and Merge, each with its date and Capture
  Context, with all its photos, recordings, Transcripts and Photo Texts.
  A Transcript or Photo Text shows whole under its recording or photo,
  dimmed and labelled "Transcript" or "Photo Text", so it never reads as
  typed text.
- A photo in the detailed view fills the card's width in its own proportions,
  its height capped, and is decoded no larger than it shows, so an
  extreme picture (12 × 34,000 px) cannot break the card; a tap opens it
  full screen with zoom.
- **A part's meta is one weak line under its text** (#170): under the
  Note's text in the detailed view, and under each Section's there and
  in the Note editor, its time, place (the Named Place, else the coordinates),
  calendar events, Bluetooth devices and source app go on one line,
  joined by " · " ("12:40 · Home · Dentist"), wrapping only where it
  must, smaller than the text and in the weak text colour, so the text
  reads first. A Section captured the same day as the part before it
  shows only its time; another day the date too, by the app's date
  rule. A Section's "Edited … by …" is a second weak line under it. The
  Captured badge's dialog keeps each detail on a line of its own, with
  Remove; in the editor a Section's line opens the same dialog for it.
- **A Note reads as one Note** (#307, Q712): a Section has no box or
  border of its own and reads on as the Note's text does, parted from
  the part before it only by a **tear line**, a row of short grey dashes
  like the perforation of a roll of paper, its meta
  line the small time stamp under it. In the detailed view each Section
  follows the Note under its tear line. In the Note editor, the Note
  dialog and a Note's own window a Plain Note's Sections read on in the
  box of its text, under the text and above the badges; a Note of a
  kind keeps them below as its diary part (#188), each under its tear
  line on the page. Everything a Section does stays: its text edited
  in place, its line's dialog, who edited it and when, its photos,
  recordings, files, Transcripts and Photo Texts.
- **A Note shows the Notes that link to it** (obf-1035, Q22): at its
  end, after its Sections, "Linked from" in the weak text colour, then
  one row per Note whose text or Sections hold an App link to it, the
  link icon before its first line with words as it reads (its capture
  time where it has none), newest first; a tap or click opens that Note
  as its link does. In the detailed view, the Note editor, the Note
  dialog and a Note's own window, its reading view too; never on the
  compact view, nor on a card in the trash, and only while a Note links
  there. Only Notes this reader may see: none from the trash, merged
  ones, a Shared Area they are no member of, or a closed locked Area.
  On a desktop card a row takes its own click, not the card's.
- **The compact view is a glance, by the Note's kind** (#189): a Plain
  Note its first line and the two lines after it, a Checklist its name
  and how many Lines are open ("3 of 7 open"), a Shopping List its name
  and how many are open ("4 open"), a Tally its name and each field's
  but one counting sets total of the current bar, today's for a Tally
  of days (#264), against its target ("Steps 7000 / 10000 steps"), "×3"
  after the target for a Tally of three Sets (obf-1159), in a
  Shared Area of the user's own entries, as its detailed view counts at
  first (#190), a Session
  its name, the next run of any of its Routines and "done 4 / skipped 1
  this week" over all of them (#191); a Note without words its photos and recordings.
  It shows no Capture Context and **none of the buttons at the bottom of
  the card** (File into, Routine, Merge, the pencil); **its ⋮ sits top
  right** and offers in its groups what the action row would (#281,
  obf-1014): File into… under This Note, Remind me… (#344) and in
  Advanced Frog and Session… (#197) under Add, and Merge into… under
  Tidy: the spawns only for who may set them up (ADR 0013) and Session…
  only on a plain Note. **An Unsorted Note's card names its Merge
  "Merge into…" too** (#308, obf-1014), in the action row, the ⋮ and on
  the desktop the right-click, as the Note editor, the Note dialog and a
  selection do: the same Merge, its page titled Merge into… and picking
  the Note as Add to Note… does, the Notes added to last first (see
  Dialogs and pages). In the detailed view a Shopping List shows its open Lines,
  tickable, the struck ones folded under them ("2 struck through") until
  a tap shows them, kept open per Note on this Device. **A Tally's
  detailed view is the Tally** (#190): under its name and description
  "Add entry", in a Shared Area "Show entries" (Mine at first,
  Everyone's, or one person's), a chart per field shown but those
  counting sets, a bar per the Tally's bar length with no choice of it
  here (#264): slim bars of a fixed width with gaps, the newest on the
  right, as many as fit, the target as a thin line, never scrolled
  sideways; then its entries (ticks as "✓ Wiped", fewer sets than the
  Tally's number as "2/3 sets" (obf-1159), several sets a line each,
  "1. Weight 60 kg · Reps 10"), newest first, the latest five and
  the older
  folded under how many ("12 older entries") until a tap shows them,
  kept open per Note on this Device; a tap on an entry edits it, a long
  press offers Edit and Delete, and the folded "Removed entries" restore.
  On the desktop the card shows the same, its buttons taking their own
  clicks. **A Session's detailed view is the Session** (#191): under its
  name and description its pool, then today's picks of each of its
  Routines under the Routine's time, each pick opening its entry form
  and offering "Another one" as on the Agenda, then "done N / skipped
  M" for each of the last four weeks, this week first, then its latest
  five past runs (#264), each its date and time and each pick "done" or
  "not done", with how many older ones its page lists; a Session
  without a Routine shows its pool alone.
- **A card ticks today's Occurrences of its Note** (#215): in either
  view, under where it is filed and over its text, a box with the time
  for each of its Routines and Reminders with an Occurrence today, its
  first one neither Done nor skipped, else its last one, Done or
  skipped ("08:00 · Skipped") and struck through. The box
  checks off and unchecks as the Agenda row's does and opens nothing; a
  Session's only shows Done, one Done from every Line checked stays
  checked, and where this Device may not check it off it only shows. A
  long press on the card still opens the editor.
- **The Note editor checks off what is due** (#215): at the top, over
  the name, for each Routine and Reminder of the Note an Occurrence due
  now, "Due 15:00" (with its day when it began on an earlier one), else
  its latest Missed one, "Missed" with its day and time, whether the
  Missed Summary was seen or not, each with **Done**, beside it
  **Snooze** while it is not Missed and **Not today** (#269), each
  opening its choices where the finger is, Tomorrow only for an
  Occurrence of today or later, and for a one-time
  Reminder that is the Note's only Routine or Reminder also **"Done and
  move to trash"**, where this Device may move the Note: it checks it
  off, moves the Note with the Reminder to the trash and closes the
  editor. On the desktop the same lines open the Note dialog, Done on the
  accent and "Done and move to trash" on the error colour.
- **A card's ⋮ holds Edit and the Note editor's four groups** (#281,
  obf-1014): Edit, then **This Note ▸**, **Add ▸**, **Share ▸** and
  **Tidy ▸**, each holding what the card offers of the editor's group
  of that name, as the editor names it; a group with nothing this
  Device may do is left out. On the phone the Importance is not in it
  (the editor's badge and the selection change it, #182, #273); see
  Phone ▸ Cards for what each group holds there.
- **The detailed card's action row has a pencil** (#281), right before
  its ⋮: one tap opens the Note editor, as the long press and ⋮ ▸ Edit
  do. Where the card is too narrow for the whole row, as with Fat
  fingers on a small phone, the pencil and ⋮ go onto a line of their own
  under the other buttons, at the right, so no button is cut.
- **Copy text** in a card's ⋮ (under Share ▸) puts the text
  of the Note and of each Section on the clipboard as they show, oldest first, a blank line
  between them, and says nothing (#183); a Checklist's or Shopping List's Lines come after its
  text as a Markdown task list, `- [ ] Milk`, `- [x] Eggs`, a group indented (#323).
- **A Section's text is edited in place in the Note editor** (#183), in
  the same field and with the same format bar as the Note's text, by the
  same people (the Section's author, in a Shared Area also the
  Managers); others read it. Under its meta line it says, small and
  grey, when it was last edited and by whom, once it was.
- **A Note's History** (#193) is History on the Note editor's bar on
  the phone (the clock with an arrow, after Undo, Redo and Find) and
  History in the ⋮ of the Note dialog on the desktop (obf-1065). It lists
  every change of the Note, **the newest first** (unlike a box's
  History, which goes oldest first): its text before and after, its
  Amends added and changed, its Lines added, renamed, checked,
  unchecked, deleted and restored, its Importance, Area and Topic, kind
  and text format before and after, the trash, and photos and
  recordings added and removed; each with who made it, by their
  Profile's name (none for a person not known or blocked), and when.
  An Area or Topic of a locked Area that is not open is named "A
  locked Area", never by its name. **Use this version** under an old
  text of the Note or an Amend, where this Device may edit it and it is
  not the text showing now, writes it as a new edit; nothing is
  deleted, and the History keeps every version.
- **A Note is its kind** (#188): a Plain Note, a Checklist, a Shopping
  List, a Tally or a Session (#191). A Plain Note shows as its text. Every other kind's
  page, in the Note editor and in the detailed view, starts with its
  **name**, the text's first line, as a heading (a list without one,
  made of typed text whose first line was no heading, shows its kind
  there, "Checklist" or "Shopping List", #309), its **description**,
  the rest of the text, small right under it and folded to one line
  until a tap unfolds it (on a card kept open per Note on this Device,
  #189), then **the kind's content** (the Lines; of a Tally its setup in
  the editor, its chart and entries in the detailed view, #190; of a
  Session its pool and count in the editor, its picks and history in the
  detailed view, #191), then
  the badges, the photos and recordings, and the Amends
  below as the dated diary part, each under a tear line in no box
  (#307). In the editor the name is a one-line
  field and the description its own field; nothing is rewritten for the
  display, and leaving both unchanged writes nothing.
- **The Note editor shows each setting of the Note as a badge** (#160),
  in one wrapping row under the text and its Sections (#307), or under
  the kind's content for a Note of a kind (#188), like a shield: left an icon for *which
  setting* the badge is, the same for every value (one for the kind,
  one for the Importance, …), right its value (#192), as an **icon**
  where the setting has a fixed set of values (the text
  format, the Importance, Show on the Agenda, Show ticks, a Tally
  field's rule, a Session's rule; an on-or-off setting as `toggle_on` /
  `toggle_off`),
  as **short text** otherwise ("Every day · 08:00", "Household",
  "50 pages", "Reps"). **Never an icon plus the same word.** The
  desktop draws the same Material icons as the phone. **The kind comes
  first and in words** (#310), Plain Note included (#188): "Plain
  Note ▾", "Checklist ▾", the ▾ (`arrow_drop_down`) only while a tap
  opens its choice, and a screen reader reads the words alone. Then the
  Importance; Area and Topic in one; Captured (time, place,
  calendar event, Bluetooth device, app); each Routine and Reminder with
  its schedule; its Frog (#259) while one is open or was eaten today or
  yesterday, the frog with "Open", "Eaten today" or "Eaten yesterday",
  in Simple too, its tap offering at the finger Eat now while it is open
  and this Device may check it off and Move to trash for who may
  manage it, which asks nothing and is undone as the Frog view's Move
  to trash is (#268, obf-1014); for a Member an
  eaten one only shows; the text format of
  a Plain Note (#213), `text_format` with `short_text`, `format_bold` or
  `code`; Show on the Agenda for a Checklist without a Routine and for a
  Shopping List (#186); Show ticks while it has a box; "From Ann" while
  a Pack brought the Note unclaimed (#164), `inventory_2` with the
  name, its tap offering "Remove “From Ann”" at the finger, which asks
  nothing and is undone (#268). A Tally's setup
  is the Tally's own badges, "One bar per" (#264) and "Sets", its number
  of Sets with − and +, 1 at first, each step written at once
  (obf-1159), each opening its small editor, then its fields (#190),
  each its name with its rule, unit and target as badges, as on the
  Tally page, the target with "×3" for three Sets, a
  Check field its kind and target, a field that counted sets as a
  multiplier did "Counts the sets of older entries" without its rule and
  target, and "Hidden" while it is hidden, a tap
  on the name or a badge opening the field's page, then "Add field"; the
  editor shows no chart and no entries. A Session's setup (#191) is its
  pool, a list of checkboxes ending in "New Tally" as on its "+" page,
  and how many it picks with − and +, then its rule and its weekdays'
  counts as badges; each of its Routines is a Routine badge, and the
  Routine editor of one holds only its schedule; its past runs (#264)
  follow the badges. A setting that does not apply has no badge. A tap
  opens its small editor, a dialog with the same setting icon and the
  setting's name, its values in words, which changes it at once: Importance as choice chips (only
  here, not in the card's ⋮, #182), a Shopping List's Show on the
  Agenda as radio buttons (here and in the card's ⋮), the switches,
  the kind as choice chips of the kinds it may become: from a Plain
  Note a Checklist, a Shopping List, a Tally or a Session, from a list
  the other list, keeping its Lines, or a Plain Note, from a Session a
  Plain Note; a Tally's and a Code text's only show it; the text format
  as choice chips of Plain text, Formatted text and Code, a formatted
  text never Plain text again;
  Captured only shows its details. Area and Topic opens File into…, a
  Routine or Reminder its editor. Where
  this Device may not change a setting, its badge only shows it. A
  screen reader reads each as "setting: value" in words, an icon's value
  too. The row ends with "+",
  whose menu at the finger offers Remind me…, Frog and Session…,
  there only for who may set them up; no Frog while the Note has an open
  one (#259); on a Session it offers no
  Session, and a Routine or Reminder added there runs the Session. The text, the Lines, the photos and
  recordings and the Sections stay the page's content; the editor has no
  "Make it a Checklist" or "Make it a Shopping List" button of its own,
  the kind badge does it. On the desktop the badges sit in the same place
  in the Note dialog, their small editors
  are dialogs over it, and the menu of "+" opens under it.
- Another person's Note shows their Profile picture, or its
  placeholder, on its card; one's own Notes show none. Tapping another
  person's picture, anywhere it shows, opens their Profile.
- **A Note a Pack brought unclaimed says who it came from** (#164):
  "From Ann" on a weak line with `inventory_2` under the card's top row (#330),
  in either view, until "Remove “From Ann”" in the card's ⋮ ▸ This Note or its badge
  in the Note editor takes it away.
- **"Export as Pack…"** (#164) is in the menu of every Area and Topic,
  for anyone who may read them, in a Note card's ⋮ after Copy text
  (under Share ▸, #281, obf-1014) and
  in the Note editor's Share group after Share to an app
  (`inventory_2`, obf-1073). It
  opens the Pack's tree: Areas and Topics as headings, each Note with
  its kind's icon and its first line, under it its files with their
  sizes, their texts, its Routines, Reminders and Frog, under a Session
  the Tallies it brings from elsewhere, each a box at the start; an item
  under a Note not ticked cannot be ticked. "Tick all" and "Untick all"
  sit on the bar, the size over the tree. A Pack opened shows the same
  tree, all ticked, under who it is from; its ticks ("Add my Profile",
  "Claim as mine") go below their text.
- **"Export as AXT file…"** (obf-1145, ADR 0056) stands right after each
  "Export as Pack…" and in the bar of a selection of Notes: the Notes of
  the Note, the selection, the Topic or the Area go at once as an AXT
  file named after it (`<title>.axt`), nothing asked, to the share
  sheet; with no Note in it nothing is written and the top says so.
- **An AXT file opened shows its records as an opened Pack does**
  (obf-1146, ADR 0056): told by what it holds, not by its name, it opens
  a page titled "AXT file" with who it is from ("From your own Device",
  else "From" and the app name its file gives) and the key's short
  fingerprint under it, then every record to take, each ticked, by its
  first line with its type and its Reminder or Routine under it, "Tick
  all" and "Untick all" on the bar; the records not taken follow greyed
  under "Not taken", each named by its type. "Take them into" with its
  place, Unsorted at first, opens at the finger Unsorted, then the Areas
  and Topics this Device may file into, as File into… lists them, and
  while a ticked record brings a Reminder or Routine only where it may
  make them. "Take it" makes the ticked records Notes there, says how
  many at the top and closes the page; greyed while nothing is ticked.
  A file that does not read, and a place refused, are announced at the
  top with why.
- **A later AXT file of the same sender changes what was taken**
  (obf-1147, ADR 0056): on the same page each update says "Updates
  “Dentist”" and each deletion "Moves “Dentist” to the trash" under its
  line, by the Note it changes, ticked as the new records are; one whose
  Note the user changed since is greyed and cannot be ticked, its line
  ending "· Kept your change". A record refused under the id of a Note
  taken ("May not change “Dentist”") and one no newer than it
  ("“Dentist” has this already") are named under "Not taken". The place
  shows only while the file holds a new record. "Take it" says "1
  record taken" when changes were among them, and "Nothing changed:
  these records were taken before." when it wrote nothing, as for the
  same file taken twice; a Note this Device may not change is refused at
  the top with why, nothing written, and the page stays.
- **"Open a Backup…"** (#285) is on the Backups page, in a card of its
  own ("Take Notes back"), set up or not, and on the desktop also in the
  File menu, its page then alone, without the way back to Backups. It
  picks the file (on Android and the desktop the file picker starts in
  the Backups folder), asks for the
  passphrase or the recovery key as restoring does and refuses what
  restoring refuses, announced at the top. Then the same page shows the
  Pack's tree under the Backup's date: no Note ticked, everything under
  each; a Note from the Backup's Trash says "In the Trash" under its
  name, not by colour alone. The Notes of a locked Area closed now are
  not in the tree; a line over it says so, never naming the Area, and
  "Unlock and show them" asks for the PIN or biometrics and reads the
  Backup again with their Notes, the ticks back at the start; the
  Areas stay closed everywhere but on this page.
  "Untick all" goes back to that start, "Add" is announced as a Pack's
  and closes the page; a failed Add leaves the page open and the log as
  it was.
- An Area's colour and icon mark its cards, Agenda rows and alarms.
- **Important** cards get an accent marker and a stronger emboss;
  **unimportant** ones are dimmed; normal is plain.

### Dialogs and pages

- A dialog has one question and at most two actions plus Cancel. More
  than that is a page. A list to choose from (Areas, Topics, People) is
  a menu at the finger or a page, never a sheet.
- **File into…** (#272), from a card, the browser or capture alike, is
  a filter field over the Areas and Topics (focused on the desktop, not
  on the phone), then "Filed into recently" with the last three places
  this Device filed into (ADR 0008), the latest first, while nothing is
  typed, then the Areas alphabetically, each with its Topics. An Area
  whose name matches shows with all its Topics, else with those that
  match. **"New Topic…" ends each Area** for who may set up a Topic
  there, only in Advanced, never in a closed locked Area; **"New Area…"
  ends the list**, also when it is empty or nothing matches. Each asks
  the name (New Area… with the filter's text filled in), makes the
  place and files into it at once, from capture too, where the place
  stays when the Note is not saved; Cancel makes nothing and stays. A
  place File into… does not list (deleted, a Topic of a closed locked
  Area, a Shared Area the Device left) is not offered among the recent
  ones either.
- **Add to Note…** (#308), from capture or an answer of Ask (#316), and
  the page of an Unsorted Note's Merge into… (obf-1014), is
  a filter over the Notes, the core's search (focused on the desktop,
  not on the phone), then "Added to recently" with the last three Notes
  this Device added to, the latest first, while nothing is typed, then
  every Note found, newest first. It offers only Notes this Device may
  add to (ADR 0013), none in the trash, none of a closed locked Area,
  not the Note being added, and to a capture Hidden from AI none of a
  Shared Area (#267). A Note it does not offer is not among the recent
  ones either.
- **The Shopping List pick** of Add to a Shopping List lists the lists
  this Device may add to, oldest capture first, but the one a capture on
  this Device went into last at the top (#276, ADR 0008).
- **A Tally's new entry** starts with each number field's target, and a
  field without one with the value of the user's latest entry that
  holds one (#276), selected as the field takes the cursor, so typing
  replaces it. Left as it opened, it writes nothing.
- **A short single choice is choice chips, a long one radio buttons**
  (#226). A choice of one value whose options are fixed words of one to
  about four, at most about seven of them, with no detail line under
  any, is a wrapping row of choice chips, exactly one selected, as the
  Routine editor's weekday chips look: the Routine editor's Simple or
  Advanced, repeat kind and Ends, the reappear time, Day, Week, Month or
  Year, the Importance, kind and text format badges, Re-nag minutes, the
  theme. A choice whose options are long, carry a detail line or are
  names the user made (people, a Tally's fields, Shopping Lists) is a
  column of radio buttons, one option per line: Show on the Agenda, the
  unit system, a Session's rule, the person's Simple or Advanced (#197),
  the speech model, a role, whom a Note is sent to. Never segmented
  buttons. The choices inside a menu stay the menu's own marked entries.
  The chips are one radio group to a screen reader, each chip a
  selectable button, the chosen one announced as selected. Several
  options that can be picked together are checkboxes or filter chips. A
  setting that is only on or off is a switch, not a choice. Tabs that
  switch pages are not a choice of value. Suggestions under a text field
  are chips that fill in what is typed, such as the known units under a
  Tally field's unit (#159): the value is the text, so they are not a
  choice.
- **The Routine editor's schedule starts with its sentence** (#227, ADR
  0044): a line, "When", that always shows the schedule as a sentence
  in the app's language ("Weekdays Mon, Thu at 08:00, until 12/31").
  Typing in it fills the chips and times as it is typed, with the next
  word, the weekday names and times as suggestion chips under it; the
  words it does not understand are marked in the error colour, said
  under it, and change nothing. Left, or when a chip changes, it shows
  the schedule again. It also reads a one-time Reminder in words that
  count from now (#271): "today", "tonight", "tomorrow", "in N
  minutes/hours/days/weeks" and their German words, "tomorrow" without a
  time at the end of the Device's Quiet Hours (09:00 while off),
  "tonight" at 19:00, against the store's clock. On an Advanced schedule
  it is read-only, the schedule's summary with "Switch to Simple to
  type". The sentence is never stored.
- **The Routine editor lists the Heads-ups** (obf-1046): under the
  schedule, "Heads-ups" lists each one set ("20 min before", "1 day
  before"), earliest warning first, each with its remove ✕, and "Add
  heads-up" offers 5, 10, 15 and 30 min, 1 h, 2 h, 1 day, 2 days and
  1 week before, and "Custom…" at its end (obf-1087): a number and a
  unit, minutes, hours, days or weeks, up to 30 days, listed as the
  others are ("45 min before"), with its ✕; it stays when every other
  is set, and one set already is not set twice. An all-day schedule
  offers and keeps whole days only (1, 2 and 3 days, 1 week, and in
  Custom… days and weeks only). The lists come from the core
  (`heads_up_keys`). They are the person's own, on their own
  Devices: in a Shared Area a Member who may not change the Routine
  still sets their own, and nobody else sees them: the editor opens for
  them with everything else shown, not edited, and its Save keeps only
  the Heads-ups. Saved with the editor's Save.
- **A new Reminder offers its chips** (#271): on the phone and the
  desktop, a new Reminder, or a new Routine set to Once in Simple, shows
  a wrapping row of chips under the When line: In 1 h, Tonight (until
  19:00), Tomorrow and In a week, each with its time, Tomorrow and In a
  week at the end of the Quiet Hours. One tap or click sets the
  schedule, read again at that moment, then Save; nothing is saved by
  it. They are shortcuts that set a value, not a choice: none is shown
  as selected, and on the desktop they are plain buttons. A saved
  Reminder and a Session show none.
- **Send feedback shows what it sends** (#214): Settings ▸ "Send
  feedback" is a page with a field for the words, ticks for what goes
  along (version, system and device, install method ticked at first,
  and the last notification taps while there are any (obf-1039); the
  phone's last crash report and Notes as a Pack not) and the
  preview, the exact text sent, always above its buttons, which wait
  for words: Mail, a mail link to the developer while the text fits one,
  and on the phone Share…, the text and the Pack to the app picked, on
  the desktop "Save as file…" and, while Notes go along, "Save the
  Pack…" to attach it. Notes are picked in the Pack's tree of every
  open Area, nothing ticked at first. Nothing leaves without the
  preview and a tap.
- Two sentences at most in a dialog body. Concrete nouns, no metaphors.
- **Deleting for good confirms once; what can be restored is undone
  instead** (#268, Q646 (c)). Emptying the Trash, deleting from the
  Trash, deleting an Area, a Topic or a Named Place ask once, in a
  dialog whose confirm button carries the verb and the error colour.
  Deleting what comes back asks nothing: a Note moved to the Trash, a
  Routine or Reminder, a Frog, a Tally entry, a Checklist or Shopping
  List Line, the "From …" label, and a Transcript or Photo Text, which
  can be made again. It goes at once and is undone at the moment of the
  mistake: on the phone by the announcement at the top, on the desktop
  by Ctrl+Z and Edit ▸ Undo. Done that moves a Note to the Trash
  ("Done and move to trash", #211) is a check-off, not a delete: it
  offers no undo on the phone, and the Note comes back from the Trash. The undo uses the restore that exists and writes
  nothing more than it does; where it has nothing left to bring back,
  it does nothing.
- Deleting a Note takes its Routines and Reminders into the trash with
  it, on every path and in both apps (#262), and its undo, as restoring
  the Note, brings them back. There is no "Delete only the Note". All
  Routines and Reminders marks one whose Note is in the trash or
  deleted.

### Text editor

- **The format bar goes with the field**: ☐, bold, italic,
  strike-through, two headings, bullets, numbers, quote, link, "Link
  to…" (#261: a picker of the app's pages, Areas, Topics and Notes with
  a search, never an id typed), code,
  code block (on the lines touched, or taking a block's fences off), in
  and out, each on the selection or the word at the cursor, and table,
  which puts a new table at the cursor and edits its first cell, or goes
  into the one at the cursor. On the phone
  it sits directly above the bottom row while the keyboard is open, and
  the buttons without room go into ⋮ at its end, opening at the finger.
  On the desktop it sits above the field, which shows a formatted text's
  Markdown while it is edited and the text as it reads otherwise. Its ☐
  puts ☐ in at the cursor, also in the middle of a word.
- **In Simple there is no format bar** (#197): no ☐ and no formats
  over any field, while a field's own buttons, such as a Line's in and
  out, stay. Typed and pasted Markdown still formats, a formatted text
  shows formatted and a table is still edited as its grid, with its +,
  menus and Show as lines.
- **In the editor a table is edited as its grid** (#257), on both apps,
  pasted tables included: a tap or click on a cell edits it in place;
  Tab and Shift+Tab go to the next and previous cell row by row, Enter
  to the cell below, and the arrow keys at a cell's edge to the cell
  beside it, past the grid's edge into the text before or after it; the
  arrow keys from the text go into it. "+" after the last row and after
  the last column adds one. A row's number and a column's letter open
  its menu: insert, move, delete (asking once while it holds words; the
  last one deletes the table), for a column Align left, Centre and Align
  right, and for the first column First column as headings. A column's
  menu groups them (obf-1014): Insert ▸, Move ▸, Align ▸, Sort rows ▸,
  Delete column and First column as headings; full screen Insert ▸,
  Move ▸, Delete column, Show sorted ▸ and First column as headings. While a
  cell is edited, the format bar, where the field has one, holds only ☐,
  bold, italic, strike-through, link and code, on the cell's words.
  Every change writes the table's lines anew (ADR 0033); the Markdown
  stays the source. **Show as lines** on the grid shows that table's
  lines as text until the cursor leaves them, the field loses focus or
  Show as table is pressed. Column widths are the view's only, never
  stored. Cards and texts that are only read show the grid at the text
  size, long cells wrapping and a table wider than its room moving
  sideways inside it, the header row and a heading first column in view;
  a card shows the first 10 rows and "Show all N" (obf-1048). A click
  opens the table full screen; a click on a cell of its header row sorts
  the rows shown by that column instead, in the view only (obf-1052,
  see Phone ▸ Text editor and Desktop ▸ Text editor and Lines). A small ⋮ at a table's top right corner
  (obf-1050) offers Copy wherever the table shows, and Move up, Move
  down (past the block above or below) and Delete table where the Note
  is edited; Undo takes them back. **Copy in the bar of the table full screen**
  (#324) puts the table on the clipboard as an HTML table and, as the
  plain text, as Markdown, each box in the state it shows, the cell
  being edited written first; on the phone the HTML goes through the
  app's own channel, else the Markdown alone. Copied says nothing.
- **A Code text has no format bar and no ☐** (#213): the field shows it
  in the fixed-width font on the light tint of code, unwrapped, and it
  moves sideways. **A paste that looks like code** into a Plain Note's
  text in the Note editor formats nothing and puts "Keep as code" under
  the text, with a cross that lets the paste be; Capture never asks.
- **Code is coloured by its language** (obf-1036, ADR 0054): a Code text
  with a language and a code block whose fence names a language a
  grammar knows are drawn in the syntax colours wherever they show
  (cards, read views, the editor's field), only the glyphs' colour
  changing: font, size, tint and line room stay as above. Eleven colours
  (comment, string, number, constant, keyword, operator, type, function,
  tag, attribute, variable), the same values in both apps, one set for the
  light look and one for the dark, each readable on the code tint.
  Without a language, with one this Device does not know, or a fence
  naming none, code stays in the text's colour. The text format badge's
  dialog, while Code is picked, offers "Language": "No language" and the
  grammars by name, and a pick is written at once; "Keep as code" picks
  the language the first line names. Search, Share and Copy give the
  text as written.
- **Grammars of one's own in Settings ▸ Code languages** (obf-1037):
  the section's line says what a grammar is, then each grammar the
  person added by name, with "Replaces the built-in grammar for …"
  under it when it takes a built-in one's place (else its scope), and
  Remove beside it, which removes it at once; under them "Add
  grammar…" opens the system's file picker, any file shown. A file
  refused says so red under the button, with the file's name and what
  is wrong, and nothing is added; the next add clears it. A setting of
  the person, the same on all of their own Devices, in Simple and
  Advanced, on the phone and the desktop alike. An added grammar's
  language is offered in the text format badge's Language after the
  built-in ones, one replacing a built-in one in its place.

### Boxes

- **A tap on a box opens its menu at the finger**: Check, Fail and
  Open, the state it shows left out, and History once it has been
  ticked. It works wherever the box is drawn: in either view of a
  card, in a Section, in a table full screen and in the editor's field,
  also where the text is only read. A box being typed becomes one when
  the text is saved.
- **A tick is no edit of the text.** In a Shared Area everyone with a
  role there ticks any box of any Note and Section; adding, moving and
  deleting boxes stays an edit of the text, the author's and the
  Managers'.
- **A box's id is never seen.** In the field the cursor and a selection
  step over a box as one character and never end inside it; typing next
  to it keeps its id with its glyph, and deleting the glyph, or any part
  of the box, deletes all of it.
- **Show ticks** is a switch in the ⋮ of a Note's card and in its badge
  in the editor, there only while the Note or a Section of it has a box: while
  it is on, each ticked box says after it, small and grey, who ticked it
  last and when ("Ann, Tue 14:02", "You" for one's own, the time alone
  for a person not known or blocked). It is kept per Note in a setting
  of this Device.
- **History** opens a dialog listing every change of the box, the oldest
  first: the state, who and when.

### Checklist and Shopping List Lines

- **A Checklist and a Shopping List are edited the same way**, as one
  block of text with boxes (#184): in the Note editor every Line is a
  text row starting with its box, the rows close together, and the
  cursor moves between them. Cards only check them.
- **A new list starts with one empty boxed Line**, the cursor in it. A
  list this Device may add to always has a Line to type in. **Opened
  from its group's heading on the Agenda** (#276), a list starts with
  the cursor in a new empty Line after its open Lines, the keyboard up
  (on the desktop the keyboard's focus there); left unused, the Line is
  dropped on leaving, as any empty one.
- **Return** adds a new Line with an empty box under the cursor's Line,
  in the group of a Line with one; in the middle of a Line it moves the
  rest of the Line into the new one. **Backspace at the start** of a
  Line joins it with the Line above: its text goes to the end of that
  Line, which keeps its box and state, and the Line goes, its checks
  staying in the log; an empty Line simply goes. Return makes new Lines,
  and **Add a Line** under the Lines of the one card a Checklist or
  Shopping List is (obf-1071) puts the cursor into a new empty Line.
- **The frame around a group holds the group's own Line** and the Lines
  under it, on cards and in the editor.
- **Nothing typed is lost on leaving**: what was typed is stored when
  the cursor leaves a Line and when the editor saves or is left; Lines
  left empty are dropped on leaving.
- On a Shopping List the open Lines keep the order the user gives them,
  as typed in or moved by ⋮ Move up and Move down (#283), also the next
  time; a Line checked while the editor is open stays in its row, and
  shows below the open ones the next time. A struck Line does not move;
  it comes back where it was among the open ones. A name the list
  has already brings its Line back instead of a second.
- **A Shopping List's struck Lines show the soonest back first** (#328,
  Q731 (a)), the same day in the order of their checks, those without a
  reappear time last; each with a day back has it faint beside it at the
  end of its row, before the badge: "tomorrow", "in 3 days", "in 3
  weeks", "in 2 months". A Line ticked shows struck in its row until its
  save, then goes to its place among them (#338); in the editor it stays
  in its row while the editor is open.
- ⋮ of a Line holds the rest (moving it, Delete); a long press in a
  Line's text selects, as in any field.
- **"Removed Lines"** (#268) folds under a list's Lines in the Note
  editor while it has deleted Lines, as a Tally's "Removed entries":
  each deleted Line's text, the last deleted first, with Restore below
  it where this Device may change the Line, which brings it back where
  it was; open or folded as last left, on this Device. A Line goes
  there by ⋮ Delete, and, without an announcement, by emptying or
  joining it while typing, with the text it last had stored.
- **A Shopping List's Line ends in its reappear time badge** (#195),
  before ⋮, in the editor: short text, "—" for none, "Once", "4w",
  "3m", read as "Reappear time: 4 weeks". A tap opens its dialog: None,
  Once or After as choice chips, After with a number from 1 to 99 (−
  and +) and Day, Week, Month or Year under it, each change written at
  once, then Close. Where this Device may not change the Line, the badge
  only shows it; a new Line's badge stores it first.

### Lists

- A part of a page that is rarely needed folds; the fold is remembered
  per Device in a setting of that Device, closed by default when there
  is already a choice.
- **Every Note list has a compact/detailed switch** on its bar: each
  Area, each Topic, Unsorted (with the Nudge screen), the search results
  and a person's Notes. It is an on/off toggle of the view, not a choice
  of value: one button that says what tapping it does. Detailed is the
  default; each list remembers its own choice in a setting of this
  Device. A search inside an Area or Topic follows that list's switch.
  **A card a tap switched keeps its view** in another setting of this
  Device, per list, until the list's switch changes again, which shows
  every card in the view it sets (#189).
- **Sorting Unsorted goes on to the next Note** (#313, Q718): once a
  Note Unsorted shows is filed, merged or added to a Note, made a
  Checklist, Shopping List or Tally, or given a Routine, Reminder, Frog
  or Session, the Note editor opened from Unsorted or the Nudge screen,
  on the desktop the Note dialog opened from Unsorted, shows the next
  Unsorted Note the list shows (its person filter, on the desktop its
  search), the one after it in the list's order, else the first. The
  list itself stays, on the desktop with its first card selected. Once
  nothing is Unsorted the Agenda shows. A Note opened anywhere else, an
  Area, a search, the Agenda, stays open; a Note moved to the trash
  moves nothing on. On the desktop Ctrl+Z, bringing a sorted Note back,
  shows Unsorted again: the Note in the Note dialog where it was sorted
  there, else its card selected.
- Rows show what the user needs to act, not the stored encoding: dates
  in the device format, values translated, storage keys never.
- **The Areas stand in the person's own order** (#280): on the phone's
  Areas list, the desktop's sidebar and every list of Areas (File
  into…, Area Locks, Link to…, Notes for the feedback), the same on all
  of the person's Devices, a setting of the person (ADR 0040), so the
  others of a Shared Area keep their own. Until the first move they
  stand by the name the app shows, in lower case, a starter Area by its
  translated name, and a new Area sorts in among them; after it a new
  Area goes to the end. Every list passes the core the name it shows,
  so a move counts its place in the order on screen. Topics stay by
  name.

### Wording

- The UI speaks in the terms of `CONTEXT.md`: Notes, Areas, Topics,
  Routines, Reminders, People; never of entries, entities or hashes.
- **One action has one name everywhere** (#321, Q724): in a menu, on a
  badge, in the editor and on a button, on the phone and the desktop
  alike.
- **A menu shows at most seven entries** (#321, Q724); the rest go under
  "More…". Entries of one kind go under one entry that opens them as a
  group (the card's This Note ▸, Add ▸, Share ▸, Tidy ▸, a table
  column's Insert ▸, Move ▸, Align ▸, Sort rows ▸, Show sorted ▸,
  obf-1014). The law covers menus of actions only; exempt are **a list
  of choices** (Heads-up lengths, people, calendars, Shopping Lists), **a text
  field's edit menu**, **the badge group** (This Note) and **the
  phone's format bar's ⋮**, which holds every format without room.
- **The names are one per action** (obf-1014): Add a part (the box it
  opens confirms with Add), Merge into…, Find in Note wherever the
  Note's find bar opens (Search stays for the lists and Search all
  Notes), Move to trash (also for a Frog; "Move this Note to the
  trash?" where it asks), Send to a person, Copy text, Open in
  another app, Save photo (Save photo… on the desktop) and Copy photo,
  Share as calendar file,
  Session…, New Topic, Quick note, Share to an app, Attach a file…;
  the card keeps Edit and the quick note Open in the editor, which
  saves first. An Area's entry is Share with people…; the Note's menu
  and group keep Share.
- Errors name the cause and the fix in one sentence each.

### Spotlights

- **Each screen explains itself** (obf-1043, ADR 0055): on a person's
  first visit to a screen its Spotlights show one after another, and
  from then on only a new thing on it gets its Spotlight, once. Each
  screen has its manifest of tips, in the order they show (phone:
  `spotlightManifest` in `spotlight.dart`; desktop: `spotlight.rs`); a
  ticket that adds something to a screen adds its tip there, in English
  and German.
- **The look**: the whole window dims, black at 60 %, with the element
  cut out of the dim, a little larger than it and with rounded
  corners. A card of the theme beside it, on the side with room, over
  it only where a tall element leaves no room elsewhere, at most
  400 wide and 24 from the window's edges, holds one sentence of at
  most 15 words, what the element does for the person, without "tap" or
  "click" where it can do without, then No more tips, Skip and Next at
  its bottom right, Next reading "Got it" on the screen's last tip.
- **It never traps input**: Skip is always on the card; Skip, a tap or
  click outside the card, Esc and Back end the screen's tour and count
  its other tips as shown. A tip is counted as shown before it shows.
- A tour starts only once the screen is on top, its page transition
  settled, with no dialog, menu or other page over it; an element below
  the fold is scrolled into sight first. A tip whose element is not on
  the screen (Advanced only in Simple, Linked from without a link)
  waits for a visit where it is.
- The marks are this Device's (local settings), and "Show tips again"
  in the Settings brings every tip back on it. Tests, kittests and the
  screenshot generators show none; no guide frame shows a tip.
- **They can be switched off** (obf-1060, C3): the switch "Show tips"
  on the Settings' card Tips, on by default and kept on this Device
  (the local setting `tipsOff`), with "Show tips again" under it. Off,
  no screen shows a Spotlight, a new one neither, and none is counted
  as shown; "No more tips" on every tip's card turns it off at once,
  the rest of the tour still due. "Show tips again" turns it on.

### Look

- Light mode: a light teal background. Dark mode: anthracite. The
  accent is teal in both. The theme follows the system, with a manual
  override.
- The background is scattered hand-drawn question marks, a touch darker
  than the background in light mode and a touch brighter in dark mode,
  moving in slow parallax. The parallax stops when the system's "remove
  animations" setting is on.
- Cards and controls must always stand out clearly from the question
  marks.
- **Fat fingers** (#224): one switch of the Device, off at first, on the
  phone in Settings ▸ Look, on the desktop "Bigger text and icons" in
  Settings ▸ This computer. On the phone it grows text 1.3 times on top
  of the system's font size, and icons and tap targets 1.3 times, in one
  place: the theme (`lookTheme`, its `LookScale`) and the text scaler at
  the app's root; a switch, which no density reaches, through
  `LookGrown`. The Agenda's measures (`AgendaSizes`) grow by the same
  factor, and the widget's text with them. On the desktop it is egui's
  zoom at 1.3.
- **The text has a size of its own** (#326, obf-1019, Q729): Text size,
  a local setting of the Device, Normal (1), Larger (1.15) or Largest
  (1.3), normal at first and for anything unknown, on the phone in
  Settings ▸ Look between the theme and Fat fingers, on the desktop in
  View ▸ Text size and Settings ▸ This computer beside "Bigger text and
  icons". It grows content text only: Notes, Lines, Sections, a table's
  cells, the Agenda's rows and Ask's answers, as they read and in the
  fields they are typed in; buttons, icons, boxes, menus, bars, headings
  of pages and the labels around the text keep their size, which stays
  Fat fingers' alone. On the desktop the words of labels, buttons and
  captions grow with it too, through egui's text styles (Design system
  ▸ Spacing and type, obf-1081); its icons keep their size. The step multiplies the system's font size and
  Fat fingers (the desktop's zoom), never replaces them, and applies
  once where content nests. A formatted text's room, a list item's
  indent and a code block's padding grow with the size it shows at, so
  "Formatted text reads calm" holds at every step. On the desktop
  Ctrl+Plus (and Ctrl+=) and Ctrl+Minus step it, kept at Normal and
  Largest, kept across starts, as the menu bar's keys do (not while
  another dialog is open), and egui's own zoom keys are off: Ctrl+0 does
  nothing. The Agenda's widget keeps Fat fingers alone. Phone:
  `TextSizeScale` and `ContentText` in `look.dart`; desktop:
  `theme::content` and `Look::of` in `text_marks_view.rs`. The AI port
  reads it in read_settings (`textSize`) and, as every setting, does not
  set it.
- **Formatted text reads calm** (#327, obf-1020): wherever a formatted
  text shows, on both apps, every measure in parts of its font's size,
  so a bigger text keeps the look. Its lines lie 1.5 sizes apart. Between
  two lines as it reads lie 0.5 more, 0.8 before a heading, 0.3 after
  one, 0.2 between two items or two quote lines; a blank line makes 1.0
  instead of a line; a compact card's lines keep only the line height.
  Headings are 1.3 and 1.15 sizes, semi-bold on the phone (the desktop's
  bold is its drawn-twice bold). Code is in the fixed-width font at 0.9
  size, on a light tint of the text's colour (8 %, light and dark), no
  frame, the tint only as high as its words (1.3 of code's size) and on
  the desktop a little wider; a code block's lines are 1.3 apart on the
  same tint, and so is a Code text's ground. The font stays the system's
  on the phone and egui's own on the desktop. The phone's field keeps
  the 1.5 line height but no room between lines (a Flutter field could
  only add it unevenly to a wrapped paragraph); the desktop's field
  shows the Markdown with the same room, a code block's and a table's
  lines together, the table's grid where its first line lies. egui puts a
  taller line height under the words and an underline with it, so the
  desktop moves the rows of a laid-out text apart instead (`spread`),
  and a click still finds its character. Phone: `formattedLineHeight`,
  `lineGap`, `codeStyle`, `codeTint` in `text_marks_view.dart`; desktop:
  `LINE_HEIGHT`, `line_gap`, `spread`, `source_spread` in
  `text_marks_view.rs` and `theme::code_tint`.

### The sync line

- **While a sync of this Device runs, a thin line moves under the bar**
  (#225): 3 px in the accent along the bottom edge of the phone's top
  bar (`pageBar`, Capture's bar too) and of the desktop's menu bar, a
  quarter of its width moving back and forth (Knight Rider), taking no
  room, so nothing else on the screen moves. It shows only while a
  folder round or a join of this Device runs, on the desktop also while
  its paired host answers, or an Import runs (obf-1061), or an Area is
  being deleted (obf-1093), or the trash is being emptied (obf-1095), and the app is in view; every sync shows it, also the one each minute. The sync runs beside the app (an isolate on
  the phone, a thread with its own connection on the desktop): the app
  stays usable and draws on. With the system's "remove animations" the
  phone's line stands still across the whole width; the desktop reads
  no such setting, so its line always moves. On the phone a screen
  reader hears "Syncing…" once each time the app comes into view, not
  with every sync; on the desktop the line is a progress indicator named
  "Syncing…". A tap or click on the line, or a little above it, opens
  the page Running (obf-1078; see Phone ▸ Dialogs and pages and Desktop ▸
  Menu bar and keys).
- **Deleting an Area runs beside the app** (obf-1093), in both apps:
  once the dialog is answered, the Area's row says how many of its Notes
  have gone ("Being deleted: 120 of 340 Notes moved"), a spinner in
  place of its menu (on the desktop, of its chevron), and takes no tap,
  click or drag until the Area is gone; a failure is announced at the
  top, success is not. On the desktop it runs on a thread with its own
  connection, one Note per transaction, its Topics hidden under the
  row; the line is named "Deleting an Area…", and the Delete in the band's ⋮
  asks nothing for it meanwhile.
- On the desktop the same line moves while the Agenda is read for the
  first time after the window opened (obf-1090), named "Reading the
  Agenda…": the window shows at once, the Agenda read on its thread.
- On the desktop it also moves while an Import runs (obf-1061), named
  "Importing…": the Import writes on a thread with its own connection,
  one Note per transaction, and the Import dialog counts its Notes
  ("120 of 340 Notes"); it may be closed and File ▸ Import… shows it
  again, at the end with what went where and Open the Area.

### Alarms and notifications

- An Occurrence's notification offers **Done**, **Snooze** (10 min,
  30 min, 1 h) and **Not today** (Skip, and Tomorrow) without opening
  the app (#269). Done is also at the top of its Note's editor and on
  its card (#215); Snooze and Not today sit in the notification, the
  Agenda row's ⋮ and the Note editor's Due line. **Skip** marks the
  Occurrence neither Done nor Missed: its alarm and Re-nag stop, it
  never shows in the Missed Summary, and it leaves the Adherence, which
  counts it beside it ("92 % this month · 3 skipped"). **Tomorrow**
  skips it and adds a one-time Occurrence of the same Routine the day
  after at the same time; it is offered only for an Occurrence of today
  or later. Neither changes the schedule. Done moves a Plain Note with
  its only Reminder to the trash where the person chose so (#211,
  "Agenda").
- **Quiet Hours are quiet** (obf-1045): in them an Occurrence's
  notification and its Re-nags show on time without sound or vibration,
  never full screen; a Routine with Escalation on rings with sound from
  its first ring, as Escalation is the only way through. An all-day
  Occurrence's first ring still waits until they end (#201).
- **A Heads-up is one plain notification before its Occurrence**
  (obf-1046): the Note's title with how long until it starts ("in
  20 min", "in 1 day"), no Done, Snooze or Not today, no Re-nag, never
  full screen, without sound in the Quiet Hours whether or not the
  Routine escalates. It shows in its Occurrence's notification, so the
  ring replaces it; on the iPhone, which cannot replace a delivered
  notification, it sits in the Occurrence's group beside the ring. Done, Skip or Snooze before it cancels it; the
  Occurrence still rings at its time.

## Phone

### Navigation

- **Never a bottom bar**, on any screen. The bottom edge belongs to the
  system's navigation buttons and gestures; app navigation there only
  causes wrong taps.
- **The end of every page clears the system navigation bar** (#61), on
  gesture and button navigation alike: the last row scrolls fully above
  it, and controls anchored at the bottom sit above it. **The end of a
  page also clears its floating buttons** (#158): the last row scrolls
  fully above them, their margin and a gap included.
- The Agenda carries one floating button: **"+"** captures a Note.
  Nothing else floats, except the ways "+" fans out while it is open (see
  Capture). **The folder**, first in the Agenda's top bar, opens the
  Areas list (#221); search by text and by #tag stays inside the
  browser.
- **Tags under the Areas list's search** (obf-1074): while its search
  field is empty, one row of chips right under it shows the most used
  tags, at most twelve, of every level, those more Notes carry first and
  of as many the one first in the tag tree, and ends in "All tags…",
  there also without a tag; a tap on a tag searches for it, as a tap on
  a tag in a text does. Wider than the page the row moves sideways under
  the finger. It leaves out the Notes of a locked Area while it is
  closed, and is read only while the Areas list shows.
- **The tag tree** (#267, Q623, Q639): "All tags…" at the end of the row
  of tags opens every tag of the Notes by its levels, the top
  level as written (`#life`), a level below by its own part (`health`),
  indented, each with how many Notes carry it or a tag below it; the
  levels below a tag unfold by its arrow, the top level alone at first.
  A tap on a tag goes back to the Areas list searching for it, as a tap
  on a tag in a text does. Typing `#` in the Areas list's search shows
  the tags starting with what is typed as chips under the field, at most
  twelve, the one typed exactly left out; a tap fills it in. The tree
  and the chips leave out the Notes of a locked Area while it is closed,
  as every list does.
- **Ask** (#267, ADR 0051) is experimental and off until "Ask
  (experimental)" (#333), a switch of the phone in Settings offered in
  Advanced only, is on; while it is off there is no Ask button, no Ask
  page and no address by hand, and nothing else of pairing or sync
  changes. It is a `BarAction` in the Agenda's bar after
  the folder, shown once a Device of the user's own that hosts is
  paired, and opens the Ask page, which asks the user's computer as the
  desktop's Ask panel does (see Desktop, "Menu bar and keys"). Before
  the first question to a provider, protocol, host and model alike, the
  page shows in place of the question field the computer, the provider,
  its model and host and what is sent, with Allow and Not now; nothing
  is sent before Allow, and a provider not allowed is asked about again
  before the next question, which stays in the field. While no computer
  of the user's answers, the page says why and what to do in the error
  colour, with Try again; while its provider is not set up or its AI is
  off, the question field says so with the fix as its error, Ask cannot
  be pressed, and Try again asks again (#317). While a question runs, a
  status line under it says "Asking Desk…", once Notes were read
  "Asking Desk… reading 3 Notes", with Stop, which puts the question
  back into the field. A question not answered goes back into the field,
  and its cause and fix stay under it, with Details, until the next is
  asked (#317). **The questions stand as a thread above the field, the
  oldest first** (#315), each with Edit, greyed while a question runs,
  which puts it into the field with the cursor at its end to be asked
  again after the whole thread, and under it its answer with "From
  these Notes", "Also read" and a row of Copy text, Keep as Note, Add to
  Note… and Share to an app (#316): Copy text and Share to an app hand
  on the answer alone as Markdown, its links kept, Share with its HTML;
  Add to Note… adds the question and answer, as Keep as Note keeps them,
  to the Note picked as a Section, then "Added to “…”" opens that Note;
  Copy text and Share only for an answer with words, none of them under
  a question not answered; the question asked, or
  the last one not answered, ends the thread, which scrolls to it once
  it is asked and once it comes back. Start over in the bar drops
  them, and so does leaving the page. **An address by hand** sits under each such
  paired Device on the Sync page, a field with its line of what it is
  for and its error under it once it is left or submitted; one kept
  while Ask is off is still tried by sync.
- Settings, Backups, Sync and About are reached only through the
  **Profile picture in the Agenda's top corner**. It
  opens the Profile page: picture, name and short code; Backups and
  recovery; Your Devices; People; Settings of this Device; then About
  Old Brain Fix, an entry of its own beside them, never inside the
  Settings (#325, Q728). No hamburger menu, no ⋮ menu for them.
- The levels run Agenda, Areas list, Area, Topic: **the Agenda is the
  level above the Areas list**. **Swipe left on the Agenda opens the
  Areas list**, as the folder does. In the notes browser, **swipe
  right goes back** one level (Topic, Area, Areas list, Agenda) and
  **swipe left goes forward** into the last selected Area or Topic, if
  there is one. The last selected Area and Topic are highlighted.
- **The Agenda and the Areas list are two pages of one `PageView`**
  (#258), side by side: the platform's own drag and fling
  (`PageScrollPhysics`), counted from the touch-down so a quick short
  flick switches both ways; letting go past the middle or with a fling
  switches, otherwise they spring back. The Areas list is built with
  the Agenda and kept, so a swipe builds neither page. Back on the
  Areas list, by the system or its bar's arrow, and the folder move
  between them; nothing else does, and the lock goes back to the
  Agenda. An Area, a Topic and their Notes are pages pushed on top, as
  is the Areas list of a tag's search. On iOS a drag that starts at the
  left edge moves neither page (Q570). With the system's animations off
  the pages still follow the finger, and letting go lands at once.
- **In an Area, a Topic and All Notes "+" fans out** its kinds (#161),
  each an icon with its words, upward from the button as the Agenda's
  fan does. Its Note opens, in an Area or a Topic, the Note editor on a
  new Note filed there, the cursor in its text and the keyboard up, with
  no "Add a part" (obf-1140); in All Notes, the quick note.

### Gestures

- **The green "!" comes with the buttons' ink** (obf-1143): every
  button of the look's theme, and each "+" and fan entry through
  TapMarked, marks a tap once it is a tap, not a press that became a
  scroll. "Show button taps" in Settings ▸ Look, under Fat fingers,
  switches it off on this phone; tests and screenshots start without it.
- **Tap is the primary action.** On a list row it opens the thing; on a
  checkbox it checks. **A tap on a Note card switches that card** between
  its compact and its detailed view, in place (#189); in "merge into…" it
  picks the Note. A tap on a link in its text, a formatted text's or a
  bare `http://` or `https://` address (#222), opens that link instead; a
  long press on it is still the card's. A tap on a row or card never
  opens a menu, nor selects it: a list selects with its boxes (see
  Cards, #273).
- **An app link opens its place without asking** (#261, ADR 0046): a
  Note, an Area or Topic, the Agenda, Unsorted or one of the app's
  pages, as its own menu opens it. A target this Device cannot see is an
  announcement, "Not available here", never an error dialog. A file link
  is the one link that asks first, once, naming the file and the app
  that opens it, and opens only a file of the shared folder.
- **Long-press is the secondary action**: a menu, or the editor when
  there is only one thing to do. A long-press on a Note card opens its
  editor, as Edit in its ⋮ does; nothing else opens it from a card. A
  long-press on "+" of the Agenda or the lock screen opens text capture
  (#282; see Capture). A long-press never changes data by itself.
- **An Area moves while rearranging** (#280, obf-1089): the phone's
  Areas list's ⋮ in the top bar offers Rearrange with two Areas or more.
  Until Done, which takes ⋮'s place, leaving the page or a search, each
  row shows a drag handle in place of its ⋮; a drag that starts anywhere
  on the row, either way, moves the Area, a tap on a row opens nothing,
  a swipe on it does not change the page, and the swipe left does
  nothing; a tag's chip searches and "All tags…" opens the tag tree,
  either ending it. Each move is kept at once. Outside of it a row has no
  handle and a long press moves nothing. A screen reader moves an Area
  with the row's actions, Move up, Move down, Move to the start and Move
  to the end. The search's hits carry no handles.
- **A horizontal swipe only navigates** (see Navigation); it never
  changes data or swipes a row or card away. The exceptions are the
  full-screen table, where a one-finger drag on its cells moves it
  sideways, as its bar does, a table wider than the editor's field
  (#257), which a drag on it moves sideways, a table wider than its room
  on a card or in a text being read (obf-1048), which a drag on its
  cells moves sideways, as its bar does, and a Code text in the Note editor (#213),
  which a one-finger drag moves sideways; on a card a Code text is cut
  off at the edge instead; and an Area row while the Areas list
  rearranges (obf-1089), where a drag moves the row; and the row of tags
  under the Areas list's search while it is wider than the page
  (obf-1074), which a drag on it moves sideways.
- **On the iPhone a pushed page goes back with the edge swipe** (#243,
  Q570), iOS's own, from the left edge; a browser level goes back with
  a swipe right anywhere, as on Android. A page that holds its leaving,
  such as one asking to keep changes, takes no edge swipe. A long-press
  on a Note card gives the iPhone's haptic where Android vibrates.

### Announcements

- **An offer to undo** (#268): what was just deleted on the phone and
  can be restored says so at the top at once ("Moved to the trash",
  "Line deleted", "Entry deleted", "Frog moved to the trash", "“From”
  label removed", "Transcript removed"), and so does a Skip or Tomorrow
  made in the app ("Skipped", "The day after instead", #269), whose undo
  takes the skip back; one made on a notification offers none. A
  selection moved to the Trash is one offer for all of its Notes (#273).
  It ends
  with "Undo"; its
  tap undoes it, and a screen reader hears "Undo" as the tap's hint. It
  shows before what waits, a newer one takes the place of the one
  before, and left alone it goes by itself after 10 seconds, undoing
  nothing. What it deleted stays where it always went: the Trash,
  "Removed entries", "Removed Lines", or "again" under its file; a
  "From …" label has no such place and stays gone.
- **"+" stays whatever filter is on** (obf-1070, C13): in an Area, a
  Topic and All Notes, "Show Notes by…" never hides it. A Note the
  filter hides is made and opened as usual; back in the list the top
  says "Made, hidden by the filter", ending with "Show", whose tap
  clears the filter, as Both apps ▸ Announcements names. It is an offer as the one to undo is: it shows
  before what waits, takes the place of the offer before, a screen
  reader hears "Show" as the tap's hint, and left alone it goes by
  itself after 10 seconds. With "Me" in the filter nothing is said.
- **A restored Backup's speech model is offered** (obf-1098): right
  after the restore, "Transcribe voice Notes on the phone?" offers the
  model the Backup names, with **Download** and its size, or **Pick the
  file again** for a model taken from a file, and **Not now**, which
  drops the offer. A model fetched drops it once it is in use. A model
  in use here already, or a phone where Whisper does not run, gets no
  offer. Still not in place on the next start, the top says "The
  speech model from the Backup is not on this phone yet", ending with
  "Download" or "Pick its file", once; its tap fetches it, and Settings
  offers it after that.

### Capture

- **The Agenda's "+" fans out Text, Voice, Photo, Gallery, Paste and
  File** (File: #267), and so does the lock screen's, each an icon with
  its words, upward from the button; it is not a bottom sheet. While
  the fan is open, "+" reads as close; a second tap on it, a tap
  outside or Back closes the fan. Text opens text capture, Voice starts
  recording at once, Photo opens the camera, Gallery opens the system's
  picture picker and continues as a photo Note, Paste opens text capture
  filled from the clipboard: its text in the field, its picture as the
  Note's photo, both when it holds both, and a line on the screen, no
  announcement, when it holds neither. File opens the system's file
  picker, several files of any kind at once, and continues as text
  capture with them attached as the Note's Attachments; a file larger
  than 256 MB, or one that cannot be read, is not attached and is
  announced, the others are. The clipboard is read only on
  that tap, never on opening or in the background. **A long press on
  "+" opens text capture at once** (#282), the Agenda's and the lock
  screen's alike, as Text in the fan does, with no fan; a tap still fans
  out. A screen reader announces the long press with the button ("write
  a Note at once"), and on the iPhone, whose VoiceOver has no long press,
  it is among the button's actions. The text capture
  screen keeps its Photo, Voice, Gallery, Paste and File buttons. Every other
  quick entry point (widget, tile, shortcut, also when the phone's key
  or gesture settings or a button-mapper app start it; no code reads a
  key) opens its capture screen directly, with no fan and no screen in between.
- **The Agenda's fan has Editor at the top** (obf-1141), above File,
  the entry farthest from "+", with the editor's pencil: it opens an
  empty Note in the Note editor, the cursor in its text and the keyboard
  up, with no "Add a part". Left by Back it is kept Unsorted with a
  Nudge in 1 h, as the quick note's Open in the editor keeps it, and
  shows on top of the Agenda as a Note captured there does; left with
  nothing in it, it makes nothing. The lock screen's fan has no Editor,
  as the editor shows the Areas; the long press on "+" stays text
  capture.
- **A Tally's shortcut opens its entry page directly** (#284): "Add to
  home screen" in a Tally's card's ⋮, offered where this Device may add
  entries to it, pins a shortcut named as the Tally (Android's own
  prompt places it; "Private Tally" for one in a locked Area set to
  Masked, the home screen being as public as the lock screen). On the
  iPhone, where no app may place an icon, the entry reads "Add to the
  app icon" and puts the Tally among the actions of the app icon's long
  press, the newest four, said at the top. A tap opens the Tally's
  entry page with no screen in between, over whatever the app showed;
  as the Agenda widget's pick, it opens its locked Area first and,
  behind the App Lock, after the unlock, since the page shows the
  Tally's targets and latest values. Once the page closes, saved or
  not, Android goes back to the home screen with the app kept as it
  was, unless the user left the app between the tap and the page, or
  while it was open; the iPhone, which lets no app leave itself, stays in the app
  where it was. A Tally deleted, in the trash or no longer this
  Device's to add to opens only the app, with "This Tally is no longer
  there." at the top.
- **On the iPhone a share waits for the app** (#241): iOS lets no Share
  extension open its app, so "Old Brain Fix" in the share sheet keeps
  the text, links, pictures, the app's own files or any other file and
  says so on a small card with OK. The next time the app opens or comes back, each share
  opens Capture as on Android, and a Pack, Profile card, Sent Item,
  bundle or AXT file (obf-1146) opens as "Open with" opens it. A share carries no source app on
  the iPhone.
- **On the iPhone iOS's own prompts may come beside Capture** (#243,
  #244): the first Photo asks iOS's camera permission, and Paste may
  show iOS's own "Allow Paste" prompt each time it reads the clipboard.
  Both are iOS's, not the app's, and are allowed beside "Capture never
  asks anything"; the app itself still asks nothing there.
- **A file shared in becomes an Attachment** (#267): any file another
  app shares, other than a picture or a sound file, opens Capture with it
  attached, one Note with the text and pictures shared with it, written
  at once; a file of the app's own types (a bundle, Sent Item, Profile
  card or Pack) and an AXT file (`.axt` or `application/vnd.axt`,
  obf-1146) shared to the app open as "Open with" opens them.
- **A page printed to the app comes in as a shared PDF does** (#293):
  on Android, Old Brain Fix is a print service with one printer, "Old
  Brain Fix"; a print job opens Capture with its PDF attached, named
  after the job's document, without a source app, as Android names
  none. Where Android keeps the app from coming up by itself, the print
  waits and opens Capture the next time the app opens or comes back.
  Settings ▸ Capture ▸ "Print to Old Brain Fix", in Simple and Advanced,
  says whether Android has the service on and opens Android's print
  settings, where only the user turns it on; the iPhone has no row.
- **The quick note is the text, the Nudge buttons and Open in the
  editor** (#142, #352, obf-1080): nothing else, in Simple and Advanced
  alike, on every capture path (text, Voice, Photo, Gallery, Paste,
  share, widget, tile, shortcut, also one a phone key or gesture
  starts). What a path brought in, a recording, photos or files, shows
  above the text. **Open in the editor** saves the Note at once,
  Unsorted with the default Nudge, and opens the Note editor on it in
  place of the capture screen; leaving the editor
  returns to where Capture was opened from. It waits while there is
  nothing to save and is not offered behind the App Lock.
- **The quick note takes its recording's Transcript** (obf-1082): with
  a speech model, "Transcribing…" shows under the recording made there,
  over the sync line's moving stretch (3 px, still across the width
  with "remove animations"), until Whisper is done or could not make
  it. The Transcript then goes into the text field, filling an empty
  one or after the text as a new paragraph, the cursor left where it
  was; while the user types it waits until typing rests for 1 s. Under
  the recording it stays, folded under "Transcript" until a tap opens
  it, and it stays the recording's Transcript. A Transcript still
  waiting for the pause is saved with the text by a Nudge button, Back
  or Open in the editor. Left before Whisper is done, the Note is saved
  with the text as it was and the recording gets its Transcript later,
  not the text. A recording shared in is not taken into the text.
- **The Note editor holds what the quick note no longer shows** (#352):
  under This Note its badges the Kind, the place (File into…) and the
  Importance; under Add "+" with Remind me…, in Advanced Frog and
  Session…, Voice, Photo, Gallery, Paste and Attach a file…; under Tidy
  Merge into…, which adds it to another Note, and Add to a Shopping
  List; the format bar rides on the keyboard.
- **The Note editor's actions stand in four named groups** (obf-1073),
  as in the desktop's Note dialog: **This Note**, the row of badges
  under the text; at the page's end, after all the Note holds, **Add**
  ("+", Voice, Photo, Gallery, Paste, Attach a file…, Add a part),
  **Share** (Send to a person, Share to an app, Export as Pack…, Export
  as AXT file…, obf-1145) and
  **Tidy** (Merge into…, Add to a Shopping List, Move to trash, the
  last filled on the error colour, obf-1099). Each group is a small heading, read as a
  heading, over its compact buttons side by side, wrapping; a group
  with nothing this Device may do shows nothing. The top bar keeps
  Undo, Redo and Find first, then the calendar, History and Revert all
  changes; none of the groups' actions goes into its ⋮. Voice records on a page of its own
  with the big stop button; Stop, Back and Stop on the notification keep
  the recording on the Note.
- **A pause ends a recording only where the Device says so**
  (obf-1053): Settings ▸ Capture ▸ "End a recording after a pause",
  kept on this Device only, offers Off, the default, which ignores
  pauses, and 2, 3, 5 or 10 s. Set, every voice recording on the phone,
  the quick note's and the editor's page alike, also one the iPhone
  goes on with locked, ends once the microphone stayed quiet that long
  after it heard something, and is kept as Stop keeps it; a recording
  nothing was said into yet goes on. Android ends a recording on
  locking anyway (#368). The row shows only where the phone records.
- **Settings ▸ Quick note** (per Device, ADR 0008) keeps only the switch
  Nudge buttons, on by default. The switches that once added File
  into…, Frog, Session or Tally, Add to a Shopping List, the Importance
  and the format bar to the quick note are hidden (#352); what they
  stored stays and has no effect.
- After saving, the new Note's card shows with the same options, for
  whatever was not done while capturing (merging into another Note
  among them): in its detailed view, a tap switching it to compact. Enriching is never required; a Note left without an
  Area or Topic stays Unsorted.

### Agenda

- **The Agenda's bar**, in this order: All Routines and Reminders,
  Calendar, new Routine, New Session (only in Advanced, #246), then the
  Profile picture in the corner; the desktop's has the same without
  Calendar, and a ⋮ last with Quiet today while today is not quiet
  (#354). When the phone's bar is full, the last of All Routines,
  Calendar, new Routine and New Session go into ⋮, after the Profile
  picture (see The bar); the folder and the Profile picture never do. All
  Routines and Reminders lists every one alphabetically by name, also
  those beyond the week, and a tap opens its editor; its bar's New
  Session opens the editor of a new Session whose Note is the text typed
  there, and its Export writes one calendar file of all of them. Calendar opens the Calendar
  page: "Read from your calendar" (the appointment running at Capture,
  and the switch that shows the calendar's appointments on the Agenda,
  whose calendars are picked on a page of checkboxes, none at first) and
  "Write to your calendar" (the calendar mirror); Settings has a row
  leading to the same page. Nothing on the Agenda selects rows to share.
- **The Agenda's rows are tight, their targets are not** (#208): on the
  phone every box and ⋮ is 40 by 40 dp, a heading at least 40 high with
  2 above and below it in its card, an entry 40 high, 6 between two
  cards, entries indented 48; an Occurrence's card is 48 high and the
  next starts 54 below it, where 0.3.1 had 56 and 68. The "Eat a frog"
  rows and the appointments' rows are as tight. Not as tight as the
  widget, whose rows lie 4 apart. With Fat fingers every one of these
  measures is 1.3 times (#224).
- **The row a notification tap asked for is marked** (obf-1039): its
  card on the find bar's tint (tertiary container) in a 3 dp ring of
  the tint's strong colour (tertiary), a Missed Occurrence's line in
  the Missed Summary alike with corners of 8, scrolled a fifth down
  from the top or a little lower, so the heading or row above it starts
  whole under the bar and a first row of today keeps the Today heading
  (obf-1137); one mark at a time, gone once the app goes out of
  sight. See Alarms and notifications.
- **A calendar appointment's row is read only**: no checkbox, the
  calendar's colour down its side, the calendar icon and the calendar's
  name. A tap unfolds its place and description and folds them again;
  ⋮ or a long press opens "Copy into a Note", which makes a new Unsorted
  Note of its title, time, place and description and shows its card on
  top. The calendar mirror's own entries never show as appointments.
  Nothing of an appointment is kept or synced.
- **The home-screen Agenda widget shows exactly what the Agenda shows**
  (#153, #194), in the Agenda's order: the Missed Summary with its
  Missed Occurrences, the "x Unsorted" row, which opens the Unsorted
  list, then under "Today" the rows of today and the "Eat a frog"
  section, then the Agenda's coming days, "Tomorrow" and the five days
  after it under their weekday and date, each with its rows and
  appointments, dimmed, an empty one with "Nothing planned". It moves on
  at midnight without the app, showing the days its snapshot still
  holds. It is one list that scrolls when it is taller than the
  widget. **Its groups are the Agenda's** (#209): a Shopping List's and
  a Checklist's name as the heading, without a box, its words where a
  boxed heading's begin, with their open Lines under it and those
  checked today struck, box checked (#276); a Checklist
  Routine's Occurrence with its Lines; a Session's Occurrence with its
  picks; each entry with its box, indented alike in every group so its
  box stands where the heading's words begin, a Line under another
  Line one step further. A Shopping List shown as its count, or Masked,
  and a Masked Checklist are the heading alone, as on the Agenda. A box
  there checks what the same box on the Agenda checks, with the same
  result: an Occurrence, a Line for its Occurrence, a Checklist's Line,
  a Frog, a Missed Occurrence, a Shopping List's Line; unchecking is
  done in the app. **Only today's rows and the Missed Occurrences tick
  there** (#194): a coming day's box, unlike the Agenda's, checks
  nothing and opens the app. **A Session's box only shows Done**, and a tap on it
  does nothing; a pick's box shows whether it has an entry, and a tap
  on the pick, box or words, opens the app at its entry form, its
  locked Area first, once the App Lock is open. Appointments, the lists'
  headings and the other headings have no box; every other tap opens
  the app. Masked rows show as on the Agenda and the lock screen.
  Nothing of an appointment is kept for the widget either.
- **On the iPhone the Agenda widget** (#242) is medium or large and
  differs where WidgetKit decides: it does not scroll, so it shows the
  rows from the top as far as its height allows, never a heading as its
  last row, and "+n" for the rows left out, which opens the app like
  every other tap. With Fat fingers fewer rows fit. A box ticks without
  opening the app and shows checked at once; the app applies it the
  next time it opens or comes back. The appointments show while the app
  has full access to the calendars. Capture there is one small Capture
  widget, its button opening voice, text or photo as chosen when the
  widget is edited (text until then), the same widget on the lock
  screen, and a Control Center control each for voice, text and photo;
  each opens its capture screen directly, as Android's widgets and
  tiles do.

### Cards

- **A photo gets out of the Note** (#149): full screen its ⋮ offers
  Save photo, Copy photo and Share to an app on the phone (obf-1014),
  Save photo putting it into the phone's Pictures in the album "Old
  Brain Fix" (Android 10 and later; before it, Save photo is announced
  as failed with Share to an app as the fix), and only while
  its file is on the phone. Saved and copied say nothing, nor does
  a share sheet left without an app; a failure is announced.
- **An Attachment shows on its card** (#267, ADR 0047): each file a row
  with the icon of its kind (a PDF, a text document, a table, plain text
  or Markdown, any other file), its name, its size and its pages when
  they were counted. A tap opens it, on the card and in the Note
  editor, asking nothing, as the file is the Note's own (unlike a file
  link, #261): **a PDF full screen like a photo** (#286), drawn by the
  system's own renderer (Android's `PdfRenderer`, the iPhone's PDFKit),
  its pages swiped sideways and zoomed with two fingers, a zoomed page
  panning instead of turning, "Page 2 of 12" at the top when it has
  more than one, its Attachment Text one tap away on the bar's
  Attachment Text button and back, and its ⋮ opening it in another app;
  back closes it. Only the page shown and the one on either side are
  drawn, as large as they show, the page shown anew when zoomed. A PDF
  the system cannot draw (one locked with a password), an Office file
  and any other file open with the system's app for its type, except
  an Android app (`.apk`, `.apks`, `.apkm`, `.xapk` or
  the media type of one), which asks once first: "… could install an
  app. Open it anyway?", with the person who added it when another
  person did; a file whose bytes have not synced yet, or one no app
  opens, is announced. Its Attachment Text shows under it as a Photo Text does:
  dimmed, labelled "Attachment Text" in the detailed view and whole
  there, its first three lines in the compact view. While a computer is
  to read it (a PDF of more than ten pages or not counted, a larger
  file), "The text follows from the computer." stands there instead. A
  Note of one file alone is that file as its card, its name as the
  title, and goes by the file's name wherever only its words fit, rows
  and alarms included. In the Note editor each file has Replace and
  Append for its text, Remove for the text (with the undo at the top,
  #268), Read the text again (not for a file left to a computer), and
  "Remove Attachment, keep text", asked once, where this Device may;
  the card's ⋮ ▸ Tidy offers the last too. "Attach a file…" under the
  editor's Add (obf-1073) picks more files.
- **What each group of a card's ⋮ holds on the phone** (obf-1014), each
  opening as a second menu at the ⋮, one thumb's reach from the first:
  **This Note** File into… (compact card), Show ticks, Hidden from AI,
  Remove "From …", a Tally's Add to home screen, Make it a Tally, Make
  it a Checklist, Make it a Shopping List, Show on the Agenda ▸ (its
  choices in a menu of their own) and Back to a plain Note; **Add**
  Remind me…, in Advanced Frog and Session… (compact card), Add a part
  and a Tally's Add entry; **Share** Send to a person, Share to an app,
  Copy text, Export as Pack…, Export as AXT file… (obf-1145), Add to
  calendar and Share as calendar file; **Tidy** Merge into… (compact card), Add to a Shopping List,
  Uncheck all, Move to trash, which asks nothing and is undone at the
  top (#268), and Remove photo, keep text and its kin, one per photo,
  recording or file. A group past seven entries shows six and More…,
  which opens the rest the same way.
- **Hidden from AI** (#267, ADR 0049): a card's ⋮ and an Area's menu in
  the Areas list have the switch "Hidden from AI", the Note editor its
  badge, offered on a Note or Area in no Shared Area and, once marked,
  always to take the mark off; it is read as the menu opens. A shield
  beside the Important marker on a card and on an Area's row shows what
  is hidden, a Note by its own mark or its Area's; the editor's badge
  says "Off, hidden by its Area" when only the Area hides it.
- The editor opens only through a long press on the card, Edit in its
  ⋮ or the pencil of a detailed card's action row (#281). Only a
  search's card that lists places found opens on a tap, at its first
  place, and a tap on a place opens the Note there (obf-1047).
- In the Note editor a text that is only read, the Note's or a
  Section's, selects with a long press, with Copy and Find in Note in
  the toolbar above the selection (#183, obf-1055; see Text editor).
- **Several Notes are selected with "Select"** (#273, Q655): every Note
  list, Unsorted and its Nudge page, an Area, a Topic, All Notes, the
  search's hits and the Trash, has Select in its bar while it shows a
  card this Device may change, as a long press opens the editor and a
  tap switches the view.
  It puts a box at the top left of every card this Device may change
  (ADR 0013); only the box ticks, a tap and a long press on the card
  still do what they do. While it selects the bar's title says how many
  are ticked ("3 selected") and its buttons are, in this order, Stop
  selecting, File into…, Merge into…, Importance, Export as AXT file…
  (obf-1145; a single Note names the file, several go under the day's
  name) and Move to trash, in
  the Trash Stop selecting and Restore, each greyed while none is
  ticked. File into… opens the picker of #272, marking the place when
  all of them sit in one; Merge into… opens the Note picker with every
  Note, the ticked ones included, and a ticked Note picked takes the
  others in, all in one go; Importance opens its three at the button.
  An action ends the selection; Move to trash asks nothing and offers
  one undo for all of them at the top (#268). Back and swipe right drop
  the selection and stay on the page, as do Stop selecting and a search
  emptied; on the Areas list beside the Agenda back leaves for the
  Agenda, and leaving drops it too. The cards a reload no longer shows
  leave the selection.

### Dialogs and pages

- Text and buttons must be readable at the phone's default font size:
  buttons stacked full width or in one row of two, never a cramped row
  of four. A row of three is allowed when every label is a few
  characters long, like "10 min", "30 min", "1 h". The content scrolls if it does not fit; nothing shrinks to
  fit.
- **The control goes below its text.** In every row that holds text and
  a control (switch, button, dropdown), the control sits on its own line
  below the text, always, also when the text is short.
- An editor with several fields is a full-screen page with Save in the
  top bar; secondary state changes are rows at the bottom of the page.
  The Note editor's settings are its badges (see Cards).
- **A custom Heads-up** (obf-1087): "Custom…", last in the Routine
  editor's Add heads-up menu, opens a dialog "Custom heads-up" with a
  typed number ("How long before", "From 1 to 720." under it for hours)
  and the units as choice chips under it, minutes, hours, days and weeks,
  for an all-day schedule days and weeks only; Add stays off until the
  number fits the unit, Cancel adds nothing. One set already is not set
  twice; set, it lists like the others ("45 min before") with its ✕.
- **Something Hidden from AI is not given away** (#267, ADR 0049):
  filing it into a Shared Area (a card, the selection, File into…),
  merging it with a Note of a Shared Area either way (a card, the
  selection, Merge into…; the selection stays when nothing merged),
  sharing an Area that is hidden or holds a hidden Note, Send to a
  person and Export as Pack ask once, naming what is hidden: "… hidden
  from AI. What is hidden from AI is not given to other people. Show it
  to the AI first." with Cancel and "Show to the AI", which takes the
  marks of what is given off and goes on. A Note hidden by its Area's
  mark is only named with the Area, and the question says the mark
  comes off in the Area's menu; one tap never shows the other Notes of
  that Area to the AI. Send and sharing an Area ask before whom.
- **Running, last in the ⋮ of the Agenda and of the Areas list, and a
  tap on a moving line open the page Running** (obf-1078, C21): the
  line under a bar takes the tap a little above itself, never on the
  bar's buttons, and the quick note's line under "Transcribing…" a
  little above and below; the page's own line opens nothing more, and
  while the app is locked no line opens it, as it names Areas and
  folders; it closes when the app locks. A card for each job beside the
  app (a sync with a Device, an Import, the Transcripts, Photo Texts and
  Attachment Texts being read, the speech model's download or copy, an
  Area being deleted, the trash being emptied) says what it does, how far it is when known ("120
  of 340 Notes", "3 of 12"), "Since 14:32", what Abort does where it
  does more or less than stop at once, and an outlined Abort, which
  reads "Stopping…" until the job stops; with none, "Nothing runs in the
  background." An Import stops before its next Note and keeps the Notes
  it made, its page and the page Running say "Stopped: 120 of 340
  imported", and the top says "Import stopped.", what it made behind
  the tap. Deleting an Area stops before its next Note, the Area staying
  with the Notes not moved. A sync round or join, a recording, photo or
  file under way ends first; then the automatic syncs pause (a paired
  Device asking hears 503 and comes back later) until Sync now, or the
  other recordings, photos and files wait, those asked for again and a
  recording something waits for still read; either until Go on on the
  page or the next start. The speech model's fetch is
  cancelled. What Abort stopped stays under "Stopped" while the app
  runs, the paused ones first with Go on, then the newest, at most ten.
  The search index is built within a search, so it is no job here.
- **Empty the trash now**, last on the Trash's bar (in its ⋮ when the
  bar is full) and not while selecting (obf-1095), is greyed while
  nothing in the trash can go and while the trash is being emptied; what
  can go is counted beside the app when the page opens and as the log
  changes, never in a frame. It asks once, "Delete 128 Notes and 4
  Routines for good? They can't come back.", a part that would say 0
  left out, with Cancel and **Delete for good** on the error colour, and
  cannot be undone. It runs beside the app as one batch: the line under
  the bar moves, the page Running lists "Emptying the trash" without
  Abort, a failure is announced at the top, success is not, and the
  Trash reads itself again once the log changed. Notes this Device may
  not change and Frogs stay.

### Text editor

- **The Note editor and Amend type in two rows** on the phone and the
  tablet, unless Settings of this Device switch it off, which gives the
  system's text field back.
- The top row is the whole text in the system's text field, so the
  keyboard's suggestions, autocorrect, swipe typing and voice input
  work. A tap puts the cursor where the finger is (#312, Q717); there is
  no magnifier and no dragging the cursor. A long press selects a word
  as usual, its handles drag its ends, with Copy, Paste, Search and
  Delete in the toolbar above the selection.
- The bottom row shows the cursor's line as a text field does: from
  its start, the cursor right after the text before it; a line wider
  than the row scrolls sideways only as far as keeps the cursor in
  view, so ◀ and ▶ move the cursor, not the letters. It sits directly
  above the keyboard, only while the keyboard is open; it is a bar of
  the field, not a sheet, and never takes the keyboard's focus. It
  rides on the keyboard and never sits on the screen's bottom edge, so
  it is no bottom bar. **◀ and ▶ move the cursor one character** (an
  emoji is one); held, they repeat, faster the longer they are held.
- **A long press on the bottom row starts selecting** where the cursor
  is and never moves it. While selecting, ◀ and ▶ show in inverted
  colours and move only the selection's moving end; the toolbar above
  the selection offers Copy, Paste, Find in Note and Delete; typing replaces the selection and ends
  selecting; another long press ends it with the cursor at the moving
  end.
- **A small toolbar floats just above a selection** (obf-1055), in
  place of the system's menu, wherever a Note's words are read or typed:
  the Note editor's fields and Section texts, a Line, a table's cell,
  Amend and a table's own page. It offers Copy, Paste where the field
  may change and the clipboard holds text, Search in the Note editor,
  and Delete where the field may change; no Cut, no Select all. A long
  press on an empty spot of a field shows only Paste, which puts the
  text in there, or nothing with nothing to paste. Search opens the
  Note's find bar with the words selected as they read, their first
  line holding any. The toolbar sits above the selection, below it
  where there is no room, not at the finger. Other selectable texts,
  such as an error report or a code to read out, and Capture keep the
  system's menu; a card selects no text.
- **Undo and Redo lead the Note editor's bar** (obf-1051, C3a): first in
  its order, so they stay on the bar of a phone 360 wide, with Fat
  fingers too, greyed while there is nothing to undo or redo. They go
  step by step through everything changed since the editor opened: the
  text and the Sections' texts, a text History brought back or a
  Transcript or Photo Text taken over, the words a removed recording or
  photo put into the text (obf-1084; the file stays gone), a table's cells, the Lines (typed,
  added, deleted, checked, moved, in and out, a Shopping List's reappear
  time) and the badges (kind between Plain Note, Checklist and Shopping
  List, Importance, Area and Topic, text format, code language, Show on
  the Agenda, Show ticks, Hidden from AI). Typing is one step until a
  pause of a second, a new word after a space or a line break, or a turn
  from typing to deleting; a format of the bar and a table's row or
  column changed are a step each; a Line's words are one step, stored
  first when Undo is tapped while they are typed. A step goes back through the same
  calls that made it, the earlier value written as a new edit; what can
  no longer go back, such as a Line a sync took away, stays as it is.
  **"Revert all changes"**, on the bar after History, in ⋮ where the
  bar is full, asks once, "Revert all changes?" with
  Cancel and Revert, then takes every step back to the Note as it was
  opened; Redo brings them back one by one. A formatted text does not go
  back to plain, and a Tally or a Session not back to a Plain Note, so
  those changes are no step. The history ends with the editor.
- **The find bar's "Search all Notes"** (obf-1055), between ↓ and ✕,
  opens the search over every Note for its words on top, as a tap on a
  tag does, greyed while it holds none; back returns to the Note with
  its find bar.
- **The Amend box shows only when asked** (obf-1072, C15c): the Note
  editor has no "Add to Note" field or button of its own. An existing
  Note has **Add a part** (`post_add`) last under Add at the page's end
  (obf-1073), for everyone who may Amend it, and so has a card's ⋮ ▸
  Add (obf-1014); it opens the box "Add a part" with its format bar,
  "What's new?" and Add. A Note
  being made, in the editor opened by the quick note's Open in the
  editor or a list's "+", has none; opened again later, it has. Its
  Spotlight is the editor's last tip.
- A screen reader reads both rows and the buttons by name, and the
  character the cursor passes.
- **On the phone a table in the field is its grid**, never its
  Markdown, edited in place (see "In the editor a table is edited as
  its grid"); a grid wider than the field moves sideways with a drag
  on it. For the text around it, it is one block: the cursor and a
  selection never end inside its lines, ◀ and ▶ go into its first or
  last cell, what is typed right before or after it goes on a line of
  its own, and deleting any of it deletes all of it, asking first while
  its cells hold words. While a cell is edited the bottom row is hidden,
  the cell's own field shows its words, and Show as table sits on the
  bar above the keyboard while the cursor is in a table shown as lines.
- **A table's ⋮ sits at its top right corner** (obf-1050), beside its
  header row on a card, in a text being read and on the editor's grid,
  and full screen above the table beside the bar that moves it sideways,
  Copy staying in the page's bar too. It takes its own tap, never the
  card's, the search card's or the table's, and opens at the finger.
  It offers Copy everywhere, the same as the full screen's; on the
  editor's grid also Move up and Move down, past the block of the text
  above or below, greyed with none there, and Delete table, asking once
  while its cells hold words. A block is a table, a code block, a
  heading, the items of a list one under the other, a quote's lines or
  a paragraph's; blank lines part them and stay where they are. A table
  that would stand right above a paragraph or another table, which would
  read as its rows, gets a blank line between. Each is a step of Undo.
- **A tap on a cell of the header row sorts a table being read** by its
  column (obf-1052), on a card, the search card's included, in a text
  being read and full screen where it is only read: ascending,
  descending, then as written, an arrow in that cell saying which, in the
  view only; the Note's text stays as written and the order is forgotten
  on leaving, a card's first 10 rows being the first in it. The tap is
  the header row's alone, never the card's, the search card's or the
  table's, which the rows under it keep. Full screen where the table is
  edited, a tap on a cell of the header row edits it, the column's letter
  offers under Show sorted ▸ Show sorted ascending, Show sorted
  descending and Show as written, and putting in, moving or deleting a
  row or column shows the rows as written again. On the editor's grid
  the column's letter offers under Sort rows ▸ Sort rows by this column
  and Sort rows by this column, descending, which rewrite the rows in the text, each a step of Undo, none when the
  rows already stand so.
- **A long press on a cell selects cells** (obf-1054), on the grid in
  the Note editor's field (not on the cell being edited, whose field
  selects its words), in a Section's text being read and full screen;
  never on a card, which selects no text. Dragging on without lifting
  the finger selects the rectangle from that cell to the one under it,
  in the rows as they show; a drag that does not start with a long
  press still moves a wide table sideways, and a tap still opens, sorts
  or edits. On lifting, the toolbar above the cells offers Copy, and
  Search with the words of one cell. Copy puts the cells on the
  clipboard as a table of their own, as the table's Copy does, the
  first row its header row, also a single cell. A tap, Copy, a scroll
  of the page or of the rows and the table moving sideways end the
  selection, the tap doing nothing else. A selection that starts
  outside a table runs through its cells as through the rest of the
  text and copies as text.

### Checklist and Shopping List Lines

- **In and out** sit on the bar above the keyboard, as the format bar
  does, and put the Line with the cursor under the Line above or back
  at the top.
- **A Checklist, Shopping List or Tally is one card** in the Note
  editor (obf-1071, C14): its name as the heading, then its Lines and
  **"Add a Line"** (of a Tally its setup, as before), in the card of
  the name and in no panel of its own, without the kind as a second
  heading. "Add a Line" is a field under the Lines, where this Device
  may add Lines; a tap puts the cursor into a new empty Line after the
  open Lines, on a Shopping List before the struck ones, as Return
  would, or into the empty new Line there is already, and is no step of
  Undo. This
  replaces "No Add Line button and no Add field" of Both apps on the
  phone; Return still makes new Lines.
- **Its description shows only after "Add a description"** (obf-1071),
  a small text button under the name where this Device may change the
  text and the description is empty: a tap opens its field, the cursor
  in it; left empty, the button comes back. A description that is there
  shows as before, folded to one line until a tap. A Session keeps its
  description field with its hint.

### The bar

- **One bar on every phone page.** On a phone the buttons sit on their
  own row and the title on the line under them; on a wide screen the
  title sits beside them.
- **An action goes into ⋮ only when the bar is full.** A page lists its
  actions in a fixed order; the bar shows as many as fit at the width it
  has, the rest overflow into ⋮, and ⋮ exists only when it holds
  something. ⋮ is last. The one exception is the Areas list's ⋮
  (obf-1089, obf-1096), which stands whenever nothing is searched, also
  with fewer than two Areas, and gives way to Done while rearranging:
  Rearrange with two Areas or more, Import… where a folder can be
  picked, Running last; and the
  Agenda's ⋮, which always holds Running last (obf-1078,
  `BarAction.inMenu`) and so always stands, on a wide screen too.
  Nothing adapts to use. Settings, Backups and
  Sync never go into ⋮; they stay behind the Profile picture. A page
  gives such an action as a `BarAction`; the bar counts a button 48
  wide, 1.3 times with Fat fingers (#224).
- A destructive action has no special place: last in the order, on the
  bar when there is room; the offer to undo at the top is the guard, or
  for what goes for good its confirm dialog.
- An action in ⋮ shows its icon and its words, as it does on the bar.
- Labelled buttons (Save) and buttons that open a menu of their own
  never overflow.
- **A list's search bar scrolls with it** (#336, Q749): on the Areas
  list, an Area and a Topic the search field and its chips scroll away
  with the list and come back over it, the list staying still, once it
  is scrolled up a little; half shown, it snaps. While the field has the
  focus or a search it stays; otherwise no edge under the bar cuts the
  list. The bar sits on a band (#351, obf-1041), and the page's top bar
  takes the band's fill, the pattern's (marks') tone (obf-1081), set off from the
  page's ground in the light and the dark look, so top bar and band read
  as one header with one line under the band only; shown over the list,
  and with the list pulled down, the cards go under it. The pages whose
  search bar stands still over what they list, the Settings, File
  into…, Merge into…, Add to Note… and Link to…, set it on the same
  band, their top bar of its fill, with the line under it, so what
  scrolls goes under the line and no edge in the page's colour cuts it
  (obf-1088). Pages without a search bar keep their top bar in the
  ground's colour.

### The big screen

- **From 700 dp wide the phone app takes the desktop's layout**
  (obf-1148), on a tablet, an iPad, and a phone held sideways that wide:
  **the sidebar** on the left, 240 wide, with the Agenda, Unsorted with
  its count as the desktop's, All Notes, Frogs, Areas, under it each
  Area with its Topics, then Tags and the Trash, marking the place
  shown; **the list** beside it, the phone's own pages, each as it is
  on the phone with the room left of the window. Below 700 dp nothing
  of it shows and the phone's layout stands as above. It replaces
  Navigation's folder and the swipe between the Agenda and the Areas
  list: the Agenda's bar has no folder, the two pages do not swipe, and
  the sidebar's Agenda and Areas move between them.
- **A tap in the sidebar shows its place anew**: the pages over the
  Agenda or the Areas list close, as the desktop's sidebar shows a
  place. Areas shows the Areas list, with its search, row of tags, ⋮
  and "+"; an Area and a Topic open over the Areas list, a Topic over
  its Area, so back leads up the levels as on the phone; Tags opens the
  tag tree, and a tag tapped there the Areas list searching for it. The
  sidebar stays put while the list's pages come and go beside it. Pages
  that are no place (Settings, Capture, the Routine editor and the like)
  cover the window as on the phone.
- **A closed locked Area lists no Topics in the sidebar**, its padlock
  at the end of its row; a tap asks for the PIN first, as its row in the
  Areas list does. While the App Lock is locked the sidebar is hidden
  with the Agenda.
- **A Note opens as a dialog over the list and the sidebar**, as the
  desktop's Note dialog: at most 840 wide, 32 from the window's edges,
  8 from the sides of a window under 700 wide and from the top and
  bottom of one under 600 high, so a phone held sideways keeps room
  over its keyboard.
  Back, its bar's arrow and a tap beside it leave it, keeping what was
  typed. The editor in it is the phone's.
- **Turning the screen across 700 dp keeps the place shown**: the same
  pages stay, the sidebar comes or goes beside them, and a Note open
  stays open with what was typed, as a dialog or as the page it opened
  as.

### Settings

- **Every Settings section is a card with its title on top** (obf-1085,
  C28): on the phone and the desktop each section's rows sit on one
  card under its title, Look, Units and Complexity included; a search
  shows the card with only its matching rows.
- **The phone's Tips card** (obf-1060, C3): the switch "Show tips", on
  by default and kept on this Device, and "Show tips again" under it.
  Off, no screen shows a Spotlight, a new one neither, and none is
  counted as shown; "No more tips" on every tip's card turns it off at
  once. "Show tips again" brings every tip back and turns the switch on.
- **The phone's Imports are actions, not settings** (obf-1096): a
  folder and Google Keep sit on the page Import… in the Areas list's ⋮
  opens, each a row that shows its running Import and opens its page
  again. Settings has no Import section; a search there for the Imports
  shows the line "Import… is in the Areas list's ⋮", which opens the
  Areas list. The desktop keeps File ▸ Import….
- **An Import of a source imported before asks once** (obf-1078):
  after the folder or the Keep export is picked and listed, and before
  any of its files is read, when an earlier Import of the same picked
  location made Notes of any of its paths, the phone asks "Fill in the
  Import of <date> into <Area>?", Fill in and Import as new stacked
  full width, Cancel below, which writes nothing and closes the
  Import's page. Fill in adds only the items whose path that Import
  never made into its Area, and its page and announcement say how many
  went in and how many were imported before; with none missing, "Nothing
  new: 340 already imported", without Open the Area. Import as new, as a
  first Import, makes a new Area with every item. A name alone never
  matches. On the iPhone the pickers hand no stable location yet, so
  the question does not come there and a second Import makes a new Area.
- **A search field sits at the top of Settings** (#282), under the bar,
  staying there while the page scrolls, on the band of the list pages'
  search bar (obf-1088); nothing focuses it on opening.
  It filters to the settings whose label or detail line holds every
  word typed, in any case, in the app's language, a setting without a
  label of its own by the names of its options, each under its
  section's heading; a heading that matches shows its section whole.
  The intro line goes while a search runs, and a search that finds
  nothing says so in the page ("No setting matches “…”."), not at the
  top. ✕ in the field, or emptying it, shows everything again.

### Alarms and notifications

- A Nudge is a normal notification with the same snooze choices; tapping
  it opens the Nudge screen with the three oldest Unsorted cards and
  "+x".
- **A tap on an Occurrence's notification, its ring's or its
  Heads-up's, opens the Agenda with that Occurrence marked** (obf-1039,
  Q39–Q41), from whatever page the app showed, the Areas list too,
  within 500 ms, whether the app was running, in the background or not
  started; Capture and the full-screen alarm stay on top of it, nothing
  typed lost. The Agenda scrolls the row into view, a fifth down from
  the top or a little lower so nothing is cut by the bar, and marks it
  on the find bar's tint in a ring of its colour (tertiary) until the
  app goes out of sight or another tap marks another; Done is its box, Snooze and Not today are in its ⋮. One
  today's rows do not hold is marked where the Agenda lists it: in the
  Missed Summary, as far as its card shows, or on a coming day. One the
  Agenda lists nowhere opens the Agenda with "Already done" or
  "Skipped" at the top when it was Done or skipped meanwhile, and with
  nothing said otherwise. A Masked row is marked as it shows, asking
  for no PIN. Behind the App Lock it is marked after the unlock. A
  Routine or Note gone or in the trash shows the Agenda alone.
- **Every notification tap goes into a trace of its own** (obf-1039):
  the kind ("occurrence", "headsUp", "ring", "nudge", "changes",
  "catchUp"), the Routine's id and the Occurrence's key where it has
  them, whether the app was running, in the background or started by it
  ("unknown" on the iPhone), when, and where the app took it; never a
  Note's text. It keeps the last 10 taps in a file beside the crash
  report, and Send feedback sends it along under "The last notification
  taps", ticked at first while there are any, greyed while there are
  none, as the desktop's clicks.
- **On Android an Occurrence's notification has three buttons** (#269,
  Q650): Done · Snooze · Not today, as Android shows no more. Snooze
  replaces them in place with 10 min, 30 min and 1 h, and Not today with
  Skip and Tomorrow, under "Skip it, or tomorrow at the same time?";
  Tomorrow only for an Occurrence of today or later.
- An Occurrence from a locked Area whose Area is set to Masked never
  shows its title in a notification or on the Agenda, and stays
  checkable.
- **On the iPhone** (#238) the alarm laws differ where iOS decides:
  - There is no full-screen alarm page. An escalated Occurrence rings
    as an AlarmKit alarm, through silent mode and Focus, and AlarmKit's
    own alarm screen replaces the page: its Stop answers Done, and its
    one Snooze waits 10 min, as there is room for one only. Without the
    Alarms permission it comes as a time-sensitive notification.
  - An Occurrence's notification is time-sensitive, in the Quiet Hours
    a normal one without sound (obf-1045), and offers Done, the
    three snoozes ("Snooze 10 min", "Snooze 30 min", "Snooze 1 h"),
    Skip and Tomorrow at once, not Snooze and Not today and then their
    choices (#269); a Re-nag after midnight of an earlier day's
    Occurrence offers no Tomorrow. Each Re-nag is a notification of its
    own in the Occurrence's group.
  - The setup asks for Notifications and Alarms only; exact and
    full-screen alarms do not exist there.
  - iOS rings only what was handed to it beforehand, at most 64 rings
    ahead; every plan, tap and answer hands over the next 64. There is
    no Catch-up Notification; the Missed Summary shows what was Missed
    when the app opens.

## Desktop

### The window

- The desktop window is a classic desktop window (#145): **the menu
  bar** on top (File, Edit, View, AI, Sync, Help); **the
  sidebar** on the left with the Agenda, Unsorted, All Notes, Frogs, the
  Areas with their Topics and the Trash, marking the place shown (the
  tag tree opens from All Notes, obf-1074); **the list** in
  the middle. **A Note opens as a dialog over the window** (#146), one at
  a time, over the list and the sidebar but not the menu bar, which
  stays at hand for it; closing it keeps what was typed: its cross, Esc,
  Alt+Left, the mouse's back button and a click beside it. The Routine
  editor, a Tally's pages and the New Tally page open as dialogs over
  the dialog or the list they were opened from. **A click beside one of
  these dialogs saves as leaving does** (#275): it, Esc, Alt+Left and
  the mouse's back button save the Routine editor, a Tally entry, the
  New Tally page and the Session pages once something changed, and ask
  "Discard changes?" only about a change that can't be saved; Cancel
  leaves without saving.
- **A Note can open in a window of its own** (#287): "Open in its own
  window" beside the Note dialog's cross and in a card's ⋮ and
  right-click menu opens it beside the main window, which stays usable,
  for reading and editing as the dialog does. Several can be open, one
  window per Note: opening it again brings its window forward, and the
  Note dialog does not open a Note that has one. What is typed there is
  kept as in the dialog; its cross saves as closing the dialog does, and
  the Undo slot is the main window's, while its bar has the dialog's
  Undo, Redo and ⋮ for its own Note's steps (obf-1051). A Note going to the trash, deleted
  or merged, here or on another Device, closes its window. The Note
  windows close, saved, as the main window closes or hides into the
  tray. **Each has "Keep on top" above the Note**, off as it opens and
  kept while it is open: the window stays above all others. On Wayland,
  which lets no app ask for that, it is greyed with the reason under
  it. The last Note window closed gives the next one its size, a setting
  of the Device (ADR 0008), never a maximized one's. **A Note window is
  maximized as the system's windows are** (#322), by its title bar.
- **A Note window has a reading view** (#322, obf-1015, Q725): "Reading
  view" at the end of the window's bar, and F9, which also works while
  typing, show the Note to read: its text as it reads (a Note of a kind
  its name as a heading over its description, a Checklist's or Shopping
  List's Lines under them, the struck ones last), its photos, and each
  Section's text under a tear line with its photos. It has no badge,
  field, button, capture line or Keep on top; the button stays, pressed,
  as the way back. Recordings, files, Transcripts, Photo Texts, the Due
  lines and a Tally's or Session's content stay in the Note to edit. Boxes
  and Lines tick as on a card; tags, links, tables and photos open as
  there. What was typed is saved as it opens. The text stands in a column
  in the middle, at most 35 sizes of its text wide (about 70 characters,
  `note_window::measure`), so it grows with Text size. The wheel, Page Up
  and Page Down, Home and End scroll it, and Space and Shift+Space, Up and
  Down while no widget has the keyboard (a Line ticked by keyboard keeps
  Space for itself). Esc, Alt+Left, the mouse's back button, F9 and the
  button lead back, after closing a menu, photo, table or dialog that lies
  over it. It opens off in every new window and stays as switched while
  the window is open; nothing is stored, so the AI port has nothing to
  read or set.
- **A Markdown file opens in a viewer of its own** (obf-1056, Q1–Q7):
  from the command line (`oldbrainfix notes.md more.md`, several at
  once, into the running app, no second app), from "Open with" for `.md`
  and `.markdown` files on Linux, Windows and macOS, offered but never
  made their default, and from File ▸ Open ▸ Markdown file…. A second
  start hands its files, like a capture and `oldbrainfix open`, to the
  running app over a local socket on Linux and macOS and over a named
  pipe on Windows that only the user may open, a pipe run by another
  user being handed nothing; the Finder hands them over as the open
  documents Apple Event (Q12). It is a window beside the main one, one per file, drawn as the reading view
  draws a Note: all of CommonMark with GitHub's tables (as obf-1048 draws
  them), task lists, strike-through and footnotes, six heading levels,
  the front matter as a small table of properties at the top, Obsidian's
  callouts and highlights. Nothing goes into the store or syncs, so the
  AI port has nothing to read or set; it shows the file again as it
  changes on disk. `[[Name]]`, `[[Name|label]]` and `[[Name#Heading]]`
  open Name.md in the same window at that heading, found in the nearest
  folder above holding `.obsidian`, else the file's own, and their
  folders; a name not found is dimmed, and nothing outside that folder is
  read. Back and forward (the arrows at the top, Alt+Left and Alt+Right,
  the mouse's back and forward buttons) go through the files it showed.
  It has the find bar (Ctrl+F), selection with the right-click's Copy and
  Find, Ctrl+Plus and Ctrl+Minus for the text size, and in ⋮ Find,
  Open in another app and Import as Note…, which also stands in its
  bar: it imports by the Import rules and, when the file links to others,
  asks first "Also import the N files it links to?"; imported together,
  their links become App links to each other's Notes, their pictures
  photos. While an Import runs, the bar says so in its place. What the
  Import made is announced at the top of the main window. The viewer
  asks for no folder; its links are found where the file lies
  (obf-1151).
- **Unsorted says how many Notes it holds in the sidebar** ("Unsorted
  7", #279), as the one list that asks to be emptied; no other place
  shows a count, and an empty Unsorted shows none. **An Area with Topics
  folds and unfolds them with the chevron at the end of its row**
  (#279), a click on it selecting nothing; this Device keeps each fold
  (ADR 0008), as lists keep theirs, Areas starting unfolded. A folded
  Area still shows the Topic shown under it, marked.
- **Tags under All Notes' search** (obf-1074): the desktop's Areas
  list is its sidebar, which has no search; All Notes' search, over
  every Note as the Areas list's on the phone, has the row of tags. While
  its field is empty, one row right under it in the band shows the most
  used tags as on the phone (see Phone ▸ Navigation, "Tags under the
  Areas list's search"), at most twelve, and ends in "All tags…", there
  also without a tag; a click on a tag searches All Notes for it, and
  each reads "Search for" its tag to a screen reader. A row wider than
  the band leaves out the least used tags, never "All tags…". Its tags
  are read on a thread of their own while All Notes shows, once for each
  change, never per frame. The sidebar lists no tags.
- **"All tags…" opens the tag tree in the list** (#267): every
  tag by its levels as on the phone (see Phone ▸ Navigation, "The tag
  tree"), each level's arrow unfolding the levels below, kept while the
  app runs; a click on a tag shows All Notes searching for it. Typing
  `#` into a list's search (not the Trash's) shows the tags starting
  with what is typed as buttons under the field, at most twelve, the one
  typed exactly left out; a click fills it in. The desktop has no Area
  Locks, so nothing is left out. **An Area Hidden from AI has a shield
  at the end of its row**, before the chevron, and so has each card
  hidden in its top row, beside the view button; both name it to a screen reader
  and in a tooltip.
- **The window keeps its geometry on each computer** (#278): its size
  and position, whether it is maximized, the sidebar's width and the
  Note dialog's size are settings of the Device (ADR 0008) and come
  back at the next start; a position on no monitor any more opens the
  window centred. The Note dialog is resized by its edges and corners,
  staying centred.
- **On the desktop nothing floats over a list** (#158); every list ends
  with the same room after its last card, so that card scrolls fully
  into view.
- **On the desktop "+" sits at the end of the list's heading** (#146, #161),
  in a list of Notes last in its band's second row (obf-1064) as "+ New Note" in a button's frame (obf-1067), and opens a menu of the same kinds, Note first, each with its icon and
  words. The Agenda's "+" and Unsorted's "+ New Note" open a small menu
  (obf-1141): "Quick note" first, which opens the quick note, then
  "Editor" with the editor's pencil, which opens the Note dialog on an
  empty Unsorted Note nudged in 1 h, as the quick note's Open in the
  editor keeps it, its text taking the keyboard, without "Add a part";
  left empty, the Note is not kept.
  An Area's or a Topic's "Note" opens the Note dialog on a new Note
  filed there, its text taking the keyboard, without "Add a part"; left
  empty, the Note is not kept (obf-1140). The
  pages open as dialogs over the window; the desktop's Session page is
  the Routine editor's, its name first, the count, rule and schedule as
  its panels.
- **Back and forward go through the places the list showed**, as a
  browser does: the arrows at the top of the list, Alt+Left and
  Alt+Right, and the mouse's back and forward buttons; the keys rest
  while text is being typed. Showing a new place forgets what lay
  ahead. **Back and forward show each place as it was left** (#278):
  its search, its person filter and how far its list was scrolled; a
  place shown anew starts with none and at the top. Over a dialog, back leads to the dialog it was opened from, or
  closes it; over a photo or table full screen, it closes that. They
  take the place of the phone's swipes.
- **The path beside the arrows leads up the levels**, such as "Areas ›
  Household › Repairs"; a click on a level shows it, "Areas" leading to
  All Notes.
- **A list of Notes has one header band** (obf-1064, C7, as the phone's
  band, obf-1041): All Notes, Unsorted, an Area, a Topic and the Trash
  start with one band on the search band's fill with its line under it
  (`theme::search_band`), the cards scrolling under the line. Its first
  row holds back, forward and the path, the place shown last and large
  as the list's title; its second the search field, then the view
  switch, "Show Notes by…" and "+", in an Area or a Topic ⋮ last
  (obf-1065, C8, see Menu bar and keys), all at one height, the Trash's
  the field and "Empty the trash now" (obf-1095). The view switch and "+" are icon and word in the frame
  "Show Notes by…" has, "View" and "New Note" (obf-1067); to a screen
  reader and in its tooltip the switch says what a click does, and
  "+" with a menu says "Add here". The search field is as tall as those buttons, its text
  a card's text's size, the magnifier inside it at its left and ✕ to
  clear it inside at its right while something is typed (obf-1063).
  A person the filter keeps is a chip with ✕ in the second row, after
  "Show Notes by…", a click taking them out; there is no row
  "Only Notes by". Too narrow for them beside the field, the buttons go
  to the line under it. The band, its words, icons and heights, grows
  with Text size (`BandSizes` in `app.rs`); for the band this replaces
  Both apps ▸ Look, "The text has a size of its own", where bars and
  headings keep their size. The other places keep their arrows and
  path over their own heading.
- **"Empty the trash now" asks once with the counts** (obf-1095): it is
  greyed while nothing in the trash can be deleted, counted beside the
  window whenever the log changed while the Trash shows, and while the
  trash is being emptied. Its question names what goes, "Delete 128
  Notes and 4 Routines for good?", a count of none left out, with
  "They can't come back.", Cancel and **Delete for good** on the error
  colour. The trash is then emptied beside the window, the line under
  the menu bar and the page Running showing it; the Trash shows what
  is left once it is done, and a failure is announced at the top.
- **"New Note" stays whatever filter is on** (obf-1070, C13): in All
  Notes, Unsorted, an Area and a Topic, "Show Notes by…" never hides
  it. A Note the filter hides is made and opened as usual: the quick
  note saves it, a Checklist or Shopping List opens in the Note dialog.
  Back in the main window's list, not over a dialog, the top says
  "Made, hidden by the filter" with the button "Show"; a click on it or
  on the words clears the list's person filter. It shows before every
  other announcement and goes by itself 10 seconds after it first
  showed, or once the list shows another place or its filter changes.
  With "Me" in the filter nothing is said, nor for a Note the quick
  note's choices filed elsewhere, which this list would not show.
- **A restored Backup's speech model is offered at the top** (obf-1098):
  once "Backup restored." is clicked away, in the main window, the top
  says the speech model used before is not on this computer yet, with
  **Download**, or **Use a file of my own** for a model taken from a
  file, and **Not now**. A click on the words or the first button
  fetches it; the line hides while the model is fetched and goes once
  that model is in use, so a failed download offers it again. The file
  dialog left without a file keeps it; Not now drops it for good. A
  model the Backup names that is in use here already is not offered.
- **The Agenda's bar on the desktop holds All Routines and Reminders and
  New Routine or Reminder** (draft, #196): no Calendar and no Profile
  picture; the menu bar leads to the rest.
- **Quiet today is a button last in the desktop Agenda's bar, no ⋮**
  (obf-1066), in place of Both apps' ⋮ for it: its bell and words in
  the framed look of a list band's buttons. While today is quiet it
  reads **End quiet day** in the same place, its right edge and the
  buttons before it where they were, and ends it as End on the line
  "Quiet day until midnight" does; the line stays on top. The bar's row
  is as tall as the button, all of it centred in it. Too narrow for it
  beside the other buttons, as with Bigger text and icons in the
  smallest window, it goes to the line under the bar, at the right.

### Menu bar and keys

- **The menu bar holds what belongs to the window** (obf-1065, C8):
  File, Edit, View, AI, Sync and Help. It has no Area and no Note menu;
  an Area's or a Topic's actions are in the ⋮ of its list's band, a
  Note's in its card's ⋮ and right-click and in the Note dialog (its
  badges, its buttons and its bar's ⋮, which holds History). Its menus
  keep to seven entries (#321, obf-1014): **File** holds New Note
  (Ctrl+N; the quick note, while an Area or a Topic is shown the Note
  dialog on a new Note filed there, obf-1140), Quick note, Import…, Open ▸ (Markdown file…, Pack…, AXT file…
  (obf-1146), A Backup…), Export as vault… and Quit; **Edit** Undo, Cut, Copy, Paste,
  Find in Note (Ctrl+F) where it opens the Note's find bar, else Search,
  and Settings…; **View** Go to ▸ (Agenda, Unsorted, All Notes, Frogs,
  All Routines and Reminders, the one shown marked), Running, Detailed
  views, Compact views, Theme ▸ and Text size ▸. Attach a file… and
  Save as Markdown… are the Note dialog's, under Add and Share. The
  tray's menu has Open Old Brain Fix, Quick note, which opens the
  capture popup, and Quit.
- **Every menu entry acts on what it names and closes its menu**, also
  when picked by keyboard or screen reader. The keys (Ctrl+N, Ctrl+F,
  Ctrl+M for File into…, Ctrl+Shift+M for Merge into…, Ctrl+,; Cmd on
  macOS) stand beside the menu bar's entries where it has them and work
  without any entry, while typing, over the Note dialog too, not while
  another dialog is open; Ctrl+M and Ctrl+Shift+M act on the Note in the
  Note dialog, else on the cards selected in the list (#148, #273), and
  Delete and 1 to 3 on the cards selected (#274). Ctrl+F searches the list shown where it has a search field
  (All Notes, Unsorted, an Area, a Topic, the Trash, whose search also
  finds its Routines, Reminders and Frogs by their Note), else All
  Notes'; over the Note dialog and in a Note's own window it opens the
  Note's find bar (obf-1047); over the Settings dialog it searches the
  Settings (#282). On the desktop the menu bar leads to Settings, Backups and
  Sync in place of the Profile picture, and only Help ▸ About Old Brain
  Fix to About, which the Settings do not show (#325, Q728).
- **View ▸ Text size** (#326, obf-1019): Normal, Larger and Largest,
  then Larger text (Ctrl+Plus, Ctrl+= too) and Smaller text
  (Ctrl+Minus), each greyed at its end; the keys work as the menu bar's
  others do, and egui's own zoom keys are off (Both apps ▸ Look).
- **View ▸ Running and a click on the line under the menu bar open the
  page Running** (obf-1078, C21): never greyed, a Page dialog as the
  Settings, listing each job beside the window (a sync with a Device, an
  Import, the Transcripts, Photo Texts and Attachment Texts being read,
  the speech model's download, an Area being deleted, the trash being
  emptied, a Pack being
  opened, an AXT file being read (obf-1146), the Agenda's first read) with what it does, how far it is when
  known ("120 of 340 Notes"), "Since 14:32", and Abort, which stops it at
  its next safe point and reads "Stopping…" until then; with none, "Nothing
  runs in the background." An Import stops before its next Note and keeps
  the Notes it made; the page says "Stopped: 120 of 340 imported" and the
  top "Import stopped.", what it made behind the click. Deleting an Area
  stops before its next Note, the Area staying with the Notes not moved.
  A sync round or answer, a Transcript or a Photo Text under way is not
  cut and ends first; then the automatic syncs pause (a paired Device
  asking is told to come back later) until Sync now, or the other
  recordings, photos and files wait, until Go on on the page or the next
  start. The Agenda's first read has no Abort, nor has emptying the
  trash, which deletes all of it at once or nothing. What Abort stopped stays
  listed under "Stopped" while the app runs, the newest first, at most
  ten. The line takes the click a little above and below itself, never on
  the menus.
- **Attach a file…** (#267) picks files of any kind and does with them
  what a drop does (see Capture): under Add in the Note dialog onto its
  Note, in a card's Add ▸ onto the card's Note (obf-1014).
- **An Area's menu is the same in the sidebar and the band** (#279,
  obf-1065, obf-1014): a right-click on an Area in the sidebar and the
  ⋮ last in its list band's second row ("More for this Area" to a
  screen reader) offer New Topic (in Advanced), **Name, colour and
  icon…** (F2), **Share with people…**, Move ▸ (Move up with Alt+Up,
  Move down with Alt+Down), Export ▸ (Export as Pack… and Export as AXT
  file…, obf-1145), the switch Hidden from AI
  (#267) and Delete, seven entries. The right-click leaves out what this
  Device may not do there, the band's ⋮ greys it, and greys all of it
  while a dialog lies over the list, the Note dialog included, so that
  none acts on a place behind it; its keys rest there too. A Topic's
  right-click and band ⋮ ("More for this Topic") offer Rename (F2),
  Export as Pack…, Export as AXT file… (obf-1145) and Delete. F2 rests in a text field and while a menu
  is open. Name, colour and icon… is one dialog: the name, the colours
  and the icons, each marked as it is picked, kept with Save or Enter
  and dropped with Cancel. Share with people… is offered to whoever
  manages the Area and to everyone in a Shared Area: it lists the
  Area's people with their roles first, where it is shared, a Manager
  changing them and taking them out there, then for who manages it the
  role, Show code and "Someone not nearby"; a Member sees the people
  and Close. Move up and Move down move the Area shown one place in the
  person's own order (#280), for whoever is in it, greyed or left out at
  the ends; they rest while a text field has the keyboard, so in a
  Checklist's Line they move the Line instead (#283).
- **The sidebar's Areas are dragged by their rows** (#280, Q676): a
  line shows the gap the Area goes into; let go where it stands or
  away from the Areas, it stays and nothing is written. A card dropped
  on an Area's row files there instead.
- **The AI menu leads to every AI setting** (#279, #335): its main
  switch "AI (experimental)" (#334) comes first, off by default and
  also at the top of the AI settings; only while it is on are there
  Ask… (Ctrl+K, #267), the Ask panel and its provider's card, and does
  the desktop answer the phone's questions. The AI port works either
  way. Ask… shows the Ask panel, AI settings… opens the page with every
  AI setting, and Review the AI Task shows the review of the AI Task,
  never greyed (obf-1077): with no change yet it says "No changes yet",
  and while the port is open "Close the port" stands in for Accept all
  and Reject all, under the changes so far.
- **The AI settings are one page** (#335, Q748): a Page dialog like
  the Settings, with "AI (experimental)" on top, under it the
  provider's card only while it is on, then the AI port's card with its
  token's client config to copy. The Settings keep one row "AI…" in
  its place, which opens the page with the arrow back to the Settings;
  the Settings' search finds the row by the words of every setting on
  the page. The announcement of the open AI port opens the page at the
  port's card. The standing line at the top says "AI port open" or "AI
  Task waits for review", no word of a pause: sync goes on (obf-1076),
  the drafts sent nowhere until Accept. On every page and dialog it
  offers "Review (3 changes)" while the AI Task holds changes, which
  closes the dialogs over the list and opens the review, and with
  none yet "Close the port" (obf-1075, C18); the count is the one
  value an announcement holds, replacing "Values never go into an
  announcement" for this line. A Note window shows the line alone.
  The review marks each change a
  later edit synced in touches: one of the user's other Devices' as
  staying, another person's, in the warning colour, as written over by
  Accept.
- **The Ask panel is a place in the list** (#267, ADR 0051), reached
  by Ask… and Ctrl+K, which also work while typing and over the Note
  dialog and put the cursor into its question field. Under its heading
  a line names the provider and the model and says that only what the
  model finds and reads is sent, never what is Hidden from AI; while
  the provider cannot be asked, a line under the question field says
  what is missing in the error colour with "AI settings…", which opens
  the AI settings at the provider's card, and Ask cannot be pressed
  (#317). Enter or Ask (Ctrl+Enter) asks; while it waits, a status line
  under the question says whom it asks and how many Notes were read,
  and Stop puts the question back into the field. **The questions stand
  as a thread above the field, the oldest first** (#315), each with
  Edit, greyed while a question runs, which puts it into the field with
  the cursor at its end to be asked again after the whole thread, then
  its answer as a formatted text whose links open their Notes, "From
  these Notes" and "Also read" with each Note's title as a link, and a
  row of Copy text, Keep as Note, then "Kept as a Note" opening it,
  Add to Note… and Save as Markdown…, the desktop's Share to an app
  (#316): Copy text and Save as Markdown… hand on the answer alone as
  Markdown with HTML beside, the file named after the question; Add to
  Note… adds the question and answer, as Keep as Note keeps them, to the
  Note picked as a Section, then "Added to “…”" opens that Note; Copy
  and Save only for an answer with words, none of them under a question
  not answered; the question asked,
  or the last one not answered, ends the thread, which scrolls to it
  once it is asked and once it comes back. Up in the empty field
  fetches the last question. A question not answered goes back into the field, and
  its cause and fix stay under it until the next is asked, "Details…"
  opening them with the provider's message and Report (#317). A Note
  opens in the Note dialog over the panel, which stays. A follow-up is
  asked after the earlier answers; Start over drops them, and so does
  showing another place. While an AI Task waits, a line says that its
  changes wait for review and stay on this computer until decided,
  beside Review. The provider's
  card in the AI settings sits before the AI port's: the protocol as
  radio buttons, the base URL only for an OpenAI-compatible provider
  with its error under it, the model, the API key hidden as typed, and
  that the key stays on this computer; for an OpenAI-compatible provider
  then "Let the model think", off by default, with its line; last, for
  either protocol, "Models tried with Ask", each model with its size on
  disk, the memory it needs, how it did and its licence, as text without
  links (#318).
- **Edit ▸ Undo and Ctrl+Z take back the last delete, check-off,
  Skip or Tomorrow made in the app (#269), filing or Merge** (#268), the
  desktop's offer to undo, with no announcement: Edit ▸ Undo is first in its menu and greyed while
  nothing waits; Ctrl+Z works over the list and the Note dialog but not
  in a text field, where it undoes the typing. Only the last one waits;
  cards selected and moved to the trash or merged together come back
  with one (#273). Over the Note dialog and in a Note's own window they
  take back the Note's last step instead (obf-1051), unless the window's
  undo came after it, and Ctrl+Shift+Z and Ctrl+Y outside a text field
  are its Redo.
- **The keys work the card lists** (#274, Q657): over a Note list's
  cards, with no dialog, menu or Note dialog on top and no field or
  other widget holding the keyboard, Up and Down select the next card
  alone (with nothing selected Down the first, Up the last), Shift+Up
  and Shift+Down run the selection on from where it started, Ctrl+A
  selects every card, Enter opens the Note of the card reached last,
  Delete moves the cards selected to the Trash asking nothing (Ctrl+Z
  undoes it), and 1, 2 and 3 make them Important, Normal or
  Unimportant; in the Trash Delete and the digits do nothing. Down in
  the search field goes to the first card found. The card reached
  scrolls into view; after an action, which ends the selection, Up and
  Down go on from it while it still shows, else from the top. Sorting Unsorted is Down once, then Ctrl+M,
  type, Enter for each Note, the first card left selected after each
  (#313).
- **Keys finish what is typed** (draft, #196, #277): Ctrl+Enter (Cmd on
  macOS) saves the quick note's Note and the text being edited in the
  Note dialog, while Enter alone starts a new line; in the capture popup
  Enter saves and Shift+Enter starts a new line (Q664). Ctrl+Enter runs
  the main action of the dialog on top, the button on the accent (Save,
  Create, Send, the Routine editor's, a Session's and a Tally's pages
  included), and only while it may be pressed; Merge into… merges into
  the first Note found once something is typed, File into… takes its
  first row as Enter does; a question whose answer deletes or discards,
  or opens a file that could run a program (#267), keeps Ctrl+Enter
  from it (Q665). The first field of a dialog takes the
  keyboard as it opens, where it has one (the Routine editor's text, else
  its When line, a Tally's pages, Send feedback, a person's name for me);
  the Page dialogs of sections (Settings, AI settings, Profile, People,
  Backups, Sync, Pair) open without it. Alt+Left rests in that field as in any; Esc and
  the mouse's back button leave. In the quick note and the
  popup Ctrl+1, Ctrl+2 and Ctrl+3 save with the Nudge buttons' 10 min,
  30 min and 1 h while those show, named in the hint under them; the
  list's 1 to 3 take no Ctrl and rest while a dialog is open. Enter in
  a one-line field of a dialog takes it; Ctrl+B and Ctrl+I make bold
  and italic while the Note's text is edited.
- **Esc closes what lies on top** (draft, #196): a menu, a photo or
  table full screen, a Note's find bar (obf-1047), the top dialog, the quick note keeping what was
  typed, a Note window's reading view (#322), and over the list without a
  dialog the cards selected.
- **The keyboard stays in the dialog on top** (draft, #196): while the
  Note dialog is open, Tab never walks onto the list or the sidebar
  beneath it, as the pointer does not.

### Mouse

- **A click is the primary action** (draft, #196): on a sidebar row, an
  Agenda row, a Frog or a Tally entry it opens the thing, on a box it
  opens the box's menu, and it never opens any other menu; there is no
  long press. On a Note card it selects (see Cards and selection).
- **A right-click is the desktop's long press** (draft, #196): it opens
  the same menu as ⋮ where there is one, on a Note card, an Occurrence
  row and a Checklist Line's words, and the thing's menu on an Area, a
  Topic, a Frog, a Tally entry, a table and a photo, and in a text field
  the field's menu (#253, see Text editor and Lines). Like the long
  press, it never changes data by itself.
- **Hover only shows** (draft, #196): what a click opens turns the
  pointer into a hand, an icon-only button gets its rounded background
  under the pointer and names itself in a tooltip; nothing opens and
  nothing changes on hover alone.

### Dialogs

- **The Settings, the AI settings, About and everything under Sync
  (Sync, Pair a Device, People, Profile, Backups) open as dialogs over
  the window**, never as
  views replacing the list, and no button on the main window leads to
  them. A dialog opened from another has a way back to it; the cross,
  Esc and a click beside it close it. While a dialog is open,
  announcements show at its top.
- **The phone's pages are dialogs on the desktop** (draft, #196): an
  editor with several fields, the Settings and Sync's pages open as
  dialogs over the window, with no top bar and no Save in one, in place
  of the phone's full-screen page with Save in its top bar; the phone's
  bar and its ⋮ overflow (see Phone, The bar) do not apply.
- **The control may stand beside its text** (draft, #196): unlike the
  phone, an on-or-off setting is a square checkbox (`theme::checkbox`)
  with its words beside the box, its detail in a small weak line under
  it, in place of the phone's switch on its own line. A short single
  choice is a wrapping row of choice chips (`app::chip_row`, the weekday
  chips' look, `design::Chip`), a radio group of radio buttons to a screen reader, Tab
  and the arrows moving between them, Space or Enter picking one; a
  long one is a column of radio buttons (`app::radio_column`), as in
  Both apps (#226); the Sync dialog's "Bundle for",
  a dropdown beside its words (`sync_view.rs`), does not follow that
  yet.
- **The question Hidden from AI is a dialog** (#267), asked as on the
  phone (Phone ▸ Dialogs and pages, "Something Hidden from AI is not
  given away"): before the Send dialog
  and the Share dialog open (Share with people… on an Area shared
  already opens with its people, the invite asking instead,
  obf-1014), and when File into…, a drag onto an Area,
  Merge into… or Export as Pack… is refused, with Cancel and "Show to
  the AI" on the accent, which takes the marks off and does again what
  was asked; a Note hidden by another Area's mark only names that Area,
  with Close, and its mark comes off in the Area's menu. A Merge refused
  keeps the cards selected, and nothing waits for Ctrl+Z.
- **A Pack's tree is a dialog on the desktop** (#165): "Tick all" and
  "Untick all" stand over the tree under its size, in place of the
  phone's bar; every box, "Add my Profile" and "Claim as mine" too,
  stands beside its words; Save… saves the file where the user says, in
  place of Share…; "Add … to People" takes the person in and keeps the
  dialog open.
- **An AXT file opened is a dialog on the desktop** (obf-1146): File ▸
  Open ▸ AXT file…, a drop on the window, `oldbrainfix records.axt` on
  the command line and "Open with" open it, a later start handing it to
  the running app; it is known by its content (Avro's header) or by its
  ending `.axt`, and a drop of one opens it rather than attaching it.
  While it is read beside the window the line under the menu bar says
  "Opening the AXT file…" and Running lists it with Abort. The dialog
  holds what the phone's page holds; under "Take them into" the places
  are a column of radio buttons, Unsorted first and picked, then each
  Area and its Topics ("Health / Running"), and Cancel takes nothing. A
  refused "Take it" keeps the dialog and its ticks. A later file of the
  same sender (obf-1147) lists its updates and deletions among the
  records to take, ticked, each with the Note it changes under it
  ("Updates “Dentist”", "Moves “Dentist” to the trash"); one whose Note
  the user changed since is greyed and unticked with "Kept your
  change"; one that may not change its Note, and one the Note has
  already, stand greyed under "Not taken" with why ("May not change
  “Dentist”", "“Dentist” has this already"), so the same file opened
  again takes nothing. "Take it" passes the ticked changes along, says
  "n records taken" when changes were among them, and "Nothing
  changed: these records were taken before." when it made and changed
  nothing; refused, the top says that this Device may not put them
  there or may not change a Note they update. The app is offered
  for `.axt` (`application/vnd.axt`) under "Open with", never as its
  default: on Linux the launcher's MimeType with a mime type of its own
  (`io.github.paxel.oldbrainfix.xml`, by the ending only), on macOS the
  bundle's document types, on Windows the ending's OpenWithList.
- **Export as AXT file… saves where the user says on the desktop**
  (obf-1145): in place of the phone's share sheet the system's save
  dialog asks, the file proposed under the Note's, the Topic's or the
  Area's name as the phone names it (`Buy milk.axt`), several cards
  selected under the day's (`oldbrainfix-notes-2026-11-25.axt`), and it
  is written beside the window. Backing out writes nothing and says
  nothing; a failure says "The AXT file was not saved." at the top, why
  behind the click; with no Note there the top says so and nothing is
  asked. It asks nothing about Hidden from AI, as Copy text and Save as
  Markdown… do not.
- **A custom Heads-up is a small dialog over the Routine editor**
  (obf-1087): "Custom heads-up" with "How long before" over a number
  field, "From 1 to 720." small under it for hours, the units as a
  `chip_row` under that, starting at minutes, at days on an all-day
  schedule; Cancel and Add, Add greyed until the number fits the unit,
  Enter in the field adding as Add does. Esc and a click beside it
  close it and add nothing, the editor staying open under it; back
  does nothing while it shows.
- **An Import of a source imported before asks once** (obf-1078): after
  File ▸ Import… picks a folder or a Google Keep export, or the
  Markdown viewer's Import as Note… is asked, the Import lists the
  items' paths beside the window, and when an earlier Import of the same
  folder, zip or file made Notes of any of them, the main window asks
  "Fill in the Import of <day> into <Area>?" (coming forward for the
  viewer) with Cancel, "Import as new" and "Fill in" on the accent.
  Fill in puts only the items never made into that Area, and "Import
  finished." says behind the click how many went in and how many were
  imported before; with none missing the top says "Nothing new to
  import.", behind the click "Nothing new: 340 already imported.", and
  no Area is made. Import as new makes a new Area with every item, as
  a first Import does. Cancel, Esc and a click beside it import
  nothing and say nothing; another dialog in its place puts the
  question off until it closes. A first Import asks nothing.
- **A search field over what it lists sits on a band** (obf-1088, as
  the phone's): the Settings', File into…'s, Merge into…'s, Add to
  Note…'s and Link to…'s field, and the main window's over a list of
  Notes (there the whole header band, see The window), stand on a band of the marks' tone across the width with a
  separator's line along its bottom (`theme::search_band`), and the
  list starts right at the line, so what scrolls goes under the line
  and no edge in the page's or the dialog's colour cuts it.
- **The Note dialog's buttons stand in four named groups** (obf-1073,
  C16), as in the phone's Note editor, in a Note's own window too:
  **This Note**, the badges under the Note's text (Kind, Importance,
  Area and Topic, Captured, Show on the Agenda, Hidden from AI and the
  others a Note has), with Make it a Tally in Advanced; at the dialog's
  end, after the Note's files, Sections and Linked from, **Add** ("+"
  with Remind me…, Frog and Session…, Voice, Paste, Attach a file…, Add
  a part), **Share** (Send to a person, Copy text, Save as Markdown…,
  Export as Pack…, Export as AXT file…, obf-1145) and
  **Tidy** (Merge into…, Add to a Shopping List, Move to trash in the
  error colour). Each group is a small strong heading, read as a
  heading, over its compact buttons side by side, wrapping; the groups
  stand side by side where the dialog is wide enough, else one under
  another. A group with nothing this Device may do shows nothing, and
  a Note in the trash shows This Note and Restore only. While a
  recording runs, its time and the big stop button stand over the
  groups. The desktop takes no photo, so Add has no Photo.

### Cards and selection

- In the Note lists a click selects the card; **the view button beside
  ⋮ and Edit at the card's top right switches the card** between its
  compact and its detailed view, as a tap does on the phone (#189), and
  the View menu's Compact views and Detailed views are the list's
  switch. Ctrl-click (Cmd on macOS) adds a card to the selection or takes
  it away, Shift-click selects the run from the card clicked last in the
  list's order (#148). The selected cards carry a check mark and a
  thicker outline, and say so to a screen reader; Esc and showing
  another place drop the selection. Up, Down, Shift and Ctrl+A select
  from the keyboard (#274, see Menu bar and keys). A double-click, Edit
  or Enter opens the Note dialog.
- **A card's menu has the Note dialog's groups** (obf-1014): the ⋮
  and the right-click are one menu in both views, **Edit**, Open in its
  own window, then **This Note ▸** (File into…, Importance ▸ Important,
  Normal or Unimportant, Kind ▸ with Make it a Checklist, Make it a
  Shopping List and Back to a plain Note, a list's Show on the Agenda ▸,
  Remove “From …”, Show ticks and Hidden from AI), **Add ▸** (Remind me…,
  in Advanced Frog while the Note has no open Frog and Session… on a
  plain Note, Attach a file… and Add a part, which opens the Note dialog
  with its Amend box), **Share ▸** (Send to a person, Copy text, Save as
  Markdown…, Export as Pack…, Export as AXT file…, obf-1145) and **Tidy ▸** (Merge into…, a Checklist's
  Uncheck all, Move to trash in the error colour), each entry only where
  this Device may do it and a group without one left out. Voice, Paste
  and Add to a Shopping List act on the Note open and stay in the
  dialog.
- **A selection is acted on from a card's right-click** (#273, Q656):
  File into…, Importance ▸, Merge into… (on an Unsorted Note's card
  picked as a capture picks its Note, #308) and Move to trash, each only
  on a card this Device may change, and Share ▸ Export as AXT file…,
  which writes the cards selected into one file (obf-1145); a card in the Trash has no ⋮, and
  its right-click offers Restore alone. Remind me…, Frog and Session…
  (#281) are only for who may set them up (ADR 0013) and for that card
  alone, selected or not. On a selected card each acts on all
  the cards selected, on another on that card alone, as File into…
  does, and Ctrl+M, Ctrl+Shift+M, Delete and 1 to 3 do the same while
  no Note is open (see Menu bar and keys). Merge into… opens the Note picker with every Note,
  the selected ones included, and a selected Note picked takes the
  others in, all in one go, the list staying shown; Move to trash asks
  nothing. A Note this Device may not change stays as it is, without an
  announcement. An action ends the selection; Ctrl+Z takes back the
  trash or the Merge of all of them at once.
- **In the Note dialog a text that is only read selects with the
  mouse** and copies with Ctrl+C (Cmd on macOS): another person's text,
  the Note's in the trash, a Section's (#183), a Transcript, a Photo
  Text and the History (#255). A drag selects, over its lines and
  formats, which it keeps; a click on a link, a box, a tag or a table in
  it still does what it does. A right-click on it opens Copy, greyed
  while nothing is selected, and Select all, which selects that whole
  text; the right-click and the menu keep the selection. The cards in
  the list stay click-to-select. A Section this Device may edit has its
  own field and format bar there. "Copy text" in a card's menu and
  the dialog puts on the clipboard the Note and
  its Sections as they show, a list's Lines as a Markdown task list
  (#323), with HTML beside it, the boxes as glyphs.
- **Cards move into an Area or Topic together** (#148): dragged onto it
  in the sidebar, which outlines the row under the pointer, or with
  File into… from the card's right-click or Ctrl+M,
  whose dialog names how many go. A selected card takes the selection
  along; one outside it goes alone. A card this Device may not move
  stays where it is; the announcement says so without numbers, its
  details how many moved and how many stayed.
- **An Attachment shows on its card and in the Note dialog** (#267,
  ADR 0047), as on the phone (Phone ▸ Cards): a row with the icon of its
  kind, its name, its size and its pages; a click on it opens a copy
  with the system's program for its type and selects no card. The copy
  is named by the file's hash and its ending alone, and the program is
  started without a shell (`ShellExecuteW`, `open`, `xdg-open`), as for
  a file link. Documents, pictures, sound, video and `.zip` open without
  asking; any other ending, or none, could run a program and asks once
  first, also for the user's own file: "… could run a program. Open it
  anyway?", with the person who added it when another person did, and
  only a click on Open anyway answers it. One
  whose bytes are not here yet, or that no program opens, is announced.
  Its Attachment Text shows dimmed under it, "Attachment Text" over it
  on a full card and in the Note dialog, where it selects
  to be copied, its first three lines on a compact card. The computer
  reads the text of every file waiting, those left to it by a phone
  too, on a thread of its own, a PDF page by page with its scanned pages
  read as photos are; a file that does not read as its kind gets an
  empty text, so it is not read again, and one whose bytes are not here
  waits for them. In the Note dialog each file has Replace Note text,
  Append to Note text, Remove generated text (Ctrl+Z brings it back),
  Read the text again for a file of a kind with text, and "Remove
  Attachment, keep text", asked once; "Attach a file…" under Add
  (obf-1073) picks more. The card's menu has none of these, as it has none for a
  photo.
- **Hidden from AI on the desktop** (#267, ADR 0049): a card's ⋮ and
  right-click under This Note ▸, an Area's right-click in the sidebar
  and the ⋮ of its band have the checkbox "Hidden from AI", the Note dialog its badge, offered
  as on the phone (Phone ▸ Cards) and read as the menu opens; the badge
  says "Off, hidden by its Area" while only another mark hides it.
- **A photo gets out of the Note** (#149): on the desktop a right-click
  on a photo in the Note editor or full screen offers "Save photo…",
  "Copy photo" and "Open in another app" (obf-1014). Saved and copied say
  nothing; a failure is announced.
- **The Agenda's rows are tight** (#208): on the desktop a row's card
  has 6 above and below its content, 6 between cards and 2 between a
  heading's time and words, so the next row starts 52 below, where
  0.3.1 had 84; the box stays 28. The "Eat a frog" rows are as tight.

### Capture

- **On the desktop** the capture popup and the window's quick note
  dialog, which File ▸ New Note, Ctrl+N and "Quick note" in the Agenda's
  and Unsorted's "+" open (#146, obf-1141), are the quick note; while an Area or a Topic is
  shown, File ▸ New Note and Ctrl+N open the Note dialog on a new Note
  filed there instead, as its "+ New Note ▸ Note" does (obf-1140), and
  only files dropped onto it open a quick note that files the Note
  there, with one Save in place of the Nudge buttons. The
  quick note holds (#352) the text, then, once something is typed or held,
  the Nudge buttons and under them Open in the editor, nothing else;
  desktop Settings ▸ Quick note keeps only Nudge buttons. Open in the
  editor saves the Note and opens it in the
  window's Note dialog in place of the dialog; in the popup it closes
  the popup and opens the Note there in the window, brought back from
  the tray. The Note dialog holds Voice, Paste and Attach a file… under
  Add (obf-1073), Add to a Shopping List under Tidy (between Merge
  into… and Move to trash; its lists as radio buttons, the one used last
  at the top; with one list a single entry naming it, "Add to Weekly",
  #276), the badges under This Note and the format bar. Files dropped
  onto the window open the quick note dialog and wait in it above the
  text (#267): a picture as a photo, a
  sound file as a voice recording, any other file as an Attachment, each
  named on its own line; Ctrl+V in the empty field holds a pasted
  picture the same way. **A file dropped onto the Note dialog goes onto
  its Note** while nothing lies over the dialog, a picture as its photo,
  a sound file as its recording, any other as its Attachment; onto a
  Note's own window the same. A file larger than 256 MB, or one that
  cannot be read, is told by its size before it is read, not attached
  and announced; the others are. Leaving the dialog by its cross, Esc,
  Alt+Left, the mouse's back button or a click beside it keeps what is
  typed, nudged in 1 h.
- **A pause ends a recording only where the computer says so**
  (obf-1053): desktop Settings ▸ Capture, a card of its own with "End a
  recording after a pause", kept on this computer only under the
  phone's key, offers Off, the default, which ignores pauses, and 2, 3,
  5 or 10 s as chips. Set as Voice starts, the recording ends
  once the microphone stayed under 2 % of full scale at its peak that
  long after it heard something louder, and is kept on the Note as Stop
  keeps it, the Note dialog staying open; one nothing was said into yet
  goes on, and a level that cannot be told counts as no quiet. The card
  shows only where the computer records.

### Text editor and Lines

- The desktop keeps its own field, not the phone's two rows. The
  desktop's field shows a formatted text's Markdown, except a table,
  which it shows as its grid (see "In the editor a table is edited as
  its grid"), whose column widths are dragged at their borders. As on
  the phone, what is typed right before or after a table goes on a line
  of its own, so the table keeps its rows.
- On the desktop **Tab and Shift+Tab** put a Line under the Line above
  or back at the top, in place of the in and out buttons, and Up and
  Down move the cursor to the Line above and below.
- **Alt+Up and Alt+Down move a Checklist's Line** the cursor is in
  (#283, Q678), as ⋮ Move up and Move down do, which name the keys: a
  Line of a group within its group, a Line at the top past the next one
  with its group, the cursor staying in the Line; at an end nothing
  moves. **A Line is dragged by its handle**, beside its ⋮ wherever ⋮
  is, by the same rule: a line shows the gap within its group or the
  top level it goes into; let go where it stands or away from its
  level, it stays and nothing is written. Into a group and out stays
  with Tab and Shift+Tab. **A Shopping List's open Lines move the same
  way** (#283, Q678 (b)): Alt+Up and Alt+Down, ⋮ Move up and Move down
  and the handle move one among the open Lines as they show; a struck
  Line has no handle nor Move up and Move down, the keys do nothing in
  it, and no open Line goes among the struck ones.
- **The Note dialog's bar has Undo, Redo and ⋮** (obf-1051), before
  "Open in its own window" and the cross, and so has a Note's own window
  before "Reading view", which shows none. They go step by step through
  everything changed since the Note opened: its text, its tables' cells,
  its Sections' texts, the words a removed recording or photo put into
  them (obf-1084; the file stays gone, an empty text stays as it is now),
  its Lines (their checks too, as on the phone), its
  Area and Topic and its badges, each greyed while it has
  nothing to do. Typing is one step until it rests for a second, a new
  word begins, its field is left or saved, or anything else is done; a
  table's menu (insert, move, delete, align, and what ⋮ of a table adds)
  is a step of its own after the cell typed before it. A step is written
  back as an edit writes it, nothing deleted. In ⋮, **Revert all
  changes**, greyed while nothing changed, and, in the Note dialog only,
  **History** (#193, obf-1065), which the menu bar's Note menu had.
  Revert all changes asks once ("Revert all
  changes?", Cancel and Revert) and brings the Note back to how it
  opened; Redo then makes the changes again one by one. In a text field
  Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y stay the field's own, for its typing;
  once Undo, Redo or Revert wrote a field, its own history is forgotten.
  A formatted text stays formatted (ADR 0033), and a Tally or Session made
  or unmade, or a change written elsewhere meanwhile, starts the steps
  again from there. A box ticked in a text is a check-off with its own
  history (ADR 0035), no step; a captured detail removed after its
  question has no way back. A step that also waits for Ctrl+Z, as a Line
  deleted, goes from there once the bar's Undo takes it back. Nothing is
  stored: the steps go as the Note closes.
- **A Checklist, Shopping List or Tally is one card** in the Note
  dialog and a Note's own window (obf-1071, C14), as on the phone: its
  name as the heading, then its Lines and **"Add a Line"** (of a Tally
  its setup, "Fields" over it, as before), in the card of the name,
  without the kind as a second heading. "Add a Line" is a small row
  under the Lines, a plus icon and its words, where this Device may add
  Lines: a click puts the keyboard into a new empty Line at the end, on
  a Shopping List before the struck ones, and is no step of Undo; while an empty new Line waits it does not
  show, and left unused the Line goes as any empty one. This replaces
  "No Add Line button and no Add field" of Both apps on the desktop
  too; Return still makes new Lines. **Its description shows only
  after "Add a description"**, a small weak text button under the name
  where this Device may change the text and the description is empty:
  a click opens its field with the keyboard in it, as Enter in the name
  does; left empty, the button comes back and nothing is written. A
  description that is there shows as before, folded to its first line
  until a click. A Session keeps its empty description field with its
  hint.
- **The Amend box shows only when asked** (obf-1072, C15): the Note
  dialog and a Note's own window show no format bar, "What's new?" nor
  "Add" on their own. A Note just made and opened there, a
  Checklist, a Shopping List or a Note by "+" (obf-1140) or a Note by
  the quick note's "Open in the editor", has no "Add a part" while that dialog stays open;
  opened again later, it has it. Where this Device may amend the Note,
  **"Add a part"** stands last under Add (obf-1073); a click opens the
  box in its place, above the groups Add, Share and Tidy, its field
  taking the keyboard, and opening it is no step of Undo; Add a part in
  a card's Add ▸ opens the Note dialog with the box so (obf-1014).
  The box stays while it holds text and goes once "Add" added
  the Section or the Note is left, which adds what was typed, as ever.
- **A table's ⋮ sits beside its top right corner** (obf-1050) wherever
  a table is drawn on the desktop: on a card, in a Section, in the Note
  dialog's text as it reads and on its grid while edited, in the reading
  view, in the Markdown viewer, and full screen at the end of the bar,
  Copy staying in the bar. The table gives up the ⋮'s room, so it never
  covers a cell. It takes its own click, never the card's or the
  table's, and opens under it. It offers Copy everywhere, as the full
  screen's Copy; in the Note dialog and a Note's own window, where the
  text is edited, also Move up and Move down past the block above or
  below (the phone's blocks, see Phone ▸ Text editor), greyed with none
  there, and Delete table, asking "Delete the table?" once while a cell
  holds words. Move and Delete are each a step of the bar's Undo; done
  on the text as it reads, the text is then edited, its cursor outside
  the table.
- **A click on a cell of the header row sorts a table being read** by
  its column (obf-1052), as on the phone: on a card, the search's
  included, in a Section, in the reading view, in the Markdown viewer
  and full screen where it is only read: ascending, descending, then as
  written, an arrow in that cell saying which and a screen reader
  hearing "Sorted ascending" or "Sorted descending", in the view only;
  the Note's text stays as written, a card's first 10 rows are the first
  in that order, and the order is forgotten once the table is not drawn.
  The click is the header row's alone, never the card's or the table's,
  which the rows under it keep. In the Note dialog's text as it reads, a
  click on the header row edits the cell, as on any row. Full screen
  where the table is edited, the column's letter offers Show sorted ▸
  with Show sorted ascending, Show sorted descending and Show as
  written, and putting in, moving or deleting a row or column shows the
  rows as written again. On the Note dialog's grid the column's letter
  offers Sort rows ▸ with Sort rows by this column and Sort rows by this
  column, descending, which rewrite the rows in the text, each a step
  of the bar's Undo, none when the rows already stand so. **A column's
  menu keeps to seven entries** (obf-1014): on the grid Insert ▸ (to the
  left, to the right), Move ▸ (left, right), Align ▸ (left, Centre,
  right), Sort rows ▸, Delete column and First column as headings; full
  screen Insert ▸, Move ▸, Delete column, Show sorted ▸ and First column
  as headings. A row's menu stays flat.
- **A drag that starts on a cell selects cells** (obf-1054), as the
  phone's long press does: in a text only read in the Note dialog, in
  the reading view, in the Markdown viewer, on the Note dialog's grid
  (not in the cell being edited, whose field selects its words, the
  cell edited being left as the drag starts) and full screen; never on a
  card, whose text selects nothing, nor in the Note dialog's text as it
  reads, a click on which edits it. The cells from the one pressed to
  the one under the pointer, in the rows as they show, are tinted; the
  pointer past the table's edge stays on its last row or column.
  Shift+click selects from the cell selected first to the one clicked,
  one cell where none is, also on a cell of the header row, a link, a
  tag or a box, which then sort, open and tick nothing, as they do for
  a click while cells are selected. A click and a drag that does not
  start on a cell still open, sort, edit and select words as before;
  Shift with the wheel still moves a wide table, keeping the cells
  selected, its bar still moves it too, and a column's border on the
  grid still drags it. Ctrl+C (Cmd on macOS) and Copy in the right-click
  menu of a text only read (obf-1055), and of a cell selected on the
  grid and full screen, put the cells on the clipboard as a table of
  their own, as the table's Copy does: HTML beside Markdown, the first
  row selected its header row, the line marking the first column as
  headings only while that column is selected, each box as it shows,
  one cell as a table of one cell. Where the menu has Find in Note, it takes
  the words of one cell, greyed for more. A click on the table and Esc
  end the selection, each doing nothing else; a press beside the cells,
  on the table's bar too, ends it as well, doing what it does. A
  selection that starts outside a table runs through its cells as
  through the rest of the text and copies as text; Select all selects
  words, never cells.
- **A right-click in a text field opens its menu at the pointer**
  (#253): every field one types into has Cut, Copy, Paste and Select
  all, Cut and Copy greyed without a selection or in a password field,
  Select all in an empty field. With a selection in a formatted text,
  and in the quick note and Add to Note while their format bar shows,
  the menu also has the bar's Bold, Italic, Code, Link and Code block,
  as far as the bar has them (the quick note no Link). A right-click on
  the selection keeps it, and the field keeps the cursor through the
  menu, so nothing it does on losing the cursor happens. Cut, Copy and
  Paste work as their keys do. No spelling entries.
  In the Note dialog and a Note's own window the menu also has
  **Find in Note** (obf-1055, obf-1014) after Paste, greyed without a
  selection: it opens the Note's find bar with the words selected, as
  they read, its lines one line. A text only read there (#255) has Copy,
  Find in Note and Select all, no Paste; in the Markdown viewer the
  entry reads Find, as its ⋮ does. No field outside the Note open
  offers it, and a
  card's right-click stays its ⋮ menu. The find bar has **Search all
  Notes**, an icon button beside ✕, greyed while its field is empty: All
  Notes' search finds its words in the main window.
- **A selection scrolls while it is dragged** (#256): in every text
  field and in the Note dialog's texts that are only read, while the
  button is held the wheel scrolls and the selection follows the
  pointer, and a drag past the top or bottom edge of the view scrolls by
  itself, faster the further out the pointer is. The selection stays
  when its start scrolls out of view.

### Settings

- **The hotkey is recorded by pressing it** (#283, Q679): Record keys
  beside the Hotkey field reads "Press the keys…" and takes the next
  chord pressed, Ctrl, Alt or both (Cmd on macOS) with a letter, a
  digit, F1 to F12 or Space, as the hotkey, which goes into the field.
  While it records every key is its alone, so none of the app's own
  keys works and the hotkey held lets go of its key; another key waits
  for a chord, and Esc, a click elsewhere or leaving the Settings stops
  it, keeping the hotkey. The field stays for typing a chord, also one
  with Super, which the window does not hear off macOS.
- **The Settings dialog is a column of cards** (draft, #196): each card
  embossed as a Note's, its heading strong on top (This computer, Tips,
  Capture, Alarms, Nudges, Units, Agenda, History, Features, …), its settings
  under it, each with its small weak detail line.
- **A search field sits at the top of the Settings dialog** (#282),
  under its heading, on the band of the desktop's fixed search fields
  (see Dialogs, obf-1088), staying there while the cards, the intro line
  first, scroll under its line; nothing focuses it on opening, so Alt+Left and Esc keep
  leaving, and Ctrl+F while the Settings show puts the cursor into it.
  It filters as on the phone (Phone ▸ Settings): every word typed in a
  setting's label, detail line or, without a label of its own, its
  options' names, in any case, in the app's language; each under its
  card's heading, a card left out when none of its settings match, a
  heading that matches showing its card whole. A card drawn as one
  piece (the row "AI…", Quick capture, the speech model, Units, Features)
  is one setting. The intro line goes while a search runs, and a search
  that finds nothing says so in the dialog ("No setting matches “…”."),
  not at the top. ✕ in the field, or emptying it, shows everything
  again; the search stays into Send feedback and back, and the
  Settings open anew with everything.

### Buttons and look

- **Buttons look like buttons** (#218): a rounded rectangle (radius
  8 px) with a tonal fill and a 1 px outline in light and dark, darker
  in light and lighter in dark under the pointer, and pressed or holding
  the keyboard focus a step further with a 2 px ring. The main action of
  a dialog or page (Save, Create, Done, Use, Restore, Continue) is
  on the accent with its text in `on_accent`; Delete, Remove and Move to
  Trash are on the error colour; Cancel and Close in a dialog's action
  row are text buttons without an icon (Design system ▸ Buttons,
  obf-1102); every other button is secondary. The
  text on every fill is at least 4.5:1. Links stay links, entries of a
  menu stay flat, icon-only buttons (⋮, Edit, the view button, the
  format bar) get the same rounded background under the pointer and the
  ring while focused, and badges keep their own look (#192). The look
  lives in `theme.rs`; a screen takes `theme::primary` or
  `theme::destructive` for the two emphasised kinds, and
  `theme::checkbox` (or `theme::square_boxes`) for a checkbox, whose
  box stays square, not round as a radio button.
- **The theme is picked in View ▸ Theme** (draft, #196): System, Light
  or Dark, kept on this computer. The question marks move in parallax
  as on the phone, but the desktop reads no "remove animations"
  setting yet, so its parallax never stops.
- **On the desktop the green "!" comes with a pointer's click**
  (obf-1143): a button clicked with the mouse, of egui's own or the
  design set's, the "+" and icon buttons among them, shows Material's
  priority_high in green above the pointer, 1.6 times its height above
  the click as on the phone, kept inside the window, over the Note
  dialog, the modal dialogs and the menus, in the Note windows, the
  Markdown viewers and the capture popup too. A card, a row (a Frog's,
  a Tally entry's, a Session's pick), a chip, a checkbox, a switch and
  a menu's entry show none, nor does a click by the keyboard or a
  screen reader, which has no pointer to stand above. As the parallax,
  it always grows and fades, the desktop reading no "remove
  animations". "Show button taps", the phone's words, is a switch in
  Settings ▸ This computer, the desktop having no Look card, after Text
  size; it is kept on this computer only, as the phone keeps it.

### Notifications

- **The desktop's notifications are the system's** (draft, #196), for
  as long as the app runs, in the tray too: on Linux and Windows an
  alarm offers Done, Snooze, Not today and "Open Old Brain Fix" (#269),
  as Windows toasts and some Linux desktops show only a few buttons;
  Snooze shows it again in place with 10 min, 30 min and 1 h, and Not
  today with Skip and Tomorrow, under "Skip it, or tomorrow at the same
  time?", Tomorrow only for an Occurrence of today or later. On macOS it
  has no buttons; there is no full-screen alarm. A click on the Nudge
  opens the window on Unsorted, in place of the phone's Nudge screen.
- **A click on a notification brings the window before the other apps
  and opens what it is about** (obf-1039), on Linux, Windows and macOS
  alike, the body or "Open Old Brain Fix" alike, and the notification
  goes. A Reminder's or Routine's opens the Agenda with that Occurrence
  scrolled into view and outlined in the accent colour: today's row, a
  later day's, else its Routine in the Missed Summary. Listed nowhere,
  as one Done or skipped meanwhile, the Agenda opens with "Already
  done" or "Skipped" at the top; one still open that no day shows, as a
  weekly one of yesterday, opens its Note dialog over the Agenda, its
  Due line answering it. Done, Snooze and Not today are in the
  row's ⋮ and right-click menu; Done also is its box. The outline goes
  once the Agenda is left. The Note open is kept and closed first; a
  Routine editor or Tally form stays over the Agenda with its draft. The
  Catch-up Notification's click opens the Agenda, the Nudge's Unsorted.
- **Every click on a notification is traced** (obf-1039): when, the
  notification's kind and ids, the button, whether the window showed or
  waited in the tray, and where it went, never a Note's text; the last
  eight, in memory while the app runs. Send feedback offers them as
  "Notification clicks", ticked, while there are any.

### What the desktop does not have

- **No App Lock, no Area Locks, no Calendar** (draft, #196): the
  desktop has no App Lock and no Area Locks, so nothing on it is Masked;
  no Calendar page and no appointments on the Agenda; no camera, photos
  coming from a file, the clipboard or a drop. The laws of Both apps
  about these hold where they arise and ask nothing of the desktop
  otherwise.
