package io.github.paxel.oldbrainfix

import android.app.AlarmManager
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import org.json.JSONArray
import org.json.JSONObject
import java.util.TimeZone

/**
 * Hands the alarm plan to AlarmManager (adapted from Chaos Alert). The
 * whole plan is kept, with Occurrences as wall-clock moments, and only
 * the next alarm is armed; when it fires, the one after it is armed.
 * So a reboot, a clock change or a new time zone re-arms it at the same
 * local times without the app running, and the plan reaches weeks ahead
 * without hitting Android's limit on pending alarms. An ignored alarm
 * Re-nags from the same one pending alarm, and escalates to a
 * full-screen alarm from there.
 *
 * Each stored value is read through [Stored]: one that is missing or not
 * readable, or an element of the plan that is, reads as none, so the
 * receivers never crash over it, and one of a later version is not used.
 */
object Scheduler {
    private const val PREFS = "oldbrainfix_alarms"
    private const val KEY_PLAN = "plan"
    private const val KEY_LABELS = "labels"
    private const val KEY_LAST_FIRED = "lastFired"
    private const val KEY_LAST_ARMED = "lastArmed"
    private const val KEY_NUDGE = "nudge"
    private const val KEY_CHANGES = "changes"

    const val EXTRA_ARMED = "oldbrainfix.armed"

    /**
     * Keeps [plan], the [nudge], the Changes Nudge [changes], the
     * notification [labels] and the [sound] Occurrences ring with (null
     * for the app's own), and arms them. The notifications of what is
     * [done] go away, and so does a full-screen alarm the plan no longer
     * escalates, the Nudge's once nothing is Unsorted, and the Changes
     * Nudge's once nobody waits for changes.
     */
    fun replace(
        context: Context,
        plan: JSONArray,
        labels: JSONObject,
        done: JSONArray,
        nudge: JSONObject?,
        changes: JSONObject?,
        sound: String?,
    ) {
        val edit = prefs(context).edit()
            .putString(KEY_PLAN, plan.toString())
            .putString(KEY_LABELS, labels.toString())
        if (nudge == null) edit.remove(KEY_NUDGE) else edit.putString(KEY_NUDGE, nudge.toString())
        if (changes == null) edit.remove(KEY_CHANGES) else edit.putString(KEY_CHANGES, changes.toString())
        edit.commit()
        if (nudge == null) Notifications.cancelNudge(context)
        if (changes == null) Notifications.cancelChangesNudge(context)
        SoundChannel.choose(context, sound)
        arm(context)
        RingService.ringing?.let { ringing ->
            if (!AlarmPlan.keepsRinging(alarms(context), ringing.routine, ringing.occurrence)) {
                RingService.stop(ringing.routine, ringing.occurrence)
            }
        }
        val notifications = context.getSystemService(NotificationManager::class.java)
        for (i in 0 until done.length()) {
            val occurrence = done.getJSONObject(i)
            notifications.cancel(
                AlarmPlan.notificationId(occurrence.getString("routine"), occurrence.getString("occurrence")),
            )
        }
    }

    /** The texts the app handed over for notifications, by name. */
    fun label(context: Context, name: String): String = labelOf(Stored.text(prefs(context), KEY_LABELS), name)

    /** The text [name] of the [labels] handed over; [name] itself without one. */
    internal fun labelOf(labels: String?, name: String): String =
        Stored.objectOf(labels) { it.opt(name) as? String } ?: name

    /** The plan with the taps not applied yet. */
    private fun alarms(context: Context): List<Alarm> =
        AlarmPlan.effective(planOf(Stored.text(prefs(context), KEY_PLAN)), EventQueue.pending(context))

    /** The alarms of the [plan] handed over that read, in order. */
    internal fun planOf(plan: String?): List<Alarm> = Stored.listOf(plan) { alarm(it) }

    /** The Nudge with the snooze tapped and not applied yet; null while nothing is Unsorted. */
    private fun nudge(context: Context): Nudge? {
        val nudge = nudgeOf(Stored.text(prefs(context), KEY_NUDGE)) ?: return null
        return NudgePlan.effective(nudge, EventQueue.pending(context))
    }

    /** The Nudge handed over as [text], or null. */
    internal fun nudgeOf(text: String?): Nudge? = Stored.objectOf(text) { nudge(it) }

    /**
     * The Changes Nudge (#109) and its text for the lock screen; null
     * while nobody waits for changes. It has no snooze, so the Nudge's
     * snoozes queued meanwhile do not touch it.
     */
    private fun changes(context: Context): Pair<Nudge, String>? = changesOf(Stored.text(prefs(context), KEY_CHANGES))

    /** The Changes Nudge handed over as [text] and its text for the lock screen, or null. */
    internal fun changesOf(text: String?): Pair<Nudge, String>? =
        Stored.objectOf(text) { nudge(it) to it.optString("public") }

