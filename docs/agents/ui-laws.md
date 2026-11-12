# UI laws

Rules every screen follows. Check a change against them before it lands;
a screen that breaks one is a bug, not a style choice. Terms are those of
`CONTEXT.md`. #n refers to issue n on https://github.com/paxel/oldbrainfix/issues.

The laws come in three parts (#196): **Both apps** binds the phone and the
desktop alike; **Phone** and **Desktop** hold what only one app does, and
replace a law of Both apps where they say so. A desktop law marked
"(draft, #196)" waits for the user's review.

## Both apps

### Navigation

- Every browser level (Areas, Topics, Notes) has a "+" that adds the
  thing shown on that level. On the Areas list it adds an Area. **In an
  Area, a Topic and All Notes "+" offers Note, Checklist, Shopping
  List, Tally, Session and Frog** (#161), and in an Area Topic;
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
  those most Tallies carry first; the Tallies carrying any chip picked
  show, ticks the filter hides stay and a line counts them, "New Tally"
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

### Announcements

- Everything the app has to say goes to the top of the screen, under
  the status bar: one announcement at a time, the page pushed down
  below it. Nothing ever appears at the bottom, where the gestures and
  the keyboard live: a snackbar is a bug.
- An announcement goes by itself or by a tap; it is never swiped away.
  A horizontal swipe stays navigation only.
- An announcement says only that a job finished, failed or waits; the
  tap opens the details, or, on an offer to undo, undoes. Values never
  go into an announcement.
- A quick local success the screen already shows (copied, saved) gets
  no announcement. The one exception is the phone's offer to undo what
  was just deleted, skipped or moved to tomorrow (#268, #269; see Phone,
  "Announcements").
- A failure's tap opens a dialog with cause, fix and the full text,
  Close and Report; the page it belongs to is a line inside, not a
  third button.
- An answer to a form field (name taken, not a code) is that field's
  own error text, red, under the field.

### Capture

- **Capture never asks anything and never needs unlocking.** The App
  Lock and Area Locks guard browsing, not Capture.
- Voice recording starts at once and **ends by tapping one big stop
  button**. No hold-to-talk, no stop on silence. After Stop, and after
  the photo or the pictures picked, the user is on the capture screen
  with the recording or photo on it, the buttons and room to type.
  Leaving saves.
- Every capture screen shows three buttons at the bottom: remind me in
  **10 min, 30 min, 1 h**. Leaving without tapping one means 1 h.
  The three are actions, not a choice of value: a tap saves the Note
  with that Nudge time, so the single-choice law does not apply to them.
  Where the Device switched the Nudge buttons off (Settings ▸ Quick
  note), one Save takes their place, and it nudges in 1 h as leaving does.
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

### Agenda

- Order: the Missed Summary card, then the "x Unsorted" banner, then
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
  does; History for a Routine, and Share, which hands the Routine as
  its whole series with its end, or the Reminder, to another app as a
  calendar file. Edit schedule, History and Share ask a Masked row's
  Area to open first; Snooze and Not today answer it as its box does.
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
  ("Push-ups 20 times"). The pick's box and words open its entry form,
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
- **The compact view is a glance, by the Note's kind** (#189): a Plain
  Note its first line and the two lines after it, a Checklist its name
  and how many Lines are open ("3 of 7 open"), a Shopping List its name
  and how many are open ("4 open"), a Tally its name and each field's
  but one counting sets total of the current bar, today's for a Tally
  of days (#264), against its target ("Steps 7000 / 10000 steps"), in a
  Shared Area of the user's own entries, as its detailed view counts at
  first (#190), a Session
  its name, the next run of any of its Routines and "done 4 / skipped 1
  this week" over all of them (#191); a Note without words its photos and recordings.
  It shows no Capture Context and **none of the buttons at the bottom of
  the card** (File into, Routine, Merge); **its ⋮ sits top right** and
  offers File into… and Merge into… after Edit, as far as the action row
  would. In the detailed view a Shopping List shows its open Lines,
  tickable, the struck ones folded under them ("2 struck through") until
  a tap shows them, kept open per Note on this Device. **A Tally's
  detailed view is the Tally** (#190): under its name and description
  "Add entry", in a Shared Area "Show entries" (Mine at first,
  Everyone's, or one person's), a chart per field shown but those
  counting sets, a bar per the Tally's bar length with no choice of it
  here (#264): slim bars of a fixed width with gaps, the newest on the
  right, as many as fit, the target as a thin line, never scrolled
  sideways; then its entries (ticks as "✓ Wiped", several sets a line
  each, "1. Weight 60 kg · Reps 10"), newest first, the latest five and
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
- **Copy text** in a card's ⋮ puts the text of the Note and of each
  Section on the clipboard as they show, oldest first, a blank line
  between them, and says nothing (#183).
- **A Section's text is edited in place in the Note editor** (#183), in
  the same field and with the same format bar as the Note's text, by the
  same people (the Section's author, in a Shared Area also the
  Managers); others read it. Under its meta line it says, small and
  grey, when it was last edited and by whom, once it was.
- **A Note's History** (#193) is History on the Note editor's bar on
  the phone (the clock with an arrow, before Merge and Move to trash)
  and Note ▸ History on the desktop, over the Note dialog. It lists
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
  **name**, the text's first line, as a heading, its **description**,
  the rest of the text, small right under it and folded to one line
  until a tap unfolds it (on a card kept open per Note on this Device,
  #189), then **the kind's content** (the Lines; of a Tally its setup in
  the editor, its chart and entries in the detailed view, #190; of a
  Session its pool and count in the editor, its picks and history in the
  detailed view, #191), then
  the badges, the photos and recordings, and the Amends
  below as the dated diary part. In the editor the name is a one-line
  field and the description its own field; nothing is rewritten for the
  display, and leaving both unchanged writes nothing.
- **The Note editor shows each setting of the Note as a badge** (#160),
  in one wrapping row under the text, or under the kind's content for a
  Note of a kind (#188), like a shield: left an icon for *which
  setting* the badge is, the same for every value (one for the kind,
  one for the Importance, …), right its value (#192), as an **icon**
  where the setting has a fixed set of values (the kind, the text
  format, the Importance, Show on the Agenda, Show ticks, a Tally
  field's rule, a Session's rule; an on-or-off setting as `toggle_on` /
  `toggle_off`),
  as **short text** otherwise ("Every day · 08:00", "Household",
  "50 pages", "Reps"). **Never an icon plus the same word.** The
  desktop draws the same Material icons as the phone. Importance; Area and Topic in one; Captured (time, place,
  calendar event, Bluetooth device, app); each Routine and Reminder with
  its schedule; its Frog (#259) while one is open or was eaten today or
  yesterday, the frog with "Open", "Eaten today" or "Eaten yesterday",
  in Simple too, its tap offering at the finger Eat now while it is open
  and this Device may check it off and "Remove the Frog" for who may
  manage it, which asks nothing and is undone as the Frog view's Delete
  is (#268); for a Member an
  eaten one only shows; the kind, Plain Note included (#188); the text format of
  a Plain Note (#213), `text_format` with `short_text`, `format_bold` or
  `code`; Show on the Agenda for a Checklist without a Routine and for a
  Shopping List (#186); Show ticks while it has a box; "From Ann" while
  a Pack brought the Note unclaimed (#164), `inventory_2` with the
  name, its tap offering "Remove “From Ann”" at the finger, which asks
  nothing and is undone (#268). A Tally's setup
  is the Tally's own badges, "One bar per" and "Counts in sets" (#264),
  each opening its small editor, then its fields (#190), each its name
  with its rule, unit and target as badges, as on the Tally page, a
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
  whose menu at the finger offers Routine, Reminder, Frog and Session,
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
  "From Ann" on a weak line with `inventory_2` at the top of its card,
  in either view, until "Remove “From Ann”" in the card's ⋮ or its badge
  in the Note editor takes it away.
- **"Export as Pack…"** (#164) is in the menu of every Area and Topic,
  for anyone who may read them, in a Note card's ⋮ after Copy text and
  on the Note editor's bar after Share to an app (`inventory_2`). It
  opens the Pack's tree: Areas and Topics as headings, each Note with
  its kind's icon and its first line, under it its files with their
  sizes, their texts, its Routines, Reminders and Frog, under a Session
  the Tallies it brings from elsewhere, each a box at the start; an item
  under a Note not ticked cannot be ticked. "Tick all" and "Untick all"
  sit on the bar, the size over the tree. A Pack opened shows the same
  tree, all ticked, under who it is from; its ticks ("Add my Profile",
  "Claim as mine") go below their text.
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
  along (version, system and device, install method ticked at first;
  the phone's last crash report and Notes as a Pack not) and the
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
  right, and for the first column First column as headings. While a
  cell is edited, the format bar, where the field has one, holds only ☐,
  bold, italic, strike-through, link and code, on the cell's words.
  Every change writes the table's lines anew (ADR 0033); the Markdown
  stays the source. **Show as lines** on the grid shows that table's
  lines as text until the cursor leaves them, the field loses focus or
  Show as table is pressed. Column widths are the view's only, never
  stored. Cards and texts that are only read show the grid and open the
  table full screen.
- **A Code text has no format bar and no ☐** (#213): the field shows it
  in the fixed-width font on the grey ground of code, unwrapped, and it
  moves sideways. **A paste that looks like code** into a Plain Note's
  text in the Note editor formats nothing and puts "Keep as code" under
  the text, with a cross that lets the paste be; Capture never asks.

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
  staying in the log; an empty Line simply goes. **No Add Line button
  and no Add field**: Return makes new Lines.
- **The frame around a group holds the group's own Line** and the Lines
  under it, on cards and in the editor.
- **Nothing typed is lost on leaving**: what was typed is stored when
  the cursor leaves a Line and when the editor saves or is left; Lines
  left empty are dropped on leaving.
- On a Shopping List the open Lines keep the order they were typed in,
  also the next time; a Line checked while the editor is open stays in
  its row, and shows below the open ones the next time. A name the list
  has already brings its Line back instead of a second.
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
- Rows show what the user needs to act, not the stored encoding: dates
  in the device format, values translated, storage keys never.

### Wording

- The UI speaks in the terms of `CONTEXT.md`: Notes, Areas, Topics,
  Routines, Reminders, People; never of entries, entities or hashes.
- Errors name the cause and the fix in one sentence each.

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

### The sync line

- **While a sync of this Device runs, a thin line moves under the bar**
  (#225): 3 px in the accent along the bottom edge of the phone's top
  bar (`pageBar`, Capture's bar too) and of the desktop's menu bar, a
  quarter of its width moving back and forth (Knight Rider), taking no
  room, so nothing else on the screen moves. It shows only while a
  folder round or a join of this Device runs, on the desktop also while
  its paired host answers, and the app is in view; every sync shows it,
  also the one each minute. The sync runs beside the app (an isolate on
  the phone, a thread with its own connection on the desktop): the app
  stays usable and draws on. With the system's "remove animations" the
  phone's line stands still across the whole width; the desktop reads
  no such setting, so its line always moves. On the phone a screen
  reader hears "Syncing…" once each time the app comes into view, not
  with every sync; on the desktop the line is a progress indicator named
  "Syncing…".

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
- Settings, Backups and Sync are reached only through the **Profile
  picture in the Agenda's top corner**. It
  opens the Profile page: picture, name and short code; Backups and
  recovery; Your Devices; People; Settings of this Device, with About
  inside. No hamburger menu, no ⋮ menu for them.
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
  fan does.

### Gestures

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
  long-press never changes data by itself.
- **A horizontal swipe only navigates** (see Navigation); it never
  changes data or swipes a row or card away. The exceptions are the
  full-screen table, where a one-finger drag on its cells moves it
  sideways, as its bar does, a table wider than the editor's field
  (#257), which a drag on it moves sideways, and a Code text in the Note editor (#213),
  which a one-finger drag moves sideways; on a card a Code text is cut
  off at the edge instead.
- **On the iPhone a pushed page goes back with the edge swipe** (#243,
  Q570), iOS's own, from the left edge; a browser level goes back with
  a swipe right anywhere, as on Android. A page that holds its leaving,
  such as one asking to keep changes, takes no edge swipe. A long-press
  on a Note card gives the iPhone's haptic where Android vibrates.

### Announcements

- **An offer to undo** (#268): what was just deleted on the phone and
  can be restored says so at the top at once ("Moved to the Trash",
  "Line deleted", "Entry deleted", "Frog moved to the Trash", "“From”
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

### Capture

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
- **On the iPhone a share waits for the app** (#241): iOS lets no Share
  extension open its app, so "Old Brain Fix" in the share sheet keeps
  the text, links, pictures or the app's own files and says so on a small
  card with OK. The next time the app opens or comes back, each share
  opens Capture as on Android, and a Pack, Profile card, Sent Item or
  bundle opens as "Open with" opens it. A share carries no source app on
  the iPhone.
- **On the iPhone iOS's own prompts may come beside Capture** (#243,
  #244): the first Photo asks iOS's camera permission, and Paste may
  show iOS's own "Allow Paste" prompt each time it reads the clipboard.
  Both are iOS's, not the app's, and are allowed beside "Capture never
  asks anything"; the app itself still asks nothing there.
- **The quick note is uncluttered** (#142): by default it shows the
  text field, Photo, Voice, Gallery and Paste, the Nudge buttons,
  **To full editor** and the alarm clock beside it (#223), whose menu
  offers Routine and Reminder as the Note editor's "+" does, in Simple
  too; nothing else that sorts and no chips. The Routine or Reminder
  editor opens over the quick note and comes back to it with the choice
  at the top, written only with the Note. To full editor
  saves the Note at once, Unsorted with the default Nudge, and opens the
  Note editor on it in place of the capture screen; leaving the editor
  returns to where Capture was opened from. It waits while there is
  nothing to save and is not offered behind the App Lock or while a
  Shopping List is picked.
- **Settings ▸ Quick note** (per Device, ADR 0008) has one switch each,
  all off by default, for File into…, Frog, Session or Tally (only in
  Advanced, #197), Add to a Shopping List, the Importance and the format
  bar (only in Advanced),
  and the switch Nudge buttons, on by default. What is switched on shows
  in the Note card's action row beside To full editor, on every capture
  path (text, Voice, Photo, Gallery, Paste, share, widget, tile,
  shortcut, also one a phone key or gesture starts): File into…, Frog
  and Session in the alarm clock's menu, and ⋮ with the Importance, Make
  it a Tally (with the Frog, Session or Tally switch) and Add to a
  Shopping List. Each opens its usual
  page (Add to a Shopping List none while there is one list to add to:
  its entry names it, "Add to Weekly", and picks it at once, #276) and
  comes back to the capture screen with the choice shown at the
  top ("→ Household"), not saved yet; the user can keep typing or add
  more. All of it is optional. With the format bar on, it rides on the
  keyboard; where the room is small the buttons under the text scroll
  and the text keeps two lines.
- Behind the App Lock, the row offers neither File into… nor the
  Session, which would show the Areas and the Tallies. File into… lists
  a closed locked Area without its Topics.
- After saving, the new Note's card shows with the same options, for
  whatever was not done while capturing (merging into another Note
  among them): in its detailed view, a tap switching it to compact. Enriching is never required; a Note left without an
  Area or Topic stays Unsorted.

### Agenda

- **The Agenda's bar**, in this order: All Routines and Reminders,
  Calendar, new Routine, New Session (only in Advanced, #246), then the
  Profile picture in the corner; the desktop's has the same without
  Calendar. When the phone's bar is full, the last of All Routines,
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
  Save, Copy and Share on the phone, Save putting it into the phone's
  Pictures in the album "Old Brain Fix" (Android 10 and later; before
  it, Save is announced as failed with Share as the fix), and only while
  its file is on the phone. Saved and copied say nothing, nor does
  a share sheet left without an app; a failure is announced.
- The editor opens only through a long press on the card or Edit in its
  ⋮.
- In the Note editor a text that is only read, the Note's or a
  Section's, selects with a long press, with Copy in the system's menu
  (#183).
- **Several Notes are selected with "Select"** (#273, Q655): every Note
  list, Unsorted and its Nudge page, an Area, a Topic, All Notes, the
  search's hits and the Trash, has Select in its bar while it shows a
  card this Device may change, as a long press opens the editor and a
  tap switches the view.
  It puts a box at the top left of every card this Device may change
  (ADR 0013); only the box ticks, a tap and a long press on the card
  still do what they do. While it selects the bar's title says how many
  are ticked ("3 selected") and its buttons are, in this order, Stop
  selecting, File into…, Merge into…, Importance and Move to trash, in
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

### Text editor

- **The Note editor and Amend type in two rows** on the phone and the
  tablet, unless Settings of this Device switch it off, which gives the
  system's text field back.
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

### Checklist and Shopping List Lines

- **In and out** sit on the bar above the keyboard, as the format bar
  does, and put the Line with the cursor under the Line above or back
  at the top.

### The bar

- **One bar on every phone page.** On a phone the buttons sit on their
  own row and the title on the line under them; on a wide screen the
  title sits beside them.
- **An action goes into ⋮ only when the bar is full.** A page lists its
  actions in a fixed order; the bar shows as many as fit at the width it
  has, the rest overflow into ⋮, and ⋮ exists only when it holds
  something. ⋮ is last. Nothing adapts to use. Settings, Backups and
  Sync never go into ⋮; they stay behind the Profile picture. A page
  gives such an action as a `BarAction`; the bar counts a button 48
  wide, 1.3 times with Fat fingers (#224).
- A destructive action has no special place: last in the order, on the
  bar when there is room; the offer to undo at the top is the guard, or
  for what goes for good its confirm dialog.
- An action in ⋮ shows its icon and its words, as it does on the bar.
- Labelled buttons (Save) and buttons that open a menu of their own
  never overflow.

### Alarms and notifications

- A Nudge is a normal notification with the same snooze choices; tapping
  it opens the Nudge screen with the three oldest Unsorted cards and
  "+x".
- **A tap on an Occurrence's notification opens the editor of its
  Note** (#270), with Done, Snooze and Not today in its Due line, over
  whatever the app showed; a locked Area asks to open first, and behind
  the App Lock it opens after the unlock. A Routine or Note gone or in
  the trash opens only the app.
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
  - An Occurrence's notification is time-sensitive and offers Done, the
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
  bar** on top (File, Edit, View, Area, Note, AI, Sync, Help); **the
  sidebar** on the left with the Agenda, Unsorted, All Notes, Frogs, the
  Areas with their Topics and the Trash, marking the place shown; **the list** in
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
- **Unsorted says how many Notes it holds in the sidebar** ("Unsorted
  7", #279), as the one list that asks to be emptied; no other place
  shows a count, and an empty Unsorted shows none. **An Area with Topics
  folds and unfolds them with the chevron at the end of its row**
  (#279), a click on it selecting nothing; this Device keeps each fold
  (ADR 0008), as lists keep theirs, Areas starting unfolded. A folded
  Area still shows the Topic shown under it, marked.
- **The window keeps its geometry on each computer** (#278): its size
  and position, whether it is maximized, the sidebar's width and the
  Note dialog's size are settings of the Device (ADR 0008) and come
  back at the next start; a position on no monitor any more opens the
  window centred. The Note dialog is resized by its edges and corners,
  staying centred.
- **On the desktop nothing floats over a list** (#158); every list ends
  with the same room after its last card, so that card scrolls fully
  into view.
- **On the desktop "+" sits at the end of the list's heading** (#146, #161)
  and opens a menu of the same kinds, Note first, each with its icon and
  words; the Agenda's and Unsorted's "+" open the quick note at once. The
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
- **The Agenda's bar on the desktop holds All Routines and Reminders and
  New Routine or Reminder** (draft, #196): no Calendar and no Profile
  picture; the menu bar leads to the rest.

### Menu bar and keys

- **Every menu entry acts on what it names and closes its menu**, also
  when picked by keyboard or screen reader; the Note menu acts on the
  Note in the Note dialog and is greyed while none is open, each entry
  as far as the dialog offers it, but File into…, Merge into…,
  Importance and Delete, in the Trash Restore in Delete's place, act on
  the cards selected in the list while no Note is open (#148, #273). Its keys (Ctrl+N,
  Ctrl+F, Ctrl+M, Ctrl+Shift+M for Merge into…, Ctrl+,; Cmd on macOS)
  stand beside the entries and work while typing, over the Note dialog
  too, not while another dialog is open; Delete and 1 to 3 stand beside
  Delete and Importance's entries while those act on the cards selected
  (#274). Ctrl+F searches the list shown where it has a search field
  (All Notes, Unsorted, an Area, a Topic, the Trash, whose search also
  finds its Routines, Reminders and Frogs by their Note), else All
  Notes'. On the desktop the menu bar leads to Settings, Backups and
  Sync in place of the Profile picture.
- **The Area menu acts on the Area or Topic the list shows** (#279):
  Rename (F2), Colour and icon, Share…, People, Export as Pack… and
  Delete, the sidebar right-click's entries, each greyed where that
  menu would leave it out (a Topic has no Colour and icon, Share… or
  People), and all of them while no Area or Topic shows or a dialog
  lies over the list, the Note dialog included, so that none acts on a
  place behind it. F2 rests in a text field and while a menu is open.
  The sidebar's right-click stays, and keeps New Topic.
- **The AI menu leads to the AI port** (#279): AI port… opens the
  Settings at the AI port's card, and Review the AI Task shows the
  review of the AI Task that waits, greyed while none waits or the port
  is open. Ask… joins it with the Ask panel (#267); until then it is
  left out, not greyed.
- **Edit ▸ Undo and Ctrl+Z take back the last delete, check-off,
  Skip or Tomorrow made in the app (#269), filing or Merge** (#268), the
  desktop's offer to undo, with no announcement: Edit ▸ Undo is first in its menu and greyed while
  nothing waits; Ctrl+Z works over the list and the Note dialog but not
  in a text field, where it undoes the typing. Only the last one waits;
  cards selected and moved to the trash or merged together come back
  with one (#273).
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
  Down go on from it while it still shows, else from the top. Sorting Unsorted is Down, Ctrl+M, type, Enter for
  each Note.
- **Keys finish what is typed** (draft, #196, #277): Ctrl+Enter (Cmd on
  macOS) saves the quick note's Note and the text being edited in the
  Note dialog, while Enter alone starts a new line; in the capture popup
  Enter saves and Shift+Enter starts a new line (Q664). Ctrl+Enter runs
  the main action of the dialog on top, the button on the accent (Save,
  Create, Send, the Routine editor's, a Session's and a Tally's pages
  included), and only while it may be pressed; Merge into… merges into
  the first Note found once something is typed, File into… takes its
  first row as Enter does; a question whose answer deletes or discards
  keeps Ctrl+Enter from it (Q665). The first field of a dialog takes the
  keyboard as it opens, where it has one (the Routine editor's text, else
  its When line, a Tally's pages, Send feedback, a person's name for me);
  the Page dialogs of sections (Settings, Profile, People, Backups, Sync,
  Pair) open without it. Alt+Left rests in that field as in any; Esc and
  the mouse's back button leave. In the quick note and the
  popup Ctrl+1, Ctrl+2 and Ctrl+3 save with the Nudge buttons' 10 min,
  30 min and 1 h while those show, named in the hint under them; the
  list's 1 to 3 take no Ctrl and rest while a dialog is open. Enter in
  a one-line field of a dialog takes it; Ctrl+B and Ctrl+I make bold
  and italic while the Note's text is edited.
- **Esc closes what lies on top** (draft, #196): a menu, a photo or
  table full screen, the top dialog, the quick note keeping what was
  typed, and over the list without a dialog the cards selected.
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

- **The Settings, About and everything under Sync (Sync, Pair a Device,
  People, Profile, Backups) open as dialogs over the window**, never as
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
  pills' look), a radio group of radio buttons to a screen reader, Tab
  and the arrows moving between them, Space or Enter picking one; a
  long one is a column of radio buttons (`app::radio_column`), as in
  Both apps (#226); the Sync dialog's "Bundle for",
  a dropdown beside its words (`sync_view.rs`), does not follow that
  yet.
- **A Pack's tree is a dialog on the desktop** (#165): "Tick all" and
  "Untick all" stand over the tree under its size, in place of the
  phone's bar; every box, "Add my Profile" and "Claim as mine" too,
  stands beside its words; Save… saves the file where the user says, in
  place of Share…; "Add … to People" takes the person in and keeps the
  dialog open.

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
- **A selection is acted on from a card's right-click** (#273, Q656):
  its menu offers, after File into…, Merge into… and Importance ▸
  Important, Normal or Unimportant, and Move to trash last, each only on
  a card this Device may change; a card in the Trash has no ⋮, and its
  right-click offers Restore alone. On a selected card each acts on all
  the cards selected, on another on that card alone, as File into…
  does, and the Note menu offers the same while no Note is open (see
  Menu bar and keys). Merge into… opens the Note picker with every Note,
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
  own field and format bar there. "Copy as text" in a card's right-click menu, the
  Note menu and the dialog is the desktop's "Copy text": the Note and
  its Sections as they show.
- **Cards move into an Area or Topic together** (#148): dragged onto it
  in the sidebar, which outlines the row under the pointer, or with
  File into… from the card's right-click, the Note menu or Ctrl+M,
  whose dialog names how many go. A selected card takes the selection
  along; one outside it goes alone. A card this Device may not move
  stays where it is; the announcement says so without numbers, its
  details how many moved and how many stayed.
- **A photo gets out of the Note** (#149): on the desktop a right-click
  on a photo in the Note editor or full screen offers "Save image as…",
  "Copy image" and "Open in another program". Saved and copied say
  nothing; a failure is announced.
- **The Agenda's rows are tight** (#208): on the desktop a row's card
  has 6 above and below its content, 6 between cards and 2 between a
  heading's time and words, so the next row starts 52 below, where
  0.3.1 had 84; the box stays 28. The "Eat a frog" rows are as tight.

### Capture

- **On the desktop** the capture popup and the window's quick note
  dialog, which File ▸ New Note, Ctrl+N and "+" open (#146), are the
  quick note and follow the same switches (desktop Settings ▸ Quick
  note, beside Quick capture): by default the text, Record voice, Paste
  (the dialog also From file…), the Nudge buttons, To full editor and
  the alarm clock, whose menu offers Routine and Reminder (#223), its
  editor a dialog over the quick note dialog or a page of the popup;
  switched on, File into…, Frog and Session in the clock's menu, and
  More with the Importance, Make it a Tally and Add to a Shopping List
  (its lists as radio buttons, the one used last at the top; with one
  list a single entry naming it, "Add to Weekly", #276),
  and the format bar over the field, without ☐, link and table. Files
  dropped onto the window open the dialog and wait in it. Leaving the
  dialog by its cross, Esc, Alt+Left, the mouse's back button or a click
  beside it keeps what is typed, nudged in 1 h. To full editor in the
  window opens the Note in the Note dialog in its place; in the popup it
  closes the popup and opens the Note there in the window, brought back
  from the tray.

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
- **A selection scrolls while it is dragged** (#256): in every text
  field and in the Note dialog's texts that are only read, while the
  button is held the wheel scrolls and the selection follows the
  pointer, and a drag past the top or bottom edge of the view scrolls by
  itself, faster the further out the pointer is. The selection stays
  when its start scrolls out of view.

### Settings

- **The Settings dialog is a column of cards** (draft, #196): each card
  embossed as a Note's, its heading strong on top (This computer,
  Alarms, Nudges, Units, Agenda, History, Features, …), its settings
  under it, each with its small weak detail line.

### Buttons and look

- **Buttons look like buttons** (#218): a rounded rectangle (radius
  8 px) with a tonal fill and a 1 px outline in light and dark, darker
  in light and lighter in dark under the pointer, and pressed or holding
  the keyboard focus a step further with a 2 px ring. The main action of
  a dialog or page (Save, Create, Done, Use, Restore, Continue) is
  on the accent with its text in `on_accent`; Delete, Remove and Move to
  Trash are on the error colour; every other button is secondary. The
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
- **A click on an Occurrence's notification opens its Note dialog**
  (#270), the body or "Open Old Brain Fix" alike: the window comes up
  with the Note dialog of its Note and its Due line, over whatever it
  showed; a Routine editor or Tally form open stays over it with its
  draft. A Routine or Note gone or in the trash opens only the window.

### What the desktop does not have

- **No App Lock, no Area Locks, no Calendar** (draft, #196): the
  desktop has no App Lock and no Area Locks, so nothing on it is Masked;
  no Calendar page and no appointments on the Agenda; no camera, photos
  coming from a file, the clipboard or a drop. The laws of Both apps
  about these hold where they arise and ask nothing of the desktop
  otherwise.
