# A Session is a kind of Note; its Routines run it

A Session (ADR 0029) was a kind of Routine: the pool, the count and the rule lay on the Routine spawned from a Note. The user found that backwards (#191): "sport card has a routine, the routine is a session, in the session is the pool … the card should get a session definition and the routine would only execute it … pool, how often done, how often skipped: it should be on the note." A Session is now a kind of Note (ADR 0038). The Note holds what it picks from and how. Any number of Routines of that Note say when it runs, such as Monday and Wednesday at 7:00 and Saturday at 10:00. Both cores read and write it the same way (`sessions.dart`, `sessions.rs`).

## The setup lies on the Note

The setup is the same fields as before, now on the Note's entity: `session:pool`, `session:count`, `session:count:<weekday>` and `session:rule`. Each is read only in the shape the apps write it (ADR 0029). A Note is a Session while it holds a pool. The kind reads in this order: a Tally first, then a Session, then the list its `kind` names, else a Plain Note (`NoteKinds.of`, `NoteKind::of`).

Lying on the Note, the setup is changed as the Note is (ADR 0028), by its owner or a Manager of its Area. A Member may also change it on a Note of their own (ADR 0013). The Routines of the Note stay the Manager's, as every Routine is, so a Member can set up a Session on their own Note but cannot add a Routine to it. The setup goes wherever the Note goes: with its filing, with an offer from afar, into Bundles and Backups. While the Note lies in a Shared Area, the pool takes only Tallies of that Area, as before.

The kind badge of the Note editor offers a Session from a Plain Note, and a Plain Note from a Session.

- **Becoming a Session** (`make`) writes the pool, the count and the rule on the Note. It takes away any weekday count left by an earlier setup, and takes the Note out of Unsorted as a Tally's first field does. Only a Plain Note in the lists becomes one.
- **Going back to a Plain Note** (`unmake`) writes the pool as null. The Note's Routines then run as plain ones. Their picks and the entries recorded for them stay in the log.
- An Occurrence that was Done from its entries stays Done: a Routine with any `pick@` field takes Done from its entries, whether its Note is still a Session or not.

## Every Routine of the Note runs it

Every Routine and Reminder not in the trash that was spawned from the Session's Note runs the Session. The Note is the one named in the Routine's `note` field, as stored; a merge does not move a Session. The "+" badge of the Note adds one more. Adding a Routine writes nothing of the Session: the Routine runs it because its Note is one.

The picks and the entries recorded for them stay per Occurrence on each Routine, exactly as ADR 0029 has them: `pick@<occurrence key>` and `entry@<occurrence key>@<entry id>`, written, judged, healed (#179) and Done as before. The rules work across Routines as follows:

- **Random** still hashes the Routine's id, the Occurrence's key and the Tally's Note. A Routine's draws thus stay what they were, and two Routines at the same moment draw apart.
- **Rolling** goes over the picks of every Routine that runs the Session, so Saturday's run goes on from Wednesday's instead of starting again. The picks before an Occurrence are those of earlier Occurrences, and of the same moment those of Routines with a lower id. They are taken in that order, and then by their place in each pick.
- **The Agenda** writes today's picks in that same order, by time and then by Routine across all Sessions, so the evening's pick goes on from the morning's.
- **Least selected** counts every entry of each Tally, as before.

## Older Sessions read on the new model

An older version wrote the setup on the Routine. Such logs are read without being rewritten:

- **Fallback.** A Note that has never held a `session:pool` of its own reads its setup from its Routines that hold a pool and are not in the trash.
- **Several older Routines.** The setup of the one whose setup field was written last, by latest wins, counts for all of them. The other setups stay in the log unread.
- **Made plain again.** A Note with a pool of null was made plain again, and no older Routine counts for it.

The first change through the new model writes the whole setup onto the Note, as it reads, in the same batch as the change. Such a change is setting the pool, the count, a weekday's count or the rule, or spawning a Session from the Note. From then on the Note holds the setup. The Routine's own setup fields stay as they were and are not read any more.

Two cases follow from the order of the kinds. An older Session spawned from a Checklist or a Shopping List now shows as a Session; the apps never offered that. An older Session spawned from a Tally stays a Tally, and its Routine still runs the Session.

## On the card and in the editor

The Session's page in the Note editor holds its setup, after the name and the description (ADR 0038):

- the pool, a tick list of the Tallies it may pick with "New Tally", as the Session form of #161;
- how many it picks, with − and +;
- the rule and the counts of single weekdays, as badges among the Note's settings;
- its Routines, with the "+" badge.

The detailed view of its card (#189) shows the pool and today's picks of each Routine. The entries of those picks are recorded there as on the Agenda. Under them comes how often it was done and skipped over the last four weeks. The compact view shows the name, the next run of any of its Routines that is not Done, and "done N / skipped M this week" over all of them. The Agenda shows each Occurrence of each Routine as before.

## The AI port

`read_note` reports a Session's setup with the Routines that run it, and its kind as `session`. `make_session` sets up a Note as a Session, or a new Note from its text, and spawns a Routine when a schedule is given. `set_session` changes the setup of a Session's Note. Both are Pending Changes (ADR 0026); a further Routine comes from the Routine tools. Picks and entries stay read-only there.

## Considered Options

- A Session entity of its own, keyed by the Note (rejected: as with a Tally's fields, ADR 0028, a child entity needs its own rule of who may change it; on the Note, the Note's rule decides).
- Rewriting older Sessions onto their Note on reading (rejected: every Device would write the same move, and a Device of the older version would read the Note without its pool).
- Copying the setup onto each Routine (rejected: a change would need writing on every Routine, and two Routines could disagree after a sync).
- Rolling per Routine (rejected: Monday and Saturday would each start again from the top of the pool).
- Hashing the Note's id for Random (rejected: the draws of older Sessions for the days not yet picked would change, and two Routines at the same moment would draw alike).
- Merging the setups of several older Routines field by field (rejected: a pool of one and a count of another would be a setup nobody made).

## Consequences

- Sent Items still carry no Session. A Session Note sent goes as a Plain Note, with the schedules of its Routines. The pool names Tallies the receiver does not hold, which do not travel either (ADR 0028).
- An older version reading a Session written now finds a pool on the Note and none on the Routine. Its Routines show there as plain ones.
- A Session without a Routine shows on no Agenda. Its card still shows its pool and its setup.

## Changes to earlier decisions

- ADR 0029: the setup lies on the Note, not on the Routine; a Session is a kind of Note, and every Routine of it runs it. It is changed as the Note is, not only by a Manager. Rolling goes across the Note's Routines. Today's picks are written by time across all Sessions. Done from entries no longer needs a pool on the Routine. The AI port sets up a Session.
- ADR 0038: the Session is a kind, read after the Tally and before the lists; the kind badge offers it from a Plain Note and back.
- ADR 0013: a Member sets up a Session on a Note of their own.
- ADR 0026: `make_session` and `set_session` work on the Note.
