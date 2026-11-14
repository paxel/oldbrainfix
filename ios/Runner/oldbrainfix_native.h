// The C functions of the iPhone's native libraries (#239), built by
// tool/ios-native.sh and linked into the Runner:
//
// - RustO for Photo Texts: the crate android/ocr, its `ios` module.
// - The desktop's PDF reader for Attachment Texts (#267, ADR 0047): the
//   same crate, its `pdf::ios` module.
// - whisper.cpp for Transcripts: the desktop's C bridge,
//   desktop/crates/oldbrainfix-whisper/native/obf_whisper.cpp.
//
// Both are used from one thread at a time each (Ocr's and Speech's
// queues). Every text they answer is UTF-8, freed by its own free
// function.

#ifndef OLDBRAINFIX_NATIVE_H
#define OLDBRAINFIX_NATIVE_H

#include <stddef.h>
#include <stdint.h>

// RustO with both models loaded, behind a pointer.
typedef struct ObfOcrReader ObfOcrReader;

// A reader with the bytes of det.rten, rec.rten and dict.txt, or null
// when they could not be loaded.
ObfOcrReader *obf_ocr_open(const uint8_t *detection, size_t detection_len,
                           const uint8_t *recognition, size_t recognition_len,
                           const uint8_t *characters, size_t characters_len);

// The text in [width] × [height] RGBA pixels, row by row: empty when
// there is none, null when it could not be read.
char *obf_ocr_read(ObfOcrReader *reader, const uint8_t *rgba, size_t len, uint32_t width,
                   uint32_t height);

void obf_ocr_free_text(char *text);

void obf_ocr_close(ObfOcrReader *reader);

// The pages of the PDF at the UTF-8 [path], -1 when it cannot be read.
int64_t obf_pdf_pages(const char *path);

// The page [index] of the PDF at [path], counted from 0, its length
// written to [len]: the length of its text in UTF-8 as four bytes, most
// significant first, the text, then the picture of a scanned page; null
// when it cannot be read. Freed by obf_pdf_free with that length.
uint8_t *obf_pdf_page(const char *path, int64_t index, size_t *len);

void obf_pdf_free(uint8_t *page, size_t len);

// A whisper.cpp model, behind a pointer.
typedef struct whisper_context whisper_context;

// The model at the UTF-8 [path], or null when it cannot be loaded.
whisper_context *obf_whisper_open(const char *path);

// The words in [count] 16 kHz mono [samples], led in with the [english]
// or [german] prompt, whichever language Whisper hears; null when they
// could not be transcribed.
char *obf_whisper_transcribe(whisper_context *ctx, const float *samples, int count, int threads,
                             const char *english, const char *german);

void obf_whisper_free_text(char *text);

void obf_whisper_close(whisper_context *ctx);

#endif
