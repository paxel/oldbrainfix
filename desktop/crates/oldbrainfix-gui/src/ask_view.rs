//! The Ask panel on the desktop's screen (#267, ADR 0051): AI ▸ Ask…
//! (Ctrl+K) shows it in the list. A question goes to the provider the
//! user entered in the AI settings (#335), the line under the heading
//! naming it and that only what the model finds and reads is sent; while
//! it runs, a status line under it says whom it asks and how many Notes
//! the model has read, and a question not answered stays under its
//! question with its cause and fix until the next is asked (#317). The
//! questions stand as a thread, the oldest first, the question field
//! under it (#315): each with "Edit", which puts it into the field to
//! ask again, and Up in the empty field fetches the last one. Each
//! answer shows with links to the Notes it came from, then to the other
//! Notes the model read for it under "Also read" (#314), which open them,
//! and "Copy as text", "Keep as Note", "Add to Note…" and "Save as
//! Markdown…" (#316). What the model changed waits as an AI Task for its
//! review. The thread is gone once the list shows another place.
//!
//! The AI settings' card where the provider is entered is here too: the
//! protocol, the base URL of an OpenAI-compatible provider, the model and
//! the API key, a local setting of this computer.

use egui::text::{CCursor, CCursorRange};
use egui::{Align, Id, Key, Layout, RichText, TextEdit, Ui};
use oldbrainfix_core::ask::{Answer, AskError, Exchange, Protocol, Provider, Unready};
use oldbrainfix_core::share::NoteShare;
use oldbrainfix_core::{markdown, text_format};

use crate::app::{embossed, name_field};
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
    /// How many Notes the model read in full for it so far (#317).
    pub read: usize,
    /// The last question not answered, with why, until the next is asked
    /// (#317).
    pub failed: Option<AskFailed>,
    /// Whether the question field takes the keyboard.
    pub focus: bool,
    /// Whether the cursor goes to the end of the field's text, put there
    /// by Edit or Up (#315).
    pub to_end: bool,
    /// Whether the thread scrolls to its newest question, once it was
    /// asked or came back (#315).
    pub scroll: bool,
}

/// A question not answered (#317): why, how to fix it, and what the
/// provider said, which "Details…" shows with Report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskFailed {
    pub question: String,
    pub cause: String,
    pub fix: String,
    pub details: String,
}

impl AskPanel {
    /// Whether nothing was asked nor typed.
    pub fn is_empty(&self) -> bool {
        self.question.is_empty()
            && self.exchanges.is_empty()
            && self.asking.is_none()
            && self.failed.is_none()
    }

    /// The last question asked, which Up in the empty field fetches
    /// (#315): the one not answered, else the newest answered.
    pub fn last_question(&self) -> Option<&str> {
        match &self.failed {
            Some(failed) => Some(&failed.question),
            None => self.exchanges.last().map(|asked| asked.question.as_str()),
        }
    }

    /// Puts `question` into the field to be asked again, the cursor at
    /// its end (#315).
    pub fn put(&mut self, question: &str) {
        self.question = question.to_string();
        self.focus = true;
        self.to_end = true;
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
    /// The other Notes the model read in full for it that are here, in
    /// the order read, by id with their titles: "Also read" (#314).
    pub also_read: Vec<(String, String)>,
    /// The Note it was kept as.
    pub kept: Option<String>,
    /// The Note it was added to last with Add to Note… (#316), by id with
    /// its title.
    pub added: Option<(String, String)>,
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
    /// Puts the answer at this place on the clipboard (#316).
    Copy(usize),
    /// Adds the question and answer at this place to a Note picked in Add
    /// to Note… (#316).
    AddToNote(usize),
    /// Saves the answer at this place as a Markdown file, the desktop's
    /// Share to an app (#316).
    Save(usize),
    /// Puts the question at this place into the field (#315).
    Edit(usize),
    /// Opens a Note an answer came from, or the one it was kept as.
    OpenNote(String),
    /// Shows the review of the AI Task.
    Review,
    /// Opens the AI settings at the provider's card (#335).
    Settings,
    /// Shows why the question failed, with the provider's message and
    /// Report (#317).
    Details,
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
/// provider said, shown under its question (#317); `None` for one
/// stopped, which nobody waits for.
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
        Unready::AiOff => t.ask_unready_ai_off(),
    }
}

