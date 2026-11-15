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

The model is told to link every Note it uses as an app link (ADR 0046), `[title](oldbrainfix://note/<id>)`. The answer is its Markdown text, the ids of the Notes it links in the order first linked, the ids of the Notes the tools handed out in full, whether it was cut off, and whether a source could not be checked (see "Answers only from the Notes read" below). The panel draws the text as a formatted text, each link opening its Note, and lists the Notes it came from by their titles. A question not answered goes back into the field, and its cause and fix stay under it until the next is asked, the provider's own message behind Details (#317). An answer is gone once the list shows another place, unless "Keep as Note" saved the question and the answer, with its links, as a new Unsorted Note in Markdown. A follow-up is asked after the earlier questions and answers of the panel, as text.

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
- The phone gets the answer's text and the ids of the Notes it links, which open on the phone, whether it was cut off, and whether a source could not be checked (`unchecked`, #294; a reply without it is one with none removed). Amended by #314: it also gets the ids of the Notes the model read in full (`read`), for "Also read". A question not answered comes back with its kind and, for a refused key, another error of the provider, no reply from it or one that could not be read, the provider's message.
- The key never leaves the desktop: the provider is named to the phone by its protocol, model and host alone, and every message of a failure has the key and the user and query of the address replaced before it goes out, since a provider may repeat the key it refused and an error the address.
- Before the first question the phone names the provider and what is sent and asks the user's permission (guideline 5.1.2(i)); "Keep as Note" saves the question and the answer on the phone as on the desktop. That screen is the phone's.

### Consequences

- A phone's question reads the desktop's Notes, those Hidden from AI aside, and reaches the desktop's provider; the desktop must be running, at home or through the user's VPN.
- One question at a time on the desktop: a second phone asking meanwhile hears that the desktop is busy.
- A phone's question that opened an AI Task and left it without a change declines it at its end, as the panel does; unlike the panel, it does not see whether the AI port's client works in that task meanwhile without having changed anything yet.

## What it is doing and why it failed (#317)

Amended by #317 (Q722): a question that came back after a second with nothing said looked like one never asked.

- While the provider cannot be asked, no model, no key, an address it may not ask, or "AI (experimental)" off, the question field says so with the fix and Ask cannot be pressed.
- While a question runs, a status line under it says whom it asks and how many Notes the model has read in full so far, with Stop. The count follows the one rule of what a question has read (`ask::note_read`), the same that checks the answer's links: the panel counts the tools it answers on the window's thread, `Store::ask_with` counts for the phone's question.
- The phone learns the count by asking `POST /ask/status` every two seconds by the id it gave the question, and its Stop sends `POST /ask/stop` and drops what comes for that question later (the wire in ADR 0003). A stream of the answer was rejected: the host replies once per request, and a phone of 0.6.0 reads one body.
- A failure stays under its question with its cause and fix until the next question is asked, on the phone and the panel alike, rather than as an announcement at the top, which waits unseen behind another and says only that a question was not answered. Its Details open the cause, the fix and the provider's full message, with Report.
- The phone keeps one connection to its computer for the page (`openAsk`): one made anew whenever the route builds its page again, as it does when an announcement shows at the top, had forgotten the computer, and a question waiting then went nowhere.

## Answers only from the Notes read (#294)

Amended by #294 (Q690, Q691): a plausible answer from general knowledge, such as a date the model guessed, is the worst one for a forgetful user, and the panel answers "what did I write, when, where"; a general question belongs in the provider's own chat.

- The system prompt tells the model to answer only from the Notes it read with `read_note` in this question, never from general knowledge or a guess: an excerpt `find_notes` or `list_new` gives, or a Note an earlier answer named, counts only once the model read that Note in this question; the Areas, Topics, tags, Routines and check-offs the other tools list count too. It links the Note of every statement taken from a Note, and a report of its changes links only Notes it read, and answers "That's not in your Notes", in the language of the question, when the Notes it read hold nothing about it. The prompt stays the same for every question, so that it stays cached.
- A Note counts as read in this question when `read_note` or `read_attachment_text` handed it out without an error. A tool that only names a Note, such as `read_check_offs`, answers for one Hidden from AI or one that does not exist without an error, so it reads none.
- When the model has answered, the core checks every app link to a Note in the text (`ask::checked`, in `Conversation`, so the panel's thread, `Store::ask` and the phone's question all pass through it): a link to a Note not read in this question, which includes every Note Hidden from AI and any id that names none, is removed and its label's words stay as plain text, and the answer is marked `unchecked`. A link's label is read without links, so the check runs again on the text until it removes nothing. Links to other places of the app, to the web, and text in code stay. The Notes the answer links are then read from the checked text, so they are all Notes read. An answer without such a link is not changed and not marked.
- The panel shows "A source could not be checked" under a marked answer, and the phone the same under one marked in the reply. "Keep as Note" keeps the checked text.

