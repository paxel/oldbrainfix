package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.Context
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import java.io.File
import java.io.FileOutputStream

/**
 * Hands a finished PDF, the Recovery Sheet, to Android's print dialog,
 * which prints it or saves it as a PDF wherever the user likes; no
 * library, the file is copied out as it is.
 */
object Printing {
    /** Opens the print dialog for the PDF at [path] as [name]; false when there is none. */
    fun print(activity: Activity, path: String, name: String): Boolean {
        if (!Bundles.inside(FileShare.dir(activity), path)) return false
        val file = File(path)
        if (!file.isFile) return false
        val manager = activity.getSystemService(Context.PRINT_SERVICE) as? PrintManager ?: return false
        manager.print(name, PdfAdapter(file, name), PrintAttributes.Builder().build())
        return true
    }

    /** Lays the file out as the one document it is and writes it whole. */
    class PdfAdapter(private val file: File, private val name: String) : PrintDocumentAdapter() {
        override fun onLayout(
            oldAttributes: PrintAttributes?,
            newAttributes: PrintAttributes,
            cancellationSignal: CancellationSignal,
            callback: LayoutResultCallback,
            extras: Bundle?,
        ) {
            if (cancellationSignal.isCanceled) return callback.onLayoutCancelled()
            val info = PrintDocumentInfo.Builder(name)
                .setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT)
                .setPageCount(1)
                .build()
            callback.onLayoutFinished(info, oldAttributes != newAttributes)
        }

        override fun onWrite(
            pages: Array<out PageRange>,
            destination: ParcelFileDescriptor,
            cancellationSignal: CancellationSignal,
            callback: WriteResultCallback,
        ) {
            try {
                file.inputStream().use { input ->
                    FileOutputStream(destination.fileDescriptor).use { input.copyTo(it) }
                }
                callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
            } catch (e: java.io.IOException) {
                callback.onWriteFailed(e.message)
            }
        }
    }
}
