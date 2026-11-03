package io.github.paxel.oldbrainfix

import org.json.JSONArray
import org.json.JSONObject
import java.util.Calendar
import java.util.Locale
import java.util.TimeZone

/**
 * One row of the Agenda widget, in the words the app handed over
 * (lib/src/agenda_widget.dart), or one the widget makes itself: the
 * Unsorted count, the heading, an appointment of the calendar or a
 * message. A group of the Agenda is its heading's row with its entries'
 * rows after it, indented (#209).
 */
data class WidgetRow(
    val kind: String,
    val title: String,
    /** Its time, or "All day"; empty for a row without one. */
    val time: String = "",
    val done: Boolean = false,
    /** Whether this Device may check it here, where it has a box. */
    val checks: Boolean = false,
    /** How far it is indented: 1 under its heading, 2 for a Line under another Line. */
    val indent: Int = 0,
    /** The Routine and Occurrence an Occurrence, a Missed one or a Line is checked for. */
    val routine: String = "",
    val occurrence: String = "",
    /** The Line of a Checklist or a Shopping List its box checks. */
    val line: String = "",
    /** The Frog its box checks off. */
    val frog: String = "",
    /** The Tally whose entry form a [PICK] opens. */
    val note: String = "",
    /** Where it sorts among the appointments ([ALL_DAY], [LISTS], [TIMED], [FROGS]). */
    val block: String = "",
    /** The minute of the day of a [TIMED] row. */
    val at: Int? = null,
    /** An appointment's calendar colour, ARGB; 0 for none. */
    val colour: Int = 0,
    /**
     * Whether it is on a day after the one the widget shows as today
     * (#194): its box checks nothing and opens the app, whatever
     * [checks] says, as only today's and the Missed rows tick there.
     */
    val ahead: Boolean = false,
) {
    /** Whether it shows a box. */
    val box: Boolean
        get() = when (kind) {
            OCCURRENCE, SESSION, PICK, MISSED, LINE, CHECKLIST_LINE, FROG -> true
            SHOPPING -> line.isNotEmpty()
            else -> false
        }

    /**
     * Whether it is a Shopping List's or a Checklist's heading, which has
     * no box: its words start where those of a heading with a box do.
     */
    val heading: Boolean
        get() = kind == CHECKLIST || (kind == SHOPPING && line.isEmpty())

    /**
     * What its box queues for the app, as the Agenda's box does; null
     * when it has none, is checked already, this Device may not check
     * it or it is on a coming day ([ahead]), and a tap opens the app
     * instead. A checked one is unchecked in the app. A Session's and a
     * pick's box queue nothing.
     */
    val tap: WidgetTap?
        get() = if (!box || done || !checks || ahead) {
            null
        } else {
            when (kind) {
                OCCURRENCE, MISSED -> WidgetTap("done", routine, occurrence, "")
                LINE -> WidgetTap(EventQueue.KIND_LINE, routine, occurrence, line)
                CHECKLIST_LINE -> WidgetTap(EventQueue.KIND_CHECKLIST, "", "", line)
                FROG -> WidgetTap(EventQueue.KIND_FROG, frog, "", "")
                SHOPPING -> WidgetTap(EventQueue.KIND_SHOPPING, "", "", line)
                else -> null
            }
        }

    /**
     * What a tap on its box does, as on the Agenda (#208, #209): a
     * Session's only shows Done and does nothing, a pick's opens its
     * entry form as its words do, any other queues its [tap]; where it
     * has none, it opens the app.
     */
    val onBox: AgendaTap
        get() = when (kind) {
            SESSION -> AgendaTap.None
            PICK -> onRow
            else -> tap?.let {
                AgendaTap.Check(it.kind, Alarm("", it.routine, it.occurrence, null, null, title, time), it.line)
            } ?: AgendaTap.Open
        }

    /**
     * What a tap on the rest of it does: a pick of today this Device may
     * record for opens its entry form (#209), any other row the app.
     */
    val onRow: AgendaTap
        get() = if (kind == PICK && checks && !ahead && routine.isNotEmpty() && occurrence.isNotEmpty() && note.isNotEmpty()) {
            AgendaTap.Pick(routine, occurrence, note)
        } else {
            AgendaTap.Open
        }

    companion object {
        const val OCCURRENCE = "occurrence"
        const val SESSION = "session"
        const val PICK = "pick"
        const val LINE = "line"
        const val SHOPPING = "shopping"
        const val CHECKLIST = "checklist"
        const val CHECKLIST_LINE = "checklistLine"
        const val FROG = "frog"
        const val MISSED = "missed"
        const val HEADING = "heading"
        const val TEXT = "text"

        /** The rows the widget makes itself. */
        const val UNSORTED = "unsorted"
        const val APPOINTMENT = "appointment"
        const val MESSAGE = "message"

        /** The kinds of row a snapshot of version 2 holds. */
        val HANDED = setOf(OCCURRENCE, LINE, SHOPPING, CHECKLIST, FROG, MISSED, HEADING, TEXT)

        /**
         * The kinds of row a snapshot of version 3 holds (#209): a
         * Session's heading and its picks, and a Checklist's Lines too.
         */
        val HANDED_GROUPS = HANDED + setOf(SESSION, PICK, CHECKLIST_LINE)

        const val ALL_DAY = "allDay"
        const val LISTS = "lists"
        const val TIMED = "timed"
        const val FROGS = "frogs"
    }
}

