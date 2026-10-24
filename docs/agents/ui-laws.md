# UI laws

Rules every screen follows. Check a change against them before it lands;
a screen that breaks one is a bug, not a style choice. Terms are those of
`CONTEXT.md`.

## Navigation

- **Never a bottom bar**, on any screen. The bottom edge belongs to the
  system's navigation buttons and gestures; app navigation there only
  causes wrong taps.
- The Agenda carries two floating buttons: **"+"** captures a Note and
  **the magnifier** opens the notes browser. Nothing else floats.
- Settings, with About and Sync inside, are reached only through the
  **gear in the Agenda's top corner**. No hamburger menu, no ⋮ menu for
  them.
- In the notes browser, **swipe right goes back** one level (Topic,
  Area, Agenda) and **swipe left goes forward** into the last selected
  Area or Topic, if there is one. The last selected Area and Topic are
  highlighted.
- Every browser level (Areas, Topics, Notes) has a "+" that adds the
  thing shown on that level.
- **Nothing slides in from the bottom**, anywhere: a bottom sheet is a
  bug. A menu opens where the finger is; choices that outgrow a menu
  are a page.

## Announcements

- Everything the app has to say goes to the top of the screen, under
  the status bar: one announcement at a time, the page pushed down
  below it. Nothing ever appears at the bottom, where the gestures and
  the keyboard live: a snackbar is a bug.

## Capture

- **Capture never asks anything and never needs unlocking.** The App
  Lock and Area Locks guard browsing, not Capture.
- "+" opens text capture directly; the mic and camera buttons on that
  screen switch to voice or photo. Every quick entry point (widget,
  tile, shortcut, hardware button) opens its capture screen directly,
  with no screen in between.
- Voice recording starts at once and **ends by tapping one big stop
  button**. No hold-to-talk, no stop on silence.
- Every capture screen shows three buttons at the bottom: remind me in
  **10 min, 30 min, 1 h**. Leaving without tapping one means 1 h.
- After saving, the new Note's card shows with its enrich options
  (Area or Topic, Routine, Reminder). Enriching is never required;
  leaving keeps the Note Unsorted.

## Agenda

- Order: the Missed Summary card, then the "x Unsorted" banner, then
  today's all-day Occurrences, then today's timed Occurrences by time.
- After a visible gap comes tomorrow, in dimmer colours. Tomorrow's
  Occurrences can be checked off today.
- The Missed Summary stays until the user taps "seen"; opening the app
  never clears it.

## Cards

- A Note is drawn as a card with a soft embossed look, visibly above the
  background.
- A card shows date, time and, when known, the Named Place or location,
  plus a Capture Context icon when there is more to see. Tapping the card
  opens the full editor, which shows everything about the Note.
- An Area's colour and icon mark its cards, Agenda rows and alarms.
- **Important** cards get an accent marker and a stronger emboss;
  **unimportant** ones are dimmed; normal is plain.

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
