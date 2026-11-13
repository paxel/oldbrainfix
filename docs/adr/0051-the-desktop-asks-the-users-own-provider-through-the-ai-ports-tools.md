# The desktop asks the user's own provider through the AI port's tools

A second brain must answer questions about what it holds, with the Notes the answer came from (#267, Q618, Q619, Q637, Q638, Q632, Q640–Q642). The phone runs no model and talks to no AI provider: a model on the phone is useless and drains it, and a provider in the phone app brings in App Store guideline 5.1.2(i) and the stores' AI policies for little gain. So the desktop is the AI client. Its **Ask panel** asks a provider the user enters, with their own API key; Old Brain Fix provides no model and no account. The protocol and the loop are in the Rust core (`ask.rs`), with no egui in them, so that the phone's question through the desktop (#267, next step) runs the same code; the panel is the desktop's (`ask_view.rs`).

## Two protocols, plain HTTP

- **Anthropic's Messages API** (`https://api.anthropic.com/v1/messages`, `anthropic-version: 2023-06-01`, the key in `x-api-key`). The model is `claude-opus-5-5` while the user names none. Each request sends `thinking: {type: "adaptive"}` and a `max_tokens` of 16 000, never `budget_tokens`, `temperature`, `top_p` nor a forced `tool_choice`. The system prompt and the tools never change, so the system block carries `cache_control: {type: "ephemeral"}`, which caches the tools before it too; the time and the question come in the user's turn. The assistant's content goes back as it came, its thinking blocks included, and every tool result of a turn in one user message. `stop_reason` `tool_use` answers the tools, `pause_turn` asks again as the turn stands, `refusal` shows that the model declined, `max_tokens` (and `model_context_window_exceeded`) shows the answer so far, marked as cut off.
- **OpenAI-compatible Chat Completions** at a base URL the user enters (`<base>/chat/completions`), which DeepSeek, OpenRouter, Ollama, LM Studio and most others speak; Ollama on this computer (`http://localhost:11434/v1`) while none is entered. The tools are function tools; the key goes as `Authorization: Bearer` only when one is entered, as a local model needs none. Arguments come as a JSON text: one that is no JSON object is answered to the model as an error for that call, and the question goes on. `finish_reason` `length` marks the answer as cut off, `content_filter` as declined.

There is no official Anthropic SDK for Rust, and the provider crates bring an async runtime the desktop does not have. The requests are plain HTTPS through `ureq`, which the speech model's download already uses, with no redirect followed, so that the key goes only to the address entered. An `http://` base URL is allowed only on this computer and the home network (a loopback, private or link-local address, `localhost`, a `.local` name), where a local model runs; anything else must be `https://`. Error replies are read with their body, so that a refused key or an unknown model is said in the provider's words.

## The tools: the AI port's, read without an AI Task

The model works through the AI port's tools (ADR 0026), called in-process. Two new tools only read, on the port as well: `find_notes` searches several words at once through the search the lists use (ADR 0048) and gives each Note found with its title, capture time, place, the words it holds and a short excerpt, the Notes holding most words first; `list_new` gives the Notes captured or amended since a day or a time. The panel offers its model a short list (`ASK_TOOLS`): these two, `read_note`, `read_attachment_text`, `list_places`, `list_tags`, `list_routines` and `read_check_offs`, and for filing what is new `file_note`, `add_area`, `add_topic` and `edit_note`. Not `list_notes`, which hands out every Note's text at once, nor `read_photo`, whose pictures the Photo Texts already read: only what the model finds and reads is sent. A short list also leaves room for a small local model's context.

`Store::ask_tool` answers them as the AI sees the store (`Store::for_ai`, ADR 0049), so a Note or Area Hidden from AI is in no request. A tool that reads runs in no AI Task: asking does not pause sync. The first change the model makes opens the AI Task (`Store::open_ai_task`, the one that waits if one does) and is drafted there through the screens' own calls, as the AI port drafts it; the trust rules of ADR 0026 hold unchanged. Sync then waits while that review waits, as today. An AI Task the question opened and left without a change is declined at the end (`Store::drop_empty_ai_task`), so sync goes on.

## The loop off the window's thread

A `Conversation` is the protocol alone: it gives the next request, takes the reply, and says whether the model calls tools, paused, or answered. `ask::run` drives it through a `Transport`, `Web` on the network or a fake in the tests. On the desktop the requests run on a thread of their own; each round of tools crosses a channel to the window, which answers it on its own thread and wakes as the AI port's calls do. A question stopped, or the panel left, drops the channel: the thread's next round of tools hears so, and what it then brings is dropped. At most 24 rounds per question.

## The answer

The model is told to link every Note it uses as an app link (ADR 0046), `[title](oldbrainfix://note/<id>)`. The answer is its Markdown text, the ids of the Notes it links in the order first linked, the ids of the Notes the tools handed out in full, and whether it was cut off: the links are explicit, so that #294 can check each against what the model read. The panel draws the text as a formatted text, each link opening its Note, and lists the Notes it came from by their titles. A question not answered goes back into the field, and why is announced at the top with the provider's own message. An answer is gone once the list shows another place, unless "Keep as Note" saved the question and the answer, with its links, as a new Unsorted Note in Markdown. A follow-up is asked after the earlier questions and answers of the panel, as text.

## The provider is a local setting

`ask:protocol`, and per protocol its model, key and base URL (`ask:anthropic:model`, `ask:anthropic:key`, `ask:openai:url`, `ask:openai:model`, `ask:openai:key`), are local settings of the desktop: never synced and in no Backup, like `ai:token`. Each protocol keeps its own, so switching back finds the key entered before. No tool reads them. The system keyring would protect nothing more, since whoever reads the database reads every Note (ADR 0016), and it fails on a Linux desktop without a Secret Service.

## Considered Options

- Embeddings (rejected, Q642): those from the provider would send every Note to it once and again on each change; a model shipped with the desktop would be one more to ship and update. The full-text index and the model's own searches with synonyms find what they find at a private scale.
- Every tool of the AI port in the panel (rejected: some 60 schemas fill a small local model's context, and `list_notes` would send everything at once).
- An AI Task opened for each question (rejected: sync would pause for every question).
- The provider's SDK or an HTTP crate with an async runtime (rejected: the desktop has none, and the protocol is two JSON requests).
- `http://` to any address (rejected: the key and the Notes read would cross the internet unencrypted).
- Answers kept until the app quits (rejected, Q632: most questions are throwaway, and the log keeps everything forever).

## Consequences

- What the model finds and reads reaches the provider: the names of the Areas, Topics and tags, the Routines and Reminders with their check-offs when it lists them, the titles and excerpts of the Notes found, and the Notes read in full with their Sections, Transcripts, Photo Texts, Attachment Texts and Capture Context. Nothing Hidden from AI does, a count included.
- Asking leaves sync running; only a change the model drafts pauses it, until the review.
- A small local model may search poorly or get a tool's arguments wrong; it hears what went wrong and may try again, and the question ends after 24 rounds.

## The phone asks through the desktop (#267)

Amended by #267 (Q640, Q641): the phone holds no provider and no key; it asks the user's own desktop over the paired connection (ADR 0003), and the desktop runs the question as the panel does. The wire and who may ask are in ADR 0003.

- The desktop asks with its own provider and key, on its clock, through `Store::ask` (`Store::answer_question` in `lan_ask.rs`): the same tools, the same system prompt, Hidden from AI out of every request, the earlier exchanges the phone sends asked before the question, as the panel's follow-ups. A change the model makes is a draft of the AI Task, reviewed on the desktop only; the answer says whether a review now waits, read after the question, so that one that failed after a draft says so too.
- The phone gets the answer's text and the ids of the Notes it links, which open on the phone, and whether it was cut off; not the ids of the Notes the tools read. A question not answered comes back with its kind and, for a refused key, another error of the provider, no reply from it or one that could not be read, the provider's message.
- The key never leaves the desktop: the provider is named to the phone by its protocol, model and host alone, and every message of a failure has the key and the user and query of the address replaced before it goes out, since a provider may repeat the key it refused and an error the address.
- Before the first question the phone names the provider and what is sent and asks the user's permission (guideline 5.1.2(i)); "Keep as Note" saves the question and the answer on the phone as on the desktop. That screen is the phone's.

### Consequences

- A phone's question reads the desktop's Notes, those Hidden from AI aside, and reaches the desktop's provider; the desktop must be running, at home or through the user's VPN.
- One question at a time on the desktop: a second phone asking meanwhile hears that the desktop is busy.
- A phone's question that opened an AI Task and left it without a change declines it at its end, as the panel does; unlike the panel, it does not see whether the AI port's client works in that task meanwhile without having changed anything yet.
