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
        Scheduler.rearm(context)
        // Maybe a new day, maybe a new zone: the Agenda widget looks again.
        AgendaWidget.update(context)
    }
}
