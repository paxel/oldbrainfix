package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.TimeZone

class MirrorRowTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")

    private fun utc(text: String): Long = java.text.SimpleDateFormat("yyyy-MM-dd HH:mm").apply {
        timeZone = TimeZone.getTimeZone("UTC")
    }.parse(text)!!.time

    private fun row(start: String, rule: String? = null) =
        MirrorRow.of("r1-0800@oldbrainfix", "Pills", "", start, rule, 15, berlin)

    @Test
    fun `a repeating timed event starts on the wall clock and repeats in the Device's zone`() {
        val row = row("2026-10-24T08:00", "FREQ=DAILY")!!
        // Summer time: UTC+2.
        assertEquals(utc("2026-10-24 06:00"), row.start)
        assertNull(row.end)
        assertEquals("P900S", row.duration)
        assertEquals("Europe/Berlin", row.zone)
        assertEquals("FREQ=DAILY", row.rule)
        assertFalse(row.allDay)
        assertEquals("r1-0800@oldbrainfix", row.uid)
    }

    @Test
    fun `a timed event once has an end and no duration`() {
        val row = row("2026-10-26T08:00")!!
        // Winter time: UTC+1.
        assertEquals(utc("2026-10-26 07:00"), row.start)
        assertEquals(utc("2026-10-26 07:15"), row.end)
        assertNull(row.duration)
        assertNull(row.rule)
    }

    @Test
    fun `an hour skipped in spring moves on past the gap`() {
        assertEquals(utc("2026-03-29 01:30"), row("2026-03-29T02:30")!!.start)
    }

    @Test
    fun `an all-day event is midnight UTC for a day`() {
        val once = row("2026-10-24")!!
        assertTrue(once.allDay)
        assertEquals(utc("2026-10-24 00:00"), once.start)
        assertEquals(utc("2026-10-25 00:00"), once.end)
        assertEquals("UTC", once.zone)
        val daily = row("2026-10-24", "FREQ=DAILY;UNTIL=20261231")!!
        assertNull(daily.end)
        assertEquals("P1D", daily.duration)
        assertEquals("FREQ=DAILY;UNTIL=20261231", daily.rule)
    }

    @Test
    fun `a wall-clock UNTIL becomes UTC`() {
        val row = row("2026-10-24T08:00", "FREQ=DAILY;UNTIL=20261231T080000;BYDAY=MO")!!
        assertEquals("FREQ=DAILY;UNTIL=20261231T070000Z;BYDAY=MO", row.rule)
        assertEquals(
            "FREQ=DAILY;UNTIL=20260701T060000Z",
            MirrorRow.utcUntil("FREQ=DAILY;UNTIL=20260701T080000", berlin),
        )
    }

    @Test
    fun `a UTC UNTIL or a COUNT stays`() {
        assertEquals("FREQ=DAILY;UNTIL=20261231T070000Z", MirrorRow.utcUntil("FREQ=DAILY;UNTIL=20261231T070000Z", berlin))
        assertEquals("FREQ=DAILY;COUNT=3", MirrorRow.utcUntil("FREQ=DAILY;COUNT=3", berlin))
    }

    @Test
    fun `no start is no row`() {
        assertNull(row(""))
        assertNull(row("soon"))
    }

    @Test
    fun `the marker carries the UID and nothing else is the mirror's`() {
        assertEquals("r1-0800@oldbrainfix", MirrorMarker.uid(MirrorMarker.uri("r1-0800@oldbrainfix")))
        assertNull(MirrorMarker.uid(null))
        assertNull(MirrorMarker.uid("content://something/else"))
        assertNull(MirrorMarker.uid("oldbrainfix:mirror/"))
    }
}
