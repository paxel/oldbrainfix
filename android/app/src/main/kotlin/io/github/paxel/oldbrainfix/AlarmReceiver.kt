package io.github.paxel.oldbrainfix

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * The armed alarm firing, and Done and Snooze tapped on a notification.
 * A tap is queued for the app (it applies it to the log on its next
 * start or at once while it runs) and takes effect on the alarms right
 * away: what is Done no longer rings, a snooze rings again.
 */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            ACTION_FIRE -> Scheduler.fired(context, intent.getLongExtra(Scheduler.EXTRA_ARMED, 0))
            ACTION_DONE -> {
                val alarm = alarmOf(intent)
                EventQueue.add(context, "done", alarm)
                Notifications.cancel(context, alarm)
                tapped(context)
            }
            ACTION_SNOOZE -> Notifications.showSnoozeChoices(context, alarmOf(intent))
            ACTION_SNOOZE_FOR -> {
                val alarm = alarmOf(intent)
                val minutes = intent.getIntExtra(EXTRA_MINUTES, 60)
                EventQueue.add(context, "snooze", alarm, System.currentTimeMillis() + minutes * 60_000L)
                Notifications.cancel(context, alarm)
                tapped(context)
            }
        }
    }

    private fun tapped(context: Context) {
        Scheduler.arm(context)
        MainActivity.actionsArrived()
    }

    private fun alarmOf(intent: Intent) = Alarm(
        id = intent.getStringExtra(EXTRA_ID) ?: "",
        routine = intent.getStringExtra(EXTRA_ROUTINE) ?: "",
        occurrence = intent.getStringExtra(EXTRA_OCCURRENCE) ?: "",
        local = null,
        at = null,
        title = intent.getStringExtra(EXTRA_TITLE) ?: "",
        body = intent.getStringExtra(EXTRA_BODY) ?: "",
    )

    companion object {
        const val ACTION_FIRE = "io.github.paxel.oldbrainfix.FIRE"
        const val ACTION_DONE = "io.github.paxel.oldbrainfix.DONE"
        const val ACTION_SNOOZE = "io.github.paxel.oldbrainfix.SNOOZE"
        const val ACTION_SNOOZE_FOR = "io.github.paxel.oldbrainfix.SNOOZE_FOR"
        const val EXTRA_ID = "oldbrainfix.id"
        const val EXTRA_ROUTINE = "oldbrainfix.routine"
        const val EXTRA_OCCURRENCE = "oldbrainfix.occurrence"
        const val EXTRA_TITLE = "oldbrainfix.title"
        const val EXTRA_BODY = "oldbrainfix.body"
        const val EXTRA_MINUTES = "oldbrainfix.minutes"
    }
}
