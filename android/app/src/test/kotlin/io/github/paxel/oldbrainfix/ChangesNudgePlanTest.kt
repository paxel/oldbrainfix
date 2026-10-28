package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.TimeZone

/**
 * The Changes Nudge (#109) as the alarm layer follows it while the app is
 * closed: the Nudge's rule without a snooze, one anchor per person due 30
 * minutes after their first waiting change and captured at it, the same
 * as the core's `changesNudgePlan`.
 */
class ChangesNudgePlanTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")
    private val night = listOf(22, 0, 8, 0)
    private val minute = 60_000L
    private val hour = 60 * minute

    /** The wall-clock moment in Berlin on a day of October 2026. */
    private fun local(day: Int, hour: Int, minute: Int = 0): Long =
        AlarmPlan.instantOf(listOf(2026, 10, day, hour, minute), berlin)

    /** The Changes Nudge for people whose first waiting changes came at [firsts]. */
    private fun changes(vararg firsts: Long, quiet: List<Int>? = night, since: Long? = null) = Nudge(
        anchors = firsts.map { (it + 30 * minute) to it },
        snoozeAt = null,
        snoozeUntil = null,
        repeatMs = hour,
        quiet = quiet,
        since = since,
        title = "Changes for Bob and Carol",
        body = "Tap to send them.",
    )

    /** The next [count] firings after [after]. */
    private fun firings(nudge: Nudge, after: Long, count: Int): List<Long> {
        var at = after
        return (0 until count).map { NudgePlan.firesAfter(nudge, berlin, at).also { at = it } }
    }

    @Test
    fun firesAtTheEarliestDueForEveryoneAndRepeats() {
        val nudge = changes(local(24, 9), local(24, 9, 20))
        assertEquals(
            listOf(local(24, 9, 30), local(24, 10, 30), local(24, 11, 30)),
            firings(nudge, local(24, 8), 3),
        )
    }

    @Test
    fun aPersonWaitingAfterItFiredBringsItForward() {
        val nudge = changes(local(24, 9), local(24, 9, 50))
        assertEquals(
            listOf(local(24, 9, 30), local(24, 10, 20), local(24, 11, 20)),
            firings(nudge, local(24, 8), 3),
        )
    }

    @Test
    fun waitsOutTheQuietHoursAndRepeatsFromTheirEnd() {
        val nudge = changes(local(23, 21, 45))
        assertEquals(
            listOf(local(24, 8), local(24, 9)),
            firings(nudge, local(23, 21), 2),
        )
        assertEquals(
            listOf(local(23, 22, 15), local(23, 23, 15)),
            firings(changes(local(23, 21, 45), quiet = null), local(23, 21), 2),
        )
    }

    @Test
    fun repeatsFromWhenTheAppSaysItLastFired() {
        // The app planned it last at 10:00, after the Quiet Hours of a
        // window ending then.
        val nudge = changes(local(24, 9), quiet = listOf(9, 0, 10, 0), since = local(24, 10))
        assertEquals(listOf(local(24, 11), local(24, 12)), firings(nudge, local(24, 10, 5), 2))
    }

    @Test
    fun isDueWhenItsAlarmFires() {
        val nudge = changes(local(24, 9))
        assertTrue(NudgePlan.due(nudge, berlin, local(24, 9, 30), 0, local(24, 9, 30)))
        assertFalse(NudgePlan.due(nudge, berlin, local(24, 9, 0), 0, local(24, 9, 0)))
        assertEquals(local(24, 9, 30), NudgePlan.next(nudge, berlin, 0, local(24, 9)))
    }
}
