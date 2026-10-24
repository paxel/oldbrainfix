package io.github.paxel.oldbrainfix

import android.content.Context
import android.media.AudioFormat
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import java.net.URL
import java.nio.ByteOrder
import java.security.MessageDigest
import java.util.concurrent.Executors
import javax.net.ssl.HttpsURLConnection

/**
 * whisper.cpp through the app's JNI bridge (cpp/whisper_jni.cpp). Null
 * handles mean "no model loaded". [loaded] is false where the library is
 * not built for the Device's ABI: then there is no transcription, and
 * nothing crashes.
 */
object WhisperNative {
    val loaded: Boolean = runCatching { System.loadLibrary("oldbrainfix_whisper") }.isSuccess

    external fun open(model: String): Long

    external fun transcribe(context: Long, samples: FloatArray, threads: Int): String?

    external fun close(context: Long)
}

/**
 * Transcripts for voice Notes (ADR 0007): Whisper on the phone, with a
 * model the user fetched on request, from Old Brain Fix's own GitHub
 * release or from a file of their own. The app's Dart side checks every
 * model against the checksum it pins before it is kept or used; this
 * only moves the bytes and says what their SHA-256 is.
 *
 * The model lives in the no-backup files: 142 MB has no place in a
 * backup, and it can always be fetched again.
 */
class Speech(private val context: Context) {
    /** The one thread transcribing: recordings go one at a time. */
    private val worker = Executors.newSingleThreadExecutor()

    /** The model loaded on [worker], until [idle]; 0 while none is. */
    private var loadedModel = 0L

    @Volatile
    private var cancelled = false

    private val dir: File
        get() = File(context.noBackupFilesDir, "whisper").apply { mkdirs() }

    /** The model in use, once it was checked and kept. */
    private val modelFile: File
        get() = File(dir, MODEL_NAME)

    /** A model on its way in: downloaded or picked, not yet checked. */
    private val incomingFile: File
        get() = File(dir, "$MODEL_NAME.part")

    /** The SHA-256 of the model in use, or null when there is none. */
    fun modelHash(): String? = modelFile.takeIf { it.isFile }?.inputStream()?.use {
        copyHashing(it, Nowhere)
    }

    /**
     * Downloads [url] to the side and answers its SHA-256, or null when
     * it failed or was [cancel]led. Only HTTPS. [progress] hears the
     * bytes so far and the whole size, -1 while unknown.
     */
    fun download(url: String, progress: (Long, Long) -> Unit): String? {
        cancelled = false
        val file = incomingFile.apply { delete() }
        val connection = URL(url).openConnection() as? HttpsURLConnection ?: return null
        return try {
            connection.instanceFollowRedirects = true
            connection.connectTimeout = TIMEOUT_MS
            connection.readTimeout = TIMEOUT_MS
            if (connection.responseCode != HttpsURLConnection.HTTP_OK) return null
            val total = connection.contentLengthLong
            connection.inputStream.use { input ->
                file.outputStream().use { output ->
                    copyHashing(input, output, { cancelled }) { progress(it, total) }
                }
            }
        } catch (e: Exception) {
            null
        } finally {
            connection.disconnect()
        }.also { if (it == null) file.delete() }
    }

    /** Copies a model the user picked to the side; its SHA-256, or null. */
    fun takeIn(input: () -> InputStream?): String? {
        cancelled = false
        val file = incomingFile.apply { delete() }
        return try {
            input()?.use { stream -> file.outputStream().use { copyHashing(stream, it, { cancelled }) } }
        } catch (e: Exception) {
            null
        }.also { if (it == null) file.delete() }
    }

    /** Stops a download or copy running. */
    fun cancel() {
        cancelled = true
    }

    /** Keeps the model waiting on the side as the one in use. */
    fun install(): Boolean {
        val incoming = incomingFile
        if (!incoming.isFile) return false
        worker.submit { closeModel() }.get()
        return incoming.renameTo(modelFile)
    }

    /** Drops the model waiting on the side. */
    fun discard() {
        incomingFile.delete()
    }

    /** Drops the model in use. */
    fun remove() {
        worker.submit { closeModel() }.get()
        modelFile.delete()
    }

