package io.github.paxel.oldbrainfix

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * The armed alarm firing, and Done, Snooze and Not today tapped on a
 * notification (Done also from the Agenda widget).
 * A tap is queued for the app (it applies it to the log on its next
 * start or at once while it runs) and takes effect on the alarms right
 * away: what is Done or skipped no longer rings, a snooze rings again,
 * and what Tomorrow moved rings the day after. Each stops the
 * Occurrence's full-screen alarm, and so do the first Snooze and Not
 * today taps, before their choice is picked. The Nudge's snoozes are
 * taken at once.
 */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            ACTION_FIRE -> Scheduler.fired(context, intent.getLongExtra(Scheduler.EXTRA_ARMED, 0))
            ACTION_DONE -> done(context, alarmOf(intent))
            ACTION_SNOOZE -> {
                val alarm = alarmOf(intent)
                RingService.stop(alarm.routine, alarm.occurrence)
                Notifications.showSnoozeChoices(context, alarm)
            }
            ACTION_SNOOZE_FOR -> snooze(context, alarmOf(intent), intent.getIntExtra(EXTRA_MINUTES, 60))
            ACTION_NOT_TODAY -> {
                val alarm = alarmOf(intent)
                RingService.stop(alarm.routine, alarm.occurrence)
                Notifications.showNotTodayChoices(context, alarm)
            }
            ACTION_SKIP -> notToday(context, alarmOf(intent), EventQueue.KIND_SKIP)
            ACTION_TOMORROW -> notToday(context, alarmOf(intent), EventQueue.KIND_TOMORROW)
            ACTION_NUDGE_SNOOZE -> snoozeNudge(context, intent.getIntExtra(EXTRA_MINUTES, 60))
        }
    }

    companion object {
        const val ACTION_FIRE = "io.github.paxel.oldbrainfix.FIRE"
        const val ACTION_DONE = "io.github.paxel.oldbrainfix.DONE"
        const val ACTION_SNOOZE = "io.github.paxel.oldbrainfix.SNOOZE"
        const val ACTION_SNOOZE_FOR = "io.github.paxel.oldbrainfix.SNOOZE_FOR"
        const val ACTION_NUDGE_SNOOZE = "io.github.paxel.oldbrainfix.NUDGE_SNOOZE"
        const val ACTION_NOT_TODAY = "io.github.paxel.oldbrainfix.NOT_TODAY"
        const val ACTION_SKIP = "io.github.paxel.oldbrainfix.SKIP"
        const val ACTION_TOMORROW = "io.github.paxel.oldbrainfix.TOMORROW"
        const val EXTRA_ID = "oldbrainfix.id"
        const val EXTRA_ROUTINE = "oldbrainfix.routine"
        const val EXTRA_OCCURRENCE = "oldbrainfix.occurrence"
        const val EXTRA_TITLE = "oldbrainfix.title"
        const val EXTRA_BODY = "oldbrainfix.body"
        const val EXTRA_MINUTES = "oldbrainfix.minutes"

        /** Done, from a notification or the full-screen alarm. */
        fun done(context: Context, alarm: Alarm) {
            EventQueue.add(context, "done", alarm)
            answered(context, alarm)
        }

        /** A snooze of [minutes], from a notification or the full-screen alarm. */
        fun snooze(context: Context, alarm: Alarm, minutes: Int) {
            EventQueue.add(context, "snooze", alarm, System.currentTimeMillis() + minutes * 60_000L)
            answered(context, alarm)
        }

        /** Not today ▸ Skip or Tomorrow ([kind]), from a notification (#269). */
        fun notToday(context: Context, alarm: Alarm, kind: String) {
            EventQueue.add(context, kind, alarm)
            answered(context, alarm)
        }

        /**
         * A snooze of the Nudge for [minutes]: queued for the app like any
         * tap, and armed at once for every Note Unsorted now.
         */
        fun snoozeNudge(context: Context, minutes: Int) {
            val nudge = Alarm(id = "nudge", routine = "", occurrence = "", local = null, at = null, title = "", body = "")
            EventQueue.add(context, NudgePlan.KIND_SNOOZE, nudge, System.currentTimeMillis() + minutes * 60_000L)
            Notifications.cancelNudge(context)
            Scheduler.arm(context)
            MainActivity.actionsArrived()
        }

        private fun answered(context: Context, alarm: Alarm) {
            RingService.stop(alarm.routine, alarm.occurrence)
            Notifications.cancel(context, alarm)
            Scheduler.arm(context)
            // A Done shows in the Agenda widget before the app applies it.
            AgendaWidget.update(context)
            MainActivity.actionsArrived()
        }

        /** The Occurrence an action or a ringing travels with. */
        fun alarmOf(intent: Intent) = Alarm(
            id = intent.getStringExtra(EXTRA_ID) ?: "",
            routine = intent.getStringExtra(EXTRA_ROUTINE) ?: "",
            occurrence = intent.getStringExtra(EXTRA_OCCURRENCE) ?: "",
            local = null,
            at = null,
            title = intent.getStringExtra(EXTRA_TITLE) ?: "",
            body = intent.getStringExtra(EXTRA_BODY) ?: "",
        )

        /** Puts [alarm] on [intent], for [alarmOf]. */
        fun intentOf(intent: Intent, alarm: Alarm): Intent = intent
            .putExtra(EXTRA_ID, alarm.id)
            .putExtra(EXTRA_ROUTINE, alarm.routine)
            .putExtra(EXTRA_OCCURRENCE, alarm.occurrence)
            .putExtra(EXTRA_TITLE, alarm.title)
            .putExtra(EXTRA_BODY, alarm.body)
    }
}
