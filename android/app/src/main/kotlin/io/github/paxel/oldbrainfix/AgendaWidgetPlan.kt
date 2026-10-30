package io.github.paxel.oldbrainfix

import org.json.JSONArray
import java.util.Calendar
import java.util.Locale
import java.util.TimeZone

/** One Occurrence in the Agenda widget, as the app handed it over. */
data class WidgetRow(
    val routine: String,
    val occurrence: String,
    val title: String,
    /** Its time, or "All day", in the reader's language. */
    val time: String,
    val done: Boolean,
)

/**
 * What the app handed over for the Agenda widget (lib/src/agenda_widget.dart):
 * the rows of today and tomorrow by `YYYY-MM-DD`, the Unsorted count, and
 * the texts in the reader's language.
 */
data class AgendaSnapshot(
    val days: Map<String, List<WidgetRow>>,
    val unsorted: Int,
    val today: String,
    val empty: String,
    val unsortedText: String,
    /** "and {count} more"; [AgendaWidgetPlan.moreText] puts in the number. */
    val more: String,
    val done: String,
)

/**
 * What the widget shows now: [rows] in Agenda order and how many [more]
 * did not fit. [stale] when the snapshot does not reach today, as when
 * the app was not opened for two days: the widget then asks to open it
 * rather than claim nothing is due.
 */
data class AgendaView(val rows: List<WidgetRow>, val more: Int, val stale: Boolean)

/**
 * The Agenda widget's rules, free of Android, so the JVM tests cover
 * them: which day of the snapshot shows, Done tapped and not yet applied
 * by the app, what fits, and when the day ends.
 */
object AgendaWidgetPlan {
    /** A row's height in dp, and what the heading and the Unsorted row take. */
    private const val ROW_DP = 44
    private const val CHROME_DP = 88

    /** Rows shown when the launcher does not tell the widget's height. */
    const val DEFAULT_ROOM = 3

    /**
     * The snapshot the app handed over as [text] ([Stored]): null when
     * there is none, it does not read or it is of a later version, which
     * makes the widget ask to open the app. A row that does not read is
     * left out.
     */
    fun snapshotOf(text: String?): AgendaSnapshot? = Stored.objectOf(text) { json ->
        val days = json.getJSONObject("days")
        val labels = json.getJSONObject("labels")
        AgendaSnapshot(
            days = days.keys().asSequence().associateWith { day ->
                val rows = days.optJSONArray(day) ?: JSONArray()
                Stored.elementsOf(rows) { row ->
                    WidgetRow(
                        routine = row.getString("routine"),
                        occurrence = row.getString("occurrence"),
                        title = row.optString("title"),
                        time = row.optString("time"),
                        done = row.optBoolean("done"),
                    )
                }
            },
            unsorted = json.optInt("unsorted"),
            today = labels.optString("today"),
            empty = labels.optString("empty"),
            unsortedText = labels.optString("unsorted"),
            more = labels.optString("more"),
            done = labels.optString("done"),
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

    /** How many lines fit in a widget [heightDp] high; at least one. */
    fun room(heightDp: Int): Int =
        if (heightDp <= 0) DEFAULT_ROOM else maxOf(1, (heightDp - CHROME_DP) / ROW_DP)

    /** "and {count} more" with [count] put in. */
    fun moreText(template: String, count: Int): String = template.replace("{count}", count.toString())

    /**
     * The rows of [today] from [snapshot], those checked off in the widget
     * or on a notification and still [pending] for the app shown Done.
     * When more than [room] lines are due, one line says how many more
     * there are, and Done rows give way first; the shown ones keep Agenda
     * order.
     */
    fun view(snapshot: AgendaSnapshot?, today: String, pending: List<Action>, room: Int): AgendaView {
        val day = snapshot?.days?.get(today) ?: return AgendaView(emptyList(), 0, stale = true)
        val tapped = pending.filter { it.kind == "done" }.map { it.routine to it.occurrence }.toSet()
        val rows = day.map { if (!it.done && (it.routine to it.occurrence) in tapped) it.copy(done = true) else it }
        if (rows.size <= room) return AgendaView(rows, 0, stale = false)
        val shown = maxOf(1, room - 1)
        val open = rows.indices.filter { !rows[it].done }
        val done = rows.indices.filter { rows[it].done }
        val picked = (open.take(shown) + done.take(shown - minOf(shown, open.size))).sorted()
        return AgendaView(picked.map { rows[it] }, rows.size - picked.size, stale = false)
    }
}
