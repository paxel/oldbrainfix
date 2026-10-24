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
 * Re-nags from the same one pending alarm.
 */
object Scheduler {
    private const val PREFS = "oldbrainfix_alarms"
    private const val KEY_PLAN = "plan"
    private const val KEY_LABELS = "labels"
    private const val KEY_LAST_FIRED = "lastFired"
    private const val KEY_LAST_ARMED = "lastArmed"

    const val EXTRA_ARMED = "oldbrainfix.armed"

    /**
     * Keeps [plan] and the notification [labels], and arms the plan. The
     * notifications of what is [done] go away.
     */
    fun replace(context: Context, plan: JSONArray, labels: JSONObject, done: JSONArray) {
        prefs(context).edit()
            .putString(KEY_PLAN, plan.toString())
            .putString(KEY_LABELS, labels.toString())
            .commit()
        arm(context)
        val notifications = context.getSystemService(NotificationManager::class.java)
        for (i in 0 until done.length()) {
            val occurrence = done.getJSONObject(i)
            notifications.cancel(
                AlarmPlan.notificationId(occurrence.getString("routine"), occurrence.getString("occurrence")),
            )
        }
    }

    /** The texts the app handed over for notifications, by name. */
    fun label(context: Context, name: String): String =
        JSONObject(prefs(context).getString(KEY_LABELS, "{}") ?: "{}").optString(name, name)

    /** The plan with the taps not applied yet. */
    private fun alarms(context: Context): List<Alarm> {
        val plan = JSONArray(prefs(context).getString(KEY_PLAN, "[]") ?: "[]")
        val alarms = (0 until plan.length()).map { alarm(plan.getJSONObject(it)) }
        return AlarmPlan.effective(alarms, EventQueue.pending(context))
    }

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
            prefs.getLong(KEY_LAST_FIRED, 0),
            prefs.getLong(KEY_LAST_ARMED, 0),
            System.currentTimeMillis(),
        )
        if (missed > 0) Notifications.showCatchUp(context, missed)
        arm(context)
    }

    /** Arms the next alarm of the stored plan, in place of the one before. */
    fun arm(context: Context) {
        val manager = context.getSystemService(AlarmManager::class.java)
        val now = System.currentTimeMillis()
        val lastFired = prefs(context).getLong(KEY_LAST_FIRED, 0)
        prefs(context).edit().putLong(KEY_LAST_ARMED, now).commit()
        val next = AlarmPlan.next(alarms(context), TimeZone.getDefault(), lastFired, now)
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
     * and arms the next one.
     */
    fun fired(context: Context, armed: Long) {
        val now = System.currentTimeMillis()
        val lastFired = prefs(context).getLong(KEY_LAST_FIRED, 0)
        for (alarm in AlarmPlan.due(alarms(context), TimeZone.getDefault(), armed, lastFired, now)) {
            Notifications.show(context, alarm)
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
    )

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
