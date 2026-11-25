//! Spotlights (obf-1043): each screen explains itself on a first visit,
//! one tip after another. The window dims around one element, a card
//! beside it says in one sentence what it does, Next shows the next tip
//! and Skip, a click beside the card, Esc and back end the screen's tour;
//! No more tips, like "Show tips" off in the Settings (obf-1060), ends
//! every tour to come. A tour never traps input: the card always has
//! Skip, and every way out of a dialog ends the tour first.
//!
//! Which tips this Device showed are settings of the Device (ADR 0008),
//! never synced: `spot:<screen>` holds the ids of the screen's tips shown,
//! comma-separated, and [`STARTED`] says this Device ran with Spotlights
//! once. A Device updated from 0.6.1 counts the tips of what it already
//! had as shown ([`BEFORE_070`], [`seed`]); Show tips again in the
//! Settings ([`show_again`]) brings every tip back and switches "Show
//! tips" on, which `tipsOff` keeps off as the phone does
//! ([`DeviceSettings::tips_on`]).
//!
//! Each element a tip points at says where it is drawn, frame by frame,
//! with [`anchor`]; [`Spotlights`] runs the tour over what the window
//! drew. Tours run only where the desktop binary switches them on
//! (`App::set_spotlights`), so no test and no guide frame sees one.

use std::collections::{HashMap, HashSet};

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{
    Align, Color32, Context, Frame, Id, Key, LayerId, Margin, Modifiers, Order, PointerButton,
    Pos2, Rect, RichText, Sense, Ui, UiBuilder, Vec2,
};
use oldbrainfix_core::settings::DeviceSettings;
use oldbrainfix_core::{Result, Store};

use crate::l10n::L10n;
use crate::theme::{self, Tones};

/// Set once this Device first ran with Spotlights switched on.
pub const STARTED: &str = "spotStarted";

/// The setting holding the ids of the tips of `screen` this Device
/// showed.
pub fn seen_key(screen: &str) -> String {
    format!("spot:{screen}")
}

/// One tip: its id, kept once shown; the element it points at, as
/// [`anchor`] names it; and its sentence.
pub struct Tip {
    pub id: &'static str,
    pub anchor: &'static str,
    pub text: fn(&L10n) -> &'static str,
}

/// A screen and its tips, in the order its tour shows them.
pub struct Screen {
    pub id: &'static str,
    pub tips: &'static [Tip],
}

/// The Agenda, in the window with nothing over it.
pub const AGENDA: &str = "agenda";
/// The quick note's dialog.
pub const QUICK_NOTE: &str = "quickNote";
/// The Note dialog.
pub const NOTE_DIALOG: &str = "noteDialog";
/// An Area's or a Topic's list.
pub const LIST: &str = "list";
/// Unsorted's list.
pub const UNSORTED: &str = "unsorted";
/// The Routine or Reminder editor.
pub const ROUTINE_EDITOR: &str = "routineEditor";
/// The Settings.
pub const SETTINGS: &str = "settings";

