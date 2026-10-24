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

    /** Pills at 08:00 on the 24th, Re-nagging every 10 min until 14:00. */
    private fun nagging(occurrence: String = "2026-10-24T08:00") =
        Alarm(
            "occurrence:routine:r@$occurrence", "routine:r", occurrence,
            listOf(2026, 10, 24, 8, 0), null, "pills", "08:00",
            renagMs = 10 * 60_000L, end = listOf(2026, 10, 24, 14, 0),
        )

    @Test
    fun anIgnoredAlarmRenagsUntilItsEnd() {
        val alarm = nagging()
        // 08:00 in Berlin is 06:00 UTC.
        assertEquals(utc(2026, 10, 24, 6), AlarmPlan.firesAfter(alarm, berlin, utc(2026, 10, 24, 5)))
        assertEquals(utc(2026, 10, 24, 6, 10), AlarmPlan.firesAfter(alarm, berlin, utc(2026, 10, 24, 6)))
        assertEquals(utc(2026, 10, 24, 11, 50), AlarmPlan.firesAfter(alarm, berlin, utc(2026, 10, 24, 11, 45)))
        assertNull(AlarmPlan.firesAfter(alarm, berlin, utc(2026, 10, 24, 11, 50)))
        assertEquals(
            utc(2026, 10, 24, 6, 10),
            AlarmPlan.next(listOf(alarm), berlin, utc(2026, 10, 24, 6), utc(2026, 10, 24, 6, 1)),
        )
        assertNull(AlarmPlan.next(listOf(alarm), berlin, 0, utc(2026, 10, 24, 12)))
    }

    @Test
    fun anAlarmWithoutRenagRingsOnce() {
        val alarm = occurrence("2026-10-24T08:00", listOf(2026, 10, 24, 8, 0))
        assertNull(AlarmPlan.firesAfter(alarm, berlin, utc(2026, 10, 24, 6)))
    }

    @Test
    fun aLateFireShowsEachOccurrenceOnceHoweverManyRenagsCameDue() {
        val armed = utc(2026, 10, 24, 6, 10)
        val due = AlarmPlan.due(listOf(nagging()), berlin, armed, utc(2026, 10, 24, 6), utc(2026, 10, 24, 6, 45))
        assertEquals(listOf("occurrence:routine:r@2026-10-24T08:00"), due.map { it.id })
        assertEquals(
            emptyList<Alarm>(),
            AlarmPlan.due(listOf(nagging()), berlin, utc(2026, 10, 24, 12), 0, utc(2026, 10, 24, 12, 30)),
        )
    }

    @Test
    fun aSnoozeTakesTheOccurrencesPlaceAndRenagsFromThere() {
        val until = utc(2026, 10, 24, 6, 30)
        val tapped = Action(1, "snooze", "routine:r", "2026-10-24T08:00", 0, until, "pills", "08:00")
        val effective = AlarmPlan.effective(listOf(nagging()), listOf(tapped)).single()
        assertEquals("snooze:routine:r@2026-10-24T08:00", effective.id)
        assertEquals(utc(2026, 10, 24, 6, 40), AlarmPlan.firesAfter(effective, berlin, until))
        // Nothing rings before the snooze.
        assertEquals(until, AlarmPlan.next(listOf(effective), berlin, utc(2026, 10, 24, 6, 5), utc(2026, 10, 24, 6, 5)))
    }

    @Test
    fun aSnoozePastTheEndNeverRings() {
        val tapped = Action(1, "snooze", "routine:r", "2026-10-24T08:00", 0, utc(2026, 10, 24, 12, 30), "pills", "08:00")
        val effective = AlarmPlan.effective(listOf(nagging()), listOf(tapped))
        assertNull(AlarmPlan.next(effective, berlin, 0, utc(2026, 10, 24, 11, 59)))
    }

    @Test
    fun afterDowntimeNothingRingsLateAndWhatEndedIsCaughtUp() {
        val plan = listOf(
            nagging(),
            // Still open at boot: it goes on Re-nagging, not counted.
            Alarm(
                "occurrence:routine:s@2026-10-24T13:00", "routine:s", "2026-10-24T13:00",
                listOf(2026, 10, 24, 13, 0), null, "plants", "13:00",
                renagMs = 10 * 60_000L, end = listOf(2026, 10, 25, 0, 0),
            ),
        )
        val off = utc(2026, 10, 24, 5)
        val back = utc(2026, 10, 24, 13, 3)
        assertEquals(1, AlarmPlan.catchUp(plan, berlin, off, off, back))
        // No burst: the next ring is a Re-nag ahead of now.
        assertEquals(utc(2026, 10, 24, 13, 10), AlarmPlan.next(plan, berlin, off, back))
    }

    @Test
    fun noCatchUpWithoutDowntimeOrAfterTheClockWasSetBack() {
        val plan = listOf(nagging())
        val now = utc(2026, 10, 24, 13)
        assertEquals(0, AlarmPlan.catchUp(plan, berlin, 0, 0, now))
        assertEquals(0, AlarmPlan.catchUp(plan, berlin, utc(2026, 10, 24, 14), utc(2026, 10, 24, 14), now))
        assertEquals(0, AlarmPlan.catchUp(plan, berlin, utc(2026, 10, 24, 12), utc(2026, 10, 24, 12), now))
        assertEquals(0, AlarmPlan.catchUp(plan, berlin, utc(2026, 10, 24, 5), utc(2026, 10, 24, 5), utc(2026, 10, 24, 11)))
    }

    @Test
    fun aDoneTappedButNotAppliedIsNotCaughtUp() {
        val done = Action(1, "done", "routine:r", "2026-10-24T08:00", 0, 0, "pills", "08:00")
        val effective = AlarmPlan.effective(listOf(nagging()), listOf(done))
        assertEquals(0, AlarmPlan.catchUp(effective, berlin, utc(2026, 10, 24, 5), 0, utc(2026, 10, 24, 13)))
    }

    @Test
    fun whatWasMissedWhileThePhoneWasOnIsNotCaughtUp() {
        // The last Re-nag rang at 13:50 Berlin; it became Missed at 14:00
        // with the phone on, and nothing rang after.
        val lastTick = utc(2026, 10, 24, 11, 50)
        assertEquals(0, AlarmPlan.catchUp(listOf(nagging()), berlin, lastTick, lastTick, utc(2026, 10, 24, 15)))
        // A Reminder ignored until midnight, then a reboot in the morning.
        val reminder = Alarm(
            "occurrence:routine:m@2026-10-24T20:00", "routine:m", "2026-10-24T20:00",
            listOf(2026, 10, 24, 20, 0), null, "call mum", "20:00",
            renagMs = 10 * 60_000L, end = listOf(2026, 10, 25, 0, 0),
        )
        val lastNag = utc(2026, 10, 24, 21, 50)
        assertEquals(0, AlarmPlan.catchUp(listOf(reminder), berlin, lastNag, lastNag, utc(2026, 10, 25, 6)))
    }
}