/** What a box checked in the widget queues ([EventQueue]): its kind, Routine or Frog, Occurrence and Line. */
data class WidgetTap(val kind: String, val routine: String, val occurrence: String, val line: String)

/**
 * What the app handed over for the Agenda widget: the Missed Summary,
 * the rows of the days the Agenda shows by `YYYY-MM-DD` with each day's
 * heading, the Unsorted count, the calendars whose appointments show,
 * and the texts in the reader's language.
 */
data class AgendaSnapshot(
    val days: Map<String, List<WidgetRow>>,
    val missed: List<WidgetRow>,
    val unsorted: Int,
    val calendars: List<String>,
    val today: String,
    val empty: String,
    val unsortedText: String,
    val done: String,
    val allDay: String,
    val noTitle: String,
    /** "from {time}" and "until {time}"; [AgendaWidgetPlan.appointmentRows] puts in the time. */
    val since: String,
    val until: String,
    /**
     * Each day's own heading, its weekday and date, since version 4
     * (#194); none in an older one, which shows today alone.
     */
    val headings: Map<String, String> = emptyMap(),
    /** The heading of the day after today, "Tomorrow". */
    val tomorrow: String = "",
    /** What a coming day with nothing due says, "Nothing planned". */
    val planned: String = "",
    /**
     * How much bigger the widget's text shows: [AgendaWidgetPlan.FAT_FINGERS]
     * while the Device has "Fat fingers" on (#224), else 1.
     */
    val textScale: Float = 1f,
)

/**
 * An appointment of a calendar, read by the widget as it shows and
 * never kept: an all-day one runs from midnight UTC to midnight UTC.
 */
data class CalendarEntry(
    val title: String,
    val begin: Long,
    val end: Long,
    val allDay: Boolean,
    val colour: Int,
)

/**
 * What the widget shows now: [rows] in Agenda order. [stale] when the
 * snapshot does not reach today, as when the app was not opened for two
 * days: the widget then asks to open it rather than claim nothing is
 * due.
 */
data class AgendaView(val rows: List<WidgetRow>, val stale: Boolean)

/**
 * The Agenda widget's rules, free of Android, so the JVM tests cover
 * them: which day of the snapshot shows, what was checked in the widget
 * or on a notification and not yet applied by the app, where the
 * calendar's appointments go, and when the day ends.
 */
object AgendaWidgetPlan {
    private const val DAY_MS = 86_400_000L

    /** A box's side in the widget, in dp. */
    const val BOX_DP = 40

    /** How much further a Line under another Line moves in, in dp. */
    const val STEP_DP = 24

