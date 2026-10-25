//! The user's own Profile on the desktop's screen (ADR 0017): its
//! picture, or the initials on a circle coloured by its id, where the
//! settings gear was; and the top of the Profile page with the picture,
//! the name and the short code, and the square the user picks of a new
//! picture.

use egui::{
    Color32, ColorImage, FontId, Id, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit,
    TextureHandle, TextureOptions, Ui, WidgetInfo, WidgetType,
};
use oldbrainfix_core::profile::{self, Profile, ProfileChoice};

use crate::icons;
use crate::l10n::L10n;
use crate::media::pixels;

/// What the user asked for on the Profile page.
#[derive(Debug, Clone, PartialEq)]
pub enum ProfileAction {
    /// Choose a picture file with the system's dialog.
    PickPicture,
    /// Drop the picture; the initials show instead.
    Initials,
    /// Keep the name typed.
    Rename(String),
    /// Take the square picked as the picture.
    UseSquare,
    /// Leave the picture as it was.
    CancelSquare,
    /// Open "Which one is you?" again.
    Ask,
    /// "Which one is you?": this Device's Profile (true) or the other's.
    Answer(bool),
    /// "Which one is you?" waits on the Profile page.
    Later,
}

/// A new picture while the user picks its square, in pixels of the
/// picture.
pub struct Square {
    pub source: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub left: f32,
    pub top: f32,
    pub edge: f32,
    texture: Option<TextureHandle>,
}

impl Square {
    /// The square in the middle of `source` at full height or width;
    /// none when `source` is no picture.
    pub fn new(source: Vec<u8>) -> Option<Square> {
        let (width, height) = profile::picture_size(&source)?;
        let edge = width.min(height) as f32;
        Some(Square {
            left: (width as f32 - edge) / 2.0,
            top: (height as f32 - edge) / 2.0,
            edge,
            source,
            width,
            height,
            texture: None,
        })
    }

    /// The square moved and sized, kept inside the picture.
    pub fn place(&mut self, left: f32, top: f32, edge: f32) {
        let shortest = self.width.min(self.height) as f32;
        self.edge = edge.clamp(1.0_f32.min(shortest), shortest);
        self.left = left.clamp(0.0, self.width as f32 - self.edge);
        self.top = top.clamp(0.0, self.height as f32 - self.edge);
    }

    /// The Profile picture of the square.
    pub fn make(&self) -> Option<Vec<u8>> {
        profile::make_picture(
            &self.source,
            self.left.round() as i64,
            self.top.round() as i64,
            self.edge.round() as i64,
        )
    }
}

/// The own Profile's picture as a texture, made again when the Profile
/// changes, and the name being typed.
#[derive(Default)]
pub struct ProfileView {
    /// The texture of the picture of this Profile version.
    picture: Option<(String, i64, TextureHandle)>,
    /// The name field, and the Profile version it was filled from.
    pub name: String,
    pub name_of: Option<(String, i64)>,
    pub square: Option<Square>,
}

impl ProfileView {
    /// Fills the name field from `profile` once it changed elsewhere.
    pub fn follow(&mut self, profile: Option<&Profile>) {
        let of = profile.map(|p| (p.id.clone(), p.version));
        if of != self.name_of {
            self.name = profile.map(|p| p.name.clone()).unwrap_or_default();
            self.name_of = of;
        }
    }

    fn texture(&mut self, ui: &Ui, profile: &Profile) -> Option<TextureHandle> {
        if let Some((id, version, texture)) = &self.picture
            && *id == profile.id
            && *version == profile.version
        {
            return Some(texture.clone());
        }
        let texture = load(ui, profile)?;
        self.picture = Some((profile.id.clone(), profile.version, texture.clone()));
        Some(texture)
    }
}

/// `profile`'s picture as a texture; none for the placeholder.
fn load(ui: &Ui, profile: &Profile) -> Option<TextureHandle> {
    let image = pixels(profile.picture.as_ref()?, profile::PICTURE_EDGE)?;
    let size = [image.width() as usize, image.height() as usize];
    Some(ui.ctx().load_texture(
        format!("profile:{}:{}", profile.id, profile.version),
        ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        TextureOptions::LINEAR,
    ))
}

/// Other people's pictures as textures, made once each. Keyed by the
/// Profile id and version and by the picture itself: a Sent Item's
/// preview draws a Profile that is not kept, and a copied id and version
/// with another picture must not take the place of the one kept (#51).
#[derive(Default)]
pub struct Pictures {
    textures: std::collections::HashMap<PictureKey, Option<TextureHandle>>,
}

/// What a picture is cached by: the Profile id and version, and a hash of
/// the picture's bytes.
type PictureKey = (String, i64, u64);

fn picture_key(profile: &Profile) -> PictureKey {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    profile.picture.hash(&mut hasher);
    (profile.id.clone(), profile.version, hasher.finish())
}

