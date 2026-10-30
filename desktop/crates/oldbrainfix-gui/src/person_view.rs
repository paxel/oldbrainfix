//! Another person's Profile on the desktop's screen (ADR 0017, #52):
//! their own picture and name, their short code and whether the user
//! compared it in person, with "Compare codes"; "Their Notes" (#53);
//! then "Name and picture for me", the Override only the user sees, on
//! their own Devices; last, Block, or Unblock for a blocked person (#90).
//! Just added from their Profile card, it offers "Send yours back" (#107).
//! When changes of the Areas shared with them travel as files, "Send
//! changes to Ann" saves a bundle for all of their Devices (#108); once
//! saved, the Changes Nudge no longer waits for them (#109).

use egui::{Id, RichText, TextEdit, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::{Profile, Store};

use crate::l10n::L10n;
use crate::profile_view::{Pictures, display_name, person_button, person_picture};

/// What the user asked for on a person's Profile.
#[derive(Debug, Clone, PartialEq)]
pub enum PersonAction {
    /// Keep the name typed for the user alone.
    Rename(String),
    /// Choose a picture file for the user alone.
    PickPicture,
    /// Drop the picture for the user; their own shows again.
    TheirPicture,
    /// Take the square picked as the picture for the user, or leave it.
    UseSquare,
    CancelSquare,
    /// Ask to compare their short code in person, and what that came to.
    CompareCodes,
    CodesCompared(bool),
    /// Show All Notes with only theirs (#53).
    TheirNotes,
    /// Ask whether to block them, or to end their block (#90).
    Block,
    Unblock,
    /// Put the own Profile card, as the answer to theirs, on the clipboard
    /// (#107).
    SendBack,
    /// Save a bundle of what is new for them in the Areas shared with
    /// them, to send (#108).
    SendChanges,
}

/// Where "Send yours back" stands on a person's Profile (#107).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendBack {
    /// Not offered: they were not just added from a card asking for it.
    No,
    Offered,
    /// The card went to the clipboard; what to do with it shows instead.
    Copied,
}

/// Where comparing a person's short code stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparing {
    Not,
    Asking,
    Differ,
}

/// A person's Profile as its view shows it, read once per change.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonView {
    /// With their own name and picture.
    pub theirs: Profile,
    /// As the user sees them, their Override applied.
    pub shown: Profile,
    pub code: Option<String>,
    pub verified: bool,
    /// Whether the user gave them a picture.
    pub own_picture: bool,
    /// Whether the user blocked them (ADR 0021).
    pub blocked: bool,
    /// Whether "Send changes" shows: changes travel with them as files,
    /// they are not blocked and an Area is shared with a Device of
    /// theirs (#108).
    pub sends_changes: bool,
    /// Whether a newer version of the app saved their Profile in a form
    /// this one does not know (ADR 0034).
    pub newer: bool,
    /// Whether it saved the name and picture given to them so: then they
    /// show and are not changed here.
    pub override_newer: bool,
}

impl PersonView {
    /// The person `id` as `store` keeps them.
    pub fn read(store: &Store, id: &str) -> oldbrainfix_core::Result<PersonView> {
        let blocked = store.is_blocked(id)?;
        Ok(PersonView {
            theirs: store.person_theirs(id)?,
            shown: store.person_shown(id)?,
            code: store.person_code(id)?,
            verified: store.person_verified(id)?,
            own_picture: store
                .person_override(id)?
                .is_some_and(|o| o.picture.is_some()),
            blocked,
            sends_changes: store.sends_by_bundle(id)?,
            newer: store.person_is_newer(id)?,
            override_newer: store.override_is_newer(id)?,
        })
    }
}

