//! The window (UI laws, "Desktop"): Areas and Topics on the left, the
//! Notes of the selection in the middle as embossed cards, the selected
//! Note on the right. Read only: nothing here writes to the log.

use chrono::{DateTime, Local, Utc};
use egui::{Color32, Frame, Id, Margin, RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::{Area, Importance, Note, Notes, Places, Section, Store, Topic};

use crate::background;
use crate::icons;
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::theme::{self, Tones};

/// What the middle pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Every Note neither in the trash nor merged.
    All,
    Unsorted,
    /// The Area or Topic with this id; an Area holds its Topics' Notes.
    Place(String),
    Trash,
}

/// A Note with the Area and Topic it is filed in.
#[derive(Debug, Clone)]
struct Placed {
    note: Note,
    area: Option<Area>,
    topic: Option<Topic>,
}

/// What the panes show, read once per change of selection.
#[derive(Debug, Clone, Default)]
struct View {
    areas: Vec<(Area, Vec<Topic>)>,
    notes: Vec<Placed>,
    /// The selected Note with its Sections.
    detail: Option<(Placed, Vec<Section>)>,
}

pub struct App {
    store: Store,
    t: L10n,
    selection: Selection,
    selected_note: Option<String>,
    view: View,
    /// What went wrong reading the log, shown at the top.
    error: Option<String>,
    /// How far the Notes list is scrolled, for the parallax.
    scrolled: f32,
}

impl App {
    /// The window over `store`, speaking `locale`, showing all Notes.
    pub fn new(store: Store, locale: &str) -> App {
        let mut app = App {
            store,
            t: L10n::new(locale),
            selection: Selection::All,
            selected_note: None,
            view: View::default(),
            error: None,
            scrolled: 0.0,
        };
        app.refresh();
        app
    }

