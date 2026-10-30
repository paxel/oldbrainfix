# Third-party notices

Old Brain Fix itself is dual-licensed under Apache-2.0 or MIT. The phone
and desktop apps ship the third-party components below. Their full
licence texts, with their copyright lines, are in [`licenses/`](licenses)
and in each app: on the phone under About, Open-source licenses; on the
desktop under Settings, About Old Brain Fix, Open-source licenses.

| Component                                                                                     | Ships in         | Licence                                                       | Full text                     |
| --------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------- | ----------------------------- |
| PP-OCRv6 Tiny models by the PaddlePaddle Authors (PaddleOCR), converted to RTen by RustO      | phone, desktop   | Apache-2.0                                                    | `licenses/common.txt`         |
| whisper.cpp and ggml (The ggml authors)                                                       | phone, desktop   | MIT                                                           | `licenses/common.txt`         |
| OpenAI Whisper base or small model (`ggml-base.bin`, `ggml-small.bin`), downloaded on request | phone, desktop   | MIT                                                           | `licenses/common.txt`         |
| Material Icons font by Google                                                                 | phone, desktop   | CC BY 4.0                                                     | `licenses/common.txt`         |
| Rust standard library                                                                         | phone, desktop   | MIT OR Apache-2.0                                             | `licenses/common.txt`         |
| Flutter, its engine and the Dart packages                                                     | phone            | BSD-3-Clause and others                                       | the app's own list            |
| AndroidX libraries                                                                            | phone            | Apache-2.0                                                    | `licenses/android.txt`        |
| Kotlin standard library, kotlinx.coroutines, JetBrains annotations                            | phone            | Apache-2.0                                                    | `licenses/android.txt`        |
| ZXing core, Guava ListenableFuture, ReLinker, JSpecify                                        | phone            | Apache-2.0                                                    | `licenses/android.txt`        |
| Rust crates of the OCR library (rusto-rs, rten, jni, …)                                       | phone            | mostly MIT OR Apache-2.0; see below                           | `licenses/android-crates.txt` |
| Rust crates of the desktop app (egui, eframe, rusqlite, rustls, symphonia, …)                 | desktop          | mostly MIT OR Apache-2.0; see below                           | `licenses/desktop-crates.txt` |
| SQLite                                                                                        | desktop          | public domain                                                 | `licenses/desktop.txt`        |
| egui's fonts: Hack, Noto Emoji, Ubuntu Light, emoji-icon-font                                 | desktop          | MIT and Bitstream Vera, OFL-1.1, Ubuntu Font Licence 1.0, MIT | `licenses/desktop.txt`        |
| KDE's Plasma Wayland protocols (through wayland-protocols-plasma)                             | desktop on Linux | LGPL-2.1-or-later, MIT-CMU, MIT, BSD-3-Clause                 | `licenses/desktop.txt`        |

The PP-OCRv6 models are credited as Apache-2.0 asks: the licence text with
PaddlePaddle's copyright line, the source, and the one change, RustO's
conversion to the RTen format (see also
[their NOTICE](android/app/src/main/assets/rusto/NOTICE)). Material Icons
are credited under the CC BY 4.0 licence the Flutter SDK ships with the
font; Google's repository offers them under Apache-2.0 as well.

Besides MIT and Apache-2.0, the desktop's crates come under ISC, BSD-2/3-Clause,
Zlib, Unicode-3.0, BSL-1.0 (clipper2-rust; clipboard-win, Windows only),
CDLA-Permissive-2.0 (webpki-roots), CC0-1.0 (hexf-parse) and MPL-2.0
(symphonia and option-ext, unmodified; their source is on crates.io). The
OCR library's crates come under MIT, Apache-2.0, BSL-1.0 (clipper2-rust)
and Zlib (wide). Where a crate offers a choice, MIT is taken, then
Apache-2.0; never a GPL or LGPL option (self_cell, r-efi). wide offers
Zlib, Apache-2.0 or MIT but ships only its Zlib text, so Zlib is taken.

## Keeping the lists current

`dart run tool/licenses.dart` writes `licenses/desktop-crates.txt` and
`licenses/android-crates.txt` with
[cargo-about](https://github.com/EmbarkStudios/cargo-about); CI fails when
they are not current, and when an Android library of the release build is
missing from `licenses/android.txt`. The other lists are kept by hand.
