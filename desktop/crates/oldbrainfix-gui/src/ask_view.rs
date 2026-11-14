//! The Ask panel on the desktop's screen (#267, ADR 0051): AI ▸ Ask…
//! (Ctrl+K) shows it in the list. A question goes to the provider the
//! user entered in the Settings, the line under the heading naming it and
//! that only what the model finds and reads is sent; each answer shows
//! with links to the Notes it came from, which open them, and "Keep as
//! Note". What the model changed waits as an AI Task for its review. The
//! answers are gone once the list shows another place.
//!
//! The Settings card where the provider is entered is here too: the
//! protocol, the base URL of an OpenAI-compatible provider, the model and
//! the API key, a local setting of this computer.

use egui::{Id, Key, RichText, TextEdit, Ui};
use oldbrainfix_core::ask::{Answer, AskError, Exchange, Protocol, Provider, Unready};
use oldbrainfix_core::text_format;

use crate::app::{embossed, list_scroll, name_field};
use crate::l10n::L10n;
use crate::text_marks_view::{self, BoxLook};
use crate::theme::{self, Tones};

/// The questions asked in the panel since it showed, the oldest first,
/// and the one being asked.
#[derive(Debug, Default)]
pub struct AskPanel {
    /// What is typed in the question field.
    pub question: String,
    pub exchanges: Vec<Asked>,
    /// The question waiting for its answer.
    pub asking: Option<String>,
    /// Whether the question field takes the keyboard.
    pub focus: bool,
}

impl AskPanel {
    /// Whether nothing was asked nor typed.
    pub fn is_empty(&self) -> bool {
        self.question.is_empty() && self.exchanges.is_empty() && self.asking.is_none()
    }

    /// The questions answered so far, which a follow-up is asked after;
    /// an answer without words is left out, as a provider takes no
    /// empty turn.
    pub fn earlier(&self) -> Vec<Exchange> {
        self.exchanges
            .iter()
            .filter(|asked| !asked.answer.text.trim().is_empty())
            .map(|asked| Exchange {
                question: asked.question.clone(),
                answer: asked.answer.text.clone(),
            })
            .collect()
    }
}

/// A question asked and its answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Asked {
    pub question: String,
    pub answer: Answer,
    /// The Notes its answer links that are here, by id with their titles.
    pub sources: Vec<(String, String)>,
    /// The Note it was kept as.
    pub kept: Option<String>,
}

/// What was asked of the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskAsked {
    /// Asks the question typed.
    Ask,
    /// Stops waiting for the answer.
    Stop,
    /// Drops the questions asked, for a new one.
    StartOver,
    /// Keeps the question and answer at this place as a Note.
    Keep(usize),
    /// Opens a Note an answer came from, or the one it was kept as.
    OpenNote(String),
    /// Shows the review of the AI Task.
    Review,
    /// Opens the Settings at the provider's card.
    Settings,
}

/// What the panel shows besides its questions.
pub struct PanelShown<'a> {
    pub provider: &'a Provider,
    /// Whether an AI Task waits, its changes for review.
    pub task_waits: bool,
    /// Whether its review may show: the AI port is closed.
    pub reviews: bool,
}

/// Why a question was not answered, how to fix it, and what the
/// provider said, for the failure announced (UI laws,
/// "Announcements"); `None` for one stopped, which nobody waits for.
pub fn failure(t: &L10n, provider: &str, why: &AskError) -> Option<(String, String, String)> {
    let said = |message: &str| message.to_string();
    Some(match why {
        AskError::Refused => (
            t.ask_refused().to_string(),
            t.ask_refused_fix().to_string(),
            String::new(),
        ),
        AskError::Key(message) => (
            t.ask_key_refused(provider),
            t.ask_key_fix().to_string(),
            said(message),
        ),
        AskError::Provider(status, message) => (
            t.ask_provider_failed(provider),
            t.ask_provider_fix().to_string(),
            format!("HTTP {status}: {message}"),
        ),
        AskError::Network(message) => (
            t.ask_no_reply(provider),
            t.ask_no_reply_fix().to_string(),
            said(message),
        ),
        AskError::Unreadable(message) => (
            t.ask_unreadable(provider),
            t.ask_unreadable_fix().to_string(),
            said(message),
        ),
        AskError::TooManyRounds => (
            t.ask_too_many_rounds().to_string(),
            t.ask_too_many_rounds_fix().to_string(),
            String::new(),
        ),
        AskError::Stopped => return None,
    })
}

/// The words of why `provider` cannot be asked yet.
pub fn unready_line(t: &L10n, unready: Unready) -> &'static str {
    match unready {
        Unready::NoModel => t.ask_unready_model(),
        Unready::NoKey => t.ask_unready_key(),
        Unready::BadUrl => t.ask_unready_url(),
    }
}

