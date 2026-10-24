package io.github.paxel.oldbrainfix

import android.Manifest
import android.content.ContentValues
import android.content.Context
import android.content.pm.PackageManager
import android.provider.CalendarContract
import java.util.GregorianCalendar
import java.util.TimeZone

/**
 * One event of the calendar mirror as the calendar provider keeps it.
 * The app hands over the wall-clock start as the core names an
 * Occurrence; the provider wants an instant and the zone the repeat rule
 * runs in.
 */
data class MirrorRow(
    /** The event's UID from the core, kept in the event as its marker. */
    val uid: String,
    val title: String,
    val description: String,
    /** Epoch milliseconds. */
    val start: Long,
    /** Epoch milliseconds; only for an event that happens once. */
    val end: Long?,
    /** RFC 5545 duration; only for a repeating event, as the provider requires. */
    val duration: String?,
    val allDay: Boolean,
    /** The zone [rule] repeats in: the Device's, or UTC all day. */
    val zone: String,
    val rule: String?,
) {
    /** The row's columns for the provider, marked as the mirror's, in [calendarId]. */
    fun values(calendarId: Long, packageName: String): ContentValues = ContentValues().apply {
        put(CalendarContract.Events.CALENDAR_ID, calendarId)
        put(CalendarContract.Events.TITLE, title)
        put(CalendarContract.Events.DESCRIPTION, description)
        put(CalendarContract.Events.DTSTART, start)
        put(CalendarContract.Events.DTEND, end)
        put(CalendarContract.Events.DURATION, duration)
        put(CalendarContract.Events.ALL_DAY, if (allDay) 1 else 0)
        put(CalendarContract.Events.EVENT_TIMEZONE, zone)
        put(CalendarContract.Events.RRULE, rule)
        // The app rings its own alarms: the event never blocks time.
        put(CalendarContract.Events.AVAILABILITY, CalendarContract.Events.AVAILABILITY_FREE)
        put(CalendarContract.Events.CUSTOM_APP_PACKAGE, packageName)
        put(CalendarContract.Events.CUSTOM_APP_URI, MirrorMarker.uri(uid))
    }

    companion object {
        private val key = Regex("""^(\d{4})-(\d{2})-(\d{2})(?:T(\d{2}):(\d{2}))?$""")
        private val until = Regex("""UNTIL=(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})(?=;|$)""")

        /**
         * The row of the event [uid] starting at [start], an Occurrence key
         * (`2026-10-25` all day, `2026-10-25T08:00` at a time), lasting
         * [minutes] when timed and repeating by [rule]; null for a start
         * that is none. A timed start is the wall clock in [zone], an hour
         * skipped by daylight saving moved on past it, and repeats there,
         * its UNTIL turned into UTC as RFC 5545 wants beside a zone. An
         * all-day one is midnight UTC, as the provider keeps all-day
         * events.
         */
        fun of(
            uid: String,
            title: String,
            description: String,
            start: String,
            rule: String?,
            minutes: Int,
            zone: TimeZone,
        ): MirrorRow? {
            val match = key.matchEntire(start) ?: return null
            val (y, m, d, h, min) = match.destructured
            val allDay = h.isEmpty()
            val at = if (allDay) {
                wallClock(TimeZone.getTimeZone("UTC"), y.toInt(), m.toInt(), d.toInt(), 0, 0, 0)
            } else {
                wallClock(zone, y.toInt(), m.toInt(), d.toInt(), h.toInt(), min.toInt(), 0)
            }
            val length = if (allDay) DAY else minutes * 60_000L
            return MirrorRow(
                uid = uid,
                title = title,
                description = description,
                start = at,
                end = if (rule == null) at + length else null,
                duration = when {
                    rule == null -> null
                    allDay -> "P1D"
                    else -> "P${minutes * 60}S"
                },
                allDay = allDay,
                zone = if (allDay) "UTC" else zone.id,
                rule = if (rule == null || allDay) rule else utcUntil(rule, zone),
            )
        }

        /** [rule] with a wall-clock UNTIL in [zone] as UTC; a date or UTC one stays. */
        fun utcUntil(rule: String, zone: TimeZone): String = until.replace(rule) {
            val (y, m, d, h, min, s) = it.destructured
            val at = wallClock(zone, y.toInt(), m.toInt(), d.toInt(), h.toInt(), min.toInt(), s.toInt())
            val utc = GregorianCalendar(TimeZone.getTimeZone("UTC")).apply { timeInMillis = at }
            "UNTIL=%04d%02d%02dT%02d%02d%02dZ".format(
                utc.get(GregorianCalendar.YEAR),
                utc.get(GregorianCalendar.MONTH) + 1,
                utc.get(GregorianCalendar.DAY_OF_MONTH),
                utc.get(GregorianCalendar.HOUR_OF_DAY),
                utc.get(GregorianCalendar.MINUTE),
                utc.get(GregorianCalendar.SECOND),
            )
        }

        private const val DAY = 24L * 60 * 60 * 1000

        private fun wallClock(zone: TimeZone, y: Int, m: Int, d: Int, h: Int, min: Int, s: Int): Long =
            GregorianCalendar(zone).apply {
                clear()
                set(y, m - 1, d, h, min, s)
            }.timeInMillis
    }
}

