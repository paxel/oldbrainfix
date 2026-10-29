package io.github.paxel.oldbrainfix

import android.content.ContentUris
import android.content.Context
import android.provider.CalendarContract

/**
 * The appointments of the calendars the user picked, read for the Agenda
 * and never copied: every visible calendar to pick from, and the
 * instances of the picked ones in a time range. Read with the calendar
 * permission of the Capture Context; no Google library, slow, so only
 * ever called off the main thread.
 */
object Appointments {
    /** One occurrence of an event as the provider lists it. */
    data class Instance(
        /** The instance's own row id. */
        val id: Long,
        val calendarId: Long,
        val title: String?,
        /** Epoch milliseconds; midnight UTC for an all-day event. */
        val begin: Long,
        val end: Long,
        val allDay: Boolean,
        val location: String?,
        val description: String?,
        val declined: Boolean,
        /** The event's custom app columns, where the mirror marks its own. */
        val appPackage: String?,
        val appUri: String?,
    )

    /** Whether [instance] is an event the calendar mirror wrote for [packageName]. */
    fun mirrors(instance: Instance, packageName: String): Boolean =
        instance.appPackage == packageName && MirrorMarker.uid(instance.appUri) != null

    /**
     * The [instances] the Agenda shows: those in [calendarIds] the user
     * did not decline, without the mirror's own events, which the Agenda
     * shows as Occurrences already.
     */
    fun shown(instances: List<Instance>, calendarIds: Set<Long>, packageName: String): List<Instance> =
        instances.filter { it.calendarId in calendarIds && !it.declined && !mirrors(it, packageName) }

    /** [instance] as the app's channel hands it over. */
    fun row(instance: Instance): Map<String, Any> = mapOf(
        "id" to instance.id.toString(),
        "calendar" to instance.calendarId.toString(),
        "title" to instance.title.orEmpty(),
        "begin" to instance.begin,
        "end" to instance.end,
        "allDay" to instance.allDay,
        "location" to instance.location.orEmpty(),
        "description" to instance.description.orEmpty(),
    )

    /** Every calendar the calendar app shows, with its id, name, account and colour; none without the permission. */
    fun calendars(context: Context): List<Map<String, Any>> {
        if (!Calendar.granted(context)) return emptyList()
        val projection = arrayOf(
            CalendarContract.Calendars._ID,
            CalendarContract.Calendars.CALENDAR_DISPLAY_NAME,
            CalendarContract.Calendars.ACCOUNT_NAME,
            CalendarContract.Calendars.CALENDAR_COLOR,
        )
        return context.contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            projection,
            "${CalendarContract.Calendars.VISIBLE} = 1",
            null,
            null,
        )?.use { cursor ->
            buildList {
                while (cursor.moveToNext()) {
                    add(
                        mapOf(
                            "id" to cursor.getLong(0).toString(),
                            "name" to cursor.getString(1).orEmpty(),
                            "account" to cursor.getString(2).orEmpty(),
                            "colour" to cursor.getInt(3),
                        ),
                    )
                }
            }
        } ?: emptyList()
    }

    /**
     * The appointments of [calendarIds] overlapping [from] until [until],
     * epoch milliseconds, as the app's channel hands them over; null
     * without the permission.
     */
    fun between(context: Context, from: Long, until: Long, calendarIds: List<String>): List<Map<String, Any>>? {
        if (!Calendar.granted(context)) return null
        val ids = calendarIds.mapNotNull { it.toLongOrNull() }.toSet()
        if (ids.isEmpty()) return emptyList()
        val uri = CalendarContract.Instances.CONTENT_URI.buildUpon().also {
            ContentUris.appendId(it, from)
            ContentUris.appendId(it, until)
        }.build()
        val projection = arrayOf(
            CalendarContract.Instances._ID,
            CalendarContract.Instances.CALENDAR_ID,
            CalendarContract.Instances.TITLE,
            CalendarContract.Instances.BEGIN,
            CalendarContract.Instances.END,
            CalendarContract.Instances.ALL_DAY,
            CalendarContract.Instances.EVENT_LOCATION,
            CalendarContract.Instances.DESCRIPTION,
            CalendarContract.Instances.SELF_ATTENDEE_STATUS,
            CalendarContract.Instances.CUSTOM_APP_PACKAGE,
            CalendarContract.Instances.CUSTOM_APP_URI,
        )
        val selection = "${CalendarContract.Instances.CALENDAR_ID} IN (${ids.joinToString(",")})"
        val instances = context.contentResolver.query(uri, projection, selection, null, null)?.use { cursor ->
            buildList<Instance> {
                while (cursor.moveToNext()) {
                    add(
                        Instance(
                            id = cursor.getLong(0),
                            calendarId = cursor.getLong(1),
                            title = cursor.getString(2),
                            begin = cursor.getLong(3),
                            end = cursor.getLong(4),
                            allDay = cursor.getInt(5) != 0,
                            location = cursor.getString(6),
                            description = cursor.getString(7),
                            declined = cursor.getInt(8) == CalendarContract.Attendees.ATTENDEE_STATUS_DECLINED,
                            appPackage = cursor.getString(9),
                            appUri = cursor.getString(10),
                        ),
                    )
                }
            }
        } ?: return null
        return shown(instances, ids, context.packageName).map(::row)
    }
}
