package io.github.paxel.oldbrainfix

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.os.ParcelFileDescriptor
import android.print.PrintAttributes
import android.print.PrinterCapabilitiesInfo
import android.print.PrinterId
import android.print.PrinterInfo
import android.printservice.PrintJob
import android.printservice.PrintService
import android.printservice.PrinterDiscoverySession
import android.provider.Settings
import java.io.File
import java.io.OutputStream
import java.util.UUID

/**
 * Print to Old Brain Fix (#293): the app as Android's print service, with
 * one printer, "Old Brain Fix". A print job sent to it becomes a share of
 * its PDF (#267): the PDF is kept in the app's files, named after the
 * job's document, and Capture opens with it attached as for a PDF shared
 * in. Android tells a print service nothing about the app that printed,
 * so a print has no source app.
 */
object PrintIn {
    /** The one printer's id within the service. */
    const val PRINTER = "old-brain-fix"

    /** What a print job's document is. */
    const val TYPE = "application/pdf"

    /** The resolution the printer offers, in dots per inch. */
    const val DPI = 300

    /** Media sizes by their id in [PrintAttributes.MediaSize]. */
    const val A4 = "ISO_A4"
    const val LETTER = "NA_LETTER"

    /**
     * Where Android keeps the print services the user turned off, from
     * Android 7 on: their names, separated by ":". Every installed one
     * not named there is on. It is a hidden setting that apps may still
     * read; no permission and no system API is needed, unlike
     * PrintManager's list of services.
     */
    const val DISABLED_SETTING = "disabled_print_services"

    /** The regions that print on Letter rather than A4. */
    private val letterRegions = setOf(
        "US", "CA", "MX", "PR", "PH", "CL", "CO", "VE", "CR", "GT", "PA", "DO", "SV", "NI", "BZ",
    )

    /** Whether the service is on, as far as Android tells. */
    enum class State { ON, OFF, UNKNOWN }

    /**
     * What the printer offers: [media] by id, [defaultMedia] picked first,
     * at [DPI], in colour and black and white, colour first.
     */
    class Capabilities(val media: List<String>, val defaultMedia: String)

    /**
     * The name a print goes by: the job's document name, else the job's
     * label, with ".pdf" added where it does not end so; "print.pdf"
     * without either.
     */
    fun nameOf(document: String?, label: String?): String {
        val name = document?.trim()?.takeIf(String::isNotEmpty)
            ?: label?.trim()?.takeIf(String::isNotEmpty)
            ?: return "print.pdf"
        return if (name.endsWith(".pdf", ignoreCase = true)) name else "$name.pdf"
    }

    /** A4 and Letter, the one of the phone's [region] first. */
    fun capabilities(region: String?): Capabilities =
        Capabilities(listOf(A4, LETTER), if (region?.uppercase() in letterRegions) LETTER else A4)

    /**
     * Whether the service [own], its class name in full, is on: off where
     * the [disabled] setting names it, in full or short form, on where it
     * does not; [UNKNOWN] where the setting could not be read.
     */
    fun state(disabled: String?, readable: Boolean, own: ComponentNames): State {
        if (!readable) return State.UNKNOWN
        val off = disabled.orEmpty().split(':').any { own.matches(it.trim()) }
        return if (off) State.OFF else State.ON
    }

    /** A component's package and class, matched against a setting's entry. */
    class ComponentNames(private val pkg: String, private val cls: String) {
        /** Whether [entry], "package/class" or "package/.Class", names it. */
        fun matches(entry: String): Boolean {
            val slash = entry.indexOf('/')
            if (slash <= 0) return false
            val entryPkg = entry.substring(0, slash)
            val entryCls = entry.substring(slash + 1).let { if (it.startsWith('.')) entryPkg + it else it }
            return entryPkg == pkg && entryCls == cls
        }
    }

    /** Where printed PDFs wait until the app takes them: kept through a restart, unlike a share's copy. */
    fun dir(context: Context): File = File(context.filesDir, "printed").apply { mkdirs() }

    /**
     * Keeps the PDF [write] writes in [dir] as [name]: the file first,
     * then its name beside it, so a print half written never comes as
     * one. The PDF, or null where writing failed.
     */
    fun keep(dir: File, name: String, write: (OutputStream) -> Unit): File? {
        dir.mkdirs()
        val id = UUID.randomUUID().toString()
        val pdf = File(dir, "$id.pdf")
        val part = File(dir, "$id.part")
        return try {
            part.outputStream().use(write)
            if (!part.renameTo(pdf)) throw java.io.IOException("not kept")
            val label = File(dir, "$id.name.part")
            label.writeText(name, Charsets.UTF_8)
            if (!label.renameTo(File(dir, "$id.name"))) throw java.io.IOException("not named")
            pdf
        } catch (_: Exception) {
            drop(pdf)
            part.delete()
            File(dir, "$id.name.part").delete()
            null
        }
    }

    /** Lets go of a print kept by [keep]: its PDF and its name. */
    fun drop(pdf: File) {
        pdf.delete()
        File(pdf.parentFile, pdf.nameWithoutExtension + ".name").delete()
    }

