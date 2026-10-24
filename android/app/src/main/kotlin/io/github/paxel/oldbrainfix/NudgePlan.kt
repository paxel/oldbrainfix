package io.github.paxel.oldbrainfix

import java.util.Calendar
import java.util.TimeZone

/**
 * The one Nudge over all Unsorted Notes, as the app handed it over: each
 * Note's own Nudge time with its capture in [anchors], the latest snooze,
 * how often it repeats while ignored, the Quiet Hours, and [since], when
 * it last fired by the rule as the app planned it.
 */
data class Nudge(
    /** (own Nudge time, capture) of each Unsorted Note, epoch millis. */
    val anchors: List<Pair<Long, Long>>,
    /** When the snooze was tapped and when it ends, epoch millis, or null. */
    val snoozeAt: Long?,
    val snoozeUntil: Long?,
    val repeatMs: Long,
    /** [startHour, startMinute, endHour, endMinute], or null while off. */
    val quiet: List<Int>?,
    /** Epoch millis, or null when it has not fired yet. */
    val since: Long?,
    val title: String,
    val body: String,
)

/**
 * The Nudge's rule, free of Android, the same as the core's
 * `PlannedNudge.firesAfter`: it fires at the earliest own time of the
 * Unsorted Notes, covering them all, and again every [Nudge.repeatMs]
 * while ignored. A Note captured after it last fired brings it forward
 * to that Note's own time. A snooze holds back the Notes there were when
 * it was tapped until it ends. A firing in the Quiet Hours waits for
 * their end, and the repeats count from there.
 */
object NudgePlan {
    /** When a Nudge due at [at] fires: then, or at the end of the Quiet Hours [quiet] it lies in. */
    fun defer(quiet: List<Int>, zone: TimeZone, at: Long): Long {
        val start = quiet[0] * 60 + quiet[1]
        val end = quiet[2] * 60 + quiet[3]
        val wall = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            timeInMillis = at + zone.getOffset(at)
        }
        val minute = wall.get(Calendar.HOUR_OF_DAY) * 60 + wall.get(Calendar.MINUTE)
        val inside = if (start <= end) minute in start until end else minute >= start || minute < end
        if (!inside) return at
        // In a window over midnight, before midnight it ends tomorrow.
        if (start > end && minute >= start) wall.add(Calendar.DAY_OF_MONTH, 1)
        val ends = AlarmPlan.instantOf(
            listOf(wall.get(Calendar.YEAR), wall.get(Calendar.MONTH) + 1, wall.get(Calendar.DAY_OF_MONTH), quiet[2], quiet[3]),
            zone,
        )
        return maxOf(ends, at)
    }

    /** When it fires next after it last fired at [fired] (null: never). */
    fun nextFiring(nudge: Nudge, zone: TimeZone, fired: Long?): Long? {
        fun ownAfter(from: Long?): Long? = nudge.anchors
            .filter { (due, captured) -> (from == null || captured > from) && (fired == null || due > fired) }
            .minOfOrNull { it.first }
        var raw = listOfNotNull(fired?.plus(nudge.repeatMs), ownAfter(fired)).minOrNull()
        val tapped = nudge.snoozeAt
        val until = nudge.snoozeUntil
        if (tapped != null && until != null) {
            if (fired == null || fired < tapped) {
                // From the tap to its end only Notes captured after it fire.
                if (raw != null && raw >= tapped) raw = listOfNotNull(until, ownAfter(tapped)).min()
            } else if (fired < until) {
                raw = listOfNotNull(raw, until).min()
            }
        }
        if (raw == null) return null
        return nudge.quiet?.let { defer(it, zone, raw) } ?: raw
    }

    /** When it fires first after [after]. */
    fun firesAfter(nudge: Nudge, zone: TimeZone, after: Long): Long {
        var last = nudge.since?.takeIf { after >= it }
        while (true) {
            val next = nextFiring(nudge, zone, last) ?: (after + nudge.repeatMs)
            if (next > after) return next
            last = next
        }
    }

    /** The Nudge with the latest snooze tapped on its notification and not applied by the app yet. */
    fun effective(nudge: Nudge, queued: List<Action>): Nudge {
        val tap = queued.filter { it.kind == KIND_SNOOZE }.maxByOrNull { it.seq } ?: return nudge
        return nudge.copy(snoozeAt = tap.at, snoozeUntil = tap.until)
    }

    /** When it fires next, by the same rule as [AlarmPlan.next]. */
    fun next(nudge: Nudge, zone: TimeZone, lastFired: Long, now: Long): Long {
        val after = maxOf(if (lastFired > now) 0 else lastFired, now - AlarmPlan.GRACE_MS)
        return firesAfter(nudge, zone, after)
    }

    /** Whether it is due when the alarm armed for [armed] fires at [now], as [AlarmPlan.due] decides. */
    fun due(nudge: Nudge, zone: TimeZone, armed: Long, lastFired: Long, now: Long): Boolean {
        val fired = if (lastFired > now) 0 else lastFired
        return firesAfter(nudge, zone, maxOf(armed - 1, fired)) <= now
    }

    /** The kind of a Nudge snooze in the [EventQueue]. */
    const val KIND_SNOOZE = "nudgeSnooze"
}