    /**
     * How much bigger "Fat fingers" shows the text (#224), as in the app;
     * the boxes keep their size, which a widget's layout sets.
     */
    const val FAT_FINGERS = 1.3f

    /**
     * How far a row of [indent] moves in, in dp: an entry of any group
     * by a box's width, so its box stands where its heading's words
     * begin, as on the Agenda (#208), and a Line under another one step
     * further.
     */
    fun indentDp(indent: Int): Int = if (indent <= 0) 0 else BOX_DP + STEP_DP * (indent - 1)

    /**
     * The snapshot the app handed over as [text] ([Stored]): null when
     * there is none, it does not read or it is of a version after 4,
     * which makes the widget ask to open the app. Version 1, absent `v`,
     * holds Occurrences only, as 0.2.0 wrote it; version 2 every kind of
     * row the Agenda shows; version 3 the Agenda's groups (#209);
     * version 4 the days the Agenda shows, each with its heading (#194).
     * A row that does not read, or of a kind not known to its version,
     * is left out, and so is a heading that is no text.
     */
    fun snapshotOf(text: String?): AgendaSnapshot? {
        val json = text?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return null
        return when {
            Stored.current(json) -> runCatching { first(json) }.getOrNull()
            isVersion(json, 2) -> runCatching { second(json, WidgetRow.HANDED) }.getOrNull()
            isVersion(json, 3) -> runCatching { second(json, WidgetRow.HANDED_GROUPS) }.getOrNull()
            isVersion(json, 4) -> runCatching {
                val labels = json.getJSONObject("labels")
                val headings = json.optJSONObject("headings") ?: JSONObject()
                second(json, WidgetRow.HANDED_GROUPS).copy(
                    headings = headings.keys().asSequence()
                        .mapNotNull { day -> (headings.opt(day) as? String)?.let { day to it } }
                        .toMap(),
                    tomorrow = labels.optString("tomorrow"),
                    planned = labels.optString("planned"),
                    textScale = if (json.optBoolean("fatFingers")) FAT_FINGERS else 1f,
                )
            }.getOrNull()
            else -> null
        }
    }

    /** Whether [json] is of [version]: that whole number, as ADR 0034 reads 1. */
    private fun isVersion(json: JSONObject, version: Int): Boolean {
        val v = json.opt("v")
        return (v is Int && v == version) || (v is Long && v == version.toLong())
    }

    /** A snapshot of version 1: every row an Occurrence. */
    private fun first(json: JSONObject): AgendaSnapshot {
        val labels = json.getJSONObject("labels")
        return AgendaSnapshot(
            days = daysOf(json.getJSONObject("days")) { row ->
                WidgetRow(
                    kind = WidgetRow.OCCURRENCE,
                    title = row.optString("title"),
                    time = row.optString("time"),
                    done = row.optBoolean("done"),
                    checks = true,
                    routine = row.getString("routine"),
                    occurrence = row.getString("occurrence"),
                )
            },
            missed = emptyList(),
            unsorted = json.optInt("unsorted"),
            calendars = emptyList(),
            today = labels.optString("today"),
            empty = labels.optString("empty"),
            unsortedText = labels.optString("unsorted"),
            done = labels.optString("done"),
            allDay = "",
            noTitle = "",
            since = "",
            until = "",
        )
    }

    /** A snapshot of version 2, 3 or 4, whose rows are of [kinds]. */
    private fun second(json: JSONObject, kinds: Set<String>): AgendaSnapshot {
        val labels = json.getJSONObject("labels")
        val calendars = json.optJSONArray("calendars") ?: JSONArray()
        val read = { row: JSONObject -> rowOf(row, kinds) }
        return AgendaSnapshot(
            days = daysOf(json.getJSONObject("days"), read),
            missed = Stored.elementsOf(json.optJSONArray("missed") ?: JSONArray(), read),
            unsorted = json.optInt("unsorted"),
            calendars = (0 until calendars.length()).mapNotNull { calendars.opt(it) as? String },
            today = labels.optString("today"),
            empty = labels.optString("empty"),
            unsortedText = labels.optString("unsorted"),
            done = labels.optString("done"),
            allDay = labels.optString("allDay"),
            noTitle = labels.optString("noTitle"),
            since = labels.optString("since"),
            until = labels.optString("until"),
        )
    }