/// Every screen's tips, in the order each tour shows them. A ticket that
/// adds something to a screen appends its tip to that screen's list here
/// (and its sentence to `desktop/l10n/desktop_*.arb`), and never adds it
/// to [`BEFORE_070`]: a Device that had the screen before then shows it
/// the new tip alone.
pub const MANIFEST: &[Screen] = &[
    Screen {
        id: AGENDA,
        tips: &[
            Tip {
                id: "agenda-add",
                anchor: "add",
                text: L10n::spot_desk_agenda_add,
            },
            Tip {
                id: "agenda-places",
                anchor: "sidebar",
                text: L10n::spot_desk_agenda_places,
            },
            Tip {
                id: "agenda-back",
                anchor: "nav",
                text: L10n::spot_desk_agenda_back,
            },
            Tip {
                id: "agenda-all-routines",
                anchor: "all-routines",
                text: L10n::spot_desk_agenda_all_routines,
            },
            Tip {
                id: "agenda-new-routine",
                anchor: "new-routine",
                text: L10n::spot_desk_agenda_new_routine,
            },
            // obf-1044: on Quiet today in the bar (obf-1066), not shown
            // while it reads End quiet day.
            Tip {
                id: "agenda-quiet",
                anchor: "quiet-today",
                text: L10n::spot_desk_agenda_quiet,
            },
        ],
    },
    Screen {
        id: QUICK_NOTE,
        tips: &[
            Tip {
                id: "quick-text",
                anchor: "quick-text",
                text: L10n::spot_desk_quick_text,
            },
            // obf-1042: shown once something is typed or held.
            Tip {
                id: "quick-more",
                anchor: "quick-more",
                text: L10n::spot_desk_quick_more,
            },
            Tip {
                id: "quick-nudges",
                anchor: "quick-nudges",
                text: L10n::spot_desk_quick_nudges,
            },
        ],
    },
    Screen {
        id: NOTE_DIALOG,
        tips: &[
            Tip {
                id: "note-window",
                anchor: "note-window",
                text: L10n::spot_desk_note_window,
            },
            Tip {
                id: "note-kind",
                anchor: "note-kind",
                text: L10n::spot_desk_note_kind,
            },
            Tip {
                id: "note-place",
                anchor: "note-place",
                text: L10n::spot_desk_note_place,
            },
            // obf-1036: on the Text format badge, while the text is Code;
            // its small editor picks the language.
            Tip {
                id: "note-code-language",
                anchor: "note-format-code",
                text: L10n::spot_desk_note_code_language,
            },
            // obf-1034: on the "+" badge whose menu holds it.
            Tip {
                id: "note-remind",
                anchor: "note-add-badge",
                text: L10n::spot_desk_note_remind,
            },
            // obf-1035: only while the Note shows it.
            Tip {
                id: "note-linked-from",
                anchor: "note-linked-from",
                text: L10n::spot_desk_note_linked_from,
            },
        ],
    },
    Screen {
        id: LIST,
        tips: &[
            Tip {
                id: "list-path",
                anchor: "nav",
                text: L10n::spot_desk_list_path,
            },
            Tip {
                id: "list-add",
                anchor: "add",
                text: L10n::spot_desk_list_add,
            },
            Tip {
                id: "list-compact",
                anchor: "compact",
                text: L10n::spot_desk_list_compact,
            },
            Tip {
                id: "list-search",
                anchor: "search",
                text: L10n::spot_desk_list_search,
            },
            Tip {
                id: "list-card",
                anchor: "first-card",
                text: L10n::spot_desk_card_drag,
            },
            // obf-1065: on the band's ⋮ of an Area or a Topic.
            Tip {
                id: "list-place-more",
                anchor: "place-more",
                text: L10n::spot_desk_list_place_more,
            },
        ],
    },
    Screen {
        id: UNSORTED,
        tips: &[
            Tip {
                id: "unsorted-what",
                anchor: "heading",
                text: L10n::spot_desk_unsorted_what,
            },
            Tip {
                id: "unsorted-card",
                anchor: "first-card",
                text: L10n::spot_desk_card_drag,
            },
            Tip {
                id: "unsorted-steps",
                anchor: "first-card",
                text: L10n::spot_desk_unsorted_steps,
            },
        ],
    },
    Screen {
        id: ROUTINE_EDITOR,
        tips: &[
            Tip {
                id: "routine-schedule",
                anchor: "routine-schedule",
                text: L10n::spot_desk_routine_schedule,
            },
            Tip {
                id: "routine-next",
                anchor: "routine-next",
                text: L10n::spot_desk_routine_next,
            },
            // obf-1046.
            Tip {
                id: "routine-heads-ups",
                anchor: "routine-heads-ups",
                text: L10n::spot_desk_routine_heads_ups,
            },
            Tip {
                id: "routine-renag",
                anchor: "routine-renag",
                text: L10n::spot_desk_routine_renag,
            },
        ],
    },
    Screen {
        id: SETTINGS,
        tips: &[
            Tip {
                id: "settings-search",
                anchor: "settings-search",
                text: L10n::spot_desk_settings_search,
            },
            // obf-1043.
            Tip {
                id: "settings-show-tips",
                anchor: "settings-show-tips",
                text: L10n::spot_desk_settings_show_tips,
            },
            // obf-1037.
            Tip {
                id: "settings-grammar",
                anchor: "settings-grammar",
                text: L10n::spot_desk_settings_grammar,
            },
            Tip {
                id: "settings-features",
                anchor: "settings-features",
                text: L10n::spot_desk_settings_features,
            },
            // obf-1060: on the switch Show tips.
            Tip {
                id: "settings-tips-switch",
                anchor: "settings-tips-switch",
                text: L10n::spot_settings_tips_switch,
            },
        ],
    },
];

