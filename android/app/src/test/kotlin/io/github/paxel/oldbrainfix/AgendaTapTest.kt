package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Test

/** What a tap on the Agenda widget's list does, from its fill-in extras (#153). */
class AgendaTapTest {
    private fun tap(vararg extras: Pair<String, String>): AgendaTap {
        val map = extras.toMap()
        return AgendaTap.of { map[it] }
    }

    private fun check(kind: String, routine: String = "", occurrence: String = "", line: String = "") = tap(
        AgendaTap.EXTRA_TAP to AgendaTap.TAP_CHECK,
        AgendaTap.EXTRA_KIND to kind,
        AlarmReceiver.EXTRA_ROUTINE to routine,
        AlarmReceiver.EXTRA_OCCURRENCE to occurrence,
        AgendaTap.EXTRA_LINE to line,
        AlarmReceiver.EXTRA_TITLE to "Pills",
        AlarmReceiver.EXTRA_BODY to "08:00",
    )

    @Test
    fun aBoxQueuesItsCheck() {
        val done = check("done", "routine:pills", "2026-10-24T08:00") as AgendaTap.Check
        assertEquals("done", done.kind)
        assertEquals("routine:pills", done.alarm.routine)
        assertEquals("2026-10-24T08:00", done.alarm.occurrence)
        assertEquals("Pills", done.alarm.title)
        assertEquals("08:00", done.alarm.body)

        val line = check("line", "routine:trip", "2026-10-24", "line:passport") as AgendaTap.Check
        assertEquals("line:passport", line.line)
        assertEquals("frog:taxes", (check("frog", "frog:taxes") as AgendaTap.Check).alarm.routine)
        assertEquals("line:milk", (check("shopping", line = "line:milk") as AgendaTap.Check).line)
        assertEquals("line:tickets", (check("checklist", line = "line:tickets") as AgendaTap.Check).line)
    }

    @Test
    fun aPickOpensItsEntryForm() {
        val pick = tap(
            AgendaTap.EXTRA_TAP to AgendaTap.TAP_PICK,
            AlarmReceiver.EXTRA_ROUTINE to "session:sport",
            AlarmReceiver.EXTRA_OCCURRENCE to "2026-10-24T07:00",
            AgendaTap.EXTRA_NOTE to "note:push-ups",
        )
        assertEquals(AgendaTap.Pick("session:sport", "2026-10-24T07:00", "note:push-ups"), pick)
        // Without its Tally it opens the app.
        assertEquals(
            AgendaTap.Open,
            tap(
                AgendaTap.EXTRA_TAP to AgendaTap.TAP_PICK,
                AlarmReceiver.EXTRA_ROUTINE to "session:sport",
                AlarmReceiver.EXTRA_OCCURRENCE to "2026-10-24T07:00",
            ),
        )
    }

    @Test
    fun aSessionsBoxDoesNothing() {
        assertEquals(AgendaTap.None, tap(AgendaTap.EXTRA_TAP to AgendaTap.TAP_NONE))
    }

    @Test
    fun everyTapReadsBackFromTheExtrasItPutsIn() {
        val taps = listOf(
            AgendaTap.Check("done", Alarm("", "routine:pills", "2026-10-24T08:00", null, null, "Pills", "08:00"), ""),
            AgendaTap.Check("checklist", Alarm("", "", "", null, null, "Tickets", ""), "line:tickets"),
            AgendaTap.Pick("session:sport", "2026-10-24T07:00", "note:push-ups"),
            AgendaTap.Open,
            AgendaTap.Unsorted,
            AgendaTap.None,
        )
        for (tap in taps) {
            val extras = tap.extras()
            assertEquals(tap, AgendaTap.of { extras[it] })
        }
    }

    @Test
    fun aBoxWhoseCheckDoesNotReadOpensTheApp() {
        assertEquals(AgendaTap.Open, check("done", "routine:pills"))
        assertEquals(AgendaTap.Open, check("line", "routine:trip", "2026-10-24"))
        assertEquals(AgendaTap.Open, check("frog"))
        assertEquals(AgendaTap.Open, check("shopping"))
        assertEquals(AgendaTap.Open, check("checklist"))
        assertEquals(AgendaTap.Open, check("snooze", "routine:pills", "2026-10-24T08:00"))
        assertEquals(AgendaTap.Open, tap(AgendaTap.EXTRA_TAP to AgendaTap.TAP_CHECK))
    }

    @Test
    fun theRestOfARowOpensTheAppAndTheUnsortedCountItsList() {
        assertEquals(AgendaTap.Open, tap(AgendaTap.EXTRA_TAP to AgendaTap.TAP_OPEN))
        assertEquals(AgendaTap.Unsorted, tap(AgendaTap.EXTRA_TAP to AgendaTap.TAP_UNSORTED))
        assertEquals(AgendaTap.Open, tap())
        assertEquals(AgendaTap.Open, tap(AgendaTap.EXTRA_TAP to "later"))
    }
}