    private fun daysOf(days: JSONObject, read: (JSONObject) -> WidgetRow): Map<String, List<WidgetRow>> =
        days.keys().asSequence().associateWith { day ->
            Stored.elementsOf(days.optJSONArray(day) ?: JSONArray(), read)
        }

    /** A row of version 2, 3 or 4; failing on one not of [kinds]. */
    private fun rowOf(row: JSONObject, kinds: Set<String>): WidgetRow {
        val kind = row.getString("kind")
        require(kind in kinds) { kind }
        return WidgetRow(
            kind = kind,
            title = row.getString("title"),
            time = row.optString("time"),
            done = row.optBoolean("done"),
            checks = row.optBoolean("checks"),
            indent = row.optInt("indent"),
            routine = row.optString("routine"),
            occurrence = row.optString("occurrence"),
            line = row.optString("line"),
            frog = row.optString("frog"),
            note = row.optString("note"),
            block = row.optString("block"),
            at = if (row.has("at")) row.getInt("at") else null,
        )
    }

    /** The day [now] shows on [zone]'s clock, `YYYY-MM-DD`. */
    fun dayKey(now: Long, zone: TimeZone): String {
        val calendar = Calendar.getInstance(zone, Locale.ROOT).apply { timeInMillis = now }
        return String.format(
            Locale.ROOT,
            "%04d-%02d-%02d",
            calendar.get(Calendar.YEAR),
            calendar.get(Calendar.MONTH) + 1,
            calendar.get(Calendar.DAY_OF_MONTH),
        )
    }

    /** The next local midnight after [now] on [zone]'s clock. */
    fun nextMidnight(now: Long, zone: TimeZone): Long {
        val calendar = Calendar.getInstance(zone, Locale.ROOT).apply { timeInMillis = now }
        calendar.add(Calendar.DAY_OF_MONTH, 1)
        return AlarmPlan.instantOf(
            listOf(calendar.get(Calendar.YEAR), calendar.get(Calendar.MONTH) + 1, calendar.get(Calendar.DAY_OF_MONTH), 0, 0),
            zone,
        )
    }

    /** The local midnight that starts the day of [now] on [zone]'s clock. */
    fun midnightOf(now: Long, zone: TimeZone): Long {
        val calendar = Calendar.getInstance(zone, Locale.ROOT).apply { timeInMillis = now }
        return AlarmPlan.instantOf(
            listOf(calendar.get(Calendar.YEAR), calendar.get(Calendar.MONTH) + 1, calendar.get(Calendar.DAY_OF_MONTH), 0, 0),
            zone,
        )
    }