/// The panel: its heading, whom the questions go to and what is sent,
/// the review an AI Task waits for, the questions asked as a thread, the
/// oldest first (#315): each answered with "Edit", its answer, the Notes
/// it came from and what can be done with it (#316), then the one asked
/// with its status line, or the last one not answered with its cause and
/// fix (#317);
/// under the thread the question field with Ask (Stop while asking,
/// Start over once something was asked), and under it what the provider
/// lacks while it cannot be asked.
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
    let provider = shown.provider;
    let unready = provider.unready();
    ui.heading(RichText::new(t.ask_title()).color(text));
    if unready.is_none() {
        ui.label(
            RichText::new(t.ask_sends(&provider.name(), provider.model.trim()))
                .small()
                .color(weak),
        );
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
    // The field under the thread: laid out from the bottom up, so the
    // thread takes the room left above it.
    ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
        ui.add_space(8.0);
        field_rows(ui, t, panel, provider, asked);
        ui.add_space(6.0);
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            thread(ui, t, tones, panel, provider, asked);
        });
    });
}

/// The question field with Ask, Stop or Start over, and under it what
/// the provider lacks while it cannot be asked (#317). Up in the empty
/// field fetches the last question (#315).
fn field_rows(
    ui: &mut Ui,
    t: &L10n,
    panel: &mut AskPanel,
    provider: &Provider,
    asked: &mut Vec<AskAsked>,
) {
    let text = ui.visuals().text_color();
    let error = ui.visuals().error_fg_color;
    let unready = provider.unready();
    let asking = panel.asking.is_some();
    let id = Id::new("ask question");
    // Laid out from the bottom up: the line under the field comes first.
    // The field says what the provider lacks, and Ask waits (#317).
    if let Some(why) = unready {
        ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(unready_line(t, why)).color(error));
                if ui
                    .button(RichText::new(t.ask_open_settings()).color(text))
                    .clicked()
                {
                    asked.push(AskAsked::Settings);
                }
            });
        });
    }
    // Up in the empty field fetches the last question (#315).
    let typing = ui.memory(|m| m.has_focus(id));
    if typing
        && !asking
        && panel.question.is_empty()
        && ui.input(|i| i.key_pressed(Key::ArrowUp))
        && let Some(last) = panel.last_question().map(str::to_string)
    {
        panel.put(&last);
    }
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 220.0).max(160.0);
        let field = ui.add_enabled(
            !asking,
            TextEdit::singleline(&mut panel.question)
                .id(id)
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
        if std::mem::take(&mut panel.to_end)
            && let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id)
        {
            let end = CCursor::new(panel.question.chars().count());
            state.cursor.set_char_range(Some(CCursorRange::one(end)));
            state.store(ui.ctx(), id);
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
}

/// The questions asked, the oldest first (#315), each answered with
/// "Edit", greyed while a question is asked; then the one asked with its
/// status line, or the last not answered with its cause and fix (#317).
/// Once a question was asked or came back, it scrolls to the newest.
fn thread(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    panel: &mut AskPanel,
    provider: &Provider,
    asked: &mut Vec<AskAsked>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    let asking = panel.asking.is_some();
    let scroll = std::mem::take(&mut panel.scroll);
    // The newest: the one asked or not answered after the answers, else
    // the last answered.
    let newest = panel.exchanges.len().saturating_sub(usize::from(
        panel.asking.is_none() && panel.failed.is_none(),
    ));
    egui::ScrollArea::vertical()
        .id_salt("ask thread")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            for (at, exchange) in panel.exchanges.iter().enumerate() {
                if scroll && at == newest {
                    ui.scroll_to_cursor(Some(Align::TOP));
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&exchange.question).strong().color(text));
                        let edit = egui::Button::new(RichText::new(t.edit()).color(text));
                        if ui.add_enabled(!asking, edit).clicked() {
                            asked.push(AskAsked::Edit(at));
                        }
                    });
                    answer_rows(ui, t, (at, exchange, &exchange.answer), asked);
                });
            }
            if scroll && newest == panel.exchanges.len() {
                ui.scroll_to_cursor(Some(Align::TOP));
            }
            if let Some(question) = &panel.asking {
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(question).strong().color(text));
                    ui.horizontal(|ui| {
                        ui.spinner();
                        let status = match panel.read {
                            0 => t.ask_asking(&provider.name()),
                            read => t.ask_asking_reading(&provider.name(), read as i64),
                        };
                        ui.label(RichText::new(status).color(weak));
                    });
                });
            } else if let Some(failed) = &panel.failed {
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(&failed.question).strong().color(text));
                    ui.label(RichText::new(&failed.cause).color(error));
                    ui.label(RichText::new(&failed.fix).color(text));
                    if ui
                        .button(RichText::new(t.ask_failure_details()).color(text))
                        .clicked()
                    {
                        asked.push(AskAsked::Details);
                    }
                });
            }
        });
}