/// The panel: its heading, whom the questions go to and what is sent,
/// the review an AI Task waits for, the question field with Ask (Stop
/// while asking, Start over once something was asked), and the
/// questions asked, the newest first, each with its answer, the Notes it
/// came from and "Keep as Note". A question not answered is announced
/// at the top, by the window.
pub fn page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    panel: &mut AskPanel,
    shown: PanelShown<'_>,
    asked: &mut Vec<AskAsked>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    let provider = shown.provider;
    let unready = provider.unready();
    ui.heading(RichText::new(t.ask_title()).color(text));
    match unready {
        None => {
            ui.label(
                RichText::new(t.ask_sends(&provider.name(), provider.model.trim()))
                    .small()
                    .color(weak),
            );
        }
        Some(why) => {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(unready_line(t, why)).color(error));
                if ui
                    .button(RichText::new(t.ask_open_settings()).color(text))
                    .clicked()
                {
                    asked.push(AskAsked::Settings);
                }
            });
        }
    }
    if shown.task_waits {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t.ask_task_waits()).color(text));
            let review = egui::Button::new(RichText::new(t.ai_review_open()).color(text));
            if ui.add_enabled(shown.reviews, review).clicked() {
                asked.push(AskAsked::Review);
            }
        });
    }
    ui.add_space(6.0);
    let asking = panel.asking.is_some();
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 220.0).max(160.0);
        let field = ui.add_enabled(
            !asking,
            TextEdit::singleline(&mut panel.question)
                .id(Id::new("ask question"))
                .hint_text(t.ask_hint())
                .text_color(text)
                .desired_width(width),
        );
        name_field(ui, &field, t.ask_hint());
        if panel.focus && !asking {
            field.request_focus();
            if !ui.is_sizing_pass() && field.has_focus() {
                panel.focus = false;
            }
        }
        let typed = !panel.question.trim().is_empty();
        let may = unready.is_none() && !asking && typed;
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if asking {
            if ui.button(RichText::new(t.ask_stop()).color(text)).clicked() {
                asked.push(AskAsked::Stop);
            }
        } else if theme::main_action(ui, may, t.ask_button()) || (entered && may) {
            asked.push(AskAsked::Ask);
        }
        if !asking
            && !panel.exchanges.is_empty()
            && ui
                .button(RichText::new(t.ask_start_over()).color(text))
                .clicked()
        {
            asked.push(AskAsked::StartOver);
        }
    });
    ui.add_space(6.0);
    list_scroll(ui, "ask answers", |ui| {
        ui.spacing_mut().item_spacing.y = 10.0;
        if let Some(question) = &panel.asking {
            embossed(ui, tones, false, None, |ui| {
                ui.label(RichText::new(question).strong().color(text));
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(t.ask_asking(&provider.name())).color(weak));
                });
            });
        }
        for (at, exchange) in panel.exchanges.iter().enumerate().rev() {
            embossed(ui, tones, false, None, |ui| {
                ui.label(RichText::new(&exchange.question).strong().color(text));
                answer_rows(ui, t, (at, exchange, &exchange.answer), asked);
            });
        }
        ui.add_space(8.0);
    });
}

/// An answer as Markdown, its links opening the Notes they name, whether
/// it was cut off, whether a link to a Note not read was removed from it
/// (#294), the Notes it came from, and "Keep as Note", or that it was
/// kept.
fn answer_rows(
    ui: &mut Ui,
    t: &L10n,
    (at, exchange, answer): (usize, &Asked, &Answer),
    asked: &mut Vec<AskAsked>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let look = BoxLook::default();
    let spots = text_marks_view::formatted_label(
        ui,
        (&answer.text, text_format::MARKDOWN, &look),
        (0, answer.text.len()),
        (None, text),
        false,
    );
    text_marks_view::link_clicks(ui, Id::new(("ask answer", at)), &spots.links);
    if answer.cut {
        ui.label(RichText::new(t.ask_cut()).small().color(weak));
    }
    if answer.unchecked {
        ui.label(RichText::new(t.ask_unchecked()).small().color(weak));
    }
    if !exchange.sources.is_empty() {
        ui.label(RichText::new(t.ask_from_notes()).small().color(weak));
        for (id, title) in &exchange.sources {
            let link = ui
                .link(RichText::new(title).color(ui.visuals().hyperlink_color))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if link.clicked() {
                asked.push(AskAsked::OpenNote(id.clone()));
            }
        }
    }
    match &exchange.kept {
        None => {
            if ui.button(RichText::new(t.ask_keep()).color(text)).clicked() {
                asked.push(AskAsked::Keep(at));
            }
        }
        Some(note) => {
            if ui
                .link(RichText::new(t.ask_kept()).color(ui.visuals().hyperlink_color))
                .clicked()
            {
                asked.push(AskAsked::OpenNote(note.clone()));
            }
        }
    }
}

