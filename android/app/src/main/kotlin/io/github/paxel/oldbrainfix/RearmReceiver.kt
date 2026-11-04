package io.github.paxel.oldbrainfix

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * After a reboot, an update, a clock change, a new time zone or a change
 * of the exact-alarm permission, arms the stored plan again at the same
 * local times (from Chaos Alert). Alarms that came due meanwhile do not
 * ring in a burst; what became Missed is told in one Catch-up
 * Notification. The Agenda widget shows the day it is now.
 */
class RearmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        // Exported for the system's broadcasts: another app can still name
        // this receiver in an explicit intent, which is ignored.
        if (!RearmActions.rearms(intent.action)) return
        Scheduler.rearm(context)
        // Maybe a new day, maybe a new zone: the Agenda widget looks again.
        AgendaWidget.update(context)
    }
}

/**
 * The broadcasts that arm the plan again, the same as the receiver's
 * filter in the manifest. Only the system may send them.
 */
object RearmActions {
    val all = setOf(
        "android.intent.action.BOOT_COMPLETED",
        "android.intent.action.MY_PACKAGE_REPLACED",
        "android.intent.action.TIME_SET",
        "android.intent.action.TIMEZONE_CHANGED",
        "android.app.action.SCHEDULE_EXACT_ALARM_PERMISSION_STATE_CHANGED",
    )

    fun rearms(action: String?): Boolean = action in all
}