/// The tips of what the desktop had in 0.6.1, frozen: a Device updated
/// from it counts them as shown ([`seed`]). Never computed from
/// [`MANIFEST`], and nothing is ever added to it: 0.7.0's new things
/// (Quiet today, Remind me…, Heads-ups, Linked from, the code's language,
/// Add grammar…, the quick note's More, now Open in the editor, Show
/// tips again) stay due.
pub const BEFORE_070: &[(&str, &[&str])] = &[
    (
        AGENDA,
        &[
            "agenda-add",
            "agenda-places",
            "agenda-back",
            "agenda-all-routines",
            "agenda-new-routine",
        ],
    ),
    (QUICK_NOTE, &["quick-text", "quick-nudges"]),
    (NOTE_DIALOG, &["note-window", "note-kind", "note-place"]),
    (
        LIST,
        &[
            "list-path",
            "list-add",
            "list-compact",
            "list-search",
            "list-card",
        ],
    ),
    (
        UNSORTED,
        &["unsorted-what", "unsorted-card", "unsorted-steps"],
    ),
    (
        ROUTINE_EDITOR,
        &["routine-schedule", "routine-next", "routine-renag"],
    ),
    (SETTINGS, &["settings-search", "settings-features"]),
];

/// The tips of `screen`, none for a screen the manifest does not know.
pub fn tips(screen: &str) -> &'static [Tip] {
    MANIFEST
        .iter()
        .find(|s| s.id == screen)
        .map_or(&[], |s| s.tips)
}

/// The ids in a `spot:<screen>` value.
fn ids(csv: &str) -> impl Iterator<Item = &str> {
    csv.split(',').map(str::trim).filter(|id| !id.is_empty())
}

/// The tips of `tips` not in `seen`, a `spot:<screen>` value, in their
/// order.
pub fn due<'a>(seen: &str, tips: &'a [Tip]) -> Vec<&'a Tip> {
    let seen: HashSet<&str> = ids(seen).collect();
    tips.iter().filter(|tip| !seen.contains(tip.id)).collect()
}

/// The ids of the tips of `screen` this Device showed.
pub fn seen(store: &Store, screen: &str) -> Result<String> {
    Ok(store.local_setting(&seen_key(screen))?.unwrap_or_default())
}

/// Counts the tips `shown` of `screen` as shown on this Device, with
/// those shown before.
pub fn mark_seen(store: &Store, screen: &str, shown: &[&str]) -> Result<()> {
    let before = seen(store, screen)?;
    let mut all: Vec<&str> = ids(&before).collect();
    for id in shown {
        if !all.contains(id) {
            all.push(id);
        }
    }
    let value = all.join(",");
    if value != before {
        store.set_local_setting(&seen_key(screen), &value)?;
    }
    Ok(())
}

/// Counts every tip of every screen as shown on this Device, as the
/// guide's frames are taken with none showing.
pub fn mark_all_seen(store: &Store) -> Result<()> {
    for screen in MANIFEST {
        let all: Vec<&str> = screen.tips.iter().map(|tip| tip.id).collect();
        mark_seen(store, screen.id, &all)?;
    }
    Ok(())
}

