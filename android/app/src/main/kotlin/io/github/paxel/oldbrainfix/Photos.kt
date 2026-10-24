package io.github.paxel.oldbrainfix

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Matrix
import android.media.ExifInterface
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.InputStream

/**
 * Photos as the app keeps them: upright, at most [MAX_EDGE] pixels on
 * the long side, JPEG without any metadata (no position, no camera), from
 * Android's own decoder: no Google library.
 */
object Photos {
    /** The long edge of a kept photo: sharp on a phone, a few hundred kB. */
    const val MAX_EDGE = 2048
    private const val QUALITY = 85

    /** Where the camera app writes the photo being taken. */
    fun cameraFile(context: Context): File =
        File(context.cacheDir, "camera").apply { mkdirs() }.let { File(it, "shot.jpg") }

    /**
     * The largest power of two to decode [width]×[height] by so that its
     * long edge still reaches [maxEdge]: decoding small saves memory, the
     * exact size comes after.
     */
    fun sampleSize(width: Int, height: Int, maxEdge: Int): Int {
        val long = maxOf(width, height)
        var sample = 1
        while (long / (sample * 2) >= maxEdge) sample *= 2
        return sample
    }

    /** The factor that brings the long edge of [width]×[height] down to [maxEdge]; never up. */
    fun scale(width: Int, height: Int, maxEdge: Int): Float {
        val long = maxOf(width, height)
        return if (long <= maxEdge) 1f else maxEdge.toFloat() / long
    }

    /** Degrees to turn a photo by for its Exif orientation tag to stand upright. */
    fun rotation(orientation: Int): Int = when (orientation) {
        3, 4 -> 180 // ORIENTATION_ROTATE_180, FLIP_VERTICAL
        5, 6 -> 90 // TRANSPOSE, ROTATE_90
        7, 8 -> 270 // TRANSVERSE, ROTATE_270
        else -> 0
    }

    /**
     * The picture [open] reads, as the app keeps photos; null when it is
     * no picture Android can decode. [open] is called more than once.
     */
    fun normalize(open: () -> InputStream?): ByteArray? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        runCatching { open()?.use { BitmapFactory.decodeStream(it, null, bounds) } }
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        val orientation = runCatching {
            open()?.use {
                ExifInterface(it).getAttributeInt(
                    ExifInterface.TAG_ORIENTATION,
                    ExifInterface.ORIENTATION_NORMAL,
                )
            }
        }.getOrNull() ?: ExifInterface.ORIENTATION_NORMAL
        val options = BitmapFactory.Options().apply {
            inSampleSize = sampleSize(bounds.outWidth, bounds.outHeight, MAX_EDGE)
        }
        val decoded = runCatching {
            open()?.use { BitmapFactory.decodeStream(it, null, options) }
        }.getOrNull() ?: return null
        val factor = scale(decoded.width, decoded.height, MAX_EDGE)
        val matrix = Matrix().apply {
            postScale(factor, factor)
            postRotate(rotation(orientation).toFloat())
        }
        val shaped = if (matrix.isIdentity) decoded else
            Bitmap.createBitmap(decoded, 0, 0, decoded.width, decoded.height, matrix, true)
        // A JPEG has no transparency: a see-through screenshot goes on white.
        val opaque = if (!shaped.hasAlpha()) shaped else
            Bitmap.createBitmap(shaped.width, shaped.height, Bitmap.Config.ARGB_8888).also {
                Canvas(it).apply {
                    drawColor(Color.WHITE)
                    drawBitmap(shaped, 0f, 0f, null)
                }
            }
        val out = ByteArrayOutputStream()
        if (!opaque.compress(Bitmap.CompressFormat.JPEG, QUALITY, out)) return null
        return out.toByteArray()
    }
}
