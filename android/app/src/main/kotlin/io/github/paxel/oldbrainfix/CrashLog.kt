package io.github.paxel.oldbrainfix

import android.app.ActivityManager
import android.app.Application
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Process
import androidx.core.content.pm.PackageInfoCompat
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlin.system.exitProcess

/**
 * Installs [CrashLog] in every process of the app before anything else
 * runs: the app, the alarm receivers, RingService and the sync work. On
 * screen a crash shows [CrashActivity] and the process ends without the
 * system's dialog.
 */
class OldBrainFixApplication : Application() {
    // Before the content providers start, so a crash in theirs is caught too.
    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)
        val crashProcess = CrashLog.processName(this).endsWith(CrashActivity.PROCESS)
        CrashLog.install(filesDir, { CrashLog.header(this) }) { report, mail ->
            // The crash screen's own process crashing shows no crash
            // screen again. Off screen (an alarm, RingService, the sync
            // work) Android starts no activity: the crash ends the process
            // as a crash, and the app offers the report at the next start.
            val state = ActivityManager.RunningAppProcessInfo().also { ActivityManager.getMyMemoryState(it) }
            if (crashProcess ||
                state.importance > ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND
            ) {
                return@install false
            }
            startActivity(
                Intent(this, CrashActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
                    .putExtra(CrashActivity.EXTRA_REPORT, report)
                    .putExtra(CrashActivity.EXTRA_MAIL, mail),
            )
            true
        }
    }
}

/**
 * A crash in the Kotlin code, written where the Dart crash guard
 * (lib/src/crash_guard.dart) reads it, in the same shape: four header
 * lines, a blank line, the error. Native crashes never reach it.
 */
object CrashLog {
    /** The files the Dart crash guard reads, in the app's files directory. */
    const val FILE = "last_crash.txt"
    const val MARKER = "running.marker"

    /** Where reports go, as on the Dart side. */
    const val MAIL = "taum@tuta.io"
    const val MAIL_SUBJECT = "Old Brain Fix crash report"

    /** How much a `mailto:` body carries, as on the Dart side. */
    const val MAIL_LIMIT = 1800

    /** How much of a report goes to the crash screen at most. */
    const val SCREEN_LIMIT = 50_000

    /**
     * Makes [dir]'s crash file the last word of every uncaught exception,
     * marks the run as died for systems without exit records, and hands
     * the report to [show]. When [show] took it, the process ends here;
     * otherwise the handler before this one ends it.
     */
    fun install(dir: File, header: () -> String, show: (report: String, mail: String) -> Boolean) {
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            var shown = false
            try {
                val head = try { header() } catch (_: Throwable) { "Old Brain Fix version unknown" }
                val mail = compact(head, error, MAIL_LIMIT)
                File(dir, FILE).writeText(mail)
                File(dir, MARKER).writeText("1")
                shown = show(full(head, error).take(SCREEN_LIMIT), mail)
            } catch (_: Throwable) {
                // Nothing more can be done for this report.
            } finally {
                if (shown) {
                    Process.killProcess(Process.myPid())
                    exitProcess(10)
                }
                previous?.uncaughtException(thread, error)
            }
        }
    }

    /** The header as the Dart side writes it: build, system, locale, time. */
    fun header(context: Context): String {
        val info = context.packageManager.getPackageInfo(context.packageName, 0)
        return header(
            "${info.versionName}+${PackageInfoCompat.getLongVersionCode(info)}",
            "android ${Build.VERSION.RELEASE} (API ${Build.VERSION.SDK_INT}, ${Build.MANUFACTURER} ${Build.MODEL})",
            Locale.getDefault().toString(),
            Date(),
        )
    }

    fun header(version: String, system: String, locale: String, at: Date): String =
        listOf(
            "Old Brain Fix $version",
            system,
            "locale $locale",
            SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS", Locale.ROOT).format(at),
        ).joinToString("\n")

    /** The whole report, as Java prints it. */
    fun full(header: String, error: Throwable): String =
        "$header\n\n${error.stackTraceToString()}"

    /**
     * The report cut to [limit] without losing the root cause, which
     * Java prints last: every throwable of the chain keeps its message
     * and as many of its first frames as fit.
     */
    fun compact(header: String, error: Throwable, limit: Int): String {
        val chain = generateSequence(error) { it.cause }.take(10).toList()
        for (frames in 8 downTo 0) {
            val text = buildString {
                append(header).append("\n\n")
                chain.forEachIndexed { i, e ->
                    if (i > 0) append("Caused by: ")
                    append(e.toString().take(300)).append('\n')
                    e.stackTrace.take(frames).forEach { append("\tat ").append(it).append('\n') }
                    val left = e.stackTrace.size - frames
                    if (left > 0) append("\t... ").append(left).append(" more\n")
                }
            }.trimEnd()
            if (text.length <= limit || frames == 0) return text.take(limit)
        }
        return ""
    }

    /** This process's name, to tell the crash screen's own process apart. */
    fun processName(context: Context): String =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            Application.getProcessName()
        } else {
            try {
                File("/proc/self/cmdline").readText().trim('\u0000', ' ', '\n')
            } catch (_: Exception) {
                context.packageName
            }
        }
}