/// Show tips again: every screen shows its tips again on its next visit,
/// "Show tips" on. [`STARTED`] stays, so they are not counted as shown
/// once more.
pub fn show_again(store: &Store) -> Result<()> {
    DeviceSettings::new(store).set_tips_on(true)?;
    for screen in MANIFEST {
        store.remove_local_setting(&seen_key(screen.id))?;
    }
    Ok(())
}

/// Once, as the app starts with Spotlights on: a Device that ran an
/// earlier version (`fresh` false: its first start is not due) counts
/// the tips of [`BEFORE_070`] as shown, a fresh one (a new person, a
/// Device added, a Backup restored) none; either way [`STARTED`] is set.
/// Whether it did anything.
pub fn seed(store: &Store, fresh: bool) -> Result<bool> {
    if store.local_setting(STARTED)?.is_some() {
        return Ok(false);
    }
    if !fresh {
        for (screen, shown) in BEFORE_070 {
            mark_seen(store, screen, shown)?;
        }
    }
    store.set_local_setting(STARTED, "1")?;
    Ok(true)
}

/// The widest a tip's card gets: one sentence across a wide window is
/// the wrong shape.
pub const CARD_WIDTH: f32 = 400.0;
/// The least room between the card and the window's edge.
pub const MARGIN: f32 = 24.0;
/// The room between the card and what it explains.
pub const GAP: f32 = 16.0;
/// How far the light around an element reaches past it.
pub const HALO: f32 = 6.0;
/// How round the light's corners are.
pub const HALO_RADIUS: f32 = 12.0;
/// How dark the window gets around the light.
pub const DIM: u8 = 153;
/// A card's height before it was drawn once.
const FIRST_HEIGHT: f32 = 120.0;
/// How long the keys rest before a tip of something that came up while
/// typing shows, in seconds.
pub const IDLE: f64 = 1.5;
/// Frames a screen shows before its tour starts: a dialog's first frame
/// measures it.
const SETTLE: u32 = 2;

/// Where a tip's card of `size` goes for the lit `target` in `screen`:
/// as wide as [`CARD_WIDTH`] allows, centred on the target and inside the
/// screen by [`MARGIN`], below the target in the upper half and above it
/// in the lower, else beside it; never over it while the screen has room
/// for that, and always inside the screen, its buttons with it.
pub fn card_rect(target: Rect, screen: Rect, size: Vec2) -> Rect {
    let inside = screen.shrink(MARGIN);
    let width = CARD_WIDTH.min(inside.width()).max(0.0);
    let height = size.y.min(inside.height()).max(0.0);
    let size = Vec2::new(width, height);
    let clamp_x = |x: f32| x.clamp(inside.left(), (inside.right() - width).max(inside.left()));
    let clamp_y = |y: f32| y.clamp(inside.top(), (inside.bottom() - height).max(inside.top()));
    let x = clamp_x(target.center().x - width / 2.0);
    let y = clamp_y(target.center().y - height / 2.0);
    let below = Rect::from_min_size(Pos2::new(x, target.bottom() + GAP), size);
    let above = Rect::from_min_size(Pos2::new(x, target.top() - GAP - height), size);
    let right = Rect::from_min_size(Pos2::new(target.right() + GAP, y), size);
    let left = Rect::from_min_size(Pos2::new(target.left() - GAP - width, y), size);
    let tries = if target.center().y < screen.center().y {
        [below, above, right, left]
    } else {
        [above, below, right, left]
    };
    tries
        .into_iter()
        .find(|place| inside.contains_rect(*place))
        .unwrap_or_else(|| {
            // No room off it: inside the screen, over the target.
            Rect::from_min_size(Pos2::new(clamp_x(below.left()), clamp_y(below.top())), size)
        })
}