### Considered Options

- Dropping a removed link with its words (rejected: the sentence around it would no longer read, and the mark already says that a source is missing).
- Replacing a marked answer with "That's not in your Notes" (rejected, Q691: an answer may hold sources read beside one that is not, and nothing the user could use is thrown away).
- Counting the Notes `find_notes` and `list_new` show as read (rejected: an excerpt is not the Note, and the model is told to read what it uses).

## A small model grounds its answers; "Also read" (#314)

Amended by #314 (Q719): with `qwen2.5:3b`, the end to end check of [obf-1024](https://github.com/paxel/oldbrainfix/issues/331) found the model answering "Your passport is in the blue folder in the desk drawer." from the excerpt of `find_notes`, never calling `read_note`, and answering "That's not in your Notes" to the wifi and the German tyres question without searching at all. Nothing was read, so nothing could be linked, and no source showed (7 of 11 checks green, the same 4 red every time).

- **The system prompt** is five numbered steps in short sentences with a worked example, the rules of #294 unchanged: search with `find_notes` first, always, giving it the question's words one by one; read every Note used with `read_note`, since what `find_notes` and `list_new` show is not enough; answer only from what was read; link every Note used; say "That's not in your Notes" only after a search found nothing. It stays the same for every question, so that it stays cached.
- **Told once to search, told once to read.** The prompt alone did not do it: the model makes one round of tools and then answers. So `Conversation` checks each answer before it counts. When the model answers having called no tool at all, it hears once, as the user's turn, `ask::SEARCH_FIRST`: "Search the Notes first: call find_notes with the main words of my question". When it answers having read no Note, and `find_notes` or `list_new` showed it some, it hears once to call `read_note` with `{"id":"note:…"}` for each Note it uses, the first three shown named; the call is spelt out because the model otherwise passed the id as `note`, the key of `read_attachment_text`. Then its next answer counts, whatever it says. An answer cut off at its length is never sent back, nor one after a search that found nothing, nor one after another tool, since an answer from the Areas, tags or Routines those list counts. Its first answer stays in the conversation before the word. That is at most two more rounds, within the 24, and a model that reads before it answers, as Claude does, never hears either.
- **What counts as read is unchanged** (`ask::note_read`): a Note an excerpt showed does not count, and no answer that quotes an excerpt is counted as having read it. "Also read" names what the model read in full, the same Notes the links are checked against.
- **"Also read".** Under each answer, the Notes it links come first ("From these Notes"), then "Also read" with the other Notes read in full for this question, in the order read (`Answer::also_read`), each opening its Note on a click or a tap. A small model often reads a Note and answers from it without a link; the user still sees where the answer came from. The phone gets the ids read as `read` in the answer (ADR 0003) and shows them through the same check as the links: none it does not hold, in the Trash, merged, or in a locked Area while it is closed. A phone of 0.6.0 ignores `read`; from a desktop of 0.6.0 it is missing and nothing shows under "Also read".

### Considered Options

- `find_notes` giving fewer and shorter excerpts that say "read_note for the full text" (rejected: with a hint in the output, a title cut to three words, or only the ids, the model still answered without reading, from a guess when it saw no text; a short Note's title is its whole text anyway).
- Counting a Note whose excerpt the answer quotes as "Also read" (rejected: an excerpt seen is not the Note read, #294 rejected counting it, and "Also read" would then promise more than the links' check holds).
- Forcing a tool with `tool_choice` (rejected: Ollama ignored it, and Anthropic's adaptive thinking takes no forced tool).
- Searching a phrase's words one by one when the phrase finds nothing (left out: once the prompt asks for the words one by one, the model gave them so).
- "In the language of my question" in the two words to the model (rejected: with it the model answered again without reading more often; the system prompt already says so).

### Consequences

- An answer before any tool, and one from an excerpt, cost one more request each; a provider that reads first pays nothing more.
- On `qwen2.5:3b` the passport, wifi and tyres questions are read and answered from their Notes; whether the answer also links them still depends on the model, which "Also read" covers.

