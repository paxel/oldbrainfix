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

Amended by #118 (2026-10-30): Photo Texts are read with RustO (crate `rusto-rs` 0.3.2, MIT, pure Rust on rten 0.27) and PaddleOCR's PP-OCRv6 Tiny models (FP32, Apache-2.0, as converted to `.rten` by the RustO project), on the phone and the desktop alike, in place of ocrs. ocrs's alphabet is ASCII only: it wrote "?" and "E" for ä, ö, ü, ß and €, and Tiny gets far fewer characters of German pages wrong (see "The comparison" below). Tiny takes 1.6 to 1.9 times ocrs's time per photo on the desktop and less than half its memory; phone timing was not measured.

- The models are bundled, as ocrs's were: det.rten, rec.rten and the character list dict.txt, 6.4 MB against ocrs's 12.2 MB, in the APK's assets (`assets/rusto/`, see its NOTICE) and built into the desktop app. Nothing is downloaded. RustO loads models only from files, so both apps run them through a small session of their own that loads them from bytes; the phone hands in the character list as a third byte array.
- The output is RustO's Spatial text: lines as laid out on the photo, so a receipt's price stays on the line of its item and tilted pages keep their line order.
- Lines of a curved page were dropped without a trace with RustO's default box threshold of 0.6. Both apps set `OcrRunOptions.postprocess.boxThreshold` to 0.3, which keeps them. It must be the top-level `postprocess`: RustO 0.3.2 accepts `detection.postprocess` but ignores it (reported upstream as [byrizki/rusto-rs#21](https://github.com/byrizki/rusto-rs/issues/21)). A test page from that report guards the setting in both apps' tests. A strongly curved book page still loses or garbles some lines (40–44% against ocrs's 8.4% on that photo).
- The native library grows by about 0.8 MB on arm64-v8a and 0.5 MB on armeabi-v7a, the desktop binary by about 1.1 MB.

### The comparison

Both engines were run as the apps call them, ocrs 0.13.1 with its bundled models and RustO 0.3.2 with PP-OCRv6 Tiny, on two sets of photos. The character error rate (CER) is the edit distance to a hand-checked text divided by that text's length, pooled over the set, with runs of whitespace taken as one space.

- The synthetic set: 61 photos, 60 of them rendered German pages (print, receipts, screenshots, whiteboards, handwriting fonts, each clean and in four phone-like variants with rotation, perspective, glare, blur, noise and JPEG compression) and one real photo of a book cover. The texts hold 305 umlauts and ß and 30 €.
- The real photos: eleven photos taken with phones, prepared as the phone keeps them (upright by Exif, at most 2048 px, JPEG quality 85), among them screenshots, print, signs, a receipt, a strongly curved book page, an upside-down letter and chalk handwriting, 4177 characters in all. Their texts are not published.

| CER in %                                                     |                         Synthetic set | Real photos |
| ------------------------------------------------------------ | ------------------------------------: | ----------: |
| ocrs                                                         | 19.2 (15.6 with ä, ö, ü and ß masked) |        36.9 |
| Tiny, RustO's defaults (box threshold 0.6, lines)            |                                   5.4 |        42.1 |
| Tiny, Spatial output, box threshold 0.3 (as the apps run it) |                                   4.4 |        32.0 |

Neither engine reads the upside-down letter or the chalk handwriting. At the default box threshold, Tiny with Spatial output read 8 of the 11 real photos better than ocrs and 2 worse; at 0.3 only the pooled figure was measured. The strongly curved book page is the exception: Tiny still loses or garbles some of its lines at 0.3, and even at 0.1, with a CER of 40–44% on it against ocrs's 8.4%, and a lost line leaves no sign that it is gone. Per photo on the desktop, an AMD Ryzen AI 9 HX 370 with a release build, Tiny took 251 ms against ocrs's 132 ms on the synthetic set and 374 ms against 229 ms on the real photos, at a peak of 304 MB of memory against 829 MB.
