package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import java.util.Calendar
import java.util.TimeZone

class AlarmPlanTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")
    private val tokyo = TimeZone.getTimeZone("Asia/Tokyo")

    private fun utc(year: Int, month: Int, day: Int, hour: Int, minute: Int = 0): Long =
        Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(year, month - 1, day, hour, minute, 0)
        }.timeInMillis

    private fun occurrence(id: String, local: List<Int>) =
        Alarm(id, "routine:r", id, local, null, "pills", "08:00")

    private fun snooze(at: Long) =
        Alarm("snooze:routine:r@2026-10-24T08:00", "routine:r", "2026-10-24T08:00", null, at, "pills", "08:00")

    @Test
    fun anOrdinaryTimeIsTheInstantTheClockShowsIt() {
        assertEquals(utc(2026, 10, 24, 6), AlarmPlan.instantOf(listOf(2026, 10, 24, 8, 0), berlin))
        assertEquals(utc(2026, 10, 26, 7), AlarmPlan.instantOf(listOf(2026, 10, 26, 8, 0), berlin))
    }

    @Test
    fun aSkippedTimeFiresWhenTheClockJumpsPastIt() {
        // On 29 March 2026 Berlin's clock jumps from 02:00 to 03:00.
        assertEquals(utc(2026, 3, 29, 1), AlarmPlan.instantOf(listOf(2026, 3, 29, 2, 30), berlin))
        assertEquals(utc(2026, 3, 29, 0, 59), AlarmPlan.instantOf(listOf(2026, 3, 29, 1, 59), berlin))
    }

    @Test
    fun aDoubledTimeFiresOnceAtTheFirst() {
        // On 25 October 2026 Berlin's clock goes from 03:00 back to 02:00.
        assertEquals(utc(2026, 10, 25, 0, 30), AlarmPlan.instantOf(listOf(2026, 10, 25, 2, 30), berlin))
    }

    @Test
    fun aNewTimeZoneKeepsTheWallClockTime() {
        val alarm = occurrence("2026-10-24T08:00", listOf(2026, 10, 24, 8, 0))
        assertEquals(utc(2026, 10, 24, 6), AlarmPlan.instantOf(alarm, berlin))
        assertEquals(utc(2026, 10, 23, 23), AlarmPlan.instantOf(alarm, tokyo))
    }

    @Test
    fun aSnoozeWaitsAFixedTimeInAnyZone() {
        val at = utc(2026, 10, 24, 7, 30)
        assertEquals(at, AlarmPlan.instantOf(snooze(at), berlin))
        assertEquals(at, AlarmPlan.instantOf(snooze(at), tokyo))
    }

    @Test
    fun theNextAlarmIsTheSoonestAhead() {
        val plan = listOf(
            occurrence("2026-10-24T20:00", listOf(2026, 10, 24, 20, 0)),
            occurrence("2026-10-24T08:00", listOf(2026, 10, 24, 8, 0)),
            occurrence("2026-10-24T12:00", listOf(2026, 10, 24, 12, 0)),
        )
        val now = utc(2026, 10, 24, 7)
        assertEquals(utc(2026, 10, 24, 10), AlarmPlan.next(plan, berlin, 0, now))
        assertNull(AlarmPlan.next(plan, berlin, 0, utc(2026, 10, 25, 0)))
    }

    @Test
    fun anAlarmJustMissedStillFiresButNothingOlder() {
        val plan = listOf(occurrence("2026-10-24T08:00", listOf(2026, 10, 24, 8, 0)))
        val due = utc(2026, 10, 24, 6)
        assertEquals(due, AlarmPlan.next(plan, berlin, 0, due + 30_000))
        assertNull(AlarmPlan.next(plan, berlin, 0, due + 5 * 60_000))
        assertNull(AlarmPlan.next(plan, berlin, due, due + 30_000))
    }

    @Test
    fun aClockSetBackStillRingsWhatIsAheadNow() {
        val plan = listOf(occurrence("2026-10-24T08:30", listOf(2026, 10, 24, 8, 30)))
        val firedAt = utc(2026, 10, 24, 8)
        val now = utc(2026, 10, 24, 6)
        assertEquals(utc(2026, 10, 24, 6, 30), AlarmPlan.next(plan, berlin, firedAt, now))
        val armed = utc(2026, 10, 24, 6, 30)
        assertEquals(
            listOf("2026-10-24T08:30"),
            AlarmPlan.due(plan, berlin, armed, firedAt, armed).map { it.id },
        )
    }

    @Test
    fun aLateFireShowsEverythingItHeldUpOnce() {
        val plan = listOf(
            occurrence("2026-10-24T08:00", listOf(2026, 10, 24, 8, 0)),
            occurrence("2026-10-24T08:10", listOf(2026, 10, 24, 8, 10)),
            occurrence("2026-10-24T09:00", listOf(2026, 10, 24, 9, 0)),
        )
        val armed = utc(2026, 10, 24, 6)
        val now = utc(2026, 10, 24, 6, 20)
        assertEquals(
            listOf("2026-10-24T08:00", "2026-10-24T08:10"),
            AlarmPlan.due(plan, berlin, armed, 0, now).map { it.id },
        )
        assertEquals(
            listOf("2026-10-24T08:10"),
            AlarmPlan.due(plan, berlin, armed, armed, now).map { it.id },
        )
    }

    @Test
    fun aDoneTappedButNotAppliedNoLongerRings() {
        val plan = listOf(
            occurrence("2026-10-24T08:00", listOf(2026, 10, 24, 8, 0)),
            occurrence("2026-10-25T08:00", listOf(2026, 10, 25, 8, 0)),
        )
        val done = Action(1, "done", "routine:r", "2026-10-24T08:00", 0, 0, "pills", "08:00")
        assertEquals(listOf("2026-10-25T08:00"), AlarmPlan.effective(plan, listOf(done)).map { it.id })
    }

    @Test
    fun aSnoozeTappedButNotAppliedRingsAgainInPlaceOfTheOld() {
        val old = snooze(utc(2026, 10, 24, 7))
        val plan = listOf(old, occurrence("2026-10-25T08:00", listOf(2026, 10, 25, 8, 0)))
        val until = utc(2026, 10, 24, 7, 30)
        val tapped = Action(1, "snooze", "routine:r", "2026-10-24T08:00", 0, until, "pills", "08:00")
        val effective = AlarmPlan.effective(plan, listOf(tapped))
        assertEquals(2, effective.size)
        assertEquals(until, effective.single { it.id == old.id }.at)
    }

    @Test
    fun doneAfterASnoozeSilencesIt() {
        val until = utc(2026, 10, 24, 7, 30)
        val queued = listOf(
            Action(1, "snooze", "routine:r", "2026-10-24T08:00", 0, until, "pills", "08:00"),
            Action(2, "done", "routine:r", "2026-10-24T08:00", 0, 0, "pills", "08:00"),
        )
        assertEquals(emptyList<Alarm>(), AlarmPlan.effective(listOf(snooze(until)), queued))
    }

    @Test
    fun anOccurrenceAndItsSnoozeShareOneNotification() {
        assertEquals(
            AlarmPlan.notificationId("routine:r", "2026-10-24T08:00"),
            AlarmPlan.notificationId("routine:r", "2026-10-24T08:00"),
        )
        assertEquals("snooze:routine:r@2026-10-24T08:00", AlarmPlan.snoozeId("routine:r", "2026-10-24T08:00"))
    }
}