    pub fn t(&self) -> &L10n {
        &self.t
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn selected_note(&self) -> Option<&str> {
        self.selected_note.as_deref()
    }

    /// Shows `selection` in the middle pane.
    pub fn select(&mut self, selection: Selection) {
        self.selection = selection;
        self.refresh();
    }

    /// Shows the Note `id` on the right.
    pub fn select_note(&mut self, id: &str) {
        self.selected_note = Some(id.to_string());
        self.refresh();
    }

    /// Installs the look and the fonts. Cheap enough to call every frame.
    pub fn install(ctx: &egui::Context) {
        theme::install(ctx);
        let bound = ctx.fonts(|f| {
            f.families()
                .contains(&egui::FontFamily::Name(icons::FAMILY.into()))
        });
        if !bound {
            ctx.set_fonts(icons::fonts());
        }
    }

    /// Reads what the panes show from the log.
    fn refresh(&mut self) {
        match self.read() {
            Ok(view) => {
                self.view = view;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn read(&self) -> oldbrainfix_core::Result<View> {
        let notes = Notes::new(&self.store);
        let places = Places::new(&self.store);
        let mut areas = Vec::new();
        for area in places.areas()? {
            let topics = places.topics(&area.id)?;
            areas.push((area, topics));
        }
        let shown = match &self.selection {
            Selection::All => notes.all()?,
            Selection::Unsorted => notes.unsorted()?,
            Selection::Place(id) => notes.inside(id)?,
            Selection::Trash => notes.trashed()?,
        };
        let place = |note: Note| -> oldbrainfix_core::Result<Placed> {
            let Some(id) = note.place.clone() else {
                return Ok(Placed {
                    note,
                    area: None,
                    topic: None,
                });
            };
            let topic = places.topic_by_id(&id)?;
            let area_id = topic.as_ref().map_or(id, |t| t.area_id.clone());
            Ok(Placed {
                note,
                area: places.area_by_id(&area_id)?,
                topic,
            })
        };
        let detail = match &self.selected_note {
            Some(id) => match notes.by_id(id)? {
                Some(note) => Some((place(note)?, notes.sections(id)?)),
                None => None,
            },
            None => None,
        };
        let mut placed = Vec::new();
        for note in shown {
            placed.push(place(note)?);
        }
        Ok(View {
            areas,
            notes: placed,
            detail,
        })
    }

    /// Draws one frame into `ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        let tones = theme::tones(ui.visuals());
        background::paint(
            ui.painter(),
            ui.max_rect(),
            tones.background,
            tones.mark,
            self.scrolled * background::PARALLAX,
        );
        if let Some(error) = self.error.clone() {
            // Announcements go to the top (UI laws).
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    ui.label(RichText::new(error).color(ui.visuals().error_fg_color));
                });
        }
        egui::Panel::left("places")
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .resizable(true)
            .default_size(260.0)
            .min_size(180.0)
            .show(ui, |ui| self.places_pane(ui, tones));
        egui::Panel::right("note")
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .resizable(true)
            .default_size(380.0)
            .min_size(240.0)
            .show(ui, |ui| self.note_pane(ui, tones));
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .show(ui, |ui| self.notes_pane(ui, tones));
    }

    fn places_pane(&mut self, ui: &mut Ui, tones: Tones) {
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .id_salt("places")
            .show(ui, |ui| {
                let text = ui.visuals().text_color();
                let rows = [
                    (Selection::All, icons::ALL_NOTES, self.t.all_notes_title()),
                    (Selection::Unsorted, icons::UNSORTED, self.t.unsorted()),
                ];
                for (selection, icon, label) in rows {
                    let on = self.selection == selection;
                    if row(ui, tones, on, 0.0, Some((icon, text)), label).clicked() {
                        chosen = Some(selection);
                    }
                }
                ui.add_space(12.0);
                ui.label(RichText::new(self.t.areas_title()).strong().color(text));
                ui.add_space(4.0);
                for (area, topics) in &self.view.areas {
                    let colour = area_colour(area.colour.as_deref()).unwrap_or(text);
                    let icon = icons::area_icon(area.icon.as_deref()).map(|i| (i, colour));
                    let on = self.selection == Selection::Place(area.id.clone());
                    if row(ui, tones, on, 0.0, icon, &area_name(&self.t, area)).clicked() {
                        chosen = Some(Selection::Place(area.id.clone()));
                    }
                    for topic in topics {
                        let on = self.selection == Selection::Place(topic.id.clone());
                        if row(ui, tones, on, 24.0, None, &topic_name(&self.t, topic)).clicked() {
                            chosen = Some(Selection::Place(topic.id.clone()));
                        }
                    }
                }
                ui.add_space(12.0);
                let on = self.selection == Selection::Trash;
                if row(
                    ui,
                    tones,
                    on,
                    0.0,
                    Some((icons::TRASH, text)),
                    self.t.trash_title(),
                )
                .clicked()
                {
                    chosen = Some(Selection::Trash);
                }
            });
        if let Some(selection) = chosen {
            self.select(selection);
        }
    }

    /// The title of the middle pane.
    fn title(&self) -> String {
        match &self.selection {
            Selection::All => self.t.all_notes_title().to_string(),
            Selection::Unsorted => self.t.unsorted().to_string(),
            Selection::Trash => self.t.trash_title().to_string(),
            Selection::Place(id) => self
                .view
                .areas
                .iter()
                .find_map(|(area, topics)| {
                    if area.id == *id {
                        return Some(area_name(&self.t, area));
                    }
                    topics.iter().find(|t| t.id == *id).map(|t| {
                        format!("{} / {}", area_name(&self.t, area), topic_name(&self.t, t))
                    })
                })
                .unwrap_or_default(),
        }
    }

    fn notes_pane(&mut self, ui: &mut Ui, tones: Tones) {
        let text = ui.visuals().text_color();
        ui.heading(RichText::new(self.title()).color(text));
        ui.add_space(8.0);
        if self.view.notes.is_empty() {
            let empty = if self.selection == Selection::Trash {
                self.t.trash_empty()
            } else {
                self.t.no_notes_yet()
            };
            ui.label(RichText::new(empty).color(ui.visuals().weak_text_color()));
            return;
        }
        let mut clicked = None;
        let output = egui::ScrollArea::vertical()
            .id_salt("notes")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Room for the emboss around every card.
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                for placed in &self.view.notes {
                    let selected = self.selected_note.as_deref() == Some(placed.note.id.as_str());
                    if note_card(ui, &self.t, tones, placed, selected).clicked() {
                        clicked = Some(placed.note.id.clone());
                    }
                }
                ui.add_space(8.0);
            });
        if output.state.offset.y != self.scrolled {
            // The marks follow on the next pass.
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
        if let Some(id) = clicked {
            self.select_note(&id);
        }
    }

