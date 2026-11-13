# Hidden from AI is a mark of the person and hides everything from the AI

The desktop's AI port (ADR 0026) and the coming Ask panel let a model the user picks read their Notes, and anything the model reads can reach its provider. Some Notes should never go there: a diary, health papers, a letter from a lawyer. The user chose (Q620, Q621 (d), Q622, Q636) a mark **Hidden from AI** on an Area or a single Note. For the AI such a Note, every Note in such an Area, and everything on them do not exist; hidden and shared exclude each other. Both cores read and write the mark the same way (`hidden_from_ai.dart`, `hidden.rs`); the AI's side is in the Rust core only, as the port is (#267).

## The mark is kept as an Override is

The mark of a Note or an Area is the entity `aiHidden:<id>` in the log, outside every Area, as an Override (ADR 0017), the person's settings (ADR 0040) and "Show on the Agenda" (ADR 0030) are. It is sealed under the personal key and goes to the user's own Devices only, by sync, the shared folder, bundles and Backups, never to another person, a Shared Area's segments or a Sent Item. Only what the user's own Devices wrote counts, read from the log alone, the newest such entry winning, so a mark another person's modified app slips into the log hides nothing, and an AI Task's draft, which no tool writes anyway, cannot set or take it off.

The field `hidden` is `on` while the mark is set; taking it off writes null; any other value is no mark. The same value again writes nothing. It carries no version.

One entity per Note or Area, not one list in `settings:person`: two of the user's own Devices that mark two Notes apart, before they meet, keep both marks. A list would keep only the newer one, and a Note would show to the AI again without anyone taking the mark off.

Not a field on the Note: a Note's fields go wherever the Note goes, and a Note filed into a Shared Area on one Device while another marks it would carry the mark to the other people.

## What is hidden

An entity is hidden when, walking up from it, a marked Note or Area is reached: a Topic by its Area; a Note by its Area or Topic and by the Note it is merged into; a Section, Routine or Reminder (a Session's too), Frog, Tally entry or Line by its Note; the user's own choice for a list (`agenda:<note id>`) and a mark by their Note or Area. Each link is followed as the log alone has it and also as the user's own Devices last wrote it, so an entry another person's modified app slips into the log, filing a Note out of a hidden Area or undoing its Merge, hides it no less; the walk only ever finds more marks. Only ids of the kind expected are followed, and each once, so the walk ends. `hiddenFromAiBy`/`hidden_from_ai_by` give the marks that hide an entity, sorted; `isHiddenFromAi`/`is_hidden_from_ai` whether any does. A Note filed out of a hidden Area is no longer hidden by it; a Note merged into a hidden one is hidden with it, as the user sees it as part of that Note. `hiddenFromAiAmong`/`hidden_from_ai_among` give those of a list that are hidden, as a list of cards shows the shield: the marks read once and each link followed once for all of them, by the same walk; the Dart core keeps what it walked until the log changes, so a list built again walks nothing.

The mark is set on a Note or an Area that is in no Shared Area (`mayHideFromAi`, `setHiddenFromAi`; `may_hide_from_ai`, `set_hidden_from_ai`); a Topic, an unknown id and anything in a Shared Area are refused. Taking the mark off is never refused. Two of the user's own Devices may still meet with a Note marked on one and its Area shared on the other: the Note stays hidden from the AI, and the mark can be taken off. Reading does not ask whether the Area is shared, so such a race never shows the Note to the AI.

## Sharing something hidden is refused

A mark the other people's Devices would have to obey makes no sense on something given to them. So each way to give something away refuses what is hidden, changes nothing, and names the marks in the way, sorted, until the user takes them off: `HiddenFromAiRefused` in the Dart core, `Error::HiddenFromAi` in the Rust core, both holding the ids of the marked Notes and Areas.

- Filing a hidden Note into an Area or Topic of a Shared Area (`Notes.file`, `Notes::file`), one hidden by its Area too: a Note in a hidden Area is hidden, as for sending and Packs. Filed into a place of the user's own first, it leaves the Area and the mark's reach, and may then be filed into the Shared Area.
- Merging a hidden Note into a Note of a Shared Area, or a Note of a Shared Area into a hidden Note (`Notes.merge`, `mergeAll`; `Notes::merge`, `merge_all`): the merged Note becomes a Section of its target, so a hidden Section would show in a Shared Area, or a Note of the others would be hidden from the AI. The marks named are those hiding the merged Note, or in the other direction those hiding the target; `mergeAll` asks for all of them at once and merges none. A Note hidden by its Area's mark is refused as when filing.
- Sharing an Area that is marked or holds a hidden Note (`share`, so "Share with…" from afar too, ADR 0023): every Note filed in it or its Topics counts, those in the trash and merged ones too, as their whole history goes along. A starter Area is refused before it moves to fresh ids.
- Sending a hidden Note as a Sent Item (`sentItemOf`, `sent_item_of`), its cancellation too, as a cancellation carries the text again.
- A Pack exported from an Area, Topic or Note (`packOf`, `pack_of`) holding a hidden Note, a Tally its Session picks from included.

The Pack of everything offered for feedback (`packOfAll`, `pack_of_all_by`, #214) leaves hidden Notes out, a Session's hidden Tallies too: the user did not pick them, and a refusal there would stop feedback for as long as anything is hidden. A Backup opened to take Notes back (`packOfEverything`, `pack_of_everything`, #285) keeps them, as they are the user's own. Backups and the vault (#267) hold hidden Notes like any other; the mark concerns the AI, not the user.

## For the AI it does not exist

`Store::for_ai` runs a body as the AI sees the store, on the thread that answers the call. `Store::ai_tool` runs every tool of the AI port in it, so the Ask panel, which calls the same tools, cannot skip it. Inside it the reads of the store itself (`history`, `logged_history`, `current_fields_of`, `entities_with_prefix`, `fields_with_prefix` and the list of the entities of a type) leave out every hidden entity, as if it had never been written. Every read built on them follows: the Notes listed and counted, search, the tag tree, the Areas and Topics, Routines and Reminders with their check-offs and skips, Frogs, Tallies with their entries, the Lines and ticks of lists, a Note's history, and the Agenda for a tool that reads it. What is worked out from the reads and kept (`worked_out`, such as the tag tree and the Sessions' set-ups) is neither taken nor kept inside it, so nothing the user's screens worked out reaches the AI, and nothing the AI saw reaches the screens. The marks are read once per call, and what each entity's walk found is kept for the call.

What the AI reads of something it can see does not name what it cannot: a Session's pool lists its visible Tallies only, its picks leave out a hidden Tally's, and a Note's history reads a hidden Area, Topic or Note it was filed in or merged into as none. A photo or an Attachment Text is handed out only by a Note that holds it, and a hidden Note holds none for the AI.

Writes cannot reach it. A tool given the id of something hidden answers as for an id there never was, as the reads it starts with find nothing. Behind that, every draft of an AI Task is checked before anything else, with the reads seeing all: a draft on a hidden entity, or one filing, merging or putting something onto a hidden Note, Area or Topic, is refused as unknown (`not known: <id>`). A Session's pool keeps the hidden Tallies it held: the AI sets the pool it sees, and `set_session` puts the hidden ones back after it, so Accept loses none. `delete_place` refuses an Area or Topic that holds a hidden Note, as the Note would stay behind in a deleted place or be moved or trashed unseen; the user deletes it in the app.

No tool reads or writes the mark: a mark the AI can read names what is hidden, and one it can write undoes the user's choice.

## Considered Options

- A setting of the person holding a list of the hidden ids (rejected: two Devices marking apart keep only one mark, see above).
- A field on the Note or Area (rejected: it travels with the Note into a Shared Area).
- Filtering in each tool (rejected: some 60 tools and the Ask panel's would each have to remember every way a hidden Note shows, a count or a pool among them; one seam in the store's reads leaves nothing out by oversight).
- Refusing the Pack of everything for feedback while anything is hidden (rejected: feedback would stop for as long as one Note is hidden; leaving hidden Notes out gives away nothing).
- Letting the AI delete a place holding a hidden Note, its Notes moved as the user chose (rejected: a write on a hidden Note, which the AI may not reach).

## Consequences

- The AI may learn that something exists only where refusing gives it away: `delete_place` on a place holding a hidden Note. It learns neither what nor how many.
- Inside `for_ai` each read of an entity walks up from it once per call while any mark is set, a few reads of the log each; without a mark nothing changes.
- The cross-core fixture `hidden-from-ai` writes the same log from both cores and reads the same marks, hiding and refusals after each step.
