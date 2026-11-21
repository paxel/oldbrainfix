package io.github.paxel.oldbrainfix

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.drawable.Icon
import android.os.Build
import android.os.IBinder
import android.os.PowerManager

/**
 * Keeps a voice recording going while the phone is locked (#350): a
 * foreground service of type microphone under a quiet notification whose
 * Stop ends the recording, modelled on [RingService]. It records nothing
 * itself; the recorder stays [MainActivity]'s [Voice]. Stop asks the app
 * to end the recording, so it is kept and transcribed as one ended on the
 * capture screen; the app's ending stops this service.
 */
class RecordingService : Service() {
    private var wakeLock: PowerManager.WakeLock? = null

    /** Whether the notification shows: only then may the service go. */
    private var foreground = false

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        running = this
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> start(intent)
            // The app ends the recording, and with it this service.
            ACTION_STOP -> {
                if (!MainActivity.recordingStopAsked()) finish()
            }
            else -> finish()
        }
        return START_NOT_STICKY
    }

    private fun start(intent: Intent) {
        val notification = notification(
            this,
            intent.getStringExtra(EXTRA_CHANNEL) ?: "Recording",
            intent.getStringExtra(EXTRA_TITLE) ?: "Recording a voice note",
            intent.getStringExtra(EXTRA_STOP) ?: "Stop",
        )
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                startForeground(ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE)
            } else {
                startForeground(ID, notification)
            }
        } catch (e: Exception) {
            // Android did not let it start: the recording runs on as long
            // as the screen is on.
            stopSelf()
            return
        }
        foreground = true
        // Ended before the service came up: Android still wants the
        // notification shown once before it goes.
        if (!wanted) {
            finish()
            return
        }
        if (wakeLock == null) {
            wakeLock = getSystemService(PowerManager::class.java)
                .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "oldbrainfix:recording")
                .apply { acquire(LONGEST_MS) }
        }
    }

    private fun finish() {
        releaseWakeLock()
        foreground = false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
    }

    private fun releaseWakeLock() {
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
    }

    override fun onDestroy() {
        releaseWakeLock()
        if (running === this) running = null
        super.onDestroy()
    }

    companion object {
        private const val ACTION_START = "io.github.paxel.oldbrainfix.RECORD"
        private const val ACTION_STOP = "io.github.paxel.oldbrainfix.RECORD_STOP"
        private const val EXTRA_CHANNEL = "oldbrainfix.channel"
        private const val EXTRA_TITLE = "oldbrainfix.title"
        private const val EXTRA_STOP = "oldbrainfix.stop"
        private const val CHANNEL = "recording"
        private val ID = "recording".hashCode()

        /**
         * How long the phone is kept awake at most for one recording; the
         * recorder itself goes on after it.
         */
        private const val LONGEST_MS = 4 * 60 * 60 * 1000L

        /** The service while it exists; it lives in the app's process. */
        private var running: RecordingService? = null

        /** Whether a recording runs that the service is to keep going. */
        private var wanted = false

        /**
         * Starts the service for the recording that just started, while
         * the app is on screen: Android lets a microphone service start
         * only then. The labels are the app's, in its language.
         */
        fun start(context: Context, channel: String?, title: String?, stop: String?) {
            wanted = true
            val intent = Intent(context, RecordingService::class.java)
                .setAction(ACTION_START)
                .putExtra(EXTRA_CHANNEL, channel)
                .putExtra(EXTRA_TITLE, title)
                .putExtra(EXTRA_STOP, stop)
            try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
            } catch (e: Exception) {
                // Not allowed from here: the recording runs on while the
                // screen is on, which it stays.
            }
        }

        /**
         * Stops the service: the recording ended or went. One asked to
         * start and not up yet stops as soon as it is.
         */
        fun stop() {
            wanted = false
            running?.takeIf { it.foreground }?.finish()
        }

        /**
         * The quiet notification of the recording running: a tap opens
         * the app on it, Stop ends it. Silent on a channel of its own, so
         * no sound of it lands in the recording.
         */
        private fun notification(context: Context, channel: String, title: String, stop: String): Notification {
            val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.getSystemService(NotificationManager::class.java).createNotificationChannel(
                    NotificationChannel(CHANNEL, channel, NotificationManager.IMPORTANCE_LOW)
                        .apply { setSound(null, null) },
                )
                Notification.Builder(context, CHANNEL)
            } else {
                @Suppress("DEPRECATION")
                Notification.Builder(context).setPriority(Notification.PRIORITY_LOW).setSound(null)
            }
            val open = PendingIntent.getActivity(
                context, ID,
                Intent(context, MainActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            val end = PendingIntent.getService(
                context, ID,
                Intent(context, RecordingService::class.java).setAction(ACTION_STOP),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            val action = Notification.Action.Builder(
                Icon.createWithResource(context, R.drawable.ic_notification),
                stop,
                end,
            ).build()
            // Shown at once: from Android 12 on a service's notification
            // may otherwise wait up to ten seconds.
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                builder.setForegroundServiceBehavior(Notification.FOREGROUND_SERVICE_IMMEDIATE)
            }
            return builder
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle(title)
                .setCategory(Notification.CATEGORY_SERVICE)
                .setVisibility(Notification.VISIBILITY_PUBLIC)
                .setOngoing(true)
                .setContentIntent(open)
                .setActions(action)
                .build()
        }
    }
}
