// The JNI bridge to whisper.cpp for voice Notes (Speech.kt's
// WhisperNative): load a model, transcribe 16 kHz mono float samples in
// English or German, led in with the prompt for that language, let the
// model go. Everything runs on the caller's
// thread; Speech.kt calls it from its own background thread.

#include <jni.h>

#include <android/log.h>

#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

#include "whisper.h"

namespace {

// whisper.cpp talks a lot; only its warnings and errors reach logcat.
void log_quietly(ggml_log_level level, const char *text, void *) {
    if (level < GGML_LOG_LEVEL_WARN) return;
    __android_log_write(ANDROID_LOG_WARN, "OldBrainFixWhisper", text);
}

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

// The UTF-8 bytes of [bytes] as a string; empty for null. Bytes, not a
// jstring: GetStringUTFChars gives modified UTF-8, which differs for
// characters beyond the BMP, such as an emoji in an Area's name.
std::string utf8(JNIEnv *env, jbyteArray bytes) {
    if (bytes == nullptr) return {};
    const jsize size = env->GetArrayLength(bytes);
    std::string text(static_cast<size_t>(size), '\0');
    env->GetByteArrayRegion(bytes, 0, size, reinterpret_cast<jbyte *>(&text[0]));
    return text;
}

}  // namespace

extern "C" JNIEXPORT jlong JNICALL
Java_io_github_paxel_oldbrainfix_WhisperNative_open(JNIEnv *env, jobject, jstring model) {
    whisper_log_set(log_quietly, nullptr);
    const char *path = env->GetStringUTFChars(model, nullptr);
    whisper_context_params params = whisper_context_default_params();
    params.use_gpu = false;
    whisper_context *ctx = whisper_init_from_file_with_params(path, params);
    env->ReleaseStringUTFChars(model, path);
    return reinterpret_cast<jlong>(ctx);
}

extern "C" JNIEXPORT jstring JNICALL
Java_io_github_paxel_oldbrainfix_WhisperNative_transcribe(JNIEnv *env, jobject, jlong handle,
                                                           jfloatArray samples, jint threads,
                                                           jbyteArray english, jbyteArray german) {
    auto *ctx = reinterpret_cast<whisper_context *>(handle);
    if (ctx == nullptr) return nullptr;
    const jsize count = env->GetArrayLength(samples);
    std::vector<float> pcm(static_cast<size_t>(count));
    env->GetFloatArrayRegion(samples, 0, count, pcm.data());

    whisper_full_params params = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
    params.n_threads = std::max(1, static_cast<int>(threads));
    params.language = english_or_german(ctx, pcm.data(), count, params.n_threads);
    // Kept until whisper_full is done with it.
    const std::string prompt = utf8(env, std::strcmp(params.language, "de") == 0 ? german : english);
    params.initial_prompt = prompt.empty() ? nullptr : prompt.c_str();
    params.translate = false;
    params.no_timestamps = true;
    params.print_progress = false;
    params.print_realtime = false;
    params.print_special = false;
    params.print_timestamps = false;
    if (whisper_full(ctx, params, pcm.data(), count) != 0) return nullptr;

    std::string text;
    const int segments = whisper_full_n_segments(ctx);
    for (int i = 0; i < segments; ++i) text += whisper_full_get_segment_text(ctx, i);
    // whisper's text is UTF-8; NewStringUTF wants modified UTF-8, which
    // differs for characters beyond the BMP, so go through bytes.
    jbyteArray bytes = env->NewByteArray(static_cast<jsize>(text.size()));
    env->SetByteArrayRegion(bytes, 0, static_cast<jsize>(text.size()),
                            reinterpret_cast<const jbyte *>(text.data()));
    jclass string_class = env->FindClass("java/lang/String");
    jmethodID init = env->GetMethodID(string_class, "<init>", "([BLjava/lang/String;)V");
    jstring charset = env->NewStringUTF("UTF-8");
    auto result = static_cast<jstring>(env->NewObject(string_class, init, bytes, charset));
    env->DeleteLocalRef(charset);
    env->DeleteLocalRef(bytes);
    env->DeleteLocalRef(string_class);
    return result;
}

extern "C" JNIEXPORT void JNICALL
Java_io_github_paxel_oldbrainfix_WhisperNative_close(JNIEnv *, jobject, jlong handle) {
    auto *ctx = reinterpret_cast<whisper_context *>(handle);
    if (ctx != nullptr) whisper_free(ctx);
}
