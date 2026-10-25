package io.github.paxel.oldbrainfix

import android.graphics.BitmapFactory
import android.os.Build
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.NotFoundException
import com.google.zxing.RGBLuminanceSource
import com.google.zxing.ReaderException
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader

/**
 * Pairing on the phone: the pair code read from a photo of the
 * computer's screen with zxing's QR reader (pure Java, Apache-2.0, no
 * Google service), and the name the phone goes by in a sync.
 */
object Pairing {
    /** The long edge a photo is brought down to before reading: plenty for a QR on a screen. */
    const val READ_EDGE = 1024

    /** The text of the QR code in the ARGB [pixels] of a [width]×[height] picture, or null. */
    fun readQr(pixels: IntArray, width: Int, height: Int): String? {
        val source = RGBLuminanceSource(width, height, pixels)
        val hints = mapOf(
            DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE),
            DecodeHintType.TRY_HARDER to true,
        )
        for (binary in listOf(BinaryBitmap(HybridBinarizer(source)), BinaryBitmap(HybridBinarizer(source.invert())))) {
            try {
                return QRCodeReader().decode(binary, hints).text
            } catch (_: NotFoundException) {
                // Try the inverted picture: a dark screen shows light modules.
            } catch (_: ReaderException) {
                // A code that is there but unreadable: as if none.
            }
        }
        return null
    }

    /** The QR code's text in the JPEG [photo], or null when it shows none. */
    fun readQr(photo: ByteArray): String? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(photo, 0, photo.size, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        val options = BitmapFactory.Options().apply {
            inSampleSize = Photos.sampleSize(bounds.outWidth, bounds.outHeight, READ_EDGE)
        }
        val bitmap = BitmapFactory.decodeByteArray(photo, 0, photo.size, options) ?: return null
        val pixels = IntArray(bitmap.width * bitmap.height)
        bitmap.getPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
        val text = readQr(pixels, bitmap.width, bitmap.height)
        bitmap.recycle()
        return text
    }

    /** The phone's maker and model, such as "Google Pixel 8". */
    fun deviceName(): String = deviceName(Build.MANUFACTURER ?: "", Build.MODEL ?: "")

    /** [model], with [maker] in front unless the model already names it. */
    fun deviceName(maker: String, model: String): String = when {
        model.isBlank() -> maker.trim()
        maker.isBlank() || model.startsWith(maker, ignoreCase = true) -> model.trim()
        else -> "${maker.replaceFirstChar { it.uppercase() }} $model".trim()
    }
}