    fn note_pane(&self, ui: &mut Ui, tones: Tones) {
        let Some((placed, sections)) = &self.view.detail else {
            return;
        };
        let note = &placed.note;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        egui::ScrollArea::vertical()
            .id_salt("note")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(&note.text).size(18.0).color(text));
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(self.t.importance()).strong().color(text));
                    let importance = match note.importance {
                        Importance::Unimportant => self.t.importance_unimportant(),
                        Importance::Normal => self.t.importance_normal(),
                        Importance::Important => self.t.importance_important(),
                    };
                    ui.label(RichText::new(importance).color(text));
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(
                        RichText::new(self.t.captured_heading())
                            .strong()
                            .color(text),
                    );
                    ui.label(RichText::new(when(&self.t, note.captured)).color(text));
                    ui.horizontal(|ui| {
                        let colour = placed
                            .area
                            .as_ref()
                            .and_then(|a| area_colour(a.colour.as_deref()))
                            .unwrap_or(weak);
                        let icon = placed
                            .area
                            .as_ref()
                            .and_then(|a| icons::area_icon(a.icon.as_deref()))
                            .unwrap_or(icons::NO_AREA_ICON);
                        icons::show(ui, icon, 18.0, colour);
                        let place = place_line(&self.t, placed)
                            .unwrap_or_else(|| self.t.unsorted().to_string());
                        ui.label(RichText::new(place).color(text));
                    });
                    if note.is_trashed() {
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::TRASH, 18.0, weak);
                            ui.label(RichText::new(self.t.trash_title()).color(text));
                        });
                    }
                });
                for section in sections {
                    embossed(ui, tones, false, None, |ui| {
                        ui.label(RichText::new(&section.text).color(text));
                        ui.label(
                            RichText::new(when(&self.t, section.captured))
                                .small()
                                .color(weak),
                        );
                    });
                }
            });
    }
}

/// An Area's stored colour (`RRGGBB`), or `None` when it has none or it
/// cannot be read.
pub fn area_colour(hex: Option<&str>) -> Option<Color32> {
    let value = u32::from_str_radix(hex?, 16).ok()?;
    let [_, r, g, b] = value.to_be_bytes();
    Some(Color32::from_rgb(r, g, b))
}

/// When something was captured, in the app's language and local time:
/// the phone's `yMd` and `Hm`.
pub fn when(t: &L10n, at: DateTime<Utc>) -> String {
    let local = at.with_timezone(&Local);
    match t.locale() {
        "de" => local.format("%-d.%-m.%Y %H:%M").to_string(),
        _ => local.format("%-m/%-d/%Y %H:%M").to_string(),
    }
}

/// Where a Note is filed, `Area / Topic`; `None` while it is in no
/// place.
fn place_line(t: &L10n, placed: &Placed) -> Option<String> {
    let area = placed.area.as_ref()?;
    Some(match &placed.topic {
        Some(topic) => format!("{} / {}", area_name(t, area), topic_name(t, topic)),
        None => area_name(t, area),
    })
}

/// A row of the left pane: self-painted so its text reads in every
/// state, with the accent while selected.
fn row(
    ui: &mut Ui,
    tones: Tones,
    on: bool,
    indent: f32,
    icon: Option<(char, Color32)>,
    label: &str,
) -> egui::Response {
    let font = egui::FontId::proportional(15.0);
    let height = 30.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, on, label));
    if ui.is_rect_visible(rect) {
        let fg = ui.visuals().text_color();
        let fill = if on {
            tones.accent.gamma_multiply(0.3)
        } else if resp.hovered() {
            tones.card.gamma_multiply(0.7)
        } else {
            Color32::TRANSPARENT
        };
        if fill.a() > 0 {
            ui.painter().rect_filled(rect, 8.0, fill);
        }
        let mut x = rect.left() + 8.0 + indent;
        if let Some((glyph, colour)) = icon {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(x + 9.0, rect.center().y),
                egui::vec2(18.0, 18.0),
            );
            icons::paint(ui.painter(), icon_rect, glyph, colour);
            x += 26.0;
        }
        let text = ui.painter().layout(
            label.to_string(),
            font,
            fg,
            (rect.right() - x - 8.0).max(0.0),
        );
        let y = rect.center().y - text.size().y / 2.0;
        ui.painter().galley(egui::pos2(x, y), text, fg);
    }
    resp
}

