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

## Capture Context (ADR 0037)

Amended by ADR 0037 (#162): a tool removes one detail of a Note's or Section's Capture Context (`remove_captured`), and an AI Task may write the fields of a Capture Context (`location`, `sourcePackage`, `sourceLabel`, `calendar`, `bluetooth`) only as null, where this Device may change the Note or Section. Such a removal is reviewed among anything else.

## Editing a Section (#183)

Amended by #183: `edit_section` replaces the text of one Section, an Amend or a merged Note, by the id `read_note` lists, which also shows for each Section whether this Device may edit it and when and by whom it was last modified. It writes the `text` the field list already allowed on a Section where this Device may change it, so no trust rule changed; the edit is reviewed with the other text changes.

## Who may connect (#167)

Amended by #167: the port is no longer on the loopback only when the user chooses so. An AI client in a Docker container, such as Claude Code in a sandbox, cannot reach 127.0.0.1 of the computer, which inside the container is the container itself. The AI port's card offers **Who may connect**, kept in the local setting `ai:reach` (`local` or `docker`) and changed only while the port is closed; the port still stays closed at every start.

- **This computer only** (`local`, the default): binds 127.0.0.1 as before. The `Host` header must name `localhost`, `127.0.0.1` or `::1`; `host.docker.internal` is refused, so a container that reaches the port anyway, through host networking or a forward, gets 403.
- **Docker containers on this computer** (`docker`): on Linux the port binds only this computer's address on the `docker0` bridge, read from `/proc/net/route` (the bridge's network) and `/proc/net/fib_trie` (the local address in it), with no new crate and no `unsafe`; the loopback and the other networks are not bound. Docker Desktop on macOS and Windows forwards `host.docker.internal` to the computer's loopback, so there the port stays on 127.0.0.1. Accepted `Host` names are the bound address and `host.docker.internal`. The address shown and the copied client config, Claude Code command and Claude Desktop config use `http://host.docker.internal:<port>/mcp`. With no `docker0` bridge on Linux the card says so, and the port does not open: no AI Task is opened and sync does not pause (`Error::NoDockerBridge`).

Unchanged in both: the token on every request, compared by its digest; the refusal of a request whose `Origin` is not the loopback; Pending Changes only; sync paused while the port is open. New in both: a request whose `Host` is not on the choice's list, or that has none, gets 403 before the `Origin` and the token are looked at. The list is never empty, so DNS rebinding, a web page whose own name was pointed at this computer, is blocked.

Security: every container on the Docker bridge, not only the one the user meant, can reach the port while it is open, and the token (256 bits) is the only barrier; anyone who can read the client's config in a container can use the port. A Linux container must be started with `--add-host=host.docker.internal:host-gateway` to resolve the name; with rootless Podman, `host-gateway` may not point at `docker0`. The card says that every container on the bridge can reach the port and how to start a Linux container. Requests cross the bridge as plain HTTP, which does not leave the computer.

Considered: an address of the user's choice (rejected: it would put the token on the network unencrypted, and no use of the app needs it); binding every address (`0.0.0.0`) with the `Host` check alone (rejected: other machines on the network could try the token).

## Tallies, Sessions, Checklists, Shopping Lists and Frogs (#168)

Amended by #168: the tools read and set up every kind that came after the port, and read, but never write, what the user recorded.