impl Pictures {
    fn texture(&mut self, ui: &Ui, profile: &Profile) -> Option<TextureHandle> {
        self.textures
            .entry(picture_key(profile))
            .or_insert_with(|| load(ui, profile))
            .clone()
    }
}

/// Another person's picture in a circle `size` wide, or their
/// placeholder, labelled `label` for screen readers and on hover; not
/// clickable.
pub fn person_picture(
    ui: &mut Ui,
    pictures: &mut Pictures,
    profile: &Profile,
    size: f32,
    label: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Image, true, label));
    let texture = pictures.texture(ui, profile);
    paint(ui, rect, texture, Some(profile));
    response.on_hover_text(label)
}

/// Paints `texture` in a circle filling `rect`; without one, the
/// placeholder of `profile`, or a neutral circle without a Profile.
fn paint(ui: &Ui, rect: egui::Rect, texture: Option<TextureHandle>, profile: Option<&Profile>) {
    let size = rect.width();
    let painter = ui.painter_at(rect.expand(2.0));
    let center = rect.center();
    match texture {
        Some(texture) => {
            egui::Image::new(&texture)
                .corner_radius(size / 2.0)
                .paint_at(ui, rect);
        }
        None => {
            let fill = profile.map_or(ui.visuals().widgets.inactive.bg_fill, |p| {
                let [_, r, g, b] = profile::color(&p.id).to_be_bytes();
                Color32::from_rgb(r, g, b)
            });
            painter.circle_filled(center, size / 2.0, fill);
            let initials = profile
                .map(|p| profile::initials(&p.name))
                .unwrap_or_default();
            let colour = if profile.is_some() {
                Color32::WHITE
            } else {
                ui.visuals().weak_text_color()
            };
            if initials.is_empty() {
                icons::paint(&painter, rect.shrink(size * 0.2), icons::PERSON, colour);
            } else {
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    initials,
                    FontId::proportional(size * 0.4),
                    colour,
                );
            }
        }
    }
}

/// How `name` shows: itself, or "No name" when it is empty.
pub fn display_name<'a>(t: &'a L10n, name: &'a str) -> &'a str {
    if name.is_empty() {
        t.profile_no_name()
    } else {
        name
    }
}

/// A Profile's picture in a circle `size` wide, clickable and labelled
/// `label`; without one, its initials on the circle coloured by its id;
/// without a Profile, a neutral circle.
pub fn avatar(
    ui: &mut Ui,
    view: &mut ProfileView,
    profile: Option<&Profile>,
    size: f32,
    label: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    let texture = profile.and_then(|p| view.texture(ui, p));
    paint(ui, rect, texture, profile);
    let painter = ui.painter_at(rect.expand(2.0));
    let center = rect.center();
    if response.hovered() {
        painter.circle_stroke(
            center,
            size / 2.0,
            Stroke::new(2.0, ui.visuals().widgets.hovered.fg_stroke.color),
        );
    }
    response.on_hover_text(label)
}

/// The top of the Profile page: the picture with the buttons that change
/// it, the name field and the short code.
pub fn header(
    ui: &mut Ui,
    t: &L10n,
    view: &mut ProfileView,
    profile: Option<&Profile>,
    code: Option<&str>,
    actions: &mut Vec<ProfileAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    view.follow(profile);
    ui.horizontal(|ui| {
        avatar(ui, view, profile, 96.0, t.profile_picture_change());
        ui.add_space(16.0);
        ui.vertical(|ui| {
            if ui
                .button(RichText::new(t.profile_picture_choose()).color(text))
                .clicked()
            {
                actions.push(ProfileAction::PickPicture);
            }
            if profile.is_some_and(|p| p.picture.is_some())
                && ui
                    .button(RichText::new(t.profile_picture_initials()).color(text))
                    .clicked()
            {
                actions.push(ProfileAction::Initials);
            }
        });
    });
    ui.add_space(8.0);
    ui.label(RichText::new(t.profile_name()).strong().color(text));
    let field = ui.add(
        TextEdit::singleline(&mut view.name)
            .id(Id::new("profile name"))
            .hint_text(t.profile_no_name())
            .desired_width(320.0),
    );
    field.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.profile_name()));
    if profile.is_some() && field.lost_focus() {
        actions.push(ProfileAction::Rename(view.name.clone()));
    }
    if let Some(code) = code {
        ui.horizontal(|ui| {
            ui.label(RichText::new(t.profile_code()).color(weak));
            ui.label(RichText::new(code).monospace().size(16.0).color(text));
        });
    }
}