/// Lays out `content` on a softly embossed card the width of `ui`;
/// `band` runs an Area's colour down its left edge.
fn embossed(
    ui: &mut Ui,
    tones: Tones,
    strong: bool,
    band: Option<Color32>,
    content: impl FnOnce(&mut Ui),
) -> egui::Response {
    let under = ui.painter().add(egui::Shape::Noop);
    let left = if band.is_some() { 22 } else { 16 };
    let inner = Frame::NONE
        .inner_margin(Margin {
            left,
            right: 16,
            top: 14,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.style_mut().interaction.selectable_labels = false;
            content(ui);
        });
    let rect = inner.response.rect;
    let mut shapes = theme::embossed(rect, tones, strong);
    if let Some(colour) = band {
        shapes.push(theme::area_band(rect, colour));
    }
    ui.painter().set(under, egui::Shape::Vec(shapes));
    inner.response
}

/// A Note as a card (UI laws, "Cards"): its text, when it was captured
/// and where it is filed, its Area's colour down the left edge and icon
/// before the place. Important cards carry the accent marker and a
/// stronger emboss; unimportant ones are dimmed.
fn note_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    placed: &Placed,
    selected: bool,
) -> egui::Response {
    let note = &placed.note;
    let important = note.importance == Importance::Important;
    let colour = placed
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    let inner = ui.scope(|ui| {
        if note.importance == Importance::Unimportant {
            ui.multiply_opacity(theme::UNIMPORTANT_OPACITY);
        }
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let card = embossed(ui, tones, important, colour, |ui| {
            if important {
                // Room for the marker on the right.
                ui.set_max_width(ui.available_width() - 36.0);
            }
            // A photo Note may have no words.
            if !note.text.is_empty() {
                ui.label(RichText::new(&note.text).size(16.0).color(text));
            }
            ui.label(RichText::new(when(t, note.captured)).small().color(weak));
            // A Note that spawned a Routine is not Unsorted, though not
            // filed.
            let place = match place_line(t, placed) {
                Some(line) => Some(line),
                None if note.spawned => None,
                None => Some(t.unsorted().to_string()),
            };
            if let Some(place) = place {
                ui.horizontal(|ui| {
                    if let Some(icon) = placed
                        .area
                        .as_ref()
                        .and_then(|a| icons::area_icon(a.icon.as_deref()))
                    {
                        icons::show(ui, icon, 16.0, colour.unwrap_or(weak));
                    }
                    ui.label(RichText::new(place).size(13.0).color(text));
                });
            }
        });
        let rect = card.rect;
        if important {
            let marker = egui::Rect::from_center_size(
                egui::pos2(rect.right() - 28.0, rect.top() + 28.0),
                egui::vec2(24.0, 24.0),
            );
            icons::paint(ui.painter(), marker, icons::IMPORTANT, tones.accent);
        }
        if selected {
            ui.painter().rect_stroke(
                rect,
                theme::CARD_RADIUS,
                egui::Stroke::new(2.0, tones.accent),
                egui::StrokeKind::Outside,
            );
        }
        rect
    });
    let rect = inner.inner;
    let resp = ui.interact(rect, Id::new(("card", &note.id)), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, &note.text));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn area_colours_read_as_the_phone_reads_them() {
        assert_eq!(
            area_colour(Some("6A1B9A")),
            Some(Color32::from_rgb(0x6A, 0x1B, 0x9A))
        );
        assert_eq!(area_colour(Some("nope")), None);
        assert_eq!(area_colour(None), None);
    }

    #[test]
    fn capture_times_read_in_the_apps_language() {
        let at = Utc.with_ymd_and_hms(2026, 3, 4, 12, 0, 0).single();
        let Some(at) = at else { panic!("a valid date") };
        let local = at.with_timezone(&Local);
        let en = when(&L10n::new("en"), at);
        let de = when(&L10n::new("de"), at);
        assert!(
            en.starts_with(&local.format("%-m/%-d/%Y").to_string()),
            "{en}"
        );
        assert!(
            de.starts_with(&local.format("%-d.%-m.%Y").to_string()),
            "{de}"
        );
    }
}
