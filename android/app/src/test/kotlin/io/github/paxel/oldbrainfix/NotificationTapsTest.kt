package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** What the trace of the notification taps learns of a tap (obf-1039). */
class NotificationTapsTest {
    private val pills = Alarm(
        id = "a", routine = "routine:pills", occurrence = "2026-10-24T08:00",
        local = null, at = null, title = "Pills", body = "08:00",
    )

    private fun kindOf(flags: Set<String> = emptySet(), extras: Map<String, String> = emptyMap(), fromHistory: Boolean = false) =
        NotificationTaps.kindOf({ it in flags }, { extras[it] }, fromHistory)

    @Test
    fun eachNotificationTellsItsKind() {
        assertEquals("ring", kindOf(setOf(MainActivity.EXTRA_RING)))
        assertEquals("nudge", kindOf(setOf(MainActivity.EXTRA_NUDGE)))
        assertEquals("changes", kindOf(setOf(MainActivity.EXTRA_CHANGES)))
        assertEquals("catchUp", kindOf(setOf(MainActivity.EXTRA_CATCH_UP)))
        assertEquals("occurrence", kindOf(extras = OccurrenceOpen.extras(pills)))
        assertEquals("headsUp", kindOf(extras = OccurrenceOpen.extras(pills, headsUp = true)))
    }

    @Test
    fun anyOtherIntentIsNoTap() {
        assertNull(kindOf())
        assertNull(kindOf(setOf(MainActivity.EXTRA_UNSORTED)))
        assertNull(kindOf(setOf(MainActivity.EXTRA_NUDGE), fromHistory = true))
    }

    @Test
    fun itTellsHowTheAppWas() {
        assertEquals("cold", NotificationTaps.from(cold = true, stopped = false))
        assertEquals("background", NotificationTaps.from(cold = false, stopped = true))
        assertEquals("running", NotificationTaps.from(cold = false, stopped = false))
    }

    @Test
    fun theArrivalHoldsNoText() {
        val arrival = NotificationTaps.arrival("occurrence", "background", 1_000L)
        assertEquals(mapOf("kind" to "occurrence", "from" to "background", "at" to 1_000L), arrival)
    }
}