    /**
     * The Transcript of the voice recording [bytes], on the transcribing
     * thread: [done] hears its text, or null when it could not be read
     * or there is no model.
     */
    fun transcribe(bytes: ByteArray, done: (String?) -> Unit) {
        worker.execute {
            done(runCatching { transcribeNow(bytes) }.getOrNull())
        }
    }

    /** Nothing more to transcribe for now: the model's memory goes. */
    fun idle() {
        worker.execute { closeModel() }
    }

    /** Lets go of everything; the app is gone. */
    fun release() {
        cancel()
        idle()
        worker.shutdown()
    }

    private fun transcribeNow(bytes: ByteArray): String? {
        if (!WhisperNative.loaded || !modelFile.isFile) return null
        val file = File(context.cacheDir, "transcribing.m4a").apply { writeBytes(bytes) }
        val samples = try {
            decode(file) ?: return null
        } finally {
            file.delete()
        }
        if (loadedModel == 0L) loadedModel = WhisperNative.open(modelFile.path)
        if (loadedModel == 0L) return null
        val threads = threadsFor(Runtime.getRuntime().availableProcessors())
        return WhisperNative.transcribe(loadedModel, samples, threads)?.trim()
    }

    private fun closeModel() {
        if (loadedModel != 0L) WhisperNative.close(loadedModel)
        loadedModel = 0L
    }

