package io.github.paxel.oldbrainfix

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import androidx.core.content.FileProvider
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.UUID
import kotlin.math.max
import kotlin.math.roundToInt
import kotlin.math.sqrt

/**
 * The PDF reader of Attachment Texts (#267, ADR 0047): the desktop's
 * `oldbrainfix-pdf` in the app's Rust library beside RustO
 * (android/ocr/src/pdf.rs). Each call reads the PDF at a path in the
 * app's cache and gives one page, so at most one scan is in memory.
 */
object PdfNative {
    /** Whether the library is built for this Device's ABI; it loads with RustO. */
    val loaded: Boolean get() = OcrNative.loaded

    external fun pages(path: String): Int

    external fun page(path: String, index: Int): ByteArray?

    /**
     * A page as [page] encodes it: the length of its text in UTF-8 as
     * four bytes, most significant first, the text, then the picture of
     * a scanned page; null for bytes that are no page.
     */
    fun decode(bytes: ByteArray): Map<String, Any?>? {
        if (bytes.size < 4) return null
        val len = ((bytes[0].toInt() and 0xFF) shl 24) or
            ((bytes[1].toInt() and 0xFF) shl 16) or
            ((bytes[2].toInt() and 0xFF) shl 8) or
            (bytes[3].toInt() and 0xFF)
        if (len < 0 || len > bytes.size - 4) return null
        val text = String(bytes, 4, len, Charsets.UTF_8)
        val scan = bytes.copyOfRange(4 + len, bytes.size)
        return mapOf("text" to text, "scan" to scan.takeIf { it.isNotEmpty() })
    }
}

/**
 * Attachments (#267, ADR 0047): files of any kind picked with the system's
 * file picker or shared in, copied into the app's cache with their names
 * and types while the app may read them, and a Note's file opened with
 * the system's app for it.
 */
object Files {
    /** A file copied to the side, with the [name] and [type] its provider said. */
    class Copied(val file: File, val name: String, val type: String?)

    /** Where the copies wait until the app attaches them. */
    fun dir(context: Context): File = File(context.cacheDir, "shared").apply { mkdirs() }

    /**
     * The name a file goes by: its provider's display name, else the
     * last part of its address, else "file".
     */
    fun nameOf(display: String?, lastSegment: String?): String =
        display?.takeIf(String::isNotBlank)
            ?: lastSegment?.substringAfterLast('/')?.takeIf(String::isNotBlank)
            ?: "file"

    /** The name the file behind [uri] goes by ([nameOf]). */
    fun nameOf(context: Context, uri: Uri): String {
        val display = runCatching {
            context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
                ?.use { if (it.moveToFirst()) it.getString(0) else null }
        }.getOrNull()
        return nameOf(display, uri.lastPathSegment)
    }

    /** Copies the files behind [uris] to the side, while the permission to read them lasts. */
    fun copy(context: Context, uris: List<Uri>): List<Copied> = uris.mapNotNull { uri ->
        runCatching {
            val resolver = context.contentResolver
            val type = runCatching { resolver.getType(uri) }.getOrNull()
            val file = File(dir(context), UUID.randomUUID().toString())
            resolver.openInputStream(uri)?.use { input ->
                file.outputStream().use { input.copyTo(it) }
            } ?: return@runCatching null
            Copied(file, nameOf(context, uri), type)
        }.getOrNull()
    }

    /** [files] as the app's channel hands them over: each its `path`, `name` and `type`. */
    fun handed(files: List<Copied>): List<Map<String, String?>> = files.map {
        mapOf("path" to it.file.path, "name" to it.name, "type" to it.type)
    }

