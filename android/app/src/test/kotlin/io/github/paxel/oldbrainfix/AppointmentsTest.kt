package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AppointmentsTest {
    private val own = "io.github.paxel.oldbrainfix"

    private fun instance(
        id: Long,
        calendarId: Long = 1,
        title: String? = "Dentist",
        allDay: Boolean = false,
        declined: Boolean = false,
        appPackage: String? = null,
        appUri: String? = null,
    ) = Appointments.Instance(
        id = id,
        calendarId = calendarId,
        title = title,
        begin = 1_000,
        end = 2_000,
        allDay = allDay,
        location = "Main Street 5",
        description = "Bring the card",
        declined = declined,
        appPackage = appPackage,
        appUri = appUri,
    )

    @Test
    fun `only the mirror's own marked events count as the mirror's`() {
        assertTrue(Appointments.mirrors(instance(1, appPackage = own, appUri = MirrorMarker.uri("r1")), own))
        assertFalse(Appointments.mirrors(instance(2, appPackage = own, appUri = "oldbrainfix:other/r1"), own))
        assertFalse(Appointments.mirrors(instance(3, appPackage = "org.other", appUri = MirrorMarker.uri("r1")), own))
        assertFalse(Appointments.mirrors(instance(4), own))
    }

    @Test
    fun `the Agenda shows the picked calendars without declined and mirrored events`() {
        val instances = listOf(
            instance(1, calendarId = 1),
            instance(2, calendarId = 2),
            instance(3, calendarId = 1, declined = true),
            instance(4, calendarId = 1, appPackage = own, appUri = MirrorMarker.uri("r1")),
            instance(5, calendarId = 3, title = null),
        )
        assertEquals(listOf(1L, 5L), Appointments.shown(instances, setOf(1L, 3L), own).map { it.id })
    }

    @Test
    fun `a row carries every detail, an untitled one with an empty title`() {
        val row = Appointments.row(instance(7, calendarId = 3, title = null, allDay = true))
        assertEquals(
            mapOf(
                "id" to "7",
                "calendar" to "3",
                "title" to "",
                "begin" to 1_000L,
                "end" to 2_000L,
                "allDay" to true,
                "location" to "Main Street 5",
                "description" to "Bring the card",
            ),
            row,
        )
    }
}