/// What an element was drawn as this frame: its rect, and the part of
/// it its scroll area shows.
#[derive(Debug, Clone, Copy)]
struct Drawn {
    rect: Rect,
    shown: Rect,
}

/// The elements drawn this frame in a viewport, by their anchor's name.
type Anchors = HashMap<&'static str, Drawn>;

fn anchors_id(ctx: &Context) -> Id {
    Id::new("spotlight anchors").with(ctx.viewport_id())
}

fn scroll_id(ctx: &Context) -> Id {
    Id::new("spotlight scroll").with(ctx.viewport_id())
}

/// Says that the element `name` names was drawn at `rect` this frame,
/// for a tip pointing at it; one asked to come into view scrolls there
/// once. A first, measuring pass of a dialog says nothing.
pub fn anchor(ui: &Ui, name: &'static str, rect: Rect) {
    if ui.is_sizing_pass() {
        return;
    }
    let ctx = ui.ctx();
    let shown = rect.intersect(ui.clip_rect());
    // The ids first: a Context access inside another one deadlocks.
    let (anchors, scroll) = (anchors_id(ctx), scroll_id(ctx));
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Anchors>(anchors)
            .insert(name, Drawn { rect, shown });
    });
    let wanted = ctx.data(|d| d.get_temp::<&'static str>(scroll));
    if wanted == Some(name) {
        ui.scroll_to_rect(rect, Some(Align::Center));
        ctx.data_mut(|d| d.remove::<&'static str>(scroll));
    }
}

/// The elements drawn since the last call, taken.
fn take_anchors(ctx: &Context) -> Anchors {
    let id = anchors_id(ctx);
    ctx.data_mut(|d| d.remove_temp::<Anchors>(id))
        .unwrap_or_default()
}

/// A tour running: the screen's tips due as it began, whose elements
/// were drawn then, and the one showing.
struct Tour {
    screen: &'static str,
    due: Vec<&'static Tip>,
    at: usize,
    /// The card's size as drawn last.
    card: Vec2,
    /// The field that had the keyboard as it began, given it back as it
    /// ends: the quick note's text goes on being typed.
    focus: Option<Id>,
}

/// The screen shown, since it came: how many frames, and the elements
/// already looked at for tips.
struct Visit {
    key: String,
    frames: u32,
    looked: bool,
    known: HashSet<&'static str>,
}

/// How a tip was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Next,
    End,
    /// No more tips (obf-1060): "Show tips" goes off.
    Off,
}

/// The tours of one window: off until switched on.
#[derive(Default)]
pub struct Spotlights {
    on: bool,
    /// "Show tips" off (obf-1060), as this Device keeps it: no tour
    /// starts and nothing is marked, what is due stays due.
    tips_off: bool,
    tour: Option<Tour>,
    visit: Option<Visit>,
    /// When a key or text last came in, in the input's seconds.
    typed: f64,
}

