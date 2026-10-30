package io.github.paxel.oldbrainfix

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.system.Os
import java.util.concurrent.Executors

/**
 * RustO through the app's Rust library (android/ocr). 0 handles mean "no
 * models loaded". [loaded] is false where the library is not built for
 * the Device's ABI: then no text is read, and nothing crashes.
 */
object OcrNative {
    val loaded: Boolean = runCatching {
        // RustO reads with one thread fewer than the Device has cores, at
        // most four, like Whisper; set before its thread pool starts.
        val threads = Speech.threadsFor(Runtime.getRuntime().availableProcessors())
        Os.setenv("RTEN_NUM_THREADS", threads.toString(), true)
        System.loadLibrary("oldbrainfix_ocr")
    }.isSuccess

    external fun open(detection: ByteArray, recognition: ByteArray, characters: ByteArray): Long

    external fun read(reader: Long, rgba: ByteArray, width: Int, height: Int): String?

    external fun close(reader: Long)
}

/**
 * Photo Texts (ADR 0007): the text in a photo, read on the phone by
 * RustO with the PP-OCRv6 Tiny detection and recognition models and
 * their character list bundled in the app's assets (assets/rusto/, see
 * its NOTICE). Offline: nothing is fetched.
 */
class Ocr(private val context: Context) {
    /** The one thread reading: photos go one at a time. */
    private val worker = Executors.newSingleThreadExecutor()

    /** The models loaded on [worker], until [idle]; 0 while none are. */
    private var reader = 0L

    /**
     * The text in the photo [jpeg], on the reading thread: [done] hears
     * it, empty when there is none, or null when it could not be read.
     */
    fun read(jpeg: ByteArray, done: (String?) -> Unit) {
        worker.execute {
            done(runCatching { readNow(jpeg) }.getOrNull())
        }
    }

    /** Nothing more to read for now: the models' memory goes. */
    fun idle() {
        worker.execute { closeReader() }
    }

    /** Lets go of everything; the app is gone. */
    fun release() {
        idle()
        worker.shutdown()
    }

    private fun readNow(jpeg: ByteArray): String? {
        if (!OcrNative.loaded) return null
        val options = BitmapFactory.Options().apply { inPreferredConfig = Bitmap.Config.ARGB_8888 }
        val bitmap = BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size, options) ?: return null
        val width = bitmap.width
        val height = bitmap.height
        val argb = IntArray(width * height)
        bitmap.getPixels(argb, 0, width, 0, 0, width, height)
        bitmap.recycle()
        if (reader == 0L) {
            reader = OcrNative.open(asset(DETECTION), asset(RECOGNITION), asset(CHARACTERS))
        }
        if (reader == 0L) return null
        return OcrNative.read(reader, rgba(argb), width, height)
    }

    private fun asset(name: String): ByteArray =
        context.assets.open("$ASSETS/$name").use { it.readBytes() }

    private fun closeReader() {
        if (reader != 0L) OcrNative.close(reader)
        reader = 0L
    }

    companion object {
        const val ASSETS = "rusto"
        const val DETECTION = "det.rten"
        const val RECOGNITION = "rec.rten"
        const val CHARACTERS = "dict.txt"

        /** Android's ARGB pixels as the RGBA bytes RustO reads, row by row. */
        fun rgba(argb: IntArray): ByteArray {
            val bytes = ByteArray(argb.size * 4)
            for ((i, pixel) in argb.withIndex()) {
                bytes[i * 4] = (pixel shr 16).toByte()
                bytes[i * 4 + 1] = (pixel shr 8).toByte()
                bytes[i * 4 + 2] = pixel.toByte()
                bytes[i * 4 + 3] = (pixel ushr 24).toByte()
            }
            return bytes
        }
    }
}
