package io.github.paxel.oldbrainfix

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
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

    private fun berlin(year: Int, month: Int, day: Int, hour: Int, minute: Int = 0): Long =
        Calendar.getInstance(berlin).apply {
            clear()
            set(year, month - 1, day, hour, minute, 0)
        }.timeInMillis

    private fun occurrence(title: String, occurrence: String, done: Boolean = false, checks: Boolean = true): WidgetRow {
        val time = occurrence.substringAfter('T', "")
        val at = if (time.isEmpty()) null else time.substring(0, 2).toInt() * 60 + time.substring(3).toInt()
        return WidgetRow(
            kind = WidgetRow.OCCURRENCE,
            title = title,
            time = time.ifEmpty { "All day" },
            done = done,
            checks = checks,
            routine = "routine:$title",
            occurrence = occurrence,
            block = if (at == null) WidgetRow.ALL_DAY else WidgetRow.TIMED,
            at = at,
        )
    }

    private fun line(of: WidgetRow, text: String, done: Boolean = false) = WidgetRow(
        kind = WidgetRow.LINE,
        title = text,
        done = done,
        checks = true,
        indent = 1,
        routine = of.routine,
        occurrence = of.occurrence,
        line = "line:$text",
        block = of.block,
        at = of.at,
    )

    private fun frog(title: String, done: Boolean = false) = WidgetRow(
        kind = WidgetRow.FROG,
        title = title,
        done = done,
        checks = true,
        frog = "frog:$title",
        block = WidgetRow.FROGS,
    )

    private val bins = occurrence("bins", "2026-10-24")
    private val water = occurrence("water", "2026-10-24T07:00")
    private val trip = occurrence("trip", "2026-10-24T08:00")
    private val plants = occurrence("plants", "2026-10-24T18:00")
    private val milk = WidgetRow(
        kind = WidgetRow.SHOPPING,
        title = "Milk · Groceries",
        checks = true,
        line = "line:milk",
        block = WidgetRow.LISTS,
    )
    private val today = listOf(
        bins,
        milk,
        water,
        trip,
        line(trip, "passport"),
        line(trip, "tickets"),
        plants,
        WidgetRow(WidgetRow.HEADING, "Eat a frog", block = WidgetRow.FROGS),
        frog("taxes"),
        frog("dentist"),
    )
    private val tomorrow = listOf(occurrence("dentist", "2026-10-25T10:00"))
    private val missed = listOf(
        WidgetRow(WidgetRow.HEADING, "Missed"),
        WidgetRow(WidgetRow.TEXT, "pills · once"),
        WidgetRow(WidgetRow.MISSED, "Fri, 10/23/2026", "08:00", checks = true, routine = "routine:pills", occurrence = "2026-10-23T08:00"),
    )

    private val snapshot = AgendaSnapshot(
        days = mapOf("2026-10-24" to today, "2026-10-25" to tomorrow),
        missed = missed,
        unsorted = 2,
        calendars = listOf("7"),
        today = "Today",
        empty = "Nothing due today.",
        unsortedText = "2 Unsorted",
        done = "Done",
        allDay = "All day",
        noTitle = "Appointment without a title",
        since = "from {time}",
        until = "until {time}",
    )

    private fun action(seq: Long, kind: String, routine: String, occurrence: String = "", line: String = "") =
        Action(seq, kind, routine, occurrence, 0, 0, "", "", line)

    private fun view(pending: List<Action> = emptyList(), appointments: List<WidgetRow> = emptyList(), day: String = "2026-10-24") =
        AgendaWidgetPlan.view(snapshot, day, pending, mapOf(day to appointments), "Open the app")

    /** [snapshot] as version 4 writes it (#194): the Agenda's days, each with its heading. */
    private val week = snapshot.copy(
        days = mapOf(
            "2026-10-24" to today,
            "2026-10-25" to tomorrow,
            "2026-10-26" to emptyList(),
            "2026-10-27" to listOf(occurrence("bins", "2026-10-27")),
        ),
        headings = mapOf(
            "2026-10-24" to "Saturday, 24 Oct",
            "2026-10-25" to "Sunday, 25 Oct",
            "2026-10-26" to "Monday, 26 Oct",
            "2026-10-27" to "Tuesday, 27 Oct",
        ),
        tomorrow = "Tomorrow",
        planned = "Nothing planned",
    )

    private fun titles(view: AgendaView) = view.rows.map { it.title }

    @Test
    fun showsTodayInAgendaOrderAfterTheMissedSummaryAndTheUnsortedCount() {
        val view = view()

        assertEquals(
            listOf(
                "Missed", "pills · once", "Fri, 10/23/2026",
                "2 Unsorted",
                "Today",
                "bins", "Milk · Groceries", "water", "trip", "passport", "tickets", "plants",
                "Eat a frog", "taxes", "dentist",
            ),
            titles(view),
        )
        assertEquals(WidgetRow.UNSORTED, view.rows[3].kind)
        assertFalse(view.stale)
    }

    @Test
    fun movesOnToTomorrowAfterMidnightWithoutTheApp() {
        val view = view(day = "2026-10-25")

        assertEquals(listOf("Missed", "pills · once", "Fri, 10/23/2026", "2 Unsorted", "Today", "dentist"), titles(view))
        assertFalse(view.stale)
    }

    @Test
    fun showsTheAgendasComingDaysUnderTheirHeadingsAfterToday() {
        val view = AgendaWidgetPlan.view(week, "2026-10-24", emptyList(), emptyMap(), "Open the app")

        assertEquals(
            listOf(
                "Missed", "pills · once", "Fri, 10/23/2026",
                "2 Unsorted",
                "Today",
                "bins", "Milk · Groceries", "water", "trip", "passport", "tickets", "plants",
                "Eat a frog", "taxes", "dentist",
                "Tomorrow", "dentist",
                "Monday, 26 Oct", "Nothing planned",
                "Tuesday, 27 Oct", "bins",
            ),
            titles(view),
        )
        assertEquals(listOf(WidgetRow.HEADING, WidgetRow.MESSAGE), view.rows.subList(17, 19).map { it.kind })
        assertFalse(view.stale)
    }

    @Test
    fun theComingDaysMoveOnAtMidnightWithoutTheApp() {
        val view = AgendaWidgetPlan.view(week, "2026-10-25", emptyList(), emptyMap(), "Open the app")

        assertEquals(
            listOf("Today", "dentist", "Tomorrow", "Nothing planned", "Tuesday, 27 Oct", "bins"),
            titles(view).drop(4),
        )
    }

    @Test
    fun onlyTodaysAndTheMissedRowsAreTickable() {
        val view = AgendaWidgetPlan.view(week, "2026-10-24", emptyList(), emptyMap(), "")
        val dentists = view.rows.filter { it.title == "dentist" }

        // Today's Frog and Missed Occurrence check off; tomorrow's dentist only opens the app.
        assertEquals(WidgetTap("frog", "frog:dentist", "", ""), dentists[0].tap)
        assertEquals(WidgetTap("done", "routine:pills", "2026-10-23T08:00", ""), view.rows[2].tap)
        val coming = view.rows.dropWhile { it.title != "Tomorrow" }.filter { it.box }
        assertEquals(listOf("dentist", "bins"), coming.map { it.title })
        for (row in coming) {
            // The snapshot says this Device may check them: on a coming day it still does not.
            assertTrue(row.checks)
            assertNull(row.title, row.tap)
            assertEquals(row.title, AgendaTap.Open, row.onBox)
            assertEquals(row.title, AgendaTap.Open, row.onRow)
        }
        // Nor does a coming day's Line or pick.
        val ahead = line(occurrence("trip", "2026-10-25T08:00"), "passport").copy(ahead = true)
        assertNull(ahead.tap)
        assertEquals(AgendaTap.Open, ahead.onBox)
        val pick = WidgetRow(
            WidgetRow.PICK, "Push-ups", checks = true, indent = 1, ahead = true,
            routine = "session:sport", occurrence = "2026-10-25T07:00", note = "note:push-ups",
        )
        assertEquals(AgendaTap.Open, pick.onBox)
        assertEquals(AgendaTap.Open, pick.onRow)
    }

    @Test
    fun theComingDaysTakeTheirOwnAppointments() {
        val holiday = WidgetRow(WidgetRow.APPOINTMENT, "Holiday", "All day", block = WidgetRow.ALL_DAY)
        val film = WidgetRow(WidgetRow.APPOINTMENT, "Film", "21:00", block = WidgetRow.TIMED, at = 21 * 60)

        val view = AgendaWidgetPlan.view(
            week, "2026-10-24", emptyList(), mapOf("2026-10-26" to listOf(holiday), "2026-10-27" to listOf(film)), "",
        )

        assertEquals(listOf("Monday, 26 Oct", "Holiday", "Tuesday, 27 Oct", "bins", "Film"), titles(view).takeLast(5))
        // Dimmed with the rest of their day.
        assertTrue(view.rows.filter { it.kind == WidgetRow.APPOINTMENT }.all { it.ahead })
        assertEquals(listOf("2026-10-24", "2026-10-25", "2026-10-26", "2026-10-27"), AgendaWidgetPlan.shownDays(week, "2026-10-24"))
        assertEquals(listOf("2026-10-26", "2026-10-27"), AgendaWidgetPlan.shownDays(week, "2026-10-26"))
        // An older snapshot holds no headings: it shows today alone, as before.
        assertEquals(listOf("2026-10-24"), AgendaWidgetPlan.shownDays(snapshot, "2026-10-24"))
    }

    @Test
    fun aDayWithNothingDueSaysSoAboveItsFrogs() {
        val frogs = snapshot.copy(
            days = mapOf("2026-10-24" to listOf(WidgetRow(WidgetRow.HEADING, "Eat a frog", block = WidgetRow.FROGS), frog("taxes"))),
            missed = emptyList(),
            unsorted = 0,
        )

        val view = AgendaWidgetPlan.view(frogs, "2026-10-24", emptyList(), emptyMap(), "Open the app")

        assertEquals(listOf("Today", "Nothing due today.", "Eat a frog", "taxes"), titles(view))
        assertEquals(WidgetRow.MESSAGE, view.rows[1].kind)
    }

    @Test
    fun aSnapshotThatDoesNotReachTodayAsksToOpenTheApp() {
        val stale = view(day = "2026-10-26")
        assertTrue(stale.stale)
        assertEquals(listOf("Missed", "pills · once", "Fri, 10/23/2026", "2 Unsorted", "Open the app"), titles(stale))
        assertTrue(view(day = "2026-10-23").stale)
        val none = AgendaWidgetPlan.view(null, "2026-10-24", emptyList(), emptyMap(), "Open the app")
        assertTrue(none.stale)
        assertEquals(listOf("Open the app"), titles(none))
    }

    @Test
    fun eachBoxQueuesWhatTheAgendasBoxDoes() {
        val rows = view().rows.associateBy { it.title }

        assertEquals(WidgetTap("done", "routine:water", "2026-10-24T07:00", ""), rows["water"]?.tap)
        assertEquals(WidgetTap("done", "routine:pills", "2026-10-23T08:00", ""), rows["Fri, 10/23/2026"]?.tap)
        assertEquals(WidgetTap("line", "routine:trip", "2026-10-24T08:00", "line:passport"), rows["passport"]?.tap)
        assertEquals(WidgetTap("frog", "frog:taxes", "", ""), rows["taxes"]?.tap)
        assertEquals(WidgetTap("shopping", "", "", "line:milk"), rows["Milk · Groceries"]?.tap)
        // Headings, texts and the Unsorted count have no box.
        for (title in listOf("Missed", "pills · once", "2 Unsorted", "Today", "Eat a frog")) {
            assertFalse(title, rows[title]?.box ?: true)
            assertNull(title, rows[title]?.tap)
        }
    }

    @Test
    fun aRowWithoutABoxOrNotToBeCheckedHereQueuesNothing() {
        val list = WidgetRow(WidgetRow.SHOPPING, "Groceries: 3 back", block = WidgetRow.LISTS)
        val checklist = WidgetRow(WidgetRow.CHECKLIST, "Before the trip: 3 of 7 open", block = WidgetRow.LISTS)
        val session = occurrence("push-ups", "2026-10-24T09:00", checks = false)

        assertFalse(list.box)
        assertNull(list.tap)
        assertFalse(checklist.box)
        assertNull(checklist.tap)
        // Its box shows, and a tap opens the app.
        assertTrue(session.box)
        assertNull(session.tap)
        assertNull(occurrence("pills", "2026-10-24T08:00", done = true).tap)
        assertNull(milk.copy(checks = false).tap)
    }

    @Test
    fun whatWasCheckedAndNotYetAppliedShowsChecked() {
        val pending = listOf(
            action(1, "done", "routine:water", "2026-10-24T07:00"),
            // Yesterday's water, and a snooze, change nothing.
            action(2, "done", "routine:water", "2026-10-23T07:00"),
            action(3, "snooze", "routine:plants", "2026-10-24T18:00"),
            action(4, "done", "routine:pills", "2026-10-23T08:00"),
            action(5, "line", "routine:trip", "2026-10-24T08:00", "line:passport"),
            action(6, "shopping", "", line = "line:milk"),
        )

        val done = view(pending).rows.filter { it.done }.map { it.title }

        assertEquals(listOf("Fri, 10/23/2026", "Milk · Groceries", "water", "passport"), done)
    }

    @Test
    fun theLastLineCheckedChecksItsOccurrenceOff() {
        val pending = listOf(
            action(1, "line", "routine:trip", "2026-10-24T08:00", "line:passport"),
            action(2, "line", "routine:trip", "2026-10-24T08:00", "line:tickets"),
        )

        val rows = view(pending).rows.associateBy { it.title }

        assertTrue(rows["trip"]?.done ?: false)
        assertNull(rows["trip"]?.tap)
        // One Line only leaves it open.
        assertFalse(view(pending.take(1)).rows.single { it.title == "trip" }.done)
    }

    @Test
    fun aFrogCheckedOffLeavesTheOtherOut() {
        val view = view(listOf(action(1, "frog", "frog:dentist")))

        assertEquals(listOf("Eat a frog", "dentist"), view.rows.filter { it.block == WidgetRow.FROGS }.map { it.title })
        assertTrue(view.rows.last().done)
    }

    @Test
    fun whatIsCheckedInTheSnapshotStaysChecked() {
        val checked = snapshot.copy(days = mapOf("2026-10-24" to listOf(occurrence("pills", "2026-10-24T08:00", done = true))))

        val view = AgendaWidgetPlan.view(checked, "2026-10-24", emptyList(), emptyMap(), "")

        assertTrue(view.rows.single { it.title == "pills" }.done)
    }

    @Test
    fun appointmentsGoWhereTheAgendaPutsThem() {
        val allDay = WidgetRow(WidgetRow.APPOINTMENT, "Holiday", "All day", block = WidgetRow.ALL_DAY)
        val early = WidgetRow(WidgetRow.APPOINTMENT, "Call", "06:30", block = WidgetRow.TIMED, at = 6 * 60 + 30)
        val atTrip = WidgetRow(WidgetRow.APPOINTMENT, "Taxi", "08:00", block = WidgetRow.TIMED, at = 8 * 60)
        val late = WidgetRow(WidgetRow.APPOINTMENT, "Film", "21:00", block = WidgetRow.TIMED, at = 21 * 60)

        val view = view(appointments = listOf(allDay, early, atTrip, late))

        assertEquals(
            listOf(
                "bins", "Holiday", "Milk · Groceries", "Call", "water",
                // An Occurrence, with its Lines, before an appointment at the same time.
                "trip", "passport", "tickets", "Taxi",
                "plants", "Film", "Eat a frog", "taxes", "dentist",
            ),
            titles(view).drop(5),
        )
        assertNull(view.rows.single { it.title == "Taxi" }.tap)
    }

    @Test
    fun theAppointmentsOfTheDayAreItsPiecesInOrder() {
        val entries = listOf(
            CalendarEntry("Film", berlin(2026, 10, 24, 21), berlin(2026, 10, 24, 23), false, 0xFF0000),
            CalendarEntry(" ", berlin(2026, 10, 24, 9), berlin(2026, 10, 24, 9), false, 0),
            // From yesterday into today, and from today into tomorrow.
            CalendarEntry("Night shift", berlin(2026, 10, 23, 22), berlin(2026, 10, 24, 6), false, 0),
            CalendarEntry("Night train", berlin(2026, 10, 24, 23), berlin(2026, 10, 25, 7), false, 0),
            // Ending at midnight, it closes the day before.
            CalendarEntry("Party", berlin(2026, 10, 23, 20), berlin(2026, 10, 24, 0), false, 0),
            CalendarEntry("Holiday", utc(2026, 10, 24, 0), utc(2026, 10, 26, 0), true, 0),
            CalendarEntry("Fair", utc(2026, 10, 23, 0), utc(2026, 10, 24, 0), true, 0),
            CalendarEntry("Away", berlin(2026, 10, 23, 12), berlin(2026, 10, 25, 12), false, 0),
        )

        val rows = AgendaWidgetPlan.appointmentRows(snapshot, entries, "2026-10-24", berlin)

        assertEquals(
            listOf("Away", "Holiday", "Night shift", "Appointment without a title", "Film", "Night train"),
            rows.map { it.title },
        )
        assertEquals(listOf("All day", "All day", "until 06:00", "09:00", "21:00–23:00", "from 23:00"), rows.map { it.time })
        assertEquals(listOf(null, null, 0, 9 * 60, 21 * 60, 23 * 60), rows.map { it.at })
        assertEquals(0xFF0000, rows[4].colour)
        assertTrue(rows.none { it.box })
    }

    @Test
    fun aSnapshotOfZeroTwoZeroStillReads() {
        // As 0.2.0 wrote it: no version, Occurrences only.
        val old = """
            {"days":{"2026-10-24":[{"routine":"routine:pills","occurrence":"2026-10-24T08:00","title":"pills","time":"08:00","done":false},
            {"routine":"routine:bins","occurrence":"2026-10-24","title":"bins","time":"All day","done":true}],"2026-10-25":[]},
            "unsorted":1,"labels":{"today":"Today","empty":"Nothing due today.","unsorted":"1 Unsorted","more":"and {count} more","done":"Done"}}
        """.trimIndent()

        val read = AgendaWidgetPlan.snapshotOf(old)

        val rows = read?.days?.get("2026-10-24") ?: emptyList()
        assertEquals(listOf("pills", "bins"), rows.map { it.title })
        assertEquals(WidgetTap("done", "routine:pills", "2026-10-24T08:00", ""), rows[0].tap)
        assertTrue(rows[1].done)
        assertEquals(emptyList<String>(), read?.calendars)
        assertEquals("1 Unsorted", read?.unsortedText)
        val view = AgendaWidgetPlan.view(read, "2026-10-24", emptyList(), emptyMap(), "")
        assertEquals(listOf("1 Unsorted", "Today", "pills", "bins"), titles(view))
    }

    @Test
    fun aSnapshotOfVersionTwoReadsEveryKind() {
        fun row(kind: String, title: String) = JSONObject().put("kind", kind).put("title", title)
        val day = JSONArray()
            .put(row("occurrence", "trip").put("routine", "routine:trip").put("occurrence", "2026-10-24T08:00").put("time", "08:00").put("checks", true).put("block", "timed").put("at", 480))
            .put(row("line", "passport").put("routine", "routine:trip").put("occurrence", "2026-10-24T08:00").put("line", "line:passport").put("indent", 2).put("checks", true).put("done", true))
            .put(row("text", "Push-ups 20 times").put("indent", 1))
            .put(row("shopping", "Milk · Groceries").put("line", "line:milk").put("checks", true).put("block", "lists"))
            .put(row("checklist", "Before the trip: 3 of 7 open").put("block", "lists"))
            .put(row("heading", "Eat a frog").put("block", "frogs"))
            .put(row("frog", "taxes").put("frog", "frog:taxes").put("checks", true).put("block", "frogs"))
            // Of a kind not known, of a later version, or without a title: left out.
            .put(row("laterKind", "?"))
            .put(row("frog", "later").put("v", 2))
            .put(JSONObject().put("kind", "text"))
            .put("x")
        val snapshot = JSONObject()
            .put("v", 2)
            .put("days", JSONObject().put("2026-10-24", day))
            .put("missed", JSONArray().put(row("missed", "Fri").put("routine", "routine:pills").put("occurrence", "2026-10-23T08:00").put("checks", true)))
            .put("unsorted", 0)
            .put("calendars", JSONArray().put("7").put(8))
            .put("labels", JSONObject().put("today", "Heute").put("since", "ab {time}").put("allDay", "Ganztägig"))

        val read = AgendaWidgetPlan.snapshotOf(snapshot.toString())

        val rows = read?.days?.get("2026-10-24") ?: emptyList()
        assertEquals(
            listOf("occurrence", "line", "text", "shopping", "checklist", "heading", "frog"),
            rows.map { it.kind },
        )
        assertEquals(480, rows[0].at)
        assertEquals(WidgetRow.TIMED, rows[0].block)
        assertEquals(2, rows[1].indent)
        assertTrue(rows[1].done)
        assertEquals("frog:taxes", rows[6].frog)
        assertEquals(listOf("Fri"), read?.missed?.map { it.title })
        assertEquals(listOf("7"), read?.calendars)
        assertEquals("Heute", read?.today)
        assertEquals("ab {time}", read?.since)
        assertEquals("Ganztägig", read?.allDay)
    }

    @Test
    fun aSnapshotOfVersionTwoTakesNoKindOfALaterOne() {
        fun row(kind: String, title: String) = JSONObject().put("kind", kind).put("title", title)
        val day = JSONArray()
            .put(row("occurrence", "Sport").put("routine", "session:sport").put("occurrence", "2026-10-24T07:00"))
            .put(row("pick", "Push-ups").put("note", "note:push-ups"))
            .put(row("checklistLine", "Tickets").put("line", "line:tickets"))
        val snapshot = JSONObject().put("v", 2).put("days", JSONObject().put("2026-10-24", day)).put("labels", JSONObject())

        val rows = AgendaWidgetPlan.snapshotOf(snapshot.toString())?.days?.get("2026-10-24") ?: emptyList()

        assertEquals(listOf("Sport"), rows.map { it.title })
    }

    @Test
    fun aSnapshotOfVersionThreeReadsTheGroups() {
        fun row(kind: String, title: String) = JSONObject().put("kind", kind).put("title", title)
        val day = JSONArray()
            .put(row("shopping", "Groceries").put("block", "lists"))
            .put(row("shopping", "Milk").put("line", "line:milk").put("checks", true).put("indent", 1).put("block", "lists"))
            .put(row("checklist", "Before the trip").put("block", "lists"))
            .put(row("checklistLine", "Tickets").put("line", "line:tickets").put("checks", true).put("indent", 1).put("block", "lists"))
            .put(row("session", "Sport").put("routine", "session:sport").put("occurrence", "2026-10-24T07:00").put("time", "07:00").put("block", "timed").put("at", 420))
            .put(row("pick", "Push-ups 20 times").put("routine", "session:sport").put("occurrence", "2026-10-24T07:00").put("note", "note:push-ups").put("checks", true).put("indent", 1).put("block", "timed").put("at", 420))
            .put(row("laterKind", "?"))
        val snapshot = JSONObject()
            .put("v", 3)
            .put("days", JSONObject().put("2026-10-24", day))
            .put("unsorted", 0)
            .put("labels", JSONObject().put("today", "Today"))

        val read = AgendaWidgetPlan.snapshotOf(snapshot.toString())

        val rows = read?.days?.get("2026-10-24") ?: emptyList()
        assertEquals(listOf("shopping", "shopping", "checklist", "checklistLine", "session", "pick"), rows.map { it.kind })
        assertEquals(listOf(0, 1, 0, 1, 0, 1), rows.map { it.indent })
        assertEquals("note:push-ups", rows[5].note)
        assertEquals("Today", read?.today)
        // The headings of the lists have no box; their Lines, the Session and its picks have.
        assertEquals(listOf(false, true, false, true, true, true), rows.map { it.box })
        assertEquals(listOf(true, false, true, false, false, false), rows.map { it.heading })
    }

    @Test
    fun aSnapshotOfVersionFourReadsTheDaysWithTheirHeadings() {
        fun row(kind: String, title: String) = JSONObject().put("kind", kind).put("title", title)
        val days = JSONObject()
            .put("2026-10-24", JSONArray().put(row("session", "Sport").put("routine", "session:sport").put("occurrence", "2026-10-24T07:00")))
            .put("2026-10-25", JSONArray().put(row("occurrence", "dentist").put("routine", "routine:dentist").put("occurrence", "2026-10-25T10:00").put("checks", true)))
            .put("2026-10-26", JSONArray())
        val snapshot = JSONObject()
            .put("v", 4)
            .put("days", days)
            .put("headings", JSONObject().put("2026-10-24", "Saturday, 24 Oct").put("2026-10-25", "Sunday, 25 Oct").put("2026-10-26", "Monday, 26 Oct").put("2026-10-27", 7))
            .put("labels", JSONObject().put("today", "Heute").put("tomorrow", "Morgen").put("planned", "Nichts geplant"))

        val read = AgendaWidgetPlan.snapshotOf(snapshot.toString())

        assertEquals(listOf("Sport"), read?.days?.get("2026-10-24")?.map { it.title })
        // A heading that is no text is left out.
        assertEquals(
            mapOf("2026-10-24" to "Saturday, 24 Oct", "2026-10-25" to "Sunday, 25 Oct", "2026-10-26" to "Monday, 26 Oct"),
            read?.headings,
        )
        assertEquals("Morgen", read?.tomorrow)
        assertEquals("Nichts geplant", read?.planned)
        val view = AgendaWidgetPlan.view(read, "2026-10-24", emptyList(), emptyMap(), "")
        assertEquals(listOf("Heute", "Sport", "Morgen", "dentist", "Monday, 26 Oct", "Nichts geplant"), titles(view))
    }

    @Test
    fun aSnapshotOfVersionThreeStillShowsTodayAlone() {
        fun row(kind: String, title: String) = JSONObject().put("kind", kind).put("title", title)
        val snapshot = JSONObject()
            .put("v", 3)
            .put("days", JSONObject().put("2026-10-24", JSONArray().put(row("occurrence", "pills"))).put("2026-10-25", JSONArray().put(row("occurrence", "dentist"))))
            .put("labels", JSONObject().put("today", "Today"))

        val read = AgendaWidgetPlan.snapshotOf(snapshot.toString())

        assertEquals(listOf("Today", "pills"), titles(AgendaWidgetPlan.view(read, "2026-10-24", emptyList(), emptyMap(), "")))
        assertEquals(listOf("Today", "dentist"), titles(AgendaWidgetPlan.view(read, "2026-10-25", emptyList(), emptyMap(), "")))
    }

    @Test
    fun aSessionsBoxOnlyShowsDoneAndAPickOpensItsEntryForm() {
        val sport = WidgetRow(
            WidgetRow.SESSION, "Sport", "07:00", checks = false,
            routine = "session:sport", occurrence = "2026-10-24T07:00", block = WidgetRow.TIMED, at = 420,
        )
        val pick = WidgetRow(
            WidgetRow.PICK, "Push-ups 20 times", checks = true, indent = 1,
            routine = "session:sport", occurrence = "2026-10-24T07:00", note = "note:push-ups",
            block = WidgetRow.TIMED, at = 420,
        )

        assertTrue(sport.box)
        assertNull(sport.tap)
        assertEquals(AgendaTap.None, sport.onBox)
        assertEquals(AgendaTap.Open, sport.onRow)
        // Even with checks, as a snapshot might say.
        assertEquals(AgendaTap.None, sport.copy(checks = true).onBox)
        val opens = AgendaTap.Pick("session:sport", "2026-10-24T07:00", "note:push-ups")
        assertTrue(pick.box)
        assertNull(pick.tap)
        assertEquals(opens, pick.onBox)
        assertEquals(opens, pick.onRow)
        // Its entry form open here or not, its box shows whether it has an entry.
        assertEquals(opens, pick.copy(done = true).onRow)
        assertEquals(AgendaTap.Open, pick.copy(checks = false).onBox)
        assertEquals(AgendaTap.Open, pick.copy(note = "").onRow)
        // Done on its notification and not yet applied, it shows Done.
        val day = snapshot.copy(days = mapOf("2026-10-24" to listOf(sport, pick)))
        val view = AgendaWidgetPlan.view(day, "2026-10-24", listOf(action(1, "done", "session:sport", "2026-10-24T07:00")), emptyMap(), "")
        assertTrue(view.rows.single { it.title == "Sport" }.done)
    }

    @Test
    fun aChecklistsLineChecksItAndShowsCheckedUntilTheAppHasIt() {
        val tickets = WidgetRow(
            WidgetRow.CHECKLIST_LINE, "Tickets", checks = true, indent = 1, line = "line:tickets", block = WidgetRow.LISTS,
        )
        val day = snapshot.copy(days = mapOf("2026-10-24" to listOf(WidgetRow(WidgetRow.CHECKLIST, "Before the trip", block = WidgetRow.LISTS), tickets)))

        assertEquals(WidgetTap("checklist", "", "", "line:tickets"), tickets.tap)
        assertEquals(AgendaTap.Check("checklist", Alarm("", "", "", null, null, "Tickets", ""), "line:tickets"), tickets.onBox)
        assertEquals(AgendaTap.Open, tickets.onRow)
        assertNull(tickets.copy(checks = false).tap)
        val view = AgendaWidgetPlan.view(day, "2026-10-24", listOf(action(1, "checklist", "", line = "line:tickets")), emptyMap(), "")
        assertTrue(view.rows.single { it.title == "Tickets" }.done)
    }

    @Test
    fun everyGroupsEntriesAreIndentedAlikeTheirBoxesWhereTheHeadingsWordsBegin() {
        assertEquals(0, AgendaWidgetPlan.indentDp(0))
        // A box is 40 wide: an entry's box stands where its heading's words begin.
        assertEquals(AgendaWidgetPlan.BOX_DP, AgendaWidgetPlan.indentDp(1))
        assertEquals(AgendaWidgetPlan.BOX_DP + AgendaWidgetPlan.STEP_DP, AgendaWidgetPlan.indentDp(2))
    }

    @Test
    fun aSnapshotOfALaterVersionAsksToOpenTheApp() {
        for (version in listOf("5", "\"4\"", "4.0", "0")) {
            val text = """{"v":$version,"days":{"2026-10-24":[]},"labels":{}}"""
            assertNull(version, AgendaWidgetPlan.snapshotOf(text))
        }
    }

    @Test
    fun theDayIsTheOneTheLocalClockShows() {
        // 22:30 UTC on Saturday is 00:30 on Sunday in Berlin.
        assertEquals("2026-10-25", AgendaWidgetPlan.dayKey(utc(2026, 10, 24, 22, 30), berlin))
        assertEquals("2026-10-24", AgendaWidgetPlan.dayKey(utc(2026, 10, 24, 21, 30), berlin))
        assertEquals("2026-10-24", AgendaWidgetPlan.dayKey(utc(2026, 10, 24, 22, 30), TimeZone.getTimeZone("UTC")))
    }

    @Test
    fun theDayStartsAtLocalMidnight() {
        assertEquals(utc(2026, 10, 23, 22), AgendaWidgetPlan.midnightOf(utc(2026, 10, 24, 9), berlin))
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