    companion object {
        const val MODEL_NAME = "ggml-base.bin"
        private const val TIMEOUT_MS = 30_000
        private const val TIMEOUT_US = 10_000L

        /**
         * The recording in [file] as Whisper hears it: 16 kHz mono
         * samples from -1 to 1, decoded by the Device's own MediaCodec;
         * null when it cannot be read.
         */
        fun decode(file: File): FloatArray? {
            val extractor = MediaExtractor()
            var codec: MediaCodec? = null
            try {
                extractor.setDataSource(file.path)
                val track = (0 until extractor.trackCount).firstOrNull {
                    extractor.getTrackFormat(it).getString(MediaFormat.KEY_MIME)?.startsWith("audio/") == true
                } ?: return null
                extractor.selectTrack(track)
                val format = extractor.getTrackFormat(track)
                val decoder = MediaCodec.createDecoderByType(format.getString(MediaFormat.KEY_MIME) ?: return null)
                codec = decoder
                decoder.configure(format, null, null, 0)
                decoder.start()
                // What comes out may differ from what went in: read it
                // from the decoder's output format.
                var rate = format.getInteger(MediaFormat.KEY_SAMPLE_RATE)
                var channels = format.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
                var floats = false
                val pcm = FloatCollector()
                val info = MediaCodec.BufferInfo()
                var inputDone = false
                while (true) {
                    if (!inputDone) {
                        val index = decoder.dequeueInputBuffer(TIMEOUT_US)
                        if (index >= 0) {
                            val buffer = decoder.getInputBuffer(index) ?: return null
                            val size = extractor.readSampleData(buffer, 0)
                            if (size < 0) {
                                decoder.queueInputBuffer(index, 0, 0, 0, MediaCodec.BUFFER_FLAG_END_OF_STREAM)
                                inputDone = true
                            } else {
                                decoder.queueInputBuffer(index, 0, size, extractor.sampleTime, 0)
                                extractor.advance()
                            }
                        }
                    }
                    val index = decoder.dequeueOutputBuffer(info, TIMEOUT_US)
                    if (index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                        val out = decoder.outputFormat
                        rate = out.getInteger(MediaFormat.KEY_SAMPLE_RATE)
                        channels = out.getInteger(MediaFormat.KEY_CHANNEL_COUNT)
                        floats = out.containsKey(MediaFormat.KEY_PCM_ENCODING) &&
                            out.getInteger(MediaFormat.KEY_PCM_ENCODING) == AudioFormat.ENCODING_PCM_FLOAT
                    } else if (index >= 0) {
                        val buffer = decoder.getOutputBuffer(index) ?: return null
                        buffer.position(info.offset)
                        buffer.limit(info.offset + info.size)
                        buffer.order(ByteOrder.nativeOrder())
                        if (floats) {
                            val chunk = FloatArray(info.size / 4)
                            buffer.asFloatBuffer().get(chunk)
                            pcm.add(chunk)
                        } else {
                            val chunk = ShortArray(info.size / 2)
                            buffer.asShortBuffer().get(chunk)
                            pcm.add(pcm16ToFloat(chunk))
                        }
                        decoder.releaseOutputBuffer(index, false)
                        if (info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0) break
                    }
                }
                return toWhisper(pcm.toArray(), channels, rate)
            } catch (e: Exception) {
                return null
            } finally {
                codec?.runCatching { stop() }
                codec?.release()
                extractor.release()
            }
        }

        /** Whisper's own rate. */
        const val WHISPER_RATE = 16_000

        /** 16-bit PCM samples as floats from -1 to 1. */
        fun pcm16ToFloat(samples: ShortArray): FloatArray =
            FloatArray(samples.size) { samples[it] / 32768f }

        /**
         * [interleaved] samples of [channels] channels at [rate] as
         * Whisper wants them: mono, the channels averaged, at 16 kHz.
         * A recording made by [Voice] already is, and comes back as it is.
         */
        fun toWhisper(interleaved: FloatArray, channels: Int, rate: Int): FloatArray =
            resample(downmix(interleaved, channels), rate, WHISPER_RATE)

        /** The average of the [channels] channels of [interleaved]; a cut-off last frame is dropped. */
        fun downmix(interleaved: FloatArray, channels: Int): FloatArray {
            if (channels <= 1) return interleaved
            return FloatArray(interleaved.size / channels) { frame ->
                var sum = 0f
                for (c in 0 until channels) sum += interleaved[frame * channels + c]
                sum / channels
            }
        }

        /** [samples] at [from] Hz linearly interpolated to [to] Hz. */
        fun resample(samples: FloatArray, from: Int, to: Int): FloatArray {
            if (from == to || samples.isEmpty() || from <= 0) return samples
            val count = (samples.size.toLong() * to / from).toInt()
            val step = from.toDouble() / to
            return FloatArray(count) { i ->
                val at = i * step
                val left = at.toInt().coerceAtMost(samples.size - 1)
                val right = (left + 1).coerceAtMost(samples.size - 1)
                val part = (at - left).toFloat()
                samples[left] + (samples[right] - samples[left]) * part
            }
        }

        /** How many threads Whisper gets on [cores] cores: one is left for the app, at most four. */
        fun threadsFor(cores: Int): Int = (cores - 1).coerceIn(1, 4)

        /**
         * Copies [input] to [output] and answers the SHA-256 of what was
         * copied, as lowercase hex; null when [stop] said so on the way.
         * [copied] hears the bytes so far, about every megabyte and at
         * the end.
         */
        fun copyHashing(
            input: InputStream,
            output: OutputStream,
            stop: () -> Boolean = { false },
            copied: (Long) -> Unit = {},
        ): String? {
            val digest = MessageDigest.getInstance("SHA-256")
            val buffer = ByteArray(64 * 1024)
            var total = 0L
            var told = 0L
            while (true) {
                if (stop()) return null
                val read = input.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
                output.write(buffer, 0, read)
                total += read
                if (total - told >= 1 shl 20) {
                    told = total
                    copied(total)
                }
            }
            copied(total)
            return hex(digest.digest())
        }

        /** [bytes] as lowercase hex. */
        fun hex(bytes: ByteArray): String =
            bytes.joinToString("") { "%02x".format(it.toInt() and 0xFF) }
    }
}

/** Where bytes go that are only hashed. */
private object Nowhere : OutputStream() {
    override fun write(b: Int) {}

    override fun write(b: ByteArray, off: Int, len: Int) {}
}

/** Float chunks gathered while decoding, joined once at the end. */
private class FloatCollector {
    private val chunks = mutableListOf<FloatArray>()
    private var size = 0

    fun add(chunk: FloatArray) {
        chunks += chunk
        size += chunk.size
    }

    fun toArray(): FloatArray {
        val all = FloatArray(size)
        var at = 0
        for (chunk in chunks) {
            chunk.copyInto(all, at)
            at += chunk.size
        }
        return all
    }
}
