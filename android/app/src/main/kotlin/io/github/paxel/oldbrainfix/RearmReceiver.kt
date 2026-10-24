package io.github.paxel.oldbrainfix

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/**
 * After a reboot, an update, a clock change, a new time zone or a change
 * of the exact-alarm permission, arms the stored plan again at the same
 * local times (from Chaos Alert).
 */
class RearmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        Scheduler.arm(context)
    }
}
