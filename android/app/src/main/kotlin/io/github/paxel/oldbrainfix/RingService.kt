package io.github.paxel.oldbrainfix

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.media.RingtoneManager
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager

/**
 * Rings an escalated Occurrence as a full-screen alarm (adapted from
 * Chaos Alert): a foreground service playing the phone's default alarm
 * sound in a loop, under a notification that opens the full-screen alarm
 * over the lock screen. It stops on Done or Snooze, and on its own at
 * the time it was handed (see [AlarmPlan.ringUntil]); stopped that way, the Occurrence goes back to its
 * Re-nag notification. One rings at a time; a new one takes over.
 */
class RingService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private var player: MediaPlayer? = null
    private var wakeLock: PowerManager.WakeLock? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        running = this
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val alarm = intent?.let { AlarmReceiver.alarmOf(it) }
        val until = intent?.getLongExtra(EXTRA_UNTIL, 0) ?: 0
        val fallback = intent?.getBooleanExtra(EXTRA_FALLBACK, true) ?: true
        if (intent?.action == ACTION_START && alarm != null) {
            start(alarm, until, fallback)
        } else {
            finish(null)
        }
        return START_NOT_STICKY
    }

    private fun start(alarm: Alarm, until: Long, fallback: Boolean) {
        val previous = ringing
        // Already ringing: it keeps its time to stop, so it never rings on.
        if (previous != null && (previous.routine to previous.occurrence) == (alarm.routine to alarm.occurrence)) return
        stopPlayback()
        // The one rung before goes back to its Re-nag notification.
        if (previous != null) Notifications.show(this, previous)
        ringing = alarm
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                startForeground(
                    Notifications.RING_ID,
                    Notifications.ringing(this, alarm),
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK,
                )
            } else {
                startForeground(Notifications.RING_ID, Notifications.ringing(this, alarm))
            }
        } catch (e: Exception) {
            // Android did not let it ring: the high-priority notification.
            ringing = null
            Notifications.show(this, alarm, urgent = true)
            finish(null)
            return
        }
        // Its Re-nag notification is replaced by the ringing one.
        Notifications.cancel(this, alarm)
        val left = maxOf(until - System.currentTimeMillis(), 1_000L)
        wakeLock = getSystemService(PowerManager::class.java)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "oldbrainfix:ring")
            .apply { acquire(left + 60_000) }
        handler.postDelayed({ finish(if (fallback) alarm else null) }, left)
        play()
        MainActivity.actionsArrived()
    }

    /** The phone's default alarm sound, looping on the alarm stream. */
    private fun play() {
        val uri = alarmSound() ?: return
        val p = MediaPlayer()
        player = p
        try {
            p.setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_ALARM)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                    .build(),
            )
            p.setDataSource(this, uri)
            p.isLooping = true
            p.setOnPreparedListener { it.start() }
            // Without sound the notification and the screen still show.
            p.setOnErrorListener { _, _, _ -> true }
            p.prepareAsync()
        } catch (e: Exception) {
            p.release()
            player = null
        }
    }

    private fun stopPlayback() {
        handler.removeCallbacksAndMessages(null)
        player?.release()
        player = null
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
    }

    /**
     * Stops ringing. A [fallback] alarm stopped by its time is not over:
     * it shows its Re-nag notification again and Re-nags on.
     */
    private fun finish(fallback: Alarm?) {
        stopPlayback()
        ringing = null
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
        if (fallback != null) Notifications.show(this, fallback)
        MainActivity.ringStopped()
    }

    override fun onDestroy() {
        stopPlayback()
        if (running === this) running = null
        ringing = null
        super.onDestroy()
    }

    companion object {
        private const val ACTION_START = "io.github.paxel.oldbrainfix.RING"
        private const val EXTRA_UNTIL = "oldbrainfix.until"
        private const val EXTRA_FALLBACK = "oldbrainfix.fallback"

        /** The service while it exists; it lives in the app's process. */
        private var running: RingService? = null

        /** The Occurrence ringing full screen now, or null. */
        var ringing: Alarm? = null
            private set

        /** The phone's default alarm sound, else its ringtone. */
        fun alarmSound(): Uri? =
            RingtoneManager.getDefaultUri(RingtoneManager.TYPE_ALARM)
                ?: RingtoneManager.getDefaultUri(RingtoneManager.TYPE_RINGTONE)

        /**
         * Starts ringing [alarm] until [until]; false when Android does
         * not let the ringing start from here, for a fallback instead.
         */
        fun start(context: Context, alarm: Alarm, until: Long): Boolean {
            val intent = AlarmReceiver.intentOf(Intent(context, RingService::class.java), alarm)
                .setAction(ACTION_START)
                .putExtra(EXTRA_UNTIL, until)
                // Stopped at the Occurrence's end, it is Missed: no Re-nag.
                .putExtra(EXTRA_FALLBACK, until < AlarmPlan.endOf(alarm, java.util.TimeZone.getDefault()))
            return try {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
                true
            } catch (e: Exception) {
                false
            }
        }

        /** Stops the ringing of [routine]'s [occurrence], if that is what rings. */
        fun stop(routine: String, occurrence: String) {
            val alarm = ringing ?: return
            if (alarm.routine != routine || alarm.occurrence != occurrence) return
            running?.finish(null)
        }
    }
}