impl Spotlights {
    /// Switches the tours on or off; off ends one running.
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
        if !on {
            self.tour = None;
            self.visit = None;
        }
    }

    pub fn on(&self) -> bool {
        self.on
    }

    /// Follows "Show tips" (obf-1060); off ends the tour showing, its
    /// rest still due.
    pub fn set_tips(&mut self, on: bool) {
        self.tips_off = !on;
        if !on {
            self.tour = None;
        }
    }

    /// Whether a tip shows.
    pub fn touring(&self) -> bool {
        self.tour.is_some()
    }

    /// The id of the tip showing.
    pub fn showing(&self) -> Option<&'static str> {
        self.tour
            .as_ref()
            .and_then(|tour| tour.due.get(tour.at))
            .map(|tip| tip.id)
    }

    /// Before anything else of the frame takes them: while a tip shows,
    /// Esc, Alt+Left and the mouse's back button end the tour and go no
    /// further. Whether a tip shows, so that nothing else goes back.
    pub fn keys(&mut self, ctx: &Context, store: &Store) -> bool {
        if self.tour.is_none() {
            return false;
        }
        let back = ctx.input_mut(|i| {
            let esc = i.consume_key(Modifiers::NONE, Key::Escape);
            let left = i.consume_key(Modifiers::ALT, Key::ArrowLeft);
            esc || left || i.pointer.button_pressed(PointerButton::Extra1)
        });
        if back {
            self.end(ctx, store, true);
        }
        true
    }

    /// After the window drew `screen` (its manifest id and what makes a
    /// visit of it, none while something lies over it): shows the tip
    /// showing over it, or starts the screen's tour where tips are due.
    pub fn show(
        &mut self,
        ctx: &Context,
        (store, t, tones): (&Store, &L10n, Tones),
        screen: Option<(&'static str, String)>,
    ) {
        let drawn = take_anchors(ctx);
        if !self.on {
            return;
        }
        let (now, typed) = ctx.input(|i| {
            let typed = i
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::Key { .. } | egui::Event::Text(_)));
            (i.time, typed)
        });
        if typed {
            self.typed = now;
        }
        let Some((id, key)) = screen else {
            // A dialog or menu came over it: its tips wait for the next
            // visit.
            self.tour = None;
            self.visit = None;
            return;
        };
        if self.tour.as_ref().is_some_and(|tour| tour.screen != id) {
            self.tour = None;
        }
        if self.tour.is_some() {
            self.draw(ctx, (store, t, tones), &drawn);
            return;
        }
        // Another screen, or another Note or place on it, is a new visit.
        let key = format!("{id}/{key}");
        if self.visit.as_ref().is_none_or(|v| v.key != key) {
            self.visit = Some(Visit {
                key,
                frames: 0,
                looked: false,
                known: HashSet::new(),
            });
        }
        let Some(visit) = &mut self.visit else {
            return;
        };
        // Show tips off: the visit is looked at all the same, so that
        // switching them on shows nothing before the next one.
        if self.tips_off {
            visit.looked = true;
            visit.known.extend(drawn.keys().copied());
            return;
        }
        visit.frames += 1;
        if visit.frames < SETTLE {
            ctx.request_repaint();
            return;
        }
        // Nothing came up since it was looked at.
        if visit.looked && drawn.keys().all(|name| visit.known.contains(name)) {
            return;
        }
        let seen = match seen(store, id) {
            Ok(seen) => seen,
            Err(e) => {
                eprintln!("tips: {e}");
                return;
            }
        };
        let present: Vec<&'static Tip> = due(&seen, tips(id))
            .into_iter()
            .filter(|tip| drawn.contains_key(tip.anchor))
            .collect();
        // On arrival every tip due whose element shows; later only one
        // whose element came up since, once the keys rest.
        let fresh: Vec<&'static Tip> = if visit.looked {
            present
                .into_iter()
                .filter(|tip| !visit.known.contains(tip.anchor))
                .collect()
        } else {
            present
        };
        let idle = now - self.typed;
        if visit.looked && !fresh.is_empty() && idle < IDLE {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(IDLE - idle));
            return;
        }
        visit.looked = true;
        visit.known.extend(drawn.keys().copied());
        if fresh.is_empty() {
            return;
        }
        self.tour = Some(Tour {
            screen: id,
            due: fresh,
            at: 0,
            card: Vec2::new(CARD_WIDTH, FIRST_HEIGHT),
            focus: ctx.memory(|m| m.focused()),
        });
        if self.arrive_at(ctx, store, &drawn, 0) {
            self.draw(ctx, (store, t, tones), &drawn);
        }
    }

    /// Shows the tour's tip at `from` or the first after it whose element
    /// is drawn, marked shown before it shows; ends the tour when none is
    /// left. Whether a tip shows.
    fn arrive_at(&mut self, ctx: &Context, store: &Store, drawn: &Anchors, from: usize) -> bool {
        let Some(tour) = &mut self.tour else {
            return false;
        };
        let next = (from..tour.due.len()).find(|at| {
            tour.due
                .get(*at)
                .is_some_and(|t| drawn.contains_key(t.anchor))
        });
        let Some((at, tip)) = next.and_then(|at| tour.due.get(at).map(|tip| (at, *tip))) else {
            self.end(ctx, store, false);
            return false;
        };
        // Marked before it shows: a tour broken off never shows it again.
        if let Err(e) = mark_seen(store, tour.screen, &[tip.id]) {
            eprintln!("tips: {e}");
            self.end(ctx, store, false);
            return false;
        }
        tour.at = at;
        // Brought into view once, where it lies below the fold.
        if drawn
            .get(tip.anchor)
            .is_some_and(|d| d.shown.width() < d.rect.width() || d.shown.height() < d.rect.height())
        {
            let id = scroll_id(ctx);
            ctx.data_mut(|d| d.insert_temp(id, tip.anchor));
        }
        ctx.request_repaint();
        true
    }

    /// Ends the tour, the keyboard back where it was; `rest` counts the
    /// tips due as it began as shown, as Skip does.
    fn end(&mut self, ctx: &Context, store: &Store, rest: bool) {
        let Some(tour) = self.tour.take() else {
            return;
        };
        if let Some(field) = tour.focus {
            ctx.memory_mut(|m| m.request_focus(field));
        }
        if rest {
            let all: Vec<&str> = tour.due.iter().map(|tip| tip.id).collect();
            if let Err(e) = mark_seen(store, tour.screen, &all) {
                eprintln!("tips: {e}");
            }
        }
    }

    /// Draws the tip showing: the window dimmed with its element lit,
    /// which follows it as it moves, and the card beside it; takes the
    /// card's buttons and a click beside it.
    fn draw(&mut self, ctx: &Context, (store, t, tones): (&Store, &L10n, Tones), drawn: &Anchors) {
        let Some(tour) = &self.tour else {
            return;
        };
        let Some(tip) = tour.due.get(tour.at).copied() else {
            self.end(ctx, store, false);
            return;
        };
        let Some(element) = drawn.get(tip.anchor) else {
            // Gone from the screen: on to the next tip there is.
            let from = tour.at + 1;
            self.arrive_at(ctx, store, drawn, from);
            return;
        };
        let last = tour
            .due
            .iter()
            .skip(tour.at + 1)
            .all(|tip| !drawn.contains_key(tip.anchor));
        let card_size = tour.card;
        let screen = ctx.content_rect();
        let lit = element.shown.expand(HALO).intersect(screen);
        let dim = Color32::from_black_alpha(DIM);
        let beside = egui::Area::new(Id::new("spotlight dim"))
            .order(Order::Tooltip)
            .fixed_pos(screen.min)
            .constrain(false)
            .show(ctx, |ui| {
                // Takes every click and drag beside the card, so none reaches the
                // window beneath.
                let beside = ui.allocate_rect(screen, Sense::click() | Sense::drag());
                ui.painter().add(dim_mesh(screen, lit, dim));
                beside
            })
            .inner;
        let mut answer = None;
        let mut card = card_size;
        // Not before its element is in view: it scrolls there first.
        if lit.is_positive() {
            let place = card_rect(lit, screen, card_size);
            let card_id = Id::new("spotlight card");
            let shown = egui::Area::new(card_id)
                .order(Order::Tooltip)
                .fixed_pos(place.min)
                .constrain(false)
                .show(ctx, |ui| {
                    // A click on the card itself ends nothing.
                    ui.interact(place, card_id.with("card"), Sense::click());
                    Frame::NONE
                        .fill(tones.card)
                        .corner_radius(theme::CARD_RADIUS)
                        .inner_margin(Margin::same(16))
                        .show(ui, |ui| {
                            ui.set_width(place.width() - 32.0);
                            let text = ui.visuals().text_color();
                            ui.label(RichText::new((tip.text)(t)).color(text));
                            ui.add_space(8.0);
                            ui.scope_builder(
                                UiBuilder::new().layout(egui::Layout::right_to_left(Align::Center)),
                                |ui| {
                                    let next = if last { t.spot_done() } else { t.spot_next() };
                                    if theme::primary(ui, true, next).clicked() {
                                        answer =
                                            Some(if last { Answer::End } else { Answer::Next });
                                    }
                                    if ui
                                        .button(RichText::new(t.spot_skip()).color(text))
                                        .clicked()
                                    {
                                        answer = Some(Answer::End);
                                    }
                                    if ui
                                        .button(RichText::new(t.spot_no_more_tips()).color(text))
                                        .clicked()
                                    {
                                        answer = Some(Answer::Off);
                                    }
                                },
                            );
                        });
                });
            card = shown.response.rect.size();
            ctx.move_to_top(LayerId::new(Order::Tooltip, card_id));
        } else {
            ctx.request_repaint();
        }
        if answer.is_none() && (beside.clicked() || beside.secondary_clicked()) {
            answer = Some(Answer::End);
        }
        if let Some(tour) = &mut self.tour {
            tour.card = card;
        }
        match answer {
            Some(Answer::Next) => {
                let from = self.tour.as_ref().map_or(0, |tour| tour.at + 1);
                self.arrive_at(ctx, store, drawn, from);
            }
            Some(Answer::End) => self.end(ctx, store, true),
            Some(Answer::Off) => {
                // At once, the rest of the tour still due should they be
                // switched on again.
                self.end(ctx, store, false);
                if let Err(e) = DeviceSettings::new(store).set_tips_on(false) {
                    eprintln!("tips: {e}");
                }
                self.set_tips(false);
            }
            None => {}
        }
    }
}

