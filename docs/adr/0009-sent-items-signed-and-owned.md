# Sent Items are signed, recognised by ID, and only their sender may change them

A Note with its Routines can be sent to another Old Brain Fix user as a file through any messenger, without a Shared Area. The receiver sees a preview and must accept it; accepted items land in Unsorted with their alarms on. Each Sent Item carries a stable ID and the sender's signature, so a later file with the same ID is shown as an update or a cancellation, and it is accepted only when it is signed by the same key as the original. Files to recipients whose key is known from Pairing or a Shared Area are encrypted to them; otherwise they are signed only.

## Considered Options

- Plain copies without IDs (rejected: rescheduled appointments would duplicate).
- Accept updates from anyone holding the file (rejected: anyone could cancel someone's appointment).
- Sending only to known recipients (rejected: kills "just send it").
