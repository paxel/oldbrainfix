# Ask end to end

How AI coding agents check an Ask change against a real local model ([obf-1024](https://github.com/paxel/oldbrainfix/issues/331)). The fakes in `tests/ask.rs`, `lan_ask/tests.rs` and `ask/tests.rs` prove the protocol, not that a model can use it. Every Ask ticket is also run through this check, and its report carries the output and the screenshots.

## Run it

```sh
tool/ask-e2e.sh run                 # build, start Ollama, run every check, stop
tool/ask-e2e.sh run the_phone       # only the tests whose name holds the filter
tool/ask-e2e.sh start               # start Ollama and pull the model, then…
tool/ask-e2e.sh test [filter]       # …run against it, as often as needed
tool/ask-e2e.sh stop
```

- The script starts Ollama, pinned by digest, in a `docker` or `podman` container on `127.0.0.1:11435`, beside any Ollama of the user's own on 11434. It pulls the model and loads it once, so no test pays for the model load. The first question after a start still takes longer, while Ollama reads the system prompt and the tools for the first time.
- The models are kept outside the repo in `ASK_E2E_CACHE` (default `~/.cache/oldbrainfix/ask-e2e`). The panel's PNG files go to `ASK_E2E_SHOTS` (default the system's temp folder), never into the repo.
- The model is `ASK_E2E_MODEL` (default `qwen2.5:3b`). Pass the user's model, such as `qwen3:4b`, for a ticket about local models.
- The Rust build runs before the model starts, at `ASK_E2E_JOBS` jobs (default 1). The gui tests link ALSA and draw through wgpu, as `tool/guide-frames.sh` does: a software Vulkan such as lavapipe draws the PNG files.
- `test` after a code change rebuilds while the model is loaded. Use `run`, or build first with Ollama stopped.
- No CI job runs it. The tests in `desktop/crates/oldbrainfix-gui/tests/ask_e2e.rs` are all `#[ignore]`, and the script runs them one at a time with `--ignored --nocapture`.

## What it checks

Real questions on seeded Notes, through the real code path: `Store::ask` with the real `Web` transport. Each seeded store switches on "AI (experimental)" first (#334); without it the desktop asks no provider.

- **Desktop**: three questions whose answer is in one Note (English and German), one whose answer is in none, an Ollama that is stopped, and a model it lacks.
- **Phone**: the same through the paired host as the app runs it (`PairedHosting` with the `AskDesk`), over TLS with the phone's proof, timed out as the Dart core's `askDesktop`. It checks `GET /ask/provider` and `POST /ask`, and polls `POST /ask/status` while it waits (#317). Two questions in one session, the second sent with the first and its answer as the Ask page sends it, each answered and read from its Note (#315).
- **Panel**: the Ask panel drawn with a real answer, with an answer that is not in the Notes, and with Ollama stopped, its failure under the question and then its Details (`ask-answered.png`, `ask-not-in-notes.png`, `ask-ollama-stopped.png`, `ask-ollama-stopped-details.png`). Two questions of one session in its thread, the first above the second, then Edit putting the first into the field (`ask-thread.png`, `ask-thread-edit.png`, #315). A real answer copied with Copy as text, its Markdown on the clipboard with HTML beside it, then added with Add to Note… to a seeded Note picked by its filter, as a Section of the question and the answer (`ask-actions.png`, `ask-add-to-note.png`, `ask-added.png`, #316). The phone's page itself is Flutter and drawn by none of these: its thread and the actions on its answers, which send nothing over the wire, are proven by `test/ask_screen_test.dart` with a fake computer.

The asserts do not depend on the model's wording. An answer arrives within `ASK_E2E_LIMIT_SECS` (default 120) and has words. The Notes read include the seeded Note that holds the answer. A question the Notes hold nothing about ends with "That's not in your Notes" or links no Note. A failure comes back as its kind (`Network`, `Provider`) with a message. Before any assert, each test prints the question, every round with its time, tokens and tool calls, the answer, its links and the Notes read. A red test therefore still shows what the model did. After the run the script prints `ollama ps` and Ollama's log lines about the context.

A red check on unchanged code is a finding for the ticket, not a reason to loosen the assert.
