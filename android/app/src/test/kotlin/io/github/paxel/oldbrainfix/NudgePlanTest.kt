package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Calendar
import java.util.TimeZone

class NudgePlanTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")
    private val night = listOf(22, 0, 8, 0)
    private val minute = 60_000L
    private val hour = 60 * minute

    private fun utc(year: Int, month: Int, day: Int, hour: Int, minute: Int = 0): Long =
        Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(year, month - 1, day, hour, minute, 0)
        }.timeInMillis

    /** The wall-clock moment in Berlin on a day of October 2026. */
    private fun local(day: Int, hour: Int, minute: Int = 0): Long =
        AlarmPlan.instantOf(listOf(2026, 10, day, hour, minute), berlin)

    private fun nudge(
        vararg anchors: Pair<Long, Long>,
        repeat: Long = hour,
        quiet: List<Int>? = night,
        snoozeAt: Long? = null,
        snoozeUntil: Long? = null,
        since: Long? = null,
    ) = Nudge(anchors.toList(), snoozeAt, snoozeUntil, repeat, quiet, since, "2 Unsorted", "Tap to sort them")

    /** The next [count] firings after [after]. */
    private fun firings(nudge: Nudge, after: Long, count: Int): List<Long> {
        var at = after
        return (0 until count).map { NudgePlan.firesAfter(nudge, berlin, at).also { at = it } }
    }

    @Test
    fun quietHoursOverMidnightEndNextMorning() {
        assertEquals(local(24, 8), NudgePlan.defer(night, berlin, local(23, 22, 30)))
        assertEquals(local(24, 8), NudgePlan.defer(night, berlin, local(24, 3)))
        assertEquals(local(24, 21, 59), NudgePlan.defer(night, berlin, local(24, 21, 59)))
        assertEquals(local(24, 8), NudgePlan.defer(night, berlin, local(24, 8)))
    }

    @Test
    fun quietHoursWithinADay() {
        val noon = listOf(13, 0, 15, 0)
        assertEquals(local(24, 15), NudgePlan.defer(noon, berlin, local(24, 13, 30)))
        assertEquals(local(24, 16), NudgePlan.defer(noon, berlin, local(24, 16)))
        assertEquals(local(24, 8), NudgePlan.defer(listOf(8, 0, 8, 0), berlin, local(24, 8)))
    }

    @Test
    fun quietHoursEndAtTheSameWallTimeAcrossTheSwitches() {
        // 25 October 2026: Berlin's clock goes from 03:00 back to 02:00.
        assertEquals(utc(2026, 10, 25, 7), NudgePlan.defer(night, berlin, local(24, 23)))
        // 29 March 2026: it jumps from 02:00 to 03:00.
        val march28 = AlarmPlan.instantOf(listOf(2026, 3, 28, 23, 0), berlin)
        assertEquals(utc(2026, 3, 29, 6), NudgePlan.defer(night, berlin, march28))
        // An end the clock skips comes when it jumps past it.
        val skipped = AlarmPlan.instantOf(listOf(2026, 3, 29, 1, 30), berlin)
        assertEquals(utc(2026, 3, 29, 1), NudgePlan.defer(listOf(1, 0, 2, 30), berlin, skipped))
    }

    @Test
    fun firesOnceAtTheEarliestOwnTimeAndCoversEveryNote() {
        val n = nudge(
            local(24, 10) to local(24, 9),
            local(24, 9, 15) to local(24, 9, 5),
            local(24, 9, 36) to local(24, 9, 6),
        )
        assertEquals(
            listOf(local(24, 9, 15), local(24, 10, 15), local(24, 11, 15)),
            firings(n, local(24, 9, 6), 3),
        )
    }

    @Test
    fun aNoteCapturedAfterItFiredBringsTheNextOneForward() {
        val n = nudge(local(24, 9, 10) to local(24, 9), local(24, 9, 40) to local(24, 9, 30))
        assertEquals(
            listOf(local(24, 9, 40), local(24, 10, 40)),
            firings(n, local(24, 9, 30), 2),
        )
    }

    @Test
    fun repeatsAtItsInterval() {
        val n = nudge(local(24, 9, 10) to local(24, 9), repeat = 30 * minute)
        assertEquals(
            listOf(local(24, 9, 10), local(24, 9, 40), local(24, 10, 10)),
            firings(n, local(24, 9), 3),
        )
    }

    @Test
    fun aSnoozeHoldsBackTheNotesThereWere() {
        val n = nudge(
            local(24, 9, 10) to local(24, 9),
            local(24, 9, 41) to local(24, 9, 11),
            snoozeAt = local(24, 9, 12),
            snoozeUntil = local(24, 10, 12),
        )
        assertEquals(listOf(local(24, 10, 12), local(24, 11, 12)), firings(n, local(24, 9, 12), 2))
    }

    @Test
    fun aNoteCapturedDuringASnoozeKeepsItsOwnTime() {
        val n = nudge(
            local(24, 9, 10) to local(24, 9),
            local(24, 9, 30) to local(24, 9, 20),
            snoozeAt = local(24, 9, 12),
            snoozeUntil = local(24, 9, 42),
        )
        assertEquals(
            listOf(local(24, 9, 30), local(24, 9, 42), local(24, 10, 42)),
            firings(n, local(24, 9, 20), 3),
        )
    }

    @Test
    fun waitsOutQuietHoursAndRepeatsFromTheirEnd() {
        val n = nudge(local(24, 22, 5) to local(24, 21, 55))
        assertEquals(
            listOf(utc(2026, 10, 25, 7), utc(2026, 10, 25, 8)),
            firings(n, local(24, 21, 55), 2),
        )
        val off = nudge(local(24, 22, 5) to local(24, 21, 55), quiet = null)
        assertEquals(listOf(local(24, 22, 5), local(24, 23, 5)), firings(off, local(24, 21, 55), 2))
    }

    @Test
    fun aSnoozeEndingInQuietHoursWaitsForTheirEnd() {
        val n = nudge(
            local(24, 21, 10) to local(24, 21),
            snoozeAt = local(24, 21, 15),
            snoozeUntil = local(24, 22, 15),
        )
        assertEquals(utc(2026, 10, 25, 7), NudgePlan.firesAfter(n, berlin, local(24, 21, 15)))
    }

    @Test
    fun startsCountingWhereTheAppLeftOff() {
        val anchors = arrayOf(local(24, 9, 10) to local(24, 9))
        val fromStart = nudge(*anchors, repeat = 45 * minute)
        val fromSince = nudge(*anchors, repeat = 45 * minute, since = local(24, 13, 40))
        for (after in listOf(local(24, 13, 40), local(24, 14), local(24, 20), local(25, 9))) {
            assertEquals(NudgePlan.firesAfter(fromStart, berlin, after), NudgePlan.firesAfter(fromSince, berlin, after))
        }
        // Before it, it counts from the start.
        assertEquals(local(24, 9, 55), NudgePlan.firesAfter(fromSince, berlin, local(24, 9, 10)))
    }

    @Test
    fun aSnoozeTappedOnTheNotificationTakesEffectAtOnce() {
        val n = nudge(local(24, 9, 10) to local(24, 9))
        val queued = listOf(
            Action(1, "snooze", "routine:r", "2026-10-24T08:00", local(24, 9, 11), local(24, 9, 21), "", ""),
            Action(2, NudgePlan.KIND_SNOOZE, "", "", local(24, 9, 11), local(24, 9, 41), "", ""),
            Action(3, NudgePlan.KIND_SNOOZE, "", "", local(24, 9, 12), local(24, 9, 22), "", ""),
        )
        val effective = NudgePlan.effective(n, queued)
        assertEquals(local(24, 9, 12), effective.snoozeAt)
        assertEquals(local(24, 9, 22), effective.snoozeUntil)
        assertEquals(local(24, 9, 22), NudgePlan.firesAfter(effective, berlin, local(24, 9, 12)))
        assertEquals(n, NudgePlan.effective(n, queued.take(1)))
    }

    @Test
    fun isDueWhenItsAlarmFires() {
        val n = nudge(local(24, 9, 10) to local(24, 9))
        val armed = local(24, 9, 10)
        assertTrue(NudgePlan.due(n, berlin, armed, local(24, 9), armed + 500))
        assertFalse(NudgePlan.due(n, berlin, armed, armed + 100, armed + 500))
        assertEquals(local(24, 10, 10), NudgePlan.next(n, berlin, armed + 100, armed + 200))
        // After a reboot it keeps to its hours.
        assertEquals(local(24, 12, 10), NudgePlan.next(n, berlin, 0, local(24, 11, 50)))
    }

    @Test
    fun theOccurrencePlanIgnoresNudgeSnoozes() {
        val plan = listOf(Alarm("occurrence:routine:r@2026-10-24T08:00", "routine:r", "2026-10-24T08:00", listOf(2026, 10, 24, 8, 0), null, "pills", "08:00"))
        val queued = listOf(Action(1, NudgePlan.KIND_SNOOZE, "", "", 0, local(24, 9), "", ""))
        assertEquals(plan, AlarmPlan.effective(plan, queued))
    }
}
