package io.github.paxel.oldbrainfix

import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.provider.CalendarContract
import androidx.core.content.FileProvider
import java.io.File
import java.util.GregorianCalendar
import java.util.TimeZone

/**
 * One event for the calendar app's own insert screen: no calendar
 * permission, the user saves it there. The app hands over the wall-clock
 * start as the core names an Occurrence.
 */
data class CalendarInsert(
    val title: String,
    val description: String,
    /** Epoch milliseconds; null lets the calendar app propose a time. */
    val begin: Long?,
    val end: Long?,
    val allDay: Boolean,
    /** The RRULE value; null for one time only. */
    val rule: String?,
) {
    /** The insert intent; the calendar app ignores what it does not support, such as the rule. */
    fun intent(): Intent = Intent(Intent.ACTION_INSERT).setData(CalendarContract.Events.CONTENT_URI).apply {
        putExtra(CalendarContract.Events.TITLE, title)
        if (description.isNotEmpty()) putExtra(CalendarContract.Events.DESCRIPTION, description)
        begin?.let { putExtra(CalendarContract.EXTRA_EVENT_BEGIN_TIME, it) }
        end?.let { putExtra(CalendarContract.EXTRA_EVENT_END_TIME, it) }
        if (allDay) putExtra(CalendarContract.EXTRA_EVENT_ALL_DAY, true)
        rule?.let { putExtra(CalendarContract.Events.RRULE, it) }
    }

    companion object {
        private val key = Regex("""^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?$""")

        /**
         * The event starting at [start], an Occurrence key (`2026-10-25`
         * all day, `2026-10-25T08:00` at a time) or null for none, and
         * lasting [minutes] when timed. A timed start is the wall clock in
         * [zone], an hour skipped by daylight saving moved on past it. An
         * all-day one is noon UTC to noon the next day: the same date
         * whether the calendar app reads it in UTC, as Android's calendar
         * keeps all-day events, or in the Device's zone.
         */
        fun of(
            title: String,
            description: String,
            start: String?,
            rule: String?,
            minutes: Int,
            zone: TimeZone,
        ): CalendarInsert {
            val match = start?.let { key.matchEntire(it) }
                ?: return CalendarInsert(title, description, null, null, false, null)
            val (y, m, d, h, min) = match.destructured
            val allDay = h.isEmpty()
            val calendar = GregorianCalendar(if (allDay) TimeZone.getTimeZone("UTC") else zone).apply {
                clear()
                set(y.toInt(), m.toInt() - 1, d.toInt(), h.toIntOrNull() ?: 12, min.toIntOrNull() ?: 0, 0)
            }
            val begin = calendar.timeInMillis
            val end = if (allDay) {
                calendar.add(GregorianCalendar.DAY_OF_MONTH, 1)
                calendar.timeInMillis
            } else {
                begin + minutes * 60_000L
            }
            return CalendarInsert(title, description, begin, end, allDay, rule)
        }

        /** Opens the calendar app's insert screen; false when there is none. */
        fun open(context: Context, insert: CalendarInsert): Boolean = try {
            context.startActivity(insert.intent())
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }
}

/**
 * Hands a file the app made, such as a calendar export, to whatever app
 * the user picks, through the app's FileProvider: no storage permission,
 * no library.
 */
object FileShare {
    /** Where shared files wait; only the newest of each name is kept. */
    fun dir(context: Context): File = File(context.cacheDir, "exports").apply { mkdirs() }

    /** [name] without any directory part, so it stays in [dir]. */
    fun safeName(name: String): String = File(name).name.ifBlank { "export" }

    /** Writes [bytes] as [name] and opens the share chooser for it as [type]; false when nothing can take it. */
    fun share(context: Context, name: String, bytes: ByteArray, type: String): Boolean {
        val file = File(dir(context), safeName(name)).apply { writeBytes(bytes) }
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.photos", file)
        val send = Intent(Intent.ACTION_SEND)
            .setType(type)
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        send.clipData = ClipData.newRawUri("", uri)
        return try {
            context.startActivity(Intent.createChooser(send, null))
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }
}