/// The person's Profile: their picture, name and code, verified or not,
/// "Compare codes", "Send yours back" while `back` offers it, "Send
/// changes" when changes travel with them as files, and the
/// name and picture for the user, with the field `name` for the name;
/// then Block, or Unblock.
pub fn form(
    ui: &mut Ui,
    t: &L10n,
    pictures: &mut Pictures,
    person: &PersonView,
    (comparing, name, back): (Comparing, &mut String, SendBack),
    actions: &mut Vec<PersonAction>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let theirs = display_name(t, &person.theirs.name).to_string();
    ui.horizontal(|ui| {
        person_picture(ui, pictures, &person.theirs, 72.0, &theirs);
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.heading(RichText::new(&theirs).color(text));
            if let Some(code) = &person.code {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.profile_their_code()).color(weak));
                    ui.label(RichText::new(code).monospace().size(16.0).color(text));
                });
            }
        });
    });
    if person.newer {
        ui.label(RichText::new(t.newer_version_needed()).color(weak));
    }
    if person.blocked {
        ui.label(RichText::new(t.person_blocked()).color(text));
    }
    let verified = if person.verified {
        t.person_verified()
    } else {
        t.person_not_verified()
    };
    ui.label(RichText::new(verified).color(text));
    if let Some(code) = person.code.as_deref().filter(|_| !person.verified) {
        match comparing {
            Comparing::Asking => {
                ui.label(RichText::new(t.sent_item_compare_hint(&theirs, code)).color(text));
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t.sent_item_codes_match()).color(text))
                        .clicked()
                    {
                        actions.push(PersonAction::CodesCompared(true));
                    }
                    if ui
                        .button(RichText::new(t.sent_item_codes_differ()).color(text))
                        .clicked()
                    {
                        actions.push(PersonAction::CodesCompared(false));
                    }
                });
            }
            Comparing::Not | Comparing::Differ => {
                if comparing == Comparing::Differ {
                    ui.label(RichText::new(t.person_codes_differed()).color(text));
                }
                if ui
                    .button(RichText::new(t.sent_item_compare()).color(text))
                    .clicked()
                {
                    actions.push(PersonAction::CompareCodes);
                }
            }
        }
    }
    if !person.blocked {
        let shown = display_name(t, &person.shown.name);
        match back {
            SendBack::No => {}
            SendBack::Offered => {
                ui.label(RichText::new(t.person_send_back_detail(shown)).color(text));
                if ui
                    .button(RichText::new(t.person_send_back()).color(text))
                    .clicked()
                {
                    actions.push(PersonAction::SendBack);
                }
            }
            SendBack::Copied => {
                ui.label(RichText::new(t.person_send_back_copied(shown)).color(text));
            }
        }
    }
    if person.sends_changes {
        let shown = display_name(t, &person.shown.name);
        ui.label(RichText::new(t.person_send_changes_detail()).color(weak));
        if ui
            .button(RichText::new(t.person_send_changes(shown)).color(text))
            .clicked()
        {
            actions.push(PersonAction::SendChanges);
        }
    }
    if ui
        .button(RichText::new(t.person_their_notes()).color(text))
        .clicked()
    {
        actions.push(PersonAction::TheirNotes);
    }
    ui.separator();
    ui.label(RichText::new(t.person_for_me()).strong().color(text));
    ui.label(RichText::new(t.person_for_me_detail()).small().color(weak));
    // Saved by a newer version of the app: shown, not changed here (ADR
    // 0034).
    if person.override_newer {
        ui.horizontal(|ui| {
            person_picture(ui, pictures, &person.shown, 56.0, t.person_for_me_picture());
            ui.add_space(12.0);
            ui.label(RichText::new(display_name(t, &person.shown.name)).color(text));
        });
        ui.label(RichText::new(t.newer_version_needed()).color(weak));
    } else {
        for_me(ui, t, pictures, person, (name, &theirs), actions);
    }
    ui.separator();
    let (label, asked) = if person.blocked {
        (t.person_unblock(), PersonAction::Unblock)
    } else {
        (t.person_block(), PersonAction::Block)
    };
    if ui.button(RichText::new(label).color(text)).clicked() {
        actions.push(asked);
    }
}

/// The picture and the field `name` the user gives the person for
/// themselves alone, `theirs` the name they go by.
fn for_me(
    ui: &mut Ui,
    t: &L10n,
    pictures: &mut Pictures,
    person: &PersonView,
    (name, theirs): (&mut String, &str),
    actions: &mut Vec<PersonAction>,
) {
    let text = ui.visuals().text_color();
    ui.horizontal(|ui| {
        if person_button(ui, pictures, &person.shown, 56.0, t.person_for_me_picture()).clicked() {
            actions.push(PersonAction::PickPicture);
        }
        ui.add_space(12.0);
        ui.vertical(|ui| {
            if ui
                .button(RichText::new(t.profile_picture_choose()).color(text))
                .clicked()
            {
                actions.push(PersonAction::PickPicture);
            }
            if person.own_picture
                && ui
                    .button(RichText::new(t.person_their_picture()).color(text))
                    .clicked()
            {
                actions.push(PersonAction::TheirPicture);
            }
        });
    });
    ui.label(RichText::new(t.person_for_me_name()).color(text));
    let field = ui.add(
        TextEdit::singleline(name)
            .id(Id::new("person name"))
            .hint_text(theirs)
            .desired_width(320.0),
    );
    field.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, t.person_for_me_name()));
    if field.lost_focus() {
        actions.push(PersonAction::Rename(name.clone()));
    }
}