/// An answer as Markdown, its links opening the Notes they name, whether
/// it was cut off, whether a link to a Note not read was removed from it
/// (#294), the Notes it came from, then those the model also read
/// (#314), and in one row "Copy as text", "Keep as Note" or that it was
/// kept, "Add to Note…" and "Save as Markdown…" (#316), Copy and Save only
/// for an answer with words, then the Note it was added to last.
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
    for (heading, notes) in [
        (t.ask_from_notes(), &exchange.sources),
        (t.ask_also_read(), &exchange.also_read),
    ] {
        if notes.is_empty() {
            continue;
        }
        ui.label(RichText::new(heading).small().color(weak));
        for (id, title) in notes {
            let link = ui
                .link(RichText::new(title).color(ui.visuals().hyperlink_color))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if link.clicked() {
                asked.push(AskAsked::OpenNote(id.clone()));
            }
        }
    }
    // An answer without words has nothing to copy or save.
    let words = !answer.text.trim().is_empty();
    ui.horizontal_wrapped(|ui| {
        if words
            && ui
                .button(RichText::new(t.copy_as_text()).color(text))
                .clicked()
        {
            asked.push(AskAsked::Copy(at));
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
        if ui
            .button(RichText::new(t.add_to_note()).color(text))
            .clicked()
        {
            asked.push(AskAsked::AddToNote(at));
        }
        if words
            && ui
                .button(RichText::new(t.save_as_markdown()).color(text))
                .clicked()
        {
            asked.push(AskAsked::Save(at));
        }
    });
    if let Some((note, title)) = &exchange.added
        && ui
            .link(RichText::new(t.ask_added_to(title)).color(ui.visuals().hyperlink_color))
            .clicked()
    {
        asked.push(AskAsked::OpenNote(note.clone()));
    }
}

/// What the answer `text` hands to another app (#316), as a Note's
/// formatted text goes (#129): its Markdown source with each item that
/// starts with a box as a task list item, its links as written, and
/// beside it the HTML, whose app links are their labels (ADR 0046).
pub fn answer_share(text: &str) -> NoteShare {
    let text = text.trim();
    let tasks = markdown::task_lists(text);
    NoteShare {
        text: tasks.clone(),
        html: Some(markdown::html(text)),
        markdown: tasks,
        photos: Vec::new(),
        recordings: Vec::new(),
    }
}

/// What Keep as Note keeps and Add to Note… adds of a question and its
/// answer (#316): the question its first line, the answer with its links
/// after it, as Markdown.
pub fn kept_text(asked: &Asked) -> String {
    format!("{}\n\n{}", asked.question.trim(), asked.answer.text.trim())
}

/// What the provider's card edits: the protocol, the base URL, model
/// and key as typed, and "Let the model think" (#318).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskFields {
    pub protocol: Protocol,
    pub url: String,
    pub model: String,
    pub key: String,
    pub thinks: bool,
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
            thinks: provider.thinks,
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
    /// "Let the model think" switched on or off (#318).
    Thinks(bool),
}

/// The provider's card in the AI settings (#335): what it is for, the protocol as
/// radio buttons, the base URL of an OpenAI-compatible provider, the
/// model and the API key, hidden as it is typed, and that the key stays
/// on this computer; under the base URL why it cannot be asked, when it
/// cannot; for an OpenAI-compatible provider "Let the model think", and
/// last the models tried with Ask, with their size and what they need
/// (#318). `at` scrolls to it, as the Ask panel asks when the provider
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
        // In the words the search of the Settings' row "AI…" finds
        // (`ai_settings_words` in app.rs).
        if fields.protocol == Protocol::OpenAi {
            let mut thinks = fields.thinks;
            if theme::checkbox(ui, &mut thinks, RichText::new(t.ask_thinks()).color(text)).changed()
            {
                fields.thinks = thinks;
                changed = Some(AskSetting::Thinks(thinks));
            }
            ui.label(RichText::new(t.ask_thinks_detail()).small().color(weak));
        }
        ui.add_space(4.0);
        ui.label(RichText::new(t.ask_models_title()).color(text));
        ui.label(RichText::new(t.ask_models_intro()).small().color(weak));
        for model in [t.ask_model_qwen25(), t.ask_model_qwen3()] {
            ui.label(RichText::new(model).small().color(weak));
        }
    });
    changed
}
