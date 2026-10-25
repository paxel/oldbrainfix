# Whisper for speech and ocrs for photo text, on every Device, no Google

Voice Notes are transcribed with Whisper (the same vendored whisper.cpp sources on Android and desktop, each through a thin bridge of its own), and text in photos is recognised with ocrs, the Rust engine Catlog's desktop already uses, on both platforms. One engine per job gives the same results on phone and desktop, runs fully offline, and keeps Google libraries out of a privacy-first app. The Whisper model is not bundled; it is downloaded once on request from a mirror in Old Brain Fix's own GitHub releases, verified by a checksum pinned in the app, or picked from a local file. The recording is always kept, so a missing or bad Transcript never loses a Note.

## Considered Options

- Android's on-device speech recognizer (rejected: missing on some devices, nothing on desktop).
- ML Kit text recognition as in Catlog's phone app (rejected: Google library that may send usage statistics).
- Downloading the model from Hugging Face (rejected: a second third-party host in the privacy statement).
