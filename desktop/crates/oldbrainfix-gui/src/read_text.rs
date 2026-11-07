//! The texts the Note dialog only shows (#255): another person's text,
//! Sections, Transcripts, Photo Texts and the History. They select as
//! labels do, by a drag, across their lines and formats, and Ctrl+C
//! copies what is selected; a right-click on one opens Copy and Select
//! all at the pointer.
//!
//! The selection is egui's own, which takes any press for a new one: a
//! right-click would lose it, and so would the click on the menu. So the
//! window keeps a right-click on such a text from the labels, and copies
//! what is selected there and then, as Ctrl+C would, for the menu's
//! Copy. Select all drags over the text, from its first character to its
//! last, as the mouse would.

use egui::plugin::Plugin;
use egui::{
    Context, Event, FullOutput, Id, Key, LayerId, Modifiers, OutputCommand, PointerButton, Pos2,
    RawInput, Rect, RichText, Ui,
};

use crate::l10n::L10n;

/// A text only shown, where it lay last drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Region {
    rect: Rect,
    layer: LayerId,
}

/// The menu, open at `at` over `region`, with what was selected there;
/// where it lay last drawn, and where it was clicked.
#[derive(Debug, Clone, PartialEq)]
struct Open {
    at: Pos2,
    region: Region,
    selected: String,
    rect: Option<Rect>,
    clicked: Option<Pos2>,
}

/// A drag over a whole text, one step a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    Press(Pos2, Pos2),
    Move(Pos2),
    Release(Pos2),
    Back,
}

/// The texts only shown and their menu, kept beside egui's selection.
#[derive(Debug, Default)]
pub struct ReadText {
    /// The texts drawn this frame; at its start, those of the last.
    drawn: Vec<Region>,
    open: Option<Open>,
    /// Whether this frame copies what is selected, for the menu.
    copying: bool,
    drag: Option<Drag>,
    /// Where the mouse really is, to go back to after a drag.
    pointer: Option<Pos2>,
}

impl ReadText {
    /// The text at `pos`, where nothing lies over it.
    fn at(&self, ctx: &Context, pos: Pos2) -> Option<Region> {
        let top = ctx.layer_id_at(pos)?;
        self.drawn
            .iter()
            .copied()
            .find(|r| r.layer == top && r.rect.contains(pos))
    }