    /**
     * Opens the file at [path], written into the exports the FileProvider
     * serves, as [type] with the system's app for it; false when it is no
     * such file or no app takes it.
     */
    fun open(context: Context, path: String, type: String): Boolean {
        if (!Bundles.inside(FileShare.dir(context), path)) return false
        val file = File(path)
        if (!file.isFile) return false
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.photos", file)
        val view = Intent(Intent.ACTION_VIEW)
            .setDataAndType(uri, type.ifBlank { "*/*" })
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        view.clipData = ClipData.newRawUri("", uri)
        return try {
            context.startActivity(view)
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }

    /** The pages of the PDF at [path], -1 where the library is missing or it is no PDF. */
    fun pdfPages(path: String): Int = if (PdfNative.loaded) PdfNative.pages(path) else -1

    /**
     * The page [index] of the PDF at [path] as a map of its `text` and
     * `scan`, or null. A scan comes as a photo does, upright and at most
     * 2048 px ([Photos.normalize]), as the OCR expects; one that does not
     * decode as it lies.
     */
    fun pdfPage(path: String, index: Int): Map<String, Any?>? {
        if (!PdfNative.loaded) return null
        val page = PdfNative.page(path, index)?.let(PdfNative::decode) ?: return null
        val scan = page["scan"] as? ByteArray ?: return page
        return page + ("scan" to (Photos.normalize { scan.inputStream() } ?: scan))
    }

    /** The longest side a page is drawn at, as the app's viewer asks (#286). */
    const val MAX_RENDER_SIDE = 4096

    /** The most pixels a page is drawn with: 32 MB as a bitmap. */
    const val MAX_RENDER_PIXELS = 8 * 1024 * 1024

    /** One page drawn at a time: each bitmap is large. */
    private val rendering = Any()

    /**
     * The size in pixels a page of [pageWidth] by [pageHeight] points is
     * drawn at when [width] pixels are asked for: that wide, at most
     * [MAX_RENDER_SIDE] a side and [MAX_RENDER_PIXELS] in all, in the
     * page's proportions; null for a page without a size.
     */
    fun renderSize(pageWidth: Int, pageHeight: Int, width: Int): Pair<Int, Int>? {
        if (pageWidth <= 0 || pageHeight <= 0) return null
        val wide = width.coerceIn(1, MAX_RENDER_SIDE).toDouble()
        val high = wide * pageHeight / pageWidth
        val shrink = minOf(1.0, MAX_RENDER_SIDE / max(wide, high), sqrt(MAX_RENDER_PIXELS / (wide * high)))
        return max(1, (wide * shrink).toInt()) to max(1, (high * shrink).roundToInt())
    }

    /**
     * The size of each page of the PDF at [path] in points, width then
     * height, as Android's own PdfRenderer reads it for the app's viewer
     * (#286); null where it cannot open it, as one with a password.
     */
    fun pdfLayout(path: String): List<Int>? = runCatching {
        synchronized(rendering) {
            ParcelFileDescriptor.open(File(path), ParcelFileDescriptor.MODE_READ_ONLY).use { fd ->
                PdfRenderer(fd).use { pdf ->
                    (0 until pdf.pageCount).flatMap { index ->
                        pdf.openPage(index).use { listOf(it.width, it.height) }
                    }
                }
            }
        }
    }.getOrNull()

    /**
     * The page [index] of the PDF at [path] drawn on white [width] pixels
     * wide ([renderSize]) as a PNG, or null.
     */
    fun pdfRender(path: String, index: Int, width: Int): ByteArray? = runCatching {
        synchronized(rendering) {
            ParcelFileDescriptor.open(File(path), ParcelFileDescriptor.MODE_READ_ONLY).use { fd ->
                PdfRenderer(fd).use { pdf ->
                    if (index !in 0 until pdf.pageCount) return@runCatching null
                    pdf.openPage(index).use { page ->
                        val (wide, high) = renderSize(page.width, page.height, width)
                            ?: return@runCatching null
                        val bitmap = Bitmap.createBitmap(wide, high, Bitmap.Config.ARGB_8888)
                        try {
                            bitmap.eraseColor(Color.WHITE)
                            page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                            ByteArrayOutputStream().also {
                                bitmap.compress(Bitmap.CompressFormat.PNG, 100, it)
                            }.toByteArray()
                        } finally {
                            bitmap.recycle()
                        }
                    }
                }
            }
        }
    }.getOrNull()
}
