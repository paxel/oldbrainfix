package io.github.paxel.oldbrainfix

import java.util.TimeZone

/**
 * One alarm the app handed over, or a snooze tapped on a notification.
 * An Occurrence alarm keeps its wall-clock moment in [local], so it is
 * found again in whatever zone the phone is in when it is armed; a
 * snooze waits a fixed time and keeps its instant in [at]. While
 * ignored it Re-nags every [renagMs] until [end], when its Occurrence
 * counts as Missed; with [escalate] set, it rings full screen once the
 * first ring and that many Re-nags went ignored.
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
    /** The Re-nag interval in millis; 0 rings once. */
    val renagMs: Long = 0,
    /** [year, month, day, hour, minute] on the wall clock when it stops, or null. */
    val end: List<Int>? = null,
    /** After how many ignored Re-nags it escalates; 0 never does. */
    val escalate: Int = 0,
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
     * A full-screen alarm rings at most this long; then its Occurrence
     * goes back to Re-nagging, and the next Re-nag rings again.
     */
    const val RING_MAX_MS = 5 * 60_000L

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

    /** When [alarm] fires first in [zone]. */
    fun instantOf(alarm: Alarm, zone: TimeZone): Long =
        alarm.local?.let { instantOf(it, zone) } ?: alarm.at ?: Long.MAX_VALUE

    /** When [alarm] stops in [zone]: its Occurrence counts as Missed. */
    fun endOf(alarm: Alarm, zone: TimeZone): Long =
        alarm.end?.let { instantOf(it, zone) } ?: Long.MAX_VALUE

    /**
     * When [alarm] fires first after [after]: at its instant or at a
     * Re-nag, never at or past its end; null when it has stopped. The
     * same rule as the core's `PlannedAlarm.firesAfter`.
     */
    fun firesAfter(alarm: Alarm, zone: TimeZone, after: Long): Long? {
        val first = instantOf(alarm, zone)
        val end = endOf(alarm, zone)
        val next = when {
            first > after -> first
            alarm.renagMs <= 0 -> return null
            else -> first + ((after - first) / alarm.renagMs + 1) * alarm.renagMs
        }
        return if (next < end) next else null
    }

    /**
     * The plan with the taps the app has not applied yet: what was
     * tapped Done is gone, snoozed Occurrences ring again at their snooze.
     */
    fun effective(plan: List<Alarm>, queued: List<Action>): List<Alarm> {
        val done = queued.filter { it.kind == "done" }.map { it.routine to it.occurrence }.toSet()
        val snoozes = queued.filter { it.kind == "snooze" }.associateBy { it.routine to it.occurrence }
        val kept = plan.filter { (it.routine to it.occurrence) !in done && (it.routine to it.occurrence) !in snoozes }
        val added = snoozes.values
            .filter { (it.routine to it.occurrence) !in done }
            .map { tap ->
                // Re-nags from the snooze on, as the Occurrence did.
                val was = plan.firstOrNull { it.routine == tap.routine && it.occurrence == tap.occurrence }
                Alarm(
                    id = snoozeId(tap.routine, tap.occurrence),
                    routine = tap.routine,
                    occurrence = tap.occurrence,
                    local = null,
                    at = tap.until,
                    title = tap.title,
                    body = tap.body,
                    renagMs = was?.renagMs ?: 0,
                    end = was?.end,
                    escalate = was?.escalate ?: 0,
                )
            }
        return kept + added
    }

    /**
     * Whether [alarm], firing at [now], rings as a full-screen alarm: its
     * first ring and [Alarm.escalate] Re-nags after it went ignored, and
     * it has not stopped. Counted by the Re-nags passed, so a late
     * inexact alarm escalates as well; a snooze counts from the snooze.
     * The same rule as the core's `PlannedAlarm.escalatesAt`.
     */
    fun escalates(alarm: Alarm, zone: TimeZone, now: Long): Boolean {
        if (alarm.escalate <= 0 || alarm.renagMs <= 0) return false
        val first = instantOf(alarm, zone)
        if (now < first || now >= endOf(alarm, zone)) return false
        return (now - first) / alarm.renagMs >= alarm.escalate + 1L
    }

    /**
     * When a full-screen alarm of [alarm] starting at [now] stops: after
     * [RING_MAX_MS], or halfway to its next Re-nag, so a short interval
     * still leaves quiet between rings, or when its Occurrence counts as
     * Missed, whichever comes first.
     */
    fun ringUntil(alarm: Alarm, zone: TimeZone, now: Long): Long {
        val half = if (alarm.renagMs > 0) alarm.renagMs / 2 else RING_MAX_MS
        return minOf(now + minOf(RING_MAX_MS, half), endOf(alarm, zone))
    }

    /**
     * Which of the alarms [due] at [now] rings full screen: the first that
     * escalates, none when [fullScreen] is not allowed. One rings at a
     * time; the others show as notifications.
     */
    fun toRing(due: List<Alarm>, zone: TimeZone, now: Long, fullScreen: Boolean): Alarm? =
        if (fullScreen) due.firstOrNull { escalates(it, zone, now) } else null

    /**
     * Whether the full-screen alarm of [routine]'s [occurrence] may go on
     * ringing under the plan [alarms] (with the taps applied): not once
     * it is Done, gone, or its Routine no longer escalates.
     */
    fun keepsRinging(alarms: List<Alarm>, routine: String, occurrence: String): Boolean =
        alarms.any { it.routine == routine && it.occurrence == occurrence && it.escalate > 0 }

    /**
     * When the next alarm after [lastFired] fires: the soonest one later
     * than both that and [now] less [GRACE_MS], or null when none is left.
     * Nothing older is fired after a reboot. A [lastFired] ahead of now
     * means the clock was set back, and counts for nothing.
     */
    fun next(alarms: List<Alarm>, zone: TimeZone, lastFired: Long, now: Long): Long? {
        val after = maxOf(if (lastFired > now) 0 else lastFired, now - GRACE_MS)
        return alarms.mapNotNull { firesAfter(it, zone, after) }.minOrNull()
    }

    /**
     * What to show when the alarm armed for [armed] fires at [now]: every
     * alarm due from then to now, so a late inexact alarm brings all it
     * held up, but nothing that already fired, and each Occurrence once
     * however many of its Re-nags came due meanwhile.
     */
    fun due(alarms: List<Alarm>, zone: TimeZone, armed: Long, lastFired: Long, now: Long): List<Alarm> {
        val fired = if (lastFired > now) 0 else lastFired
        val after = maxOf(armed - 1, fired)
        return alarms
            .filter { alarm -> firesAfter(alarm, zone, after)?.let { it <= now } ?: false }
            .distinctBy { it.routine to it.occurrence }
    }

    /**
     * How many Occurrences became Missed while the phone was off: Missed
     * by [now], with a ring or Re-nag due after the scheduler last ran
     * (at [lastFired] or [lastArmed]) that never came. None when it never
     * ran, or when the clock was set back past it. Their alarms do not
     * ring late; one Catch-up Notification tells about them instead.
     */
    fun catchUp(alarms: List<Alarm>, zone: TimeZone, lastFired: Long, lastArmed: Long, now: Long): Int {
        val since = maxOf(lastFired, lastArmed)
        if (since <= 0 || since > now) return 0
        return alarms
            .filter { endOf(it, zone) <= now && firesAfter(it, zone, since) != null }
            .distinctBy { it.routine to it.occurrence }
            .size
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