/// The dimming over `screen` with `lit` cut out of it, its corners
/// rounded by [`HALO_RADIUS`].
fn dim_mesh(screen: Rect, lit: Rect, colour: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let mut quad = |rect: Rect| {
        if rect.is_positive() {
            mesh.add_colored_rect(rect, colour);
        }
    };
    if !lit.is_positive() {
        quad(screen);
        return mesh;
    }
    // Above, below, and left and right of the light.
    quad(Rect::from_min_max(
        screen.min,
        Pos2::new(screen.max.x, lit.top()),
    ));
    quad(Rect::from_min_max(
        Pos2::new(screen.min.x, lit.bottom()),
        screen.max,
    ));
    quad(Rect::from_min_max(
        Pos2::new(screen.min.x, lit.top()),
        Pos2::new(lit.left(), lit.bottom()),
    ));
    quad(Rect::from_min_max(
        Pos2::new(lit.right(), lit.top()),
        Pos2::new(screen.max.x, lit.bottom()),
    ));
    // Each corner of the light filled from its point to its arc.
    let radius = HALO_RADIUS.min(lit.width() / 2.0).min(lit.height() / 2.0);
    let corners = [
        (lit.left_top(), Vec2::new(1.0, 1.0)),
        (lit.right_top(), Vec2::new(-1.0, 1.0)),
        (lit.left_bottom(), Vec2::new(1.0, -1.0)),
        (lit.right_bottom(), Vec2::new(-1.0, -1.0)),
    ];
    const STEPS: usize = 8;
    for (corner, inward) in corners {
        let centre = corner + inward * radius;
        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex {
            pos: corner,
            uv: WHITE_UV,
            color: colour,
        });
        for step in 0..=STEPS {
            let angle = std::f32::consts::FRAC_PI_2 * step as f32 / STEPS as f32;
            let arc = centre - Vec2::new(inward.x * angle.cos(), inward.y * angle.sin()) * radius;
            mesh.vertices.push(Vertex {
                pos: arc,
                uv: WHITE_UV,
                color: colour,
            });
        }
        for step in 0..STEPS as u32 {
            mesh.add_triangle(base, base + 1 + step, base + 2 + step);
        }
    }
    mesh
}