    /**
     * The prints waiting in [dir], oldest first, as the copies a share
     * hands over, each PDF moved [into] where a share's copies wait
     * ([Files.dir]); each one's name goes as it is taken, so none comes
     * twice, and a PDF the app never attached is left in the cache,
     * which Android clears, rather than in the app's files.
     */
    fun take(dir: File, into: File): List<Files.Copied> {
        val names = dir.listFiles { file -> file.name.endsWith(".name") }.orEmpty()
        into.mkdirs()
        return names.sortedBy { it.lastModified() }.mapNotNull { label ->
            val pdf = File(dir, label.name.removeSuffix(".name") + ".pdf")
            val name = runCatching { label.readText(Charsets.UTF_8) }.getOrNull()
            label.delete()
            if (name == null || !pdf.isFile) return@mapNotNull null
            val moved = File(into, UUID.randomUUID().toString())
            Files.Copied(if (pdf.renameTo(moved)) moved else pdf, name, TYPE)
        }
    }

    /** Whether this app's print service is on ([state]). */
    fun state(context: Context): State {
        val (disabled, readable) = try {
            Settings.Secure.getString(context.contentResolver, DISABLED_SETTING) to true
        } catch (_: SecurityException) {
            null to false
        }
        val own = ComponentName(context, PrintInService::class.java)
        return state(disabled, readable, ComponentNames(own.packageName, own.className))
    }

    /** Opens Android's print settings, where the service is turned on; false where there are none. */
    fun openSettings(context: Context): Boolean = try {
        context.startActivity(Intent(Settings.ACTION_PRINT_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        true
    } catch (_: android.content.ActivityNotFoundException) {
        false
    }
}

/**
 * The print service itself: one printer, "Old Brain Fix", with A4 and
 * Letter, colour and [PrintIn.DPI]. A job queued for it is started, its
 * PDF kept ([PrintIn.keep]) off the main thread, completed, and handed
 * over to the app's one [MainActivity] as a share is. Where Android does
 * not let the app come up from the background, the print waits in the
 * app's files until it opens or comes back.
 */
class PrintInService : PrintService() {
    private val main = Handler(Looper.getMainLooper())

    override fun onCreatePrinterDiscoverySession(): PrinterDiscoverySession = Session()

    override fun onRequestCancelPrintJob(printJob: PrintJob) {
        printJob.cancel()
    }

    override fun onPrintJobQueued(printJob: PrintJob) {
        if (!printJob.isQueued || !printJob.start()) return
        val document = printJob.document
        val name = PrintIn.nameOf(document.info.name, printJob.info.label)
        val data: ParcelFileDescriptor = document.data ?: return run { printJob.fail(getString(R.string.print_failed)) }
        val dir = PrintIn.dir(this)
        Thread {
            val kept = runCatching {
                ParcelFileDescriptor.AutoCloseInputStream(data).use { input ->
                    PrintIn.keep(dir, name) { input.copyTo(it) }
                }
            }.getOrNull()
            main.post { finish(printJob, kept) }
        }.start()
    }

    /** Completes [printJob] with its PDF [kept] and opens Capture; a cancelled one keeps nothing. */
    private fun finish(printJob: PrintJob, kept: File?) {
        when {
            printJob.isCancelled -> kept?.let(PrintIn::drop)
            kept == null -> printJob.fail(getString(R.string.print_failed))
            else -> {
                printJob.complete()
                runCatching {
                    startActivity(
                        Intent(this, MainActivity::class.java)
                            .putExtra(MainActivity.EXTRA_SHARED, true)
                            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
                    )
                }
            }
        }
    }

    /** Shows the one printer, its capabilities with it. */
    private inner class Session : PrinterDiscoverySession() {
        private fun printer(): PrinterInfo {
            val id: PrinterId = generatePrinterId(PrintIn.PRINTER)
            val plan = PrintIn.capabilities(resources.configuration.locales.get(0)?.country)
            val caps = PrinterCapabilitiesInfo.Builder(id).apply {
                for (media in plan.media) addMediaSize(mediaSize(media), media == plan.defaultMedia)
                addResolution(
                    PrintAttributes.Resolution("dpi${PrintIn.DPI}", "${PrintIn.DPI} dpi", PrintIn.DPI, PrintIn.DPI),
                    true,
                )
                setColorModes(
                    PrintAttributes.COLOR_MODE_COLOR or PrintAttributes.COLOR_MODE_MONOCHROME,
                    PrintAttributes.COLOR_MODE_COLOR,
                )
                setMinMargins(PrintAttributes.Margins.NO_MARGINS)
            }.build()
            return PrinterInfo.Builder(id, getString(R.string.print_printer), PrinterInfo.STATUS_IDLE)
                .setCapabilities(caps)
                .setIconResourceId(R.mipmap.ic_launcher)
                .build()
        }

        private fun mediaSize(id: String): PrintAttributes.MediaSize =
            if (id == PrintIn.LETTER) PrintAttributes.MediaSize.NA_LETTER else PrintAttributes.MediaSize.ISO_A4

        override fun onStartPrinterDiscovery(priorityList: MutableList<PrinterId>) = addPrinters(listOf(printer()))

        override fun onStopPrinterDiscovery() = Unit

        override fun onValidatePrinters(printerIds: MutableList<PrinterId>) = addPrinters(listOf(printer()))

        override fun onStartPrinterStateTracking(printerId: PrinterId) = addPrinters(listOf(printer()))

        override fun onStopPrinterStateTracking(printerId: PrinterId) = Unit

        override fun onDestroy() = Unit
    }
}