    /// Where a drag over all of `region` starts and ends: its first
    /// label's first character and its last label's last.
    fn ends(ctx: &Context, region: Region) -> Option<(Pos2, Pos2)> {
        let labels: Vec<Rect> = ctx.viewport(|v| {
            v.this_pass
                .widgets
                .get_layer(region.layer)
                .filter(|w| w.sense.senses_drag() && region.rect.contains_rect(w.rect))
                .map(|w| w.rect)
                .collect()
        });
        let first = labels.iter().min_by(|a, b| {
            (a.top(), a.left())
                .partial_cmp(&(b.top(), b.left()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        let last = labels.iter().max_by(|a, b| {
            (a.bottom(), a.right())
                .partial_cmp(&(b.bottom(), b.right()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        Some((
            first.left_top() + egui::vec2(1.0, 1.0),
            last.right_bottom() - egui::vec2(1.0, 1.0),
        ))
    }
}

fn is_pointer(event: &Event) -> bool {
    matches!(
        event,
        Event::PointerMoved(_) | Event::PointerButton { .. } | Event::PointerGone
    )
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

impl Plugin for ReadText {
    fn debug_name(&self) -> &'static str {
        "read text"
    }

    fn input_hook(&mut self, ctx: &Context, input: &mut RawInput) {
        for event in &input.events {
            if let Event::PointerMoved(pos) = event {
                self.pointer = Some(*pos);
            }
        }
        // Select all: the drag, in place of the mouse.
        if let Some(step) = self.drag.take() {
            input.events.retain(|e| !is_pointer(e));
            let (events, next) = match step {
                Drag::Press(from, to) => (
                    vec![Event::PointerMoved(from), button(from, true)],
                    Some(Drag::Move(to)),
                ),
                Drag::Move(to) => (vec![Event::PointerMoved(to)], Some(Drag::Release(to))),
                Drag::Release(to) => (vec![button(to, false)], Some(Drag::Back)),
                // The mouse where it is, the selection made.
                Drag::Back => (
                    self.pointer.map(Event::PointerMoved).into_iter().collect(),
                    None,
                ),
            };
            input.events.extend(events);
            self.drag = next;
            ctx.request_repaint();
            return;
        }
        // A click on the menu is the menu's alone: the labels keep their
        // selection.
        if let Some(open) = &mut self.open
            && let Some(rect) = open.rect
        {
            input.events.retain(|e| match e {
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    ..
                } if rect.contains(*pos) => {
                    if !*pressed {
                        open.clicked = Some(*pos);
                    }
                    false
                }
                _ => true,
            });
        }
        // Esc closes the menu alone.
        if self.open.is_some() {
            let before = input.events.len();
            input.events.retain(|e| {
                !matches!(
                    e,
                    Event::Key {
                        key: Key::Escape,
                        ..
                    }
                )
            });
            if input.events.len() != before {
                self.open = None;
            }
        }
        // A right-click on a text only shown opens the menu, the labels
        // never seeing it.
        let mut opened = None;
        let drawn: Vec<(Pos2, Option<Region>)> = input
            .events
            .iter()
            .filter_map(|e| match e {
                Event::PointerButton {
                    pos,
                    button: PointerButton::Secondary,
                    ..
                } => Some((*pos, self.at(ctx, *pos))),
                _ => None,
            })
            .collect();
        let mut on = drawn.into_iter();
        input.events.retain(|e| match e {
            Event::PointerButton {
                button: PointerButton::Secondary,
                pressed,
                ..
            } => match on.next() {
                Some((pos, Some(region))) => {
                    if !*pressed {
                        opened = Some((pos, region));
                    }
                    false
                }
                _ => true,
            },
            _ => true,
        });
        if let Some((at, region)) = opened {
            self.open = Some(Open {
                at,
                region,
                selected: String::new(),
                rect: None,
                clicked: None,
            });
            self.copying = true;
            // Only the labels' selection is copied.
            ctx.memory_mut(|m| m.stop_text_input());
            input.events.push(Event::Copy);
            ctx.request_repaint();
        }
    }

    fn on_begin_pass(&mut self, _ui: &mut Ui) {
        self.drawn.clear();
    }

    fn output_hook(&mut self, _ctx: &Context, output: &mut FullOutput) {
        if !std::mem::take(&mut self.copying) {
            return;
        }
        let mut selected = String::new();
        output.platform_output.commands.retain(|c| match c {
            OutputCommand::CopyText(text) => {
                selected.push_str(text);
                false
            }
            _ => true,
        });
        if let Some(open) = &mut self.open {
            open.selected = selected;
        }
    }
}

/// Marks `rect` as a text only shown: its right-click opens the menu.
pub fn region(ui: &Ui, rect: Rect) {
    let layer = ui.layer_id();
    ui.ctx()
        .plugin_or_default::<ReadText>()
        .lock()
        .drawn
        .push(Region { rect, layer });
}

/// The menu, where it is open: Copy, greyed while nothing is selected,
/// and Select all.
pub fn menu(ctx: &Context, t: &L10n) {
    let plugin = ctx.plugin_or_default::<ReadText>();
    let Some(open) = plugin.lock().open.clone() else {
        return;
    };
    let mut copy = false;
    let mut all = false;
    let shown = egui::Area::new(Id::new("read text menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(open.at)
        .show(ctx, |ui| {
            egui::Frame::menu(ui.style()).show(ui, |ui| {
                let colour = ui.visuals().text_color();
                let entry = |ui: &mut Ui, label: &str, enabled: bool| {
                    let button = egui::Button::new(RichText::new(label).color(colour));
                    let picked = ui.add_enabled(enabled, button);
                    let at = |p: Pos2| picked.rect.contains(p);
                    picked.clicked() || (enabled && open.clicked.is_some_and(at))
                };
                copy = entry(ui, t.menu_copy(), !open.selected.is_empty());
                all = entry(ui, t.menu_select_all(), true);
            });
        });
    let pressed_beside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .press_origin()
                .is_some_and(|at| !shown.response.rect.contains(at))
    });
    let mut state = plugin.lock();
    if copy {
        ctx.copy_text(open.selected.clone());
    }
    if all {
        state.drag = ReadText::ends(ctx, open.region).map(|(from, to)| Drag::Press(from, to));
        ctx.request_repaint();
    }
    if copy || all || pressed_beside {
        state.open = None;
    } else if let Some(open) = &mut state.open {
        open.rect = Some(shown.response.rect);
        open.clicked = None;
    }
}

/// Draws a text only shown with `add`, its labels selectable, and marks
/// it for the menu.
pub fn shown<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let drawn = ui.scope(|ui| {
        ui.style_mut().interaction.selectable_labels = true;
        add(ui)
    });
    let rect = drawn.response.rect;
    region(ui, rect.intersect(ui.clip_rect()));
    // A selection dragged from it scrolls the view (#256).
    let dragging = ui.ctx().dragged_id().is_some()
        && ui.input(|i| {
            i.pointer.primary_down() && i.pointer.press_origin().is_some_and(|at| rect.contains(at))
        });
    crate::text_menu::follow(ui, dragging);
    drawn.inner
}

/// Draws `label`, selectable where the labels of `ui` are, as
/// [`egui::Label`] does, but taking part in the selection also while it
/// is scrolled out of view: egui drops a selection whose start or end it
/// did not see in a frame, and a label out of view is not seen. Its text
/// still shows only in view (#256).
pub fn label(ui: &mut Ui, label: egui::Label) -> egui::Response {
    if !ui.style().interaction.selectable_labels {
        return ui.add(label);
    }
    let (pos, galley, response) = label.layout_in_ui(ui);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), galley.text())
    });
    let colour = ui.visuals().text_color();
    egui::text_selection::LabelSelectionState::label_text_selection(
        ui,
        &response,
        pos,
        galley,
        colour,
        egui::Stroke::NONE,
    );
    response
}
