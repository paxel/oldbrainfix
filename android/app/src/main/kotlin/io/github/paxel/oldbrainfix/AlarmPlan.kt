package io.github.paxel.oldbrainfix

import java.util.TimeZone

/**
 * One alarm the app handed over, or a snooze tapped on a notification.
 * An Occurrence alarm keeps its wall-clock moment in [local], so it is
 * found again in whatever zone the phone is in when it is armed; a
 * snooze waits a fixed time and keeps its instant in [at].
 */
data class Alarm(
    val id: String,
    val routine: String,
    val occurrence: String,
    /** [year, month, day, hour, minute] on the wall clock, or null. */
    val local: List<Int>?,
    /** Epoch millis, or null. */
    val at: Long?,
    val title: String,
    val body: String,
)

/** Done or a snooze tapped on a notification, kept until the app applied it. */
data class Action(
    /** Counts up; the app acknowledges everything up to one it applied. */
    val seq: Long,
    /** "done" or "snooze". */
    val kind: String,
    val routine: String,
    val occurrence: String,
    /** When it was tapped, epoch millis. */
    val at: Long,
    /** When a snooze rings again, epoch millis; 0 for Done. */
    val until: Long,
    val title: String,
    val body: String,
)

/**
 * The scheduling rules, free of Android, so the JVM tests cover them:
 * where a wall-clock moment lies, what the plan is with the taps not yet
 * applied, what is due when an alarm fires, and which alarm comes next.
 */
object AlarmPlan {
    /** An alarm that came due this long before it was armed still fires. */
    const val GRACE_MS = 60_000L

    /**
     * The instant [local] shows on [zone]'s clock. A time the clock shows
     * twice fires once, at the first; a time it skips fires at the next
     * valid instant, the moment the clock jumps past it. The same rule
     * as the core's `instantOf`.
     */
    fun instantOf(local: List<Int>, zone: TimeZone): Long {
        val target = utcMillis(local)
        val day = 24 * 60 * 60 * 1000L
        val before = zone.getOffset(target - day).toLong()
        val after = zone.getOffset(target + day).toLong()
        val hits = listOf(target - before, target - after)
            .distinct()
            .filter { it + zone.getOffset(it) == target }
        if (hits.isNotEmpty()) return hits.min()
        // Skipped: find where the clock jumps.
        var lo = target - after
        var hi = target - before
        if (lo >= hi) return hi
        while (hi - lo > 1) {
            val mid = lo + (hi - lo) / 2
            if (mid + zone.getOffset(mid) < target) lo = mid else hi = mid
        }
        return hi
    }

    /** When [alarm] fires in [zone]. */
    fun instantOf(alarm: Alarm, zone: TimeZone): Long =
        alarm.local?.let { instantOf(it, zone) } ?: alarm.at ?: Long.MAX_VALUE

    /**
     * The plan with the taps the app has not applied yet: what was
     * tapped Done is gone, snoozed Occurrences ring again at their snooze.
     */
    fun effective(plan: List<Alarm>, queued: List<Action>): List<Alarm> {
        val done = queued.filter { it.kind == "done" }.map { it.routine to it.occurrence }.toSet()
        val snoozes = queued.filter { it.kind == "snooze" }.associateBy { snoozeId(it.routine, it.occurrence) }
        val kept = plan.filter { (it.routine to it.occurrence) !in done && it.id !in snoozes }
        val added = snoozes.values
            .filter { (it.routine to it.occurrence) !in done }
            .map {
                Alarm(
                    id = snoozeId(it.routine, it.occurrence),
                    routine = it.routine,
                    occurrence = it.occurrence,
                    local = null,
                    at = it.until,
                    title = it.title,
                    body = it.body,
                )
            }
        return kept + added
    }

    /**
     * When the next alarm after [lastFired] fires: the soonest one later
     * than both that and [now] less [GRACE_MS], or null when none is left.
     * Nothing older is fired after a reboot. A [lastFired] ahead of now
     * means the clock was set back, and counts for nothing.
     */
    fun next(alarms: List<Alarm>, zone: TimeZone, lastFired: Long, now: Long): Long? {
        val after = maxOf(if (lastFired > now) 0 else lastFired, now - GRACE_MS)
        return alarms.map { instantOf(it, zone) }.filter { it > after }.minOrNull()
    }

    /**
     * What to show when the alarm armed for [armed] fires at [now]: every
     * alarm from then to now, so a late inexact alarm brings all it held
     * up, but nothing that already fired.
     */
    fun due(alarms: List<Alarm>, zone: TimeZone, armed: Long, lastFired: Long, now: Long): List<Alarm> {
        val fired = if (lastFired > now) 0 else lastFired
        return alarms.filter {
            val at = instantOf(it, zone)
            at in armed..now && at > fired
        }
    }

    /** The notification of an Occurrence; its snooze shows in the same one. */
    fun notificationId(routine: String, occurrence: String): Int = "$routine@$occurrence".hashCode()

    /** The id the core gives a snooze of [occurrence] of [routine]. */
    fun snoozeId(routine: String, occurrence: String): String = "snooze:$routine@$occurrence"

    private fun utcMillis(local: List<Int>): Long {
        val calendar = java.util.Calendar.getInstance(TimeZone.getTimeZone("UTC"))
        calendar.clear()
        calendar.set(local[0], local[1] - 1, local[2], local[3], local[4], 0)
        return calendar.timeInMillis
    }
}
