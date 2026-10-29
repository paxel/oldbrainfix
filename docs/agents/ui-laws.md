# UI laws

Rules every screen follows. Check a change against them before it lands;
a screen that breaks one is a bug, not a style choice. Terms are those of
`CONTEXT.md`.

## Navigation

- **Never a bottom bar**, on any screen. The bottom edge belongs to the
  system's navigation buttons and gestures; app navigation there only
  causes wrong taps.
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

- **Tap is the primary action.** On a list row or card it opens the
  thing; on a checkbox it checks. A tap on a row or card never opens a
  menu.
- **Long-press is the secondary action**: a menu, or the editor when
  there is only one thing to do. A long-press never changes data by
  itself.
- A checkbox or switch is the only thing that toggles state; the row
  around it does not.
- **A horizontal swipe only navigates** (see Navigation); it never
  changes data or swipes a row or card away.

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
- **The Agenda's "+" fans out Text, Voice, Photo and Gallery**, and so
  does the lock screen's, each an icon with
  its words, upward from the button; it is not a bottom sheet. While
  the fan is open, "+" reads as close; a second tap on it, a tap
  outside or Back closes the fan. Text opens text capture, Voice starts
  recording at once, Photo opens the camera, Gallery opens the system's
  picture picker and continues as a photo Note. The text capture screen
  keeps its Photo, Voice and Gallery buttons. Every other quick entry
  point (widget, tile, shortcut, hardware button) opens its capture
  screen directly, with no fan and no screen in between.
- Voice recording starts at once and **ends by tapping one big stop
  button**. No hold-to-talk, no stop on silence.
- Every capture screen shows three buttons at the bottom: remind me in
  **10 min, 30 min, 1 h**. Leaving without tapping one means 1 h.
  The three are actions, not a choice of value: a tap saves the Note
  with that Nudge time, so the radio-button law does not apply to them.
- After saving, the new Note's card shows with its enrich options
  (Area or Topic, Routine, Reminder, Frog, Session). Enriching is never
  required; leaving keeps the Note Unsorted.

## Agenda

- Order: the Missed Summary card, then the "x Unsorted" banner, then
  the "x Sent Items waiting" line while a stranger's Sent Item waits for
  "Show", then the "x shared Areas waiting" line while an offered Area
  waits for Accept or Decline, then today's all-day Occurrences, then today's timed
  Occurrences by time, then the "Eat a frog" section while today's pick
  holds a Frog (never above what is due).
- After a visible gap comes tomorrow, in dimmer colours, then the five
  days after it, each under its weekday and date, dimmed alike. Every
  day shows, an empty one with "Nothing planned". The later days'
  Occurrences can be checked off today.
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
- A card shows date, time and, when known, the Named Place or location,
  plus a Capture Context icon when there is more to see. Tapping the card
  opens the full editor, which shows everything about the Note.
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
- Keys replace swipes: one for back, one for forward. They are Alt+Left
  and Alt+Right, and the mouse's back and forward buttons; the keys
  rest while text is being typed.
