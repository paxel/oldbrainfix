# UI laws

Rules every screen follows. Check a change against them before it lands;
a screen that breaks one is a bug, not a style choice. Terms are those of
`CONTEXT.md`.

## Navigation

- **Never a bottom bar**, on any screen. The bottom edge belongs to the
  system's navigation buttons and gestures; app navigation there only
  causes wrong taps.
- **The end of every page clears the system navigation bar** (#61), on
  gesture and button navigation alike: the last row scrolls fully above
  it, and controls anchored at the bottom sit above it. **The end of a
  page also clears its floating buttons** (#158): the last row scrolls
  fully above them, their margin and a gap included.
- The Agenda carries two floating buttons: **"+"** captures a Note and
  **the magnifier** opens the notes browser. Nothing else floats, except
  the ways "+" fans out while it is open (see Capture).
- Settings, Backups and Sync are reached only through the **Profile
  picture in the Agenda's top corner**, where the settings gear was. It
  opens the Profile page: picture, name and short code; Backups and
  recovery; Your Devices; People; Settings of this Device, with About
  inside. No hamburger menu, no ⋮ menu for them.
- The levels run Agenda, Areas list, Area, Topic: **the Agenda is the
  level above the Areas list**. **Swipe left on the Agenda opens the
  Areas list**, as the magnifier does. In the notes browser, **swipe
  right goes back** one level (Topic, Area, Areas list, Agenda) and
  **swipe left goes forward** into the last selected Area or Topic, if
  there is one. The last selected Area and Topic are highlighted.
- Every browser level (Areas, Topics, Notes) has a "+" that adds the
  thing shown on that level.
- **Nothing slides in from the bottom**, anywhere: a bottom sheet is a
  bug. A menu opens where the finger is; choices that outgrow a menu
  are a page.

## Gestures

- **Tap is the primary action.** On a list row it opens the thing; on a
  checkbox it checks. A tap on a Note card unfolds it in place while it
  is compact and does nothing while it is full; in "merge into…" it
  picks the Note. A tap on a row or card never opens a menu.
- **Long-press is the secondary action**: a menu, or the editor when
  there is only one thing to do. A long-press on a Note card opens its
  editor, as Edit in its ⋮ does; nothing else opens it from a card. A
  long-press never changes data by itself.
- A checkbox or switch is the only thing that toggles state; the row
  around it does not.
- **A horizontal swipe only navigates** (see Navigation); it never
  changes data or swipes a row or card away. The one exception is the
  full-screen table: a one-finger drag on its cells moves it sideways,
  as its bar does.

## Announcements

- Everything the app has to say goes to the top of the screen, under
  the status bar: one announcement at a time, the page pushed down
  below it. Nothing ever appears at the bottom, where the gestures and
  the keyboard live: a snackbar is a bug.
- An announcement goes by itself or by a tap; it is never swiped away.
  A horizontal swipe stays navigation only.
- An announcement says only that a job finished, failed or waits; the
  tap opens the details. Values never go into an announcement.
- A quick local success the screen already shows (copied, saved) gets
  no announcement.
- A failure's tap opens a dialog with cause, fix and the full text,
  Close and Report; the page it belongs to is a line inside, not a
  third button.
- An answer to a form field (name taken, not a code) is that field's
  own error text, red, under the field.

## Capture

- **Capture never asks anything and never needs unlocking.** The App
  Lock and Area Locks guard browsing, not Capture.
- **The Agenda's "+" fans out Text, Voice, Photo, Gallery and Paste**,
  and so does the lock screen's, each an icon with
  its words, upward from the button; it is not a bottom sheet. While
  the fan is open, "+" reads as close; a second tap on it, a tap
  outside or Back closes the fan. Text opens text capture, Voice starts
  recording at once, Photo opens the camera, Gallery opens the system's
  picture picker and continues as a photo Note, Paste opens text capture
  filled from the clipboard: its text in the field, its picture as the
  Note's photo, both when it holds both, and a line on the screen, no
  announcement, when it holds neither. The clipboard is read only on
  that tap, never on opening or in the background. The text capture
  screen keeps its Photo, Voice, Gallery and Paste buttons. Every other
  quick entry point (widget, tile, shortcut, also when the phone's key
  or gesture settings or a button-mapper app start it; no code reads a
  key) opens its capture screen directly, with no fan and no screen in between.
- Voice recording starts at once and **ends by tapping one big stop
  button**. No hold-to-talk, no stop on silence. After Stop, and after
  the photo or the pictures picked, the user is on the capture screen
  with the recording or photo on it, the buttons and room to type.
  Leaving saves.
- Every capture screen shows three buttons at the bottom: remind me in
  **10 min, 30 min, 1 h**. Leaving without tapping one means 1 h.
  The three are actions, not a choice of value: a tap saves the Note
  with that Nudge time, so the radio-button law does not apply to them.
  Where the Device switched the Nudge buttons off (Settings ▸ Quick
  note), one Save takes their place, and it nudges in 1 h as leaving does.
- **The quick note is uncluttered** (#142): by default it shows the
  text field, Photo, Voice, Gallery and Paste, the Nudge buttons and
  **To full editor**, nothing that sorts and no chips. To full editor
  saves the Note at once, Unsorted with the default Nudge, and opens the
  Note editor on it in place of the capture screen; leaving the editor
  returns to where Capture was opened from. It waits while there is
  nothing to save and is not offered behind the App Lock or while a
  Shopping List is picked.
- **Settings ▸ Quick note** (per Device, ADR 0008) has one switch each,
  all off by default, for File into…, Routine, Reminder, Frog, Session
  or Tally, Add to a Shopping List, the Importance and the format bar,
  and the switch Nudge buttons, on by default. What is switched on shows
  in the Note card's action row beside To full editor, on every capture
  path (text, Voice, Photo, Gallery, Paste, share, widget, tile,
  shortcut, also one a phone key or gesture starts): File into… and the
  alarm clock, and ⋮ with the Importance, Make it a Tally (with the
  Routine switch, Q481) and Add to a Shopping List. Each opens its usual
  page and comes back to the capture screen with the choice shown at the
  top ("→ Household"), not saved yet; the user can keep typing or add
  more. All of it is optional. With the format bar on, it rides on the
  keyboard; where the room is small the buttons under the text scroll
  and the text keeps two lines.
- **On the desktop** the capture popup and the window's capture field
  are the quick note and follow the same switches (desktop Settings ▸
  Quick note, beside Quick capture): by default the text, Record voice,
  Paste (the field also From file…), the Nudge buttons and To full
  editor; switched on, File into…, Routine, Reminder, Frog and Session,
  and More with the Importance, Make it a Tally and Add to a Shopping
  List, and the format bar over the field, without ☐, link and table.
  To full editor in the window opens the Note in the editor on the
  right, the middle pane staying where it was; in the popup it closes
  the popup and opens the Note there in the window, brought back from
  the tray.
- **Add to a Shopping List** makes each line of the capture a Line of
  the list picked, and keeps no Note. While a list is picked, the other
  choices wait unshown. It is not offered with a photo on the screen, a
  photo taken afterwards drops it, and behind the App Lock it is not
  offered. A recording gives the lines of its Transcript; until the
  Transcript is there the Note waits Unsorted and goes into the list
  once it is, and without one it stays a Note, announced at the top.
- **Everything is saved together** when the user leaves the screen or
  taps a save button. Nothing is half-saved: for an empty Note nothing
  is saved, so no Routine, Reminder, Frog, Session or Tally ever hangs
  on an empty Note.
- **A sorted Note needs no Nudge.** Once an Area or Topic is chosen, or
  a Routine, Reminder, Frog, Session or Tally is set up, the three
  Nudge buttons become one Save and no Nudge is set; leaving saves too.
- Behind the App Lock, the row offers neither File into… nor the
  Session, which would show the Areas and the Tallies. File into… lists
  a closed locked Area without its Topics.
- After saving, the new Note's card shows with the same options, for
  whatever was not done while capturing (merging into another Note
  among them). Enriching is never required; a Note left without an
  Area or Topic stays Unsorted.

## Agenda

- Order: the Missed Summary card, then the "x Unsorted" banner, then
  the "x Sent Items waiting" line while a stranger's Sent Item waits for
  "Show", then the "x shared Areas waiting" line while an offered Area
  waits for Accept or Decline, then today's all-day Occurrences, then the
  all-day appointments of the calendars picked, then the Shopping Lists'
  rows as Settings of this Device choose (today only), then the
  Checklists' rows (today only, oldest capture first), then today's timed
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
- **The Agenda's bar**, in this order: All Routines and Reminders,
  Calendar, new Routine, then the Profile picture in the corner. All
  Routines and Reminders lists every one alphabetically by name, also
  those beyond the week, and a tap opens its editor; its bar's New
  Session opens the editor of a new Session whose Note is the text typed
  there, and its Export writes one calendar file of all of them. Calendar opens the Calendar
  page: "Read from your calendar" (the appointment running at Capture,
  and the switch that shows the calendar's appointments on the Agenda,
  whose calendars are picked on a page of checkboxes, none at first) and
  "Write to your calendar" (the calendar mirror); Settings has a row
  leading to the same page. Nothing on the Agenda selects rows to share.
- **Every Occurrence row has ⋮**, which opens the same menu as a long
  press: History for a Routine, and Share, which hands the Routine as
  its whole series with its end, or the Reminder, to another app as a
  calendar file. A Masked row asks for its Area to open first.
- **A calendar appointment's row is read only**: no checkbox, the
  calendar's colour down its side, the calendar icon and the calendar's
  name. A tap unfolds its place and description and folds them again;
  ⋮ or a long press opens "Copy into a Note", which makes a new Unsorted
  Note of its title, time, place and description and shows its card on
  top. The calendar mirror's own entries never show as appointments.
  Nothing of an appointment is kept or synced.
- **A Checklist without a Routine or Reminder is one row today** while
  it has open Lines, "Before the trip: 3 of 7 open", unless the user
  switched **Show on the Agenda** off in the ⋮ of its card or editor; the
  switch is there only for such a Checklist. The row has no box; a tap
  opens the list. A Masked one says "Private Checklist" and its counts.
- The Missed Summary stays until the user taps "seen"; opening the app
  never clears it.
- **Checked off today, there all day.** An Occurrence checked off today
  stays on the Agenda for the day as Done, box checked and title struck
  through; the box unchecks it. Checking off opens nothing.
- **A Session's row today lists its picks under its title**, in the
  order picked, each Tally's name with its targets ("Push-ups 20 times,
  Wall sit 1:00"), struck through once it has an entry. Tapping a pick
  opens its entry form, prefilled with the targets; saving records it
  for this Occurrence. Each pick without an entry offers "Another one"
  while the pool holds a Tally not picked. The row's own box checks the
  Occurrence off as any Routine's; once every pick has an entry it is
  Done and stays checked, and removing an entry undoes that. Tomorrow
  and later days show only the title and how many it picks: the picks
  are made on the day.

## Cards

- A Note is drawn as a card with a soft embossed look, visibly above the
  background.
- **A Note is read on its card; the editor is the last step.** A full
  card shows the whole Note: every Section oldest first, the original
  and then each Amend and Merge, each under its date and Capture
  Context, with all its photos, recordings, Transcripts and Photo Texts.
  A Transcript or Photo Text shows whole under its recording or photo,
  dimmed and labelled "Transcript" or "Photo Text", so it never reads as
  typed text.
- A photo on a full card fills the card's width in its own proportions,
  its height capped, and is decoded no larger than it shows, so an
  extreme picture (12 × 34,000 px) cannot break the card; a tap opens it
  full screen with zoom.
- A compact card shows date, time and, when known, the Named Place or
  location, plus a Capture Context icon when there is more to see, the
  Note's first line, its media and the action row.
- The editor opens only through a long press on the card or Edit in its
  ⋮.
- Another person's Note shows their Profile picture, or its
  placeholder, on its card; one's own Notes show none. Tapping another
  person's picture, anywhere it shows, opens their Profile.
- An Area's colour and icon mark its cards, Agenda rows and alarms.
- **Important** cards get an accent marker and a stronger emboss;
  **unimportant** ones are dimmed; normal is plain.

## Dialogs and pages

- A dialog has one question and at most two actions plus Cancel. More
  than that is a page. A list to choose from (Areas, Topics, People) is
  a menu at the finger or a page, never a sheet.
- Text and buttons must be readable at the phone's default font size:
  buttons stacked full width or in one row of two, never a cramped row
  of four. A row of three is allowed when every label is a few
  characters long, like "10 min", "30 min", "1 h". The content scrolls if it does not fit; nothing shrinks to
  fit.
- **The control goes below its text.** In every row that holds text and
  a control (switch, button, dropdown), the control sits on its own line
  below the text, always, also when the text is short.
- **Every single choice is radio buttons.** A choice of one value is a
  column of radio buttons, one option per line, two options included:
  never segmented buttons or a row of choice chips, which cramp and grow
  ugly with every option added. Several options that can be picked
  together are checkboxes or filter chips. A setting that is only on or
  off is a switch, not a choice. Tabs that switch pages are not a
  choice of value.
- Two sentences at most in a dialog body. Concrete nouns, no metaphors.
- Destructive actions (deleting a Note, an Area, a Routine) confirm
  once, in a dialog whose confirm button carries the verb and the error
  colour.
- An editor with several fields is a full-screen page with Save in the
  top bar; secondary state changes are rows at the bottom of the page.

## Text editor

- **The Note editor and Amend type in two rows** on the phone and the
  tablet, unless Settings of this Device switch it off, which gives the
  system's text field back. The desktop keeps its own field.
- The top row is the whole text in the system's text field, so the
  keyboard's suggestions, autocorrect, swipe typing and voice input
  work. A tap puts the cursor at the nearest word boundary; there is no
  magnifier and no dragging the cursor. A long press selects as usual,
  with Copy, Cut and Delete in the system's menu.
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
  colours and move only the selection's moving end; the system's menu
  offers Copy, Cut and Delete; typing replaces the selection and ends
  selecting; another long press ends it with the cursor at the moving
  end.
- A screen reader reads both rows and the buttons by name, and the
  character the cursor passes.
- **The format bar goes with the field**: ☐, bold, italic,
  strike-through, two headings, bullets, numbers, quote, link, code, in
  and out, each on the selection or the word at the cursor, and table,
  which puts a new table at the cursor or opens the one it is in. On the phone
  it sits directly above the bottom row while the keyboard is open, and
  the buttons without room go into ⋮ at its end, opening at the finger.
  On the desktop it sits above the field, which shows a formatted text's
  Markdown while it is edited and the text as it reads otherwise. Its ☐
  puts ☐ in at the cursor, also in the middle of a word.
- **On the phone a table in the field is its grid**, the same as on the
  card, never its Markdown; a tap opens it full screen. It is one block:
  the cursor and a selection never end inside it, the bottom row shows
  it as a table's sign, what is typed right before or after it goes on
  a line of its own, and deleting any of it deletes all of it, asking
  first while its cells hold words. The desktop's field shows its
  Markdown.

## Boxes

- **A tap on a box opens its menu at the finger**: Check, Fail and
  Open, the state it shows left out, and History once it has been
  ticked. It works wherever the box is drawn: on a full or compact
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
- **Show ticks** is a switch in the ⋮ of a Note's card and of its
  editor, there only while the Note or a Section of it has a box: while
  it is on, each ticked box says after it, small and grey, who ticked it
  last and when ("Ann, Tue 14:02", "You" for one's own, the time alone
  for a person not known or blocked). It is kept per Note in a setting
  of this Device.
- **History** opens a dialog listing every change of the box, the oldest
  first: the state, who and when.

## Checklist and Shopping List Lines

- **The Lines of a Checklist and a Shopping List are typed in place** in
  the Note editor: each Line is a text row beside its box, and the
  cursor moves between them. Cards only check them.
- On a Checklist, **Return** splits the Line at the cursor; at its end
  it starts a new empty Line with an open box under it, in the group of
  a Line with one; **Add Line** starts one at the end. **Backspace at
  the start** of a Line joins it with the Line above: its text goes to
  the end of that Line, which keeps its box and state; an empty Line
  simply goes.
- **In and out** sit on the bar above the keyboard, as the format bar
  does, and put the Line with the cursor under the Line above or back
  at the top. On the desktop **Tab and Shift+Tab** do it, in place of
  the buttons.
- **Nothing typed is lost on leaving**: what was typed is stored when
  the cursor leaves a Line and when the editor saves or is left; Lines
  left empty are dropped on leaving.
- A Shopping List keeps its field at the top: **Return adds** what was
  typed and leaves the field ready for the next; text left there is
  added on leaving. Its Lines are renamed in place.
- ⋮ of a Line holds the rest (moving it, its reappear time, Delete); a
  long press in a Line's text selects, as in any field.

## The bar

- **One bar on every phone page.** On a phone the buttons sit on their
  own row and the title on the line under them; on a wide screen the
  title sits beside them.
- **An action goes into ⋮ only when the bar is full.** A page lists its
  actions in a fixed order; the bar shows as many as fit at the width it
  has, the rest overflow into ⋮, and ⋮ exists only when it holds
  something. ⋮ is last. Nothing adapts to use. Settings, Backups and
  Sync never go into ⋮; they stay behind the Profile picture.
- A destructive action has no special place: last in the order, on the
  bar when there is room; its confirm dialog is the guard.
- An action in ⋮ shows its icon and its words, as it does on the bar.
- Labelled buttons (Save) and buttons that open a menu of their own
  never overflow.

## Lists

- A part of a page that is rarely needed folds; the fold is remembered
  per Device in a setting of that Device, closed by default when there
  is already a choice.
- **Every Note list has a full/compact switch** on its bar: each Area,
  each Topic, Unsorted (with the Nudge screen), the search results and a
  person's Notes. It is an on/off toggle of the view, not a choice of
  value: one button that says what tapping it does. Full is the
  default; each list remembers its own choice in a setting of this
  Device. A search inside an Area or Topic follows that list's switch.
- Rows show what the user needs to act, not the stored encoding: dates
  in the device format, values translated, storage keys never.

## Wording

- The UI speaks in the terms of `CONTEXT.md`: Notes, Areas, Topics,
  Routines, Reminders, People; never of entries, entities or hashes.
- Errors name the cause and the fix in one sentence each.

## Look

- Light mode: a light teal background. Dark mode: anthracite. The
  accent is teal in both. The theme follows the system, with a manual
  override.
- The background is scattered hand-drawn question marks, a touch darker
  than the background in light mode and a touch brighter in dark mode,
  moving in slow parallax. The parallax stops when the system's "remove
  animations" setting is on.
- Cards and controls must always stand out clearly from the question
  marks.

## Alarms and notifications

- An Occurrence's notification offers **Done** and **Snooze** (10 min,
  30 min, 1 h) without opening the app.
- A Nudge is a normal notification with the same snooze choices; tapping
  it opens the Nudge screen with the three oldest Unsorted cards and
  "+x".
- An Occurrence from a locked Area whose Area is set to Masked never
  shows its title in a notification or on the Agenda, and stays
  checkable.

## Desktop

- The desktop window is multi-pane: Areas and Topics on the left, Notes
  in the middle, the selected Note's editor on the right, the Agenda as
  a panel or view.
- In the Note lists a click unfolds a compact Note; a double-click or
  Edit opens the editor.
- Keys replace swipes: one for back, one for forward. They are Alt+Left
  and Alt+Right, and the mouse's back and forward buttons; the keys
  rest while text is being typed.

## User guide

- **A ticket that changes a screen also updates its guide page and
  frames**: the page's text under `site/en/guide/` and `site/de/guide/`,
  and the frames, written anew by `tool/guide-frames.sh` and committed
  with the change. CI fails while a committed frame no longer matches
  its screen.