/// What the provider's card edits: the protocol, and the base URL, model
/// and key as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskFields {
    pub protocol: Protocol,
    pub url: String,
    pub model: String,
    pub key: String,
}

impl AskFields {
    /// The fields of `provider`; its base URL and model empty where they
    /// are the defaults, which the fields show as their hints.
    pub fn of(provider: &Provider) -> AskFields {
        let default_model = match provider.protocol {
            Protocol::Anthropic => oldbrainfix_core::ask::ANTHROPIC_MODEL,
            Protocol::OpenAi => "",
        };
        AskFields {
            protocol: provider.protocol,
            url: match provider.url.as_str() {
                oldbrainfix_core::ask::OPENAI_URL | oldbrainfix_core::ask::ANTHROPIC_URL => {
                    String::new()
                }
                url => url.to_string(),
            },
            model: if provider.model == default_model {
                String::new()
            } else {
                provider.model.clone()
            },
            key: provider.key.clone(),
        }
    }
}

/// A change made on the provider's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskSetting {
    Protocol(Protocol),
    Url(String),
    Model(String),
    Key(String),
}

/// The provider's card in the Settings: what it is for, the protocol as
/// radio buttons, the base URL of an OpenAI-compatible provider, the
/// model and the API key, hidden as it is typed, and that the key stays
/// on this computer; under the base URL why it cannot be asked, when it
/// cannot. `at` scrolls to it, as the Ask panel asks when the provider
/// cannot be asked.
pub fn settings_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    fields: &mut AskFields,
    at: bool,
) -> Option<AskSetting> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut changed = None;
    embossed(ui, tones, false, None, |ui| {
        let title = ui.label(RichText::new(t.ask_settings_title()).strong().color(text));
        if at {
            title.scroll_to_me_animation(
                Some(egui::Align::TOP),
                egui::style::ScrollAnimation::none(),
            );
        }
        ui.label(RichText::new(t.ask_settings_intro()).small().color(weak));
        let mut protocol = fields.protocol;
        let options = [
            (Protocol::Anthropic, t.ask_anthropic().to_string()),
            (Protocol::OpenAi, t.ask_openai().to_string()),
        ];
        if crate::app::radio_column(ui, &mut protocol, options) && protocol != fields.protocol {
            changed = Some(AskSetting::Protocol(protocol));
        }
        let field = |ui: &mut Ui,
                     value: &mut String,
                     (id, label, hint): (&str, &str, &str),
                     password: bool| {
            ui.label(RichText::new(label).color(text));
            let edit = ui.add(
                TextEdit::singleline(value)
                    .id(Id::new(("ask setting", id)))
                    .hint_text(hint)
                    .password(password)
                    .text_color(text)
                    .desired_width(ui.available_width().min(420.0)),
            );
            name_field(ui, &edit, label);
            edit.changed()
        };
        if fields.protocol == Protocol::OpenAi {
            let hint = oldbrainfix_core::ask::OPENAI_URL;
            if field(ui, &mut fields.url, ("url", t.ask_url(), hint), false) {
                changed = Some(AskSetting::Url(fields.url.clone()));
            }
            ui.label(RichText::new(t.ask_url_detail()).small().color(weak));
            let url = match fields.url.trim() {
                "" => hint,
                url => url,
            };
            if !oldbrainfix_core::ask::url_allowed(url) {
                let error = ui.visuals().error_fg_color;
                ui.label(RichText::new(t.ask_unready_url()).color(error));
            }
        }
        let model_hint = match fields.protocol {
            Protocol::Anthropic => oldbrainfix_core::ask::ANTHROPIC_MODEL,
            Protocol::OpenAi => t.ask_model_hint(),
        };
        if field(
            ui,
            &mut fields.model,
            ("model", t.ask_model(), model_hint),
            false,
        ) {
            changed = Some(AskSetting::Model(fields.model.clone()));
        }
        let key_hint = match fields.protocol {
            Protocol::Anthropic => "",
            Protocol::OpenAi => t.ask_key_none(),
        };
        if field(ui, &mut fields.key, ("key", t.ask_key(), key_hint), true) {
            changed = Some(AskSetting::Key(fields.key.clone()));
        }
        ui.label(RichText::new(t.ask_key_stays()).small().color(weak));
    });
    changed
}
