# Whisper for speech and ocrs for photo text, on every Device, no Google

Voice Notes are transcribed with Whisper (the same vendored whisper.cpp sources on Android and desktop, each through a thin bridge of its own), and text in photos is recognised with ocrs, the Rust engine Catlog's desktop already uses, on both platforms. One engine per job gives the same results on phone and desktop, runs fully offline, and keeps Google libraries out of a privacy-first app. The Whisper model is not bundled; it is downloaded once on request from a mirror in Old Brain Fix's own GitHub releases, verified by a checksum pinned in the app, or picked from a local file. The recording is always kept, so a missing or bad Transcript never loses a Note.

## Considered Options

- Android's on-device speech recognizer (rejected: missing on some devices, nothing on desktop).
- ML Kit text recognition as in Catlog's phone app (rejected: Google library that may send usage statistics).
- Downloading the model from Hugging Face (rejected: a second third-party host in the privacy statement).

## Removing a recording or photo (ADR 0020)

Amended by ADR 0020 (#77): the recording and the photo are kept until the user removes one that has a Transcript or Photo Text. The text stays, and a recording or photo without one cannot be removed this way.

## Prompt repeats taken out of Transcripts (#89)

Amended by #89: Whisper is led in with a prompt (#79), and on silent or very short recordings it may write that prompt out instead of what it heard. Both cores guard the Transcript against the prompts it was led in with (`TranscriptPrompts.guard`, with the same results, checked by cross-core fixtures) before it is kept, on the phone and the desktop alike. Words are compared in any case and without the punctuation around them. Taken out are every repeat of the German lead-in, whole or from its start at least through "Wörtern:", and every run of three or more of the listed Area and Topic names and the app's words; a Transcript that is nothing but the listed names, two or more, in their order, goes whole. A lone name, two names amid speech and a part of the lead-in's later sentences stay, since real speech says them too. A Transcript left without words is kept empty and shows as such. Named Places are not in the prompt, so nothing guards against them.