    /**
     * After the phone was off (or its clock jumped ahead): what became
     * Missed meanwhile is told in one Catch-up Notification, not rung
     * late, and the next alarm is armed.
     */
    fun rearm(context: Context) {
        val prefs = prefs(context)
        val missed = AlarmPlan.catchUp(
            alarms(context),
            TimeZone.getDefault(),
            Stored.long(prefs, KEY_LAST_FIRED),
            Stored.long(prefs, KEY_LAST_ARMED),
            System.currentTimeMillis(),
        )
        if (missed > 0) Notifications.showCatchUp(context, missed)
        arm(context)
    }

    /** Arms the next alarm of the stored plan, in place of the one before. */
    fun arm(context: Context) {
        val manager = context.getSystemService(AlarmManager::class.java)
        val now = System.currentTimeMillis()
        val lastFired = Stored.long(prefs(context), KEY_LAST_FIRED)
        prefs(context).edit().putLong(KEY_LAST_ARMED, now).commit()
        val zone = TimeZone.getDefault()
        val next = listOfNotNull(
            AlarmPlan.next(alarms(context), zone, lastFired, now),
            nudge(context)?.let { NudgePlan.next(it, zone, lastFired, now) },
            changes(context)?.let { (changes, _) -> NudgePlan.next(changes, zone, lastFired, now) },
        ).minOrNull()
        if (next == null) {
            manager.cancel(fire(context, null))
            return
        }
        val operation = fire(context, next)
        if (exactAllowed(context)) {
            manager.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, next, operation)
        } else {
            // Declined: the fallback, Android picks the moment near it.
            manager.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, next, operation)
        }
    }

    /**
     * The alarm armed for [armed] fired: shows everything due since then
     * and arms the next one. An escalated one rings full screen; without
     * the full-screen permission, or when Android does not let the
     * ringing start, it shows as a high-priority notification instead.
     */
    fun fired(context: Context, armed: Long) {
        val now = System.currentTimeMillis()
        val zone = TimeZone.getDefault()
        val lastFired = Stored.long(prefs(context), KEY_LAST_FIRED)
        val due = AlarmPlan.due(alarms(context), zone, armed, lastFired, now)
        val ring = AlarmPlan.toRing(due, zone, now, Permissions.fullScreenAllowed(context))
        for (alarm in due) {
            if (alarm == ring && RingService.start(context, alarm, AlarmPlan.ringUntil(alarm, zone, now))) continue
            Notifications.show(context, alarm, urgent = AlarmPlan.escalates(alarm, zone, now))
        }
        nudge(context)?.let { if (NudgePlan.due(it, zone, armed, lastFired, now)) Notifications.showNudge(context, it) }
        changes(context)?.let { (changes, public) ->
            if (NudgePlan.due(changes, zone, armed, lastFired, now)) Notifications.showChangesNudge(context, changes, public)
        }
        prefs(context).edit().putLong(KEY_LAST_FIRED, now).commit()
        arm(context)
    }

    fun exactAllowed(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.S ||
            context.getSystemService(AlarmManager::class.java).canScheduleExactAlarms()

    /** The one pending alarm; [at] travels along, a null one only cancels. */
    private fun fire(context: Context, at: Long?): PendingIntent {
        val intent = Intent(context, AlarmReceiver::class.java).setAction(AlarmReceiver.ACTION_FIRE)
        if (at != null) intent.putExtra(EXTRA_ARMED, at)
        return PendingIntent.getBroadcast(
            context, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
    }

    private fun alarm(json: JSONObject): Alarm = Alarm(
        id = json.getString("id"),
        routine = json.getString("routine"),
        occurrence = json.getString("occurrence"),
        local = json.optJSONArray("local")?.let { local -> (0 until local.length()).map { local.getInt(it) } },
        at = if (json.has("at")) json.getLong("at") else null,
        title = json.optString("title"),
        body = json.optString("body"),
        renagMs = json.optLong("renag") * 60_000L,
        end = json.optJSONArray("end")?.let { end -> (0 until end.length()).map { end.getInt(it) } },
        escalate = json.optInt("escalate"),
    )

    private fun nudge(json: JSONObject): Nudge {
        val anchors = json.getJSONArray("anchors")
        return Nudge(
            anchors = (0 until anchors.length()).map {
                val anchor = anchors.getJSONArray(it)
                anchor.getLong(0) to anchor.getLong(1)
            },
            snoozeAt = if (json.has("snoozeAt")) json.getLong("snoozeAt") else null,
            snoozeUntil = if (json.has("snoozeUntil")) json.getLong("snoozeUntil") else null,
            repeatMs = maxOf(1, json.optLong("repeat", 60)) * 60_000L,
            quiet = json.optJSONArray("quiet")?.let { quiet -> (0 until quiet.length()).map { quiet.getInt(it) } },
            since = if (json.has("since")) json.getLong("since") else null,
            title = json.optString("title"),
            body = json.optString("body"),
        )
    }

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