/// "Which one is you?" (ADR 0017): this Device's Profile and the other
/// one, each with its picture, name and short code and a button that
/// keeps it, and "Later", which leaves the question on the Profile page.
pub fn question_form(
    ui: &mut Ui,
    t: &L10n,
    views: (&mut ProfileView, &mut ProfileView),
    mine: (Option<&Profile>, Option<&str>),
    other: &ProfileChoice,
    actions: &mut Vec<ProfileAction>,
) {
    let text = ui.visuals().text_color();
    ui.heading(RichText::new(t.profile_which_title()).color(text));
    ui.label(RichText::new(t.profile_which_text()).color(text));
    let (own_view, other_view) = views;
    let choices = [
        (
            own_view,
            mine.0,
            mine.1.unwrap_or_default(),
            t.profile_which_this_device(),
            true,
        ),
        (
            other_view,
            Some(&other.profile),
            other.code.as_str(),
            t.profile_which_other_device(),
            false,
        ),
    ];
    for (view, profile, code, place, keep_mine) in choices {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let name = display_name(t, profile.map_or("", |p| p.name.as_str()));
                avatar(ui, view, profile, 56.0, name);
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(name).strong().color(text));
                    ui.label(RichText::new(code).monospace().color(text));
                    ui.label(
                        RichText::new(place)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    );
                    if ui
                        .button(RichText::new(t.profile_which_pick()).color(text))
                        .clicked()
                    {
                        actions.push(ProfileAction::Answer(keep_mine));
                    }
                });
            });
        });
    }
    if ui
        .button(RichText::new(t.profile_which_later()).color(text))
        .clicked()
    {
        actions.push(ProfileAction::Later);
    }
}

/// The page that picks the square of a new picture: the picture, the
/// square on it following the mouse, a slider for its size, and the
/// buttons that take it or leave.
pub fn square_page(ui: &mut Ui, t: &L10n, square: &mut Square, actions: &mut Vec<ProfileAction>) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.heading(RichText::new(t.profile_crop_title()).color(text));
    ui.label(RichText::new(t.profile_crop_hint()).color(weak));
    ui.add_space(8.0);
    let texture = match &square.texture {
        Some(texture) => texture.clone(),
        None => {
            let Some(image) = pixels(&square.source, 1024) else {
                actions.push(ProfileAction::CancelSquare);
                return;
            };
            let size = [image.width() as usize, image.height() as usize];
            let texture = ui.ctx().load_texture(
                "profile square",
                ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                TextureOptions::LINEAR,
            );
            square.texture = Some(texture.clone());
            texture
        }
    };
    let room = ui.available_width().clamp(120.0, 480.0);
    let scale = (room / square.width as f32).min(room / square.height as f32);
    let shown = egui::vec2(square.width as f32 * scale, square.height as f32 * scale);
    let (rect, response) = ui.allocate_exact_size(shown, Sense::drag());
    egui::Image::new(&texture).paint_at(ui, rect);
    let picked = Rect::from_min_size(
        rect.min + egui::vec2(square.left * scale, square.top * scale),
        egui::vec2(square.edge * scale, square.edge * scale),
    );
    let painter = ui.painter_at(rect);
    let dim = Color32::from_black_alpha(150);
    for part in [
        Rect::from_min_max(rect.min, egui::pos2(rect.max.x, picked.min.y)),
        Rect::from_min_max(egui::pos2(rect.min.x, picked.max.y), rect.max),
        Rect::from_min_max(
            egui::pos2(rect.min.x, picked.min.y),
            egui::pos2(picked.min.x, picked.max.y),
        ),
        Rect::from_min_max(
            egui::pos2(picked.max.x, picked.min.y),
            egui::pos2(rect.max.x, picked.max.y),
        ),
    ] {
        painter.rect_filled(part, 0.0, dim);
    }
    painter.rect_stroke(
        picked,
        0.0,
        Stroke::new(2.0, ui.visuals().selection.stroke.color),
        StrokeKind::Inside,
    );
    if response.dragged() {
        let delta = response.drag_delta() / scale;
        square.place(square.left + delta.x, square.top + delta.y, square.edge);
    }
    ui.add_space(8.0);
    let shortest = square.width.min(square.height) as f32;
    let mut edge = square.edge;
    let slider = ui.add(
        egui::Slider::new(&mut edge, 64.0_f32.min(shortest)..=shortest)
            .show_value(false)
            .text(RichText::new(t.profile_crop_size()).color(text)),
    );
    if slider.changed() {
        let grow = (square.edge - edge) / 2.0;
        square.place(square.left + grow, square.top + grow, edge);
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.profile_crop_use()).color(text))
            .clicked()
        {
            actions.push(ProfileAction::UseSquare);
        }
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            actions.push(ProfileAction::CancelSquare);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copied_id_and_version_with_another_picture_is_another_texture() {
        let profile = |picture: &[u8]| Profile {
            id: "ann".into(),
            name: "Ann".into(),
            picture: Some(picture.to_vec()),
            version: 1,
        };
        assert_eq!(picture_key(&profile(b"one")), picture_key(&profile(b"one")));
        assert_ne!(picture_key(&profile(b"one")), picture_key(&profile(b"two")));
    }
}
