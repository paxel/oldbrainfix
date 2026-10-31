# Triage Labels

The skills speak in terms of five canonical triage roles. This file maps those roles to the actual label strings used in this repo's issue tracker.

| Label in mattpocock/skills | Label in our tracker | Meaning                                  |
| -------------------------- | -------------------- | ---------------------------------------- |
| `needs-triage`             | `needs-triage`       | Maintainer needs to evaluate this issue  |
| `needs-info`               | `needs-info`         | Waiting on reporter for more information |
| `ready-for-agent`          | `ready-for-agent`    | Fully specified, ready for an AFK agent  |
| `ready-for-human`          | `ready-for-human`    | Requires human implementation            |
| `wontfix`                  | `wontfix`            | Will not be actioned                     |

Beyond the five roles, `implemented` marks an issue whose commit is in but not released yet:

- After the commit, `implemented` replaces `ready-for-agent`, and a comment names the commit: "Implemented in <sha>, not released yet." The issue stays open.
- On release, the issue is closed with "Shipped in vX.Y.Z (<release link>)"; the label stays as a record.
- An issue done only in part (e.g. its core) gets the label once all of it is committed.

When a skill mentions a role (e.g. "apply the AFK-ready triage label"), use the corresponding label string from this table.

Edit the right-hand column to match whatever vocabulary you actually use.