    /**
     * The rows of [entries] on [day] as the Agenda shows them (#116):
     * all-day ones first, then by time, each by title after that; read
     * only, marked with their calendar's colour.
     */
    fun appointmentRows(snapshot: AgendaSnapshot, entries: List<CalendarEntry>, day: String, zone: TimeZone): List<WidgetRow> {
        val utc = TimeZone.getTimeZone("UTC")
        val pieces = entries.mapNotNull { entry ->
            if (entry.allDay) {
                // Midnight UTC to midnight UTC, the end not counted.
                val first = dayKey(entry.begin, utc)
                val end = dayKey(entry.end, utc)
                val last = if (end > first) dayKey(entry.end - DAY_MS, utc) else first
                if (day < first || day > last) return@mapNotNull null
                return@mapNotNull Piece(entry, null, null)
            }
            val first = dayKey(entry.begin, zone)
            val endDay = dayKey(entry.end, zone)
            val endMinute = minuteOf(entry.end, zone)
            // An end at midnight closes the day before.
            val last = if (endMinute == 0 && endDay > first) dayKey(entry.end - 60_000, zone) else endDay
            if (day < first || day > last) return@mapNotNull null
            Piece(
                entry,
                from = if (day == first) minuteOf(entry.begin, zone) else null,
                until = if (day == endDay) endMinute else null,
            )
        }.sortedWith(compareBy<Piece>({ it.at ?: -1 }, { it.entry.title }))
        return pieces.map { piece ->
            WidgetRow(
                kind = WidgetRow.APPOINTMENT,
                title = piece.entry.title.trim().ifEmpty { snapshot.noTitle },
                time = when {
                    piece.allDay -> snapshot.allDay
                    piece.from != null && piece.until != null && piece.from == piece.until -> clock(piece.from)
                    piece.from != null && piece.until != null -> "${clock(piece.from)}–${clock(piece.until)}"
                    piece.from != null -> snapshot.since.replace("{time}", clock(piece.from))
                    piece.until != null -> snapshot.until.replace("{time}", clock(piece.until))
                    else -> snapshot.allDay
                },
                block = if (piece.allDay) WidgetRow.ALL_DAY else WidgetRow.TIMED,
                at = piece.at,
                colour = piece.entry.colour,
            )
        }
    }

    /** The part of [entry] on one day: from and until which minute, null where it runs past. */
    private data class Piece(val entry: CalendarEntry, val from: Int?, val until: Int?) {
        val allDay: Boolean get() = from == null && until == null

        /** Where it sorts among the timed rows; null among the all-day ones. */
        val at: Int? get() = if (allDay) null else from ?: 0
    }

    private fun minuteOf(at: Long, zone: TimeZone): Int {
        val calendar = Calendar.getInstance(zone, Locale.ROOT).apply { timeInMillis = at }
        return calendar.get(Calendar.HOUR_OF_DAY) * 60 + calendar.get(Calendar.MINUTE)
    }

    private fun clock(minute: Int): String = String.format(Locale.ROOT, "%02d:%02d", minute / 60, minute % 60)

    /**
     * The day's [rows] with the [appointments] put in as the Agenda puts
     * them: the all-day ones after the all-day Occurrences, the timed
     * ones among the timed Occurrences by time, an Occurrence, with what
     * shows under it, before an appointment at the same time.
     */
    fun merged(rows: List<WidgetRow>, appointments: List<WidgetRow>): List<WidgetRow> {
        val out = rows.toMutableList()
        val allDay = appointments.filter { it.block == WidgetRow.ALL_DAY }
        val afterAllDay = out.indexOfFirst { it.block != WidgetRow.ALL_DAY }.let { if (it < 0) out.size else it }
        out.addAll(afterAllDay, allDay)
        for (piece in appointments.filter { it.block == WidgetRow.TIMED }) {
            val at = piece.at ?: 0
            val before = out.indexOfFirst {
                it.block == WidgetRow.FROGS || (it.block == WidgetRow.TIMED && (it.at ?: 0) > at)
            }
            out.add(if (before < 0) out.size else before, piece)
        }
        return out
    }

    /**
     * The days the widget shows on [today] from [snapshot], in order:
     * today, then each later day of it with a heading (#194), so a
     * snapshot of version 3 or before shows today alone; none when it
     * does not reach today.
     */
    fun shownDays(snapshot: AgendaSnapshot, today: String): List<String> {
        if (today !in snapshot.days) return emptyList()
        return listOf(today) + snapshot.days.keys.filter { it > today && it in snapshot.headings }.sorted()
    }

