package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CaptureExtrasTest {
    private val own = "io.github.paxel.oldbrainfix"

    @Test
    fun `the referrer names the sharing app`() {
        assertEquals("com.whatsapp", SourceApps.packageOf(listOf("android-app://com.whatsapp"), own))
        assertEquals("com.whatsapp", SourceApps.packageOf(listOf("android-app://com.whatsapp/https/x"), own))
    }

    @Test
    fun `the next candidate counts when the first says nothing`() {
        assertEquals(
            "org.mozilla.firefox",
            SourceApps.packageOf(listOf(null, " ", "org.mozilla.firefox"), own),
        )
    }

    @Test
    fun `the chooser, the app itself and web referrers are no source`() {
        assertNull(SourceApps.packageOf(listOf("android-app://android"), own))
        assertNull(SourceApps.packageOf(listOf("android-app://com.android.intentresolver"), own))
        assertNull(SourceApps.packageOf(listOf("android-app://$own", own), own))
        assertNull(SourceApps.packageOf(listOf("https://example.org/page"), own))
        assertNull(SourceApps.packageOf(listOf("android-app://"), own))
        assertNull(SourceApps.packageOf(emptyList(), own))
        assertEquals(
            "com.whatsapp",
            SourceApps.packageOf(listOf("android-app://com.android.intentresolver", "com.whatsapp"), own),
        )
    }

    private fun instance(title: String?, begin: Long, end: Long, allDay: Boolean = false, declined: Boolean = false) =
        Calendar.Instance(title, allDay, begin, end, declined)

    @Test
    fun `an event runs from its begin until just before its end`() {
        val events = listOf(
            instance("Before", 0, 100),
            instance("Dentist", 100, 200),
            instance("After", 201, 300),
        )
        assertEquals(listOf("Dentist"), Calendar.running(events, 100, 0).map { it.title })
        assertEquals(listOf("Dentist"), Calendar.running(events, 199, 0).map { it.title })
        assertEquals(emptyList<String>(), Calendar.running(events, 200, 0).map { it.title })
    }

    @Test
    fun `declined and untitled events do not count`() {
        val events = listOf(
            instance("Declined", 0, 200, declined = true),
            instance(null, 0, 200),
            instance(" ", 0, 200),
            instance("Kept", 0, 200),
        )
        assertEquals(listOf("Kept"), Calendar.running(events, 100, 0).map { it.title })
    }

    @Test
    fun `an all-day event runs on its local date`() {
        val day = 24L * 60 * 60 * 1000
        val hour = 60L * 60 * 1000
        // 24 Oct 2026, midnight UTC, and the day after.
        val begin = 20_750 * day
        val holiday = instance("Holiday", begin, begin + day, allDay = true)
        // 00:30 on the 24th in UTC+2 is still the 23rd in UTC.
        val justAfterMidnight = begin - 90 * 60 * 1000
        assertEquals(listOf("Holiday"), Calendar.running(listOf(holiday), justAfterMidnight, 2 * hour).map { it.title })
        assertEquals(emptyList<String>(), Calendar.running(listOf(holiday), justAfterMidnight, 0).map { it.title })
        // 23:30 on the 24th in UTC-2 is already the 25th in UTC.
        val lateEvening = begin + day + 90 * 60 * 1000
        assertEquals(listOf("Holiday"), Calendar.running(listOf(holiday), lateEvening, -2 * hour).map { it.title })
    }

    @Test
    fun `a device on two profiles is named once`() {
        assertEquals(
            listOf("My Car", "Headphones"),
            Bluetooth.names(listOf("My Car", " My Car ", null, "", "Headphones")),
        )
    }
}
