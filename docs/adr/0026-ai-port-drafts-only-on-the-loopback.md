# The AI port: an MCP server on the loopback that writes only Pending Changes

An AI Task (#112) lets a model the user picks, local or in the cloud, work on the user's Notes for one job: fixing typos, a cleanup, finding and removing everything about a topic, better alarms. The desktop opens an MCP server for it on request. Everything the model changes waits as Pending Changes (ADR 0022) until the user has reviewed it. The port is in the Rust core (`ai_port.rs`, `ai_tools.rs`, `ai_task.rs`); the phone has none, and the Dart core does not change.

## The port

The port is off by default. The user opens it in Settings, and "Done, review" closes it. Quitting the app also closes it, and the review then waits for the next start. It is plain HTTP on 127.0.0.1 only, on the port it had last time while that port is free (kept in the local setting `ai:port`). MCP's Streamable HTTP is served without a stream: `POST /mcp` carries one JSON-RPC message, the answer comes back as JSON, a notification gets 202, and any other method gets 405. Every request must carry `Authorization: Bearer <token>`. The token is 32 random bytes, hex, made once and kept in the local setting `ai:token`, so that a client the user set up once stays set up. It is compared by its SHA-256 digest, never as text. A request without the token gets 401. A request whose `Origin` is not the loopback, as from a web page in the user's browser, gets 403. Both are refused before anything reaches the store. No crate is used: the MCP crates for Rust bring an async runtime the desktop does not have, and the protocol here is a few JSON-RPC methods (`initialize`, `ping`, `tools/list`, `tools/call`) over the HTTP reading the LAN host already has.

As in the LAN host (ADR 0003), the network runs on threads of its own and never touches the store. `initialize`, `ping` and `tools/list` are answered there. Each `tools/call` crosses a channel to the app, which answers it on its own thread (`AiCall::answer`) or turns it away (`AiCall::refuse`).

## The tools

The tools read Notes with their Sections, Transcripts, Photo Texts and Capture Context, a photo scaled down to at most 2048 pixels on its longest side (1024 unless asked), the Areas and Topics, the Routines and Reminders, and this Device's settings. They change what the user can change in Notes, Areas, Topics, Merges, the trash, Importance, Routines and Reminders (the schedule, Re-nag and Escalation). Recordings are read only as their Transcripts. There are no tools that reach other people (sending, sharing, Profile cards, blocking), none that check off an Occurrence, since that says something was done, and none that write a setting.

A tool that changes something runs through the same calls the screens use, inside `Store::drafting` on the thread that answers it. There every `append` becomes a draft of the AI Task's set instead of an entry, and the reads see the draft at once. A call that fails leaves the set as it was before the call. The user's own edits, on the app's thread or after the call, reach the log as always, and win at Accept when they came later (ADR 0022).

## Trust rules

- An AI Task writes drafts only. It writes no entry, photo, recording, local setting or blob, and takes in nothing from elsewhere. Any of these inside `drafting` is an error.
- Each draft must be a field an AI Task may write: on a Note its type, capture time, text, place, Importance, trash, Merge, spawned mark and Nudge; on a Section its type, Note, capture time and text; on a Routine its type, Note, schedule, Re-nag, Escalation and trash; on an Area its type, name, colour and icon; on a Topic its type, Area and name. That rules out roles, blocks, Overrides, a Sent Item's link, Profiles, check-offs, photos and recordings. A type, capture time or link is written only when the entity is made, and a type only as what the entity's id says it is (an Area or Topic may be deleted).
- A draft must be allowed by this Device's role in the Shared Area, as the log has it (`judged`, ADR 0013 and 0022): changing a Note or Section needs `may_change`; filing, merging into or amending in a Shared Area needs a role there; changing a Routine, Area or Topic needs `may_manage`. These are the checks the screens make before they write. Other Devices would ignore what the role does not allow, but the model gets a refusal at once and the review never lists a change no one would keep.
- Sync waits while a shown set waits (`Store::sync_paused`), so from the opening of the port until the review is decided. The LAN host answers 503 "paused", joining a host, the shared folder, writing or reading a bundle, offering an Area and sending a Note are refused with `Error::SyncPaused`, and the Changes Nudge waits. Accept and Decline are not held back.
- Alarms fire from the log alone (`Store::log_only` around the alarm plan). A Routine the task added does not fire before Accept, and one it rescheduled fires as the log has it. The Agenda shows the pending changes as stored, as every read does.
- Area Locks do not exist on the desktop yet. Until they do, the port has nothing to unlock. When they come, a call that reads or changes a locked Area waits until the user unlocks it, as opening the Area does, and the AI keeps it until it locks again.

## The review

`Store::ai_review` lists every field the task changes, by entity, with what the log holds now (before) and what the task wrote last (after), and `mine_wins` where one of the user's own Devices wrote the field after the task did, so that Accept keeps the user's value. A field written back to what the log holds is no change. The changes are grouped as Notes added, filed, merged, moved to the trash, restored, text changed, Importance changed, Routines changed, Areas and Topics changed, and anything else, and each item names the Note to open. Accept all is `accept_pending` and Reject all is `decline_pending` (ADR 0022). There is one AI Task at a time: opening the port while one waits continues it.

## Considered Options

- An MCP crate (rejected: it needs an async runtime, and its stdio or SSE transports are not what a loopback port with a token needs).
- A token made each time the port opens (rejected: the user would have to copy the client config before every task).
- Separate draft versions of every write (rejected: they would drift from the calls the screens use, in the entries they write and in what they check).
- Letting the AI's drafts set alarms before Accept (rejected: a schedule the user might reject would move a real alarm, such as one for pills).
- Holding the user's own edits while the port is open (rejected, as in ADR 0022: capturing and editing never wait).

## Consequences

- A client set up once keeps working until the token is deleted. Anyone who can read the client's config can use the port while it is open.
- While an AI Task waits for review, the other Devices hear nothing from this one, and the phone shows its sync as failed.
- The phone never opens a set that is shown, so its alarm plan still reads through shown sets. The phone must read its alarms from the log alone before it shows a set.

## Changes to earlier decisions

- ADR 0022: the AI Task is a set of kind `ai-task`, shown, with no payload, holding drafts only. While a shown set waits, sync waits in the core itself, and the alarm plan reads the log alone.
