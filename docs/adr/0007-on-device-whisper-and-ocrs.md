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

## RustO in place of ocrs (#118)

Amended by #118 (2026-10-30): Photo Texts are read with RustO (crate `rusto-rs` 0.3.2, MIT, pure Rust on rten 0.27) and PaddleOCR's PP-OCRv6 Tiny models (FP32, Apache-2.0, as converted to `.rten` by the RustO project), on the phone and the desktop alike, in place of ocrs. ocrs's alphabet is ASCII only: it wrote "?" and "E" for ä, ö, ü, ß and €. The [comparison in #118](https://github.com/paxel/oldbrainfix/issues/118) found on 61 test photos, 60 of them rendered German pages, a character error rate of 4.4% for Tiny as set up below (5.4% with RustO's defaults) against 19.2% for ocrs (15.6% with the German letters masked), and on eleven real photos 32.0% for Tiny with Spatial output against 36.9%, better on 8 of 11. Tiny takes 1.6 to 1.9 times ocrs's time per photo on the desktop and less than half its memory; phone timing was not measured.

- The models are bundled, as ocrs's were: det.rten, rec.rten and the character list dict.txt, 6.4 MB against ocrs's 12.2 MB, in the APK's assets (`assets/rusto/`, see its NOTICE) and built into the desktop app. Nothing is downloaded. RustO loads models only from files, so both apps run them through a small session of their own that loads them from bytes; the phone hands in the character list as a third byte array.
- The output is RustO's Spatial text: lines as laid out on the photo, so a receipt's price stays on the line of its item and tilted pages keep their line order.
- Lines of a curved page were dropped without a trace with RustO's default box threshold of 0.6. Both apps set `OcrRunOptions.postprocess.boxThreshold` to 0.3, which keeps them. It must be the top-level `postprocess`: RustO 0.3.2 accepts `detection.postprocess` but ignores it (reported upstream as [byrizki/rusto-rs#21](https://github.com/byrizki/rusto-rs/issues/21)). A test page from that report guards the setting in both apps' tests. A strongly curved book page still loses or garbles some lines (40–44% against ocrs's 8.4% on that photo).
- The native library grows by about 0.8 MB on arm64-v8a and 0.5 MB on armeabi-v7a, the desktop binary by about 1.1 MB.