/**
 * The mark of an event the mirror wrote: the app's package and its UID in
 * the event's custom app columns, which an app may write without being a
 * sync adapter. Only marked events are ever changed or removed.
 */
object MirrorMarker {
    private const val PREFIX = "oldbrainfix:mirror/"

    fun uri(uid: String): String = PREFIX + uid

    /** The UID in [uri]; null for anything the mirror did not write. */
    fun uid(uri: String?): String? =
        uri?.takeIf { it.startsWith(PREFIX) }?.removePrefix(PREFIX)?.takeIf { it.isNotEmpty() }
}

/**
 * The calendar mirror on the calendar provider: the calendars it may
 * write into, the events it wrote, and writing them. No Google library;
 * slow, so only ever called off the main thread.
 */
object CalendarMirror {
    val permissions = arrayOf(Manifest.permission.READ_CALENDAR, Manifest.permission.WRITE_CALENDAR)

    fun granted(context: Context): Boolean = permissions.all {
        context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED
    }

    private const val WRITABLE =
        "${CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL} >= ${CalendarContract.Calendars.CAL_ACCESS_CONTRIBUTOR}"

    /** Only calendars the calendar app shows are offered; a hidden one stays the mirror's. */
    private const val OFFERED = "$WRITABLE AND ${CalendarContract.Calendars.VISIBLE} = 1"

    private const val MARKED =
        "${CalendarContract.Events.CALENDAR_ID} = ? AND ${CalendarContract.Events.CUSTOM_APP_PACKAGE} = ?" +
            " AND ${CalendarContract.Events.DELETED} = 0"

    private const val ONE_MARKED =
        "${CalendarContract.Events._ID} = ? AND ${CalendarContract.Events.CALENDAR_ID} = ?" +
            " AND ${CalendarContract.Events.CUSTOM_APP_PACKAGE} = ?" +
            " AND ${CalendarContract.Events.CUSTOM_APP_URI} LIKE 'oldbrainfix:mirror/%'"