- Reading: `list_notes` and `read_note` name each Note's kind, `plain`, `checklist`, `shoppingList` or `tally` (a Note is at most one of the last three), its open Frog and the Sessions spawned from it. `read_note` adds the parts: a Tally's fields with unit, target (in the field's unit), rule and the field each multiplies (#200), the Lines of a Checklist (text, group, checked) or Shopping List (text, reappear time, struck), its Frogs, the Boxes of its text and of each Section's text with their states as they are (ADR 0035), and each Session among its Routines with its pool, count, picking rule and weekday counts, as `list_routines` does.
- Set-up tools, each a Pending Change through the screens' own calls: `make_tally`, `add_tally_field`, `rename_tally_field`, `set_tally_field` (unit, target, rule, the field it multiplies; `add_tally_field` takes that too, #200), `remove_tally_field` (hidden, its values kept); `make_session` and `set_session` (pool, count, picking rule, weekday counts; the schedule with `reschedule_routine`); `make_list` (Checklist or Shopping List), `list_to_plain`, `switch_list` (#188), `add_line`, `rename_line`, `move_line`, `group_line`, `set_reappear`, `remove_line`; `make_frog` and `frog_to_plain` (the open Frog to the trash, the Note kept). A Tally's fields keep the order they were added in, in both cores, so there is no tool to reorder them.
- The user's own data is read only, each item with its date, narrowed by `from` and `until` (days on this computer's clock, both included): `read_tally_entries` (a multiplied field as the product with its factors, a multiplier with no value of its own, #200), `read_ticks` (the checks of a list's Lines, also per Occurrence of a Routine spawned from it, and the ticks of Boxes), `read_check_offs`, `read_eaten_frogs` and `read_session_picks`; check-offs and picks go by the day of their Occurrence, the rest by the day of their moment. `Store::ai_tool` takes this computer's clock for these days and for a Shopping List's struck Lines.
- Trust rules: an AI Task may further write on a Note its `kind` and the fields of a Tally (`tally:<field>:name`, `unit`, `target`, `rule`, `hidden`, `multiplies`); on a Routine a Session's `session:pool`, `session:count`, `session:count:<weekday>` and `session:rule`; on a Line its type, Note, making, text, order, group and reappear time; on a Frog its type, Note and trash. It never writes a Tally entry, a Line's `checked`, a check of a Line or a check-off on a Routine (`check@…`, `done@…`), a Session's pick or entry (`pick@…`, `entry@…`), a Frog's `done`, today's Frogs (`frogs:pick`) nor a Box's tick (`box@…`): the judge refuses them whatever call writes them. Making a list of a text with a line marked "✓ " is refused, as that Line would be checked, and so is adding a name a Shopping List holds checked, which would bring it back. A Line's made-once fields are its type, Note and making. The roles' checks are those of the screens: changing a Line needs `Checklists::may_edit`, adding one a role in its Area, a Frog and a Session `may_manage`.
- The review groups these as Tallies changed, Checklists and Shopping Lists changed (a Note's kind and its Lines), Frogs changed and Sessions changed (every field of a Routine that is a Session), each with its before and after: a Tally's field by its name and a target in its unit, a Line's place counted from 1, a pool by the titles of its Notes.

Amended by #186: `read_note` names how a Shopping List shows on the user's own Agenda (`agenda`: `off`, `count` or `lines`; null for any other Note), and `set_shopping_agenda` sets it as a Pending Change through the screens' own call (ADR 0030). An AI Task may thus write on the user's own `agenda:<note id>` the field `shopping`, for a Note that is a Shopping List, whatever the role in its Area, as the user may; a Checklist's switch (`agenda`) stays out of its reach. The choice is read from the log alone, so it shows on the Agenda only once accepted. The review shows it among the Checklists and Shopping Lists changed, under the list's Note.

Amended by #219: a Checklist without a Routine or Reminder has the same three choices, so `read_note` names its choice too (`agenda`: `off`, `count` or `lines`; null for a Checklist with a Routine, whose choice does not apply), and `set_checklist_agenda` sets it as a Pending Change, beside `set_shopping_agenda`. An AI Task may thus write on the user's own `agenda:<note id>` the field `agenda` as well, for a Checklist without a Routine, whatever the role in its Area; the review shows it the same way.

## A Note is its kind (#188)

Amended by ADR 0038: `read_note` reports a Note's name and description beside its kind, null for a Plain Note, and `switch_list` switches a Checklist and a Shopping List into each other as a Pending Change, refusing a list with a checked or struck Line.
