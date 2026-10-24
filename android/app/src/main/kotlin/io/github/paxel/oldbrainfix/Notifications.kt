package io.github.paxel.oldbrainfix

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.drawable.Icon
import android.net.Uri
import android.os.Build

/**
 * The notification of a due Occurrence with Done and Snooze. Android
 * shows at most three actions, so Snooze turns the same notification
 * into the three snooze choices (10 min, 30 min, 1 h). Every action is a
 * broadcast, so it works from the lock screen without opening the app.
 */
object Notifications {
    private const val CHANNEL = "occurrences"
    private val CATCH_UP_ID = "catch-up".hashCode()

    /** Minutes of each snooze choice, with the label naming it. */
    val snoozes = listOf(10 to "snooze10", 30 to "snooze30", 60 to "snooze60")

    fun show(context: Context, alarm: Alarm) {
        val actions = listOf(
            action(context, alarm, AlarmReceiver.ACTION_DONE, Scheduler.label(context, "done")),
            action(context, alarm, AlarmReceiver.ACTION_SNOOZE, Scheduler.label(context, "snooze")),
        )
        post(context, alarm, alarm.body, actions, alertOnce = false)
    }

    /** The same notification, now offering how long to snooze, quietly. */
    fun showSnoozeChoices(context: Context, alarm: Alarm) {
        val actions = snoozes.map { (minutes, label) ->
            action(context, alarm, AlarmReceiver.ACTION_SNOOZE_FOR, Scheduler.label(context, label), minutes)
        }
        post(context, alarm, Scheduler.label(context, "snoozeQuestion"), actions, alertOnce = true)
    }

    /**
     * The one Catch-up Notification after the phone was off: how many
     * Occurrences were Missed meanwhile. Tapping it opens the Agenda with
     * the Missed Summary; a later one replaces it.
     */
    fun showCatchUp(context: Context, missed: Int) {
        val text = if (missed == 1) {
            Scheduler.label(context, "catchUpOne")
        } else {
            Scheduler.label(context, "catchUpMany").replace("{count}", missed.toString())
        }
        context.getSystemService(NotificationManager::class.java).notify(
            CATCH_UP_ID,
            builder(context)
                .setSmallIcon(R.mipmap.ic_launcher)
                .setContentTitle(Scheduler.label(context, "catchUpTitle"))
                .setContentText(text)
                .setCategory(Notification.CATEGORY_REMINDER)
                .setAutoCancel(true)
                .setContentIntent(openApp(context))
                .build(),
        )
    }

    fun cancel(context: Context, alarm: Alarm) =
        context.getSystemService(NotificationManager::class.java)
            .cancel(AlarmPlan.notificationId(alarm.routine, alarm.occurrence))

    private fun post(
        context: Context,
        alarm: Alarm,
        text: String,
        actions: List<Notification.Action>,
        alertOnce: Boolean,
    ) {
        val manager = context.getSystemService(NotificationManager::class.java)
        val notification = builder(context)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentTitle(alarm.title)
            .setContentText(text)
            .setCategory(Notification.CATEGORY_REMINDER)
            // Shown in full on the lock screen, so its actions work there.
            .setVisibility(Notification.VISIBILITY_PUBLIC)
            .setOnlyAlertOnce(alertOnce)
            .setAutoCancel(true)
            .setContentIntent(openApp(context))
            .setActions(*actions.toTypedArray())
            .build()
        manager.notify(AlarmPlan.notificationId(alarm.routine, alarm.occurrence), notification)
    }

    private fun builder(context: Context): Notification.Builder =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.getSystemService(NotificationManager::class.java).createNotificationChannel(
                NotificationChannel(CHANNEL, Scheduler.label(context, "channel"), NotificationManager.IMPORTANCE_HIGH),
            )
            Notification.Builder(context, CHANNEL)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(context)
                .setPriority(Notification.PRIORITY_HIGH)
                .setDefaults(Notification.DEFAULT_ALL)
        }

    private fun openApp(context: Context): PendingIntent = PendingIntent.getActivity(
        context, 0,
        Intent(context, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    private fun action(
        context: Context,
        alarm: Alarm,
        kind: String,
        label: String,
        minutes: Int = 0,
    ): Notification.Action {
        val intent = Intent(context, AlarmReceiver::class.java)
            .setAction(kind)
            // Makes each action of each Occurrence its own PendingIntent.
            .setData(Uri.parse("oldbrainfix://alarm/${Uri.encode(alarm.id)}/$kind/$minutes"))
            .putExtra(AlarmReceiver.EXTRA_ID, alarm.id)
            .putExtra(AlarmReceiver.EXTRA_ROUTINE, alarm.routine)
            .putExtra(AlarmReceiver.EXTRA_OCCURRENCE, alarm.occurrence)
            .putExtra(AlarmReceiver.EXTRA_TITLE, alarm.title)
            .putExtra(AlarmReceiver.EXTRA_BODY, alarm.body)
            .putExtra(AlarmReceiver.EXTRA_MINUTES, minutes)
        val pending = PendingIntent.getBroadcast(
            context, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        return Notification.Action.Builder(Icon.createWithResource(context, R.mipmap.ic_launcher), label, pending).build()
    }
}
