# The Changes Nudge: what counts as sent to a person synced by bundles

With a person met from afar whose changes travel by bundles (ADR 0023), nothing moves until the user sends one. The Changes Nudge (#109) reminds them: "Changes for Bob and Carol". Both cores decide who waits and when it comes the same way (`changes_nudges.dart`, `changes_nudges.rs`, and the plan in `nudges.dart`, `nudges.rs`).

## What a bundle carries stays as it is

A bundle for a person carries what lies beyond what their Devices said they hold (ADR 0023). Sending one moves nothing there: only a bundle, folder sync or LAN session from them does. So a bundle that never arrived loses nothing, and the next one carries it again. Whether a change waits for the Changes Nudge is kept apart from that.

## The sent mark

This Device keeps, per person, how far its log had come when changes were last sent to them, by Shared Area: the local setting `changesSent:<profile id>` (ADR 0008), one JSON object with sorted keys and no spaces, `{"<area id>":<seq>}`. `seq` is the log's high-water mark (`currentSeq`), read before the bundle is written. Each mark only grows. The mark is set when:

- the user sent a bundle for the person: the app reads the mark, writes the bundle, and once the share sheet took it or the file was saved, notes it for every Area shared with the person's Devices. Writing alone notes nothing, so a share sheet left without an app keeps the Changes Nudge;
- the user shared an Area with the person ("Share with…"): the offer carries the Area's history, so the mark of that Area is the log at the share;
- this Device took an offered Area: the mark of the sharer in that Area is the log after the history was taken in, since the sharer holds it.

A change waits for a person when a bundle for them would carry it, it lies in a Shared Area past that Area's mark, and none of their own Devices wrote it. The local handle of an entry counts the order in which it reached this Device, so a change another Device wrote earlier that came here later still waits. A dseq vector would not count it (ADR 0021).

## Who waits, and since when

A person waits when changes travel with them by bundles (`transport:<profile id>` = `bundle`), they are not blocked, the grants give one of their Devices a role in a Shared Area, and a change waits for them. The person's first waiting change is the earliest of them. A change this Device wrote counts from its own `recorded` time. Any other change counts from when it came here: the Moment recorded after the sync or import that brought it, the newest one below its local handle. Another Device's clock, possibly another person's, never moves the Changes Nudge forward or back. A change with no Moment below it, which no sync records, counts from its `recorded` time.

## When it comes

The plan follows the Nudge's rule (`nudges.dart`) with one anchor per person: due 30 minutes after their first waiting change, captured at it. It fires first at the earliest due and covers everyone waiting. While they wait it comes again every Nudge repeat interval after it last fired. A person whose first waiting change came after it last fired brings it forward to their own due when that is sooner. A firing in Quiet Hours waits for their end, and the repeats count from there. There is no snooze. The plan is a pure function of the people waiting, now, the wall clock, the repeat interval and the Quiet Hours, so the platform's alarm layer can follow it while the app is closed.

## Considered Options

- Moving what the person holds when a bundle is sent (rejected: a bundle that never arrives would be lost; ADR 0003).
- Noting the bundle as sent when it is written (rejected: a share sheet left without an app would end the Changes Nudge while the person still lacks the changes).
- One mark per person instead of per Area (rejected: an offer for a second Area would mark the first Area's waiting changes as sent).
- The entry's own `recorded` time for every change (rejected: another person's clock would decide when the user is reminded).
- A dseq vector as the mark (rejected: an earlier entry that comes later would never wait).

## Consequences

- The marks are this Device's own. Another Device of the user's, with `transport:` set for the person, reminds until the person's bundle reaches it, or until a bundle goes from there too.
- A person who shared an Area by bundles before the marks existed waits once for its whole history; the next bundle ends it.
- The changes made between "Share with…" and writing the offer go with the offer and wait for one bundle more.

## Changes to earlier decisions

- ADR 0023: "Share with…" and accepting an offer also set the sent mark of the Area for the person.
