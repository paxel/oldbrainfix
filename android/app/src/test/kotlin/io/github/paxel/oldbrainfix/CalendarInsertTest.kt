package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.TimeZone

class CalendarInsertTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")

    private fun utc(text: String): Long = java.text.SimpleDateFormat("yyyy-MM-dd HH:mm").apply {
        timeZone = TimeZone.getTimeZone("UTC")
    }.parse(text)!!.time

    @Test
    fun `a timed start is the wall clock in the Device's zone`() {
        val insert = CalendarInsert.of("Pills", "", "2026-10-24T08:00", "FREQ=DAILY", 15, berlin)
        // Summer time: UTC+2.
        assertEquals(utc("2026-10-24 06:00"), insert.begin)
        assertEquals(utc("2026-10-24 06:15"), insert.end)
        assertFalse(insert.allDay)
        assertEquals("FREQ=DAILY", insert.rule)
    }

    @Test
    fun `the wall clock stays across a daylight-saving switch`() {
        // Winter time from the 25th on: UTC+1.
        val insert = CalendarInsert.of("Pills", "", "2026-10-26T08:00", null, 15, berlin)
        assertEquals(utc("2026-10-26 07:00"), insert.begin)
    }

    @Test
    fun `an hour skipped in spring moves on past the gap`() {
        val insert = CalendarInsert.of("Pills", "", "2026-03-29T02:30", null, 15, berlin)
        assertEquals(utc("2026-03-29 01:30"), insert.begin)
    }

    @Test
    fun `all day runs from noon UTC to the next, on its date in any zone`() {
        val insert = CalendarInsert.of("Party", "Bring cake", "2026-12-31", null, 15, berlin)
        assertTrue(insert.allDay)
        assertEquals(utc("2026-12-31 12:00"), insert.begin)
        assertEquals(utc("2027-01-01 12:00"), insert.end)
        assertEquals("Bring cake", insert.description)
    }

    @Test
    fun `no start leaves the time to the calendar app`() {
        val insert = CalendarInsert.of("Ring the plumber", "", null, null, 15, berlin)
        assertNull(insert.begin)
        assertNull(insert.end)
        assertFalse(insert.allDay)
        assertNull(insert.rule)
        assertNull(CalendarInsert.of("x", "", "tomorrow", "FREQ=DAILY", 15, berlin).begin)
    }

    @Test
    fun `a shared file never leaves its directory`() {
        assertEquals("oldbrainfix.ics", FileShare.safeName("oldbrainfix.ics"))
        assertEquals("evil.ics", FileShare.safeName("../../evil.ics"))
        assertEquals("export", FileShare.safeName(""))
    }
}