    /** The visible calendars events can be written into, as the app's channel hands them over. */
    fun calendars(context: Context): List<Map<String, String>> {
        if (!granted(context)) return emptyList()
        val projection = arrayOf(
            CalendarContract.Calendars._ID,
            CalendarContract.Calendars.CALENDAR_DISPLAY_NAME,
            CalendarContract.Calendars.ACCOUNT_NAME,
        )
        return context.contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            projection,
            OFFERED,
            null,
            null,
        )?.use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    val name = cursor.getString(1).orEmpty()
                    add(
                        mapOf(
                            "id" to cursor.getLong(0).toString(),
                            "name" to name,
                            "account" to cursor.getString(2).orEmpty(),
                        ),
                    )
                }
            }
        } ?: emptyList()
    }

    /**
     * The events the mirror wrote into [calendarId] as `events`, each id
     * to its UID, or `gone` when the calendar cannot be written into any
     * more; null without the permission.
     */
    fun marked(context: Context, calendarId: Long): Map<String, Any>? {
        if (!granted(context)) return null
        val writable = context.contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            arrayOf(CalendarContract.Calendars._ID),
            "${CalendarContract.Calendars._ID} = ? AND $WRITABLE",
            arrayOf(calendarId.toString()),
            null,
        )?.use { it.count > 0 } ?: false
        if (!writable) return mapOf("gone" to true)
        val events = context.contentResolver.query(
            CalendarContract.Events.CONTENT_URI,
            arrayOf(CalendarContract.Events._ID, CalendarContract.Events.CUSTOM_APP_URI),
            MARKED,
            arrayOf(calendarId.toString(), context.packageName),
            null,
        )?.use { cursor ->
            buildMap<String, String> {
                while (cursor.moveToNext()) {
                    val uid = MirrorMarker.uid(cursor.getString(1)) ?: continue
                    put(cursor.getLong(0).toString(), uid)
                }
            }
        } ?: return null
        return mapOf("events" to events)
    }

    /** Writes [row] as a new event in [calendarId]; its id, or null when it failed. */
    fun create(context: Context, calendarId: Long, row: MirrorRow): String? = try {
        context.contentResolver.insert(
            CalendarContract.Events.CONTENT_URI,
            row.values(calendarId, context.packageName),
        )?.lastPathSegment
    } catch (_: RuntimeException) {
        null
    }

    /**
     * Writes [row] over the mirror's event [id]; its id, a new one when the
     * provider would only take it as a new event, or null when it failed
     * or the event is no longer the mirror's.
     */
    fun update(context: Context, calendarId: Long, id: String, row: MirrorRow): String? {
        val args = arrayOf(id, calendarId.toString(), context.packageName)
        return try {
            val changed = context.contentResolver.update(
                CalendarContract.Events.CONTENT_URI,
                row.values(calendarId, context.packageName),
                ONE_MARKED,
                args,
            )
            if (changed > 0) id else null
        } catch (_: RuntimeException) {
            // Such as a repeating event the provider will not turn into
            // a single one: written anew instead.
            if (delete(context, calendarId, id)) create(context, calendarId, row) else null
        }
    }

    /** Removes the mirror's event [id]; whether it is gone now. Never touches an event it did not write. */
    fun delete(context: Context, calendarId: Long, id: String): Boolean = try {
        context.contentResolver.delete(
            CalendarContract.Events.CONTENT_URI,
            ONE_MARKED,
            arrayOf(id, calendarId.toString(), context.packageName),
        )
        true
    } catch (_: RuntimeException) {
        false
    }

    /**
     * Carries out a plan from the app: [deletes] first, then [updates] and
     * [creates], each event a map of `uid`, `title`, `description`,
     * `start` and `rule`, an update's with its `id` too. Answers the ids
     * the updated and created events have now, in their order, null for
     * one that failed, and whether every delete went through.
     */
    fun apply(
        context: Context,
        calendarId: Long,
        creates: List<Map<*, *>>,
        updates: List<Map<*, *>>,
        deletes: List<String>,
        minutes: Int,
    ): Map<String, Any?>? {
        if (!granted(context)) return null
        val zone = TimeZone.getDefault()
        fun row(event: Map<*, *>) = MirrorRow.of(
            uid = event["uid"] as String,
            title = event["title"] as String,
            description = event["description"] as String? ?: "",
            start = event["start"] as String,
            rule = event["rule"] as String?,
            minutes = minutes,
            zone = zone,
        )
        val deleted = deletes.map { delete(context, calendarId, it) }.all { it }
        val updated = updates.map { event ->
            row(event)?.let { update(context, calendarId, event["id"] as String, it) }
        }
        val created = creates.map { event -> row(event)?.let { create(context, calendarId, it) } }
        return mapOf("updated" to updated, "created" to created, "deleted" to deleted)
    }
}
