// The C bridge from Rust to whisper.cpp for the desktop's voice Notes,
// the desktop's twin of the phone's whisper_jni.cpp: load a model,
// transcribe 16 kHz mono float samples in English or German, let the
// model go. Rust only passes pointers and numbers, never whisper's own
// structs. Everything runs on the caller's thread.

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "whisper.h"

namespace {

// whisper.cpp talks a lot; nothing of it reaches the terminal.
void log_quietly(ggml_log_level, const char *, void *) {}

// English or German, whichever the start of [samples] sounds more like.
// whisper's own "auto" could pick any of its languages; Old Brain Fix
// speaks only these two.
const char *english_or_german(whisper_context *ctx, const float *samples, int count,
                              int threads) {
    if (whisper_pcm_to_mel(ctx, samples, count, threads) != 0) return "en";
    std::vector<float> probs(whisper_lang_max_id() + 1, 0.0f);
    if (whisper_lang_auto_detect(ctx, 0, threads, probs.data()) < 0) return "en";
    return probs[whisper_lang_id("de")] > probs[whisper_lang_id("en")] ? "de" : "en";
}

}  // namespace

extern "C" {

// The model at the UTF-8 [path], or null when it cannot be loaded.
whisper_context *obf_whisper_open(const char *path) {
    whisper_log_set(log_quietly, nullptr);
    whisper_context_params params = whisper_context_default_params();
    params.use_gpu = false;
    return whisper_init_from_file_with_params(path, params);
}

// The words in [count] [samples] as UTF-8, to be freed with
// obf_whisper_free_text; null when they could not be transcribed.
char *obf_whisper_transcribe(whisper_context *ctx, const float *samples, int count,
                             int threads) {
    if (ctx == nullptr || samples == nullptr || count <= 0) return nullptr;
    whisper_full_params params = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    params.n_threads = std::max(1, threads);
    params.language = english_or_german(ctx, samples, count, params.n_threads);
    params.translate = false;
    params.no_timestamps = true;
    params.print_progress = false;
    params.print_realtime = false;
    params.print_special = false;
    params.print_timestamps = false;
    if (whisper_full(ctx, params, samples, count) != 0) return nullptr;
    std::string text;
    const int segments = whisper_full_n_segments(ctx);
    for (int i = 0; i < segments; ++i) text += whisper_full_get_segment_text(ctx, i);
    char *out = static_cast<char *>(std::malloc(text.size() + 1));
    if (out == nullptr) return nullptr;
    std::memcpy(out, text.c_str(), text.size() + 1);
    return out;
}

void obf_whisper_free_text(char *text) { std::free(text); }

void obf_whisper_close(whisper_context *ctx) {
    if (ctx != nullptr) whisper_free(ctx);
}

}  // extern "C"
