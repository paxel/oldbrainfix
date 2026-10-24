package io.github.paxel.oldbrainfix

import android.Manifest
import android.app.Activity
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings

/**
 * The access alarms need (ADR 0004), each granted by the user: showing
 * notifications, exact alarms, and full-screen alarms (for Escalation).
 */
object Permissions {
    const val REQUEST_NOTIFICATIONS = 2
    private const val PREFS = "oldbrainfix_permissions"
    private const val KEY_ASKED = "notificationsAsked"

    fun granted(context: Context): Map<String, Boolean> {
        val notifications = context.getSystemService(NotificationManager::class.java)
        return mapOf(
            "notifications" to notifications.areNotificationsEnabled(),
            "exactAlarms" to Scheduler.exactAllowed(context),
            "fullScreen" to fullScreenAllowed(context),
        )
    }

    /** Whether alarms may show full screen; granted by default before Android 14. */
    fun fullScreenAllowed(context: Context): Boolean =
        Build.VERSION.SDK_INT < 34 ||
            context.getSystemService(NotificationManager::class.java).canUseFullScreenIntent()

    /** Asks for [permission] with the system dialog or opens its settings page. */
    fun request(activity: Activity, permission: String) {
        val app = Uri.parse("package:${activity.packageName}")
        when (permission) {
            "notifications" -> requestNotifications(activity)
            "exactAlarms" -> if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                activity.startActivity(Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM, app))
            }
            "fullScreen" -> if (Build.VERSION.SDK_INT >= 34) {
                activity.startActivity(Intent(Settings.ACTION_MANAGE_APP_USE_FULL_SCREEN_INTENT, app))
            }
        }
    }

    /**
     * The system dialog the first time and while Android still shows it;
     * once it no longer does, the app's notification settings.
     */
    private fun requestNotifications(activity: Activity) {
        val prefs = activity.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val dialog = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED &&
            (!prefs.getBoolean(KEY_ASKED, false) ||
                activity.shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS))
        if (dialog) {
            prefs.edit().putBoolean(KEY_ASKED, true).apply()
            activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQUEST_NOTIFICATIONS)
        } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            activity.startActivity(
                Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
                    .putExtra(Settings.EXTRA_APP_PACKAGE, activity.packageName),
            )
        } else {
            activity.startActivity(
                Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:${activity.packageName}")),
            )
        }
    }
}