    /**
     * What the widget shows on [today] from [snapshot], in Agenda order:
     * the Missed Summary, the Unsorted count, then the day under its
     * heading with its [appointments] put in, "Nothing due today." where
     * nothing is, then the Frogs; then each coming day of the Agenda
     * under its heading, "Tomorrow" first, with its appointments,
     * "Nothing planned" where nothing is (#194). Only today's and the
     * Missed rows tick: a coming day's row is [WidgetRow.ahead]. What was
     * checked in the widget or on a notification and is still [pending]
     * for the app shows checked, as the Agenda will show it: the last
     * Line of an Occurrence checks it off, and a Frog checked off leaves
     * the other one out. Without a snapshot that reaches today it shows
     * [openApp].
     */
    fun view(
        snapshot: AgendaSnapshot?,
        today: String,
        pending: List<Action>,
        appointments: Map<String, List<WidgetRow>>,
        openApp: String,
    ): AgendaView {
        if (snapshot == null) return AgendaView(listOf(message(openApp)), stale = true)
        val top = checked(snapshot.missed, pending) +
            if (snapshot.unsorted > 0) listOf(WidgetRow(WidgetRow.UNSORTED, snapshot.unsortedText)) else emptyList()
        val day = snapshot.days[today] ?: return AgendaView(top + message(openApp), stale = true)
        val rows = merged(checked(day, pending), appointments[today].orEmpty())
        val due = rows.filter { it.block != WidgetRow.FROGS }
        val frogs = rows.filter { it.block == WidgetRow.FROGS }
        val heading = WidgetRow(WidgetRow.HEADING, snapshot.today)
        val coming = shownDays(snapshot, today).drop(1).flatMapIndexed { i, key ->
            val later = merged(snapshot.days[key].orEmpty(), appointments[key].orEmpty()).map { it.copy(ahead = true) }
            listOf(WidgetRow(WidgetRow.HEADING, if (i == 0) snapshot.tomorrow else snapshot.headings[key].orEmpty())) +
                later.ifEmpty { listOf(message(snapshot.planned).copy(ahead = true)) }
        }
        return AgendaView(
            top + heading + due.ifEmpty { listOf(message(snapshot.empty)) } + frogs + coming,
            stale = false,
        )
    }

    private fun message(text: String) = WidgetRow(WidgetRow.MESSAGE, text)

    /** [rows] with what is still [pending] for the app shown checked. */
    private fun checked(rows: List<WidgetRow>, pending: List<Action>): List<WidgetRow> {
        val done = pending.filter { it.kind == "done" }.map { it.routine to it.occurrence }.toSet()
        val lines = pending.filter { it.kind == EventQueue.KIND_LINE }.map { Triple(it.routine, it.occurrence, it.line) }.toSet()
        val frogs = pending.filter { it.kind == EventQueue.KIND_FROG }.map { it.routine }.toSet()
        val shopping = pending.filter { it.kind == EventQueue.KIND_SHOPPING }.map { it.line }.toSet()
        val checklist = pending.filter { it.kind == EventQueue.KIND_CHECKLIST }.map { it.line }.toSet()
        val tapped = rows.map { row ->
            val now = when (row.kind) {
                WidgetRow.OCCURRENCE, WidgetRow.SESSION, WidgetRow.MISSED -> (row.routine to row.occurrence) in done
                WidgetRow.LINE -> Triple(row.routine, row.occurrence, row.line) in lines
                WidgetRow.FROG -> row.frog in frogs
                WidgetRow.SHOPPING -> row.line.isNotEmpty() && row.line in shopping
                WidgetRow.CHECKLIST_LINE -> row.line.isNotEmpty() && row.line in checklist
                else -> false
            }
            if (now && !row.done) row.copy(done = true) else row
        }
        // Every Line checked checks its Occurrence off.
        val open = tapped.filter { it.kind == WidgetRow.LINE && !it.done }.map { it.routine to it.occurrence }.toSet()
        val listed = tapped.filter { it.kind == WidgetRow.LINE }.map { it.routine to it.occurrence }.toSet()
        val complete = listed - open
        val eaten = tapped.any { it.kind == WidgetRow.FROG && it.done }
        return tapped.mapNotNull { row ->
            when {
                row.kind == WidgetRow.OCCURRENCE && !row.done && (row.routine to row.occurrence) in complete ->
                    row.copy(done = true)
                // Once a Frog is eaten, only the eaten ones show.
                row.kind == WidgetRow.FROG && eaten && !row.done -> null
                else -> row
            }
        }
    }
}
