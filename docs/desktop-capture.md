# Quick capture on the desktop

The desktop app has a small capture popup that opens over the other
windows. Type the thought and press **Enter**: it is kept as a Note with a
Nudge in 1 h, and the popup closes. The buttons **10 min**, **30 min** and
**1 h** keep it with that Nudge instead. **Esc**, or the popup's close
button, keeps what is typed the same way as Enter; with nothing typed, the
popup just closes. The Note lands in Unsorted.

You open the popup

- with the **hotkey**, **Ctrl+Alt+N** unless you pick another one under
  **Settings › Quick capture**,
- from the tray icon's menu: **Capture a Note**,
- with the command `oldbrainfix capture`, from a launcher, a key-binding
  tool or a macro pad.

## The hotkey on each desktop

Which way the hotkey works depends on the desktop. **Settings › Quick
capture** says how it stands on yours.

### X11 (Xfce, Cinnamon, MATE, KDE or GNOME on X11, i3 and others)

Old Brain Fix holds the key itself while it runs, in the tray too. If
another program already uses the key, the settings say so; pick another
one there.

### KDE Plasma on Wayland

On Wayland only the desktop can hand out global keys. Old Brain Fix asks
Plasma for one through the GlobalShortcuts portal and suggests the key
from its settings. Plasma asks you once to confirm it. Change it later in
**System Settings › Keyboard › Shortcuts**, under Old Brain Fix; changing
it in Old Brain Fix's own settings does not move a key Plasma already
gave out.

### GNOME on Wayland

Recent GNOME versions offer the same portal and ask you once to confirm
the key; change it in GNOME's keyboard settings. On older GNOME versions
there is no portal, and Old Brain Fix has no hotkey: add one yourself
under **Settings › Keyboard › Keyboard Shortcuts › Custom Shortcuts**,
with the command `oldbrainfix capture`.

### sway, Hyprland and other Wayland compositors

Where the compositor offers no GlobalShortcuts portal, bind a key to the
command in its configuration, for example

```text
# sway
bindsym Ctrl+Alt+n exec oldbrainfix capture
# Hyprland
bind = CTRL ALT, N, exec, oldbrainfix capture
```

### Windows and macOS

Old Brain Fix holds the key itself. There is no tray icon on these
systems yet, so the hotkey works while the app's window is open.

### Staying on top, and where the keyboard goes afterwards

On X11, Windows and macOS the popup stays over all other windows, and
when it closes the keyboard goes back to the window you were typing in,
where the system does that for a closed window (mostly). On Wayland an
app cannot put its window on top: the compositor decides that, whether
the popup gets the keyboard at once, and where the keyboard goes after.
If the popup opens without it, click into its field; a window rule of
your compositor can keep it on top.

## The command line

```text
oldbrainfix capture              open the capture popup
oldbrainfix capture buy milk     keep "buy milk" as a Note at once
oldbrainfix capture -            keep what comes in on stdin
oldbrainfix capture -- -5 kg     after --, everything is text
```

With a text, nothing opens and nothing is printed when the Note is kept,
with a Nudge in 1 h. While Old Brain Fix runs, the text goes to it, so
its Nudge comes at once; otherwise the command writes the Note itself and
the app finds it on its next start. Without a text, the popup opens in
the running app, or the app starts with the popup and stays in the tray.

Exit codes:

| Code | Meaning                                                  |
|------|----------------------------------------------------------|
| 0    | The Note was kept, or the popup was opened.              |
| 1    | The Note could not be kept; the reason goes to stderr.   |
| 2    | Wrong arguments, or nothing to capture (an empty text).  |

Examples for launchers and macro pads:

- A macro pad or a mouse button: let its tool run `oldbrainfix capture`.
- rofi or dmenu as a one-line capture:
  `rofi -dmenu -p Note | oldbrainfix capture -`
- A script: `echo "Call the dentist" | oldbrainfix capture -`

On Windows the app has no console, so it prints nothing there, and each
start runs on its own: `oldbrainfix capture` without a text opens a
second copy of the app.
