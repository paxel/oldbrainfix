package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.Calendar
import java.util.TimeZone

class AgendaWidgetPlanTest {
    private val berlin = TimeZone.getTimeZone("Europe/Berlin")

    private fun utc(year: Int, month: Int, day: Int, hour: Int, minute: Int = 0): Long =
        Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            clear()
            set(year, month - 1, day, hour, minute, 0)
        }.timeInMillis

    private fun row(title: String, occurrence: String, done: Boolean = false) =
        WidgetRow("routine:$title", occurrence, title, occurrence.substringAfter('T', "All day"), done)

    private val today = listOf(
        row("bins", "2026-10-24"),
        row("water", "2026-10-24T07:00"),
        row("pills", "2026-10-24T08:00"),
        row("plants", "2026-10-24T18:00"),
    )
    private val tomorrow = listOf(row("dentist", "2026-10-25T10:00"))

    private val snapshot = AgendaSnapshot(
        days = mapOf("2026-10-24" to today, "2026-10-25" to tomorrow),
        unsorted = 2,
        today = "Today",
        empty = "Nothing due today.",
        unsortedText = "2 Unsorted",
        more = "and {count} more",
        done = "Done",
    )

    private fun done(seq: Long, routine: String, occurrence: String) =
        Action(seq, "done", routine, occurrence, 0, 0, "", "")

    private fun titles(view: AgendaView) = view.rows.map { it.title }

    @Test
    fun showsTodaysRowsInAgendaOrder() {
        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", emptyList(), 10)

        assertEquals(listOf("bins", "water", "pills", "plants"), titles(view))
        assertEquals(0, view.more)
        assertFalse(view.stale)
    }

    @Test
    fun movesOnToTomorrowAfterMidnightWithoutTheApp() {
        val view = AgendaWidgetPlan.view(snapshot, "2026-10-25", emptyList(), 10)

        assertEquals(listOf("dentist"), titles(view))
        assertFalse(view.stale)
    }

    @Test
    fun aDayWithoutOccurrencesIsEmptyNotStale() {
        val empty = snapshot.copy(days = mapOf("2026-10-24" to emptyList(), "2026-10-25" to tomorrow))

        val view = AgendaWidgetPlan.view(empty, "2026-10-24", emptyList(), 10)

        assertTrue(view.rows.isEmpty())
        assertFalse(view.stale)
    }

    @Test
    fun aSnapshotThatDoesNotReachTodayIsStale() {
        assertTrue(AgendaWidgetPlan.view(snapshot, "2026-10-26", emptyList(), 10).stale)
        assertTrue(AgendaWidgetPlan.view(snapshot, "2026-10-23", emptyList(), 10).stale)
        assertTrue(AgendaWidgetPlan.view(null, "2026-10-24", emptyList(), 10).stale)
    }

    @Test
    fun aDoneTappedAndNotYetAppliedShowsDone() {
        val pending = listOf(
            done(1, "routine:pills", "2026-10-24T08:00"),
            // Yesterday's pills, and a snooze, change nothing.
            done(2, "routine:pills", "2026-10-23T08:00"),
            Action(3, "snooze", "routine:water", "2026-10-24T07:00", 0, 1, "", ""),
        )

        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", pending, 10)

        assertEquals(listOf(false, false, true, false), view.rows.map { it.done })
    }

    @Test
    fun whatIsDoneInTheSnapshotStaysDone() {
        val checked = snapshot.copy(days = mapOf("2026-10-24" to listOf(row("pills", "2026-10-24T08:00", done = true))))

        val view = AgendaWidgetPlan.view(checked, "2026-10-24", emptyList(), 10)

        assertTrue(view.rows.single().done)
    }

    @Test
    fun whenNotAllFitOneLineTellsHowManyMore() {
        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", emptyList(), 3)

        assertEquals(listOf("bins", "water"), titles(view))
        assertEquals(2, view.more)
        assertEquals("and 2 more", AgendaWidgetPlan.moreText(snapshot.more, view.more))
    }

    @Test
    fun exactlyAsManyAsFitNeedNoMoreLine() {
        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", emptyList(), 4)

        assertEquals(4, view.rows.size)
        assertEquals(0, view.more)
    }

    @Test
    fun doneRowsGiveWayFirstAndTheRestKeepsAgendaOrder() {
        val pending = listOf(done(1, "routine:bins", "2026-10-24"), done(2, "routine:water", "2026-10-24T07:00"))

        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", pending, 3)

        assertEquals(listOf("pills", "plants"), titles(view))
        assertEquals(2, view.more)
    }

    @Test
    fun doneRowsFillWhatIsLeft() {
        val pending = listOf(
            done(1, "routine:bins", "2026-10-24"),
            done(2, "routine:water", "2026-10-24T07:00"),
            done(3, "routine:pills", "2026-10-24T08:00"),
        )

        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", pending, 3)

        // One open row, so the first Done one takes the other line.
        assertEquals(listOf("bins", "plants"), titles(view))
        assertEquals(2, view.more)
    }

    @Test
    fun aTinyWidgetStillShowsOneRow() {
        val view = AgendaWidgetPlan.view(snapshot, "2026-10-24", emptyList(), 1)

        assertEquals(listOf("bins"), titles(view))
        assertEquals(3, view.more)
    }

    @Test
    fun theRoomGrowsWithTheWidgetsHeight() {
        assertEquals(AgendaWidgetPlan.DEFAULT_ROOM, AgendaWidgetPlan.room(0))
        assertEquals(1, AgendaWidgetPlan.room(60))
        assertTrue(AgendaWidgetPlan.room(400) > AgendaWidgetPlan.room(200))
    }

    @Test
    fun theDayIsTheOneTheLocalClockShows() {
        // 22:30 UTC on Saturday is 00:30 on Sunday in Berlin.
        assertEquals("2026-10-25", AgendaWidgetPlan.dayKey(utc(2026, 10, 24, 22, 30), berlin))
        assertEquals("2026-10-24", AgendaWidgetPlan.dayKey(utc(2026, 10, 24, 21, 30), berlin))
        assertEquals("2026-10-24", AgendaWidgetPlan.dayKey(utc(2026, 10, 24, 22, 30), TimeZone.getTimeZone("UTC")))
    }

    @Test
    fun theNextMidnightIsLocal() {
        // Summer time in Berlin: midnight is 22:00 UTC.
        assertEquals(utc(2026, 10, 23, 22), AgendaWidgetPlan.nextMidnight(utc(2026, 10, 23, 9), berlin))
    }

    @Test
    fun theNextMidnightHoldsAcrossTheClockChange() {
        // Berlin leaves summer time on 25 October 2026: midnight after it is 23:00 UTC.
        assertEquals(utc(2026, 10, 25, 23), AgendaWidgetPlan.nextMidnight(utc(2026, 10, 25, 9), berlin))
        // Just before midnight, the next one is minutes away.
        assertEquals(utc(2026, 10, 24, 22), AgendaWidgetPlan.nextMidnight(utc(2026, 10, 24, 21, 59), berlin))
    }
}
