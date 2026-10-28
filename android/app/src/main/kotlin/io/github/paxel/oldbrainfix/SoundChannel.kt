package io.github.paxel.oldbrainfix

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.net.Uri
import android.os.Build

/**
 * The channel an Occurrence's notification comes on, and the sound it
 * rings with (#102): the app's own chime unless another sound was
 * chosen. Android fixes a channel's sound once it exists, so a new
 * sound gets a new channel, which takes over what the user set on the
 * old one (importance, vibration, lights), and the old one is deleted.
 * Each channel gets an id never used before: a deleted channel comes
 * back with its old settings, and its old sound, when its id is reused.
 */
object SoundChannel {
    /** The channel of 0.1.1, made with the system's notification sound. */
    const val LEGACY = "occurrences"

    /** The channel made for [sound], numbered [version]. */
    data class Made(val version: Int, val sound: String) {
        val id: String get() = "$LEGACY-$version"
    }

    /**
     * The channel to make for [sound] when [made] is the one there is,
     * or null when it already rings with it. With none made yet, the
     * channel of 0.1.1 may still be there, so the first one is
     * numbered 1.
     */
    fun next(made: Made?, sound: String): Made? = when {
        made == null -> Made(1, sound)
        made.sound == sound -> null
        else -> Made(made.version + 1, sound)
    }

    /** The id of [made], or of 0.1.1's channel while none was made. */
    fun replaced(made: Made?): String = made?.id ?: LEGACY

    /** The app's own sound, as the system's sound picker names it. */
    fun ownSound(context: Context): String = "android.resource://${context.packageName}/raw/obf_alarm"

    /**
     * The answer to the sound picker, given the URI it [picked] (null
     * when it was left): null for nothing chosen, else the sound under
     * "sound", null there for the app's own.
     */
    fun picked(picked: String?, own: String): Map<String, String?>? = when (picked) {
        null -> null
        own -> mapOf("sound" to null)
        else -> mapOf("sound" to picked)
    }

    /** The sound chosen, as stored; the app's own when none is. */
    fun sound(context: Context): String = prefs(context).getString(KEY_SOUND, null) ?: ownSound(context)

    /** Keeps [sound], null for the app's own, and moves the channel to it. */
    @Synchronized
    fun choose(context: Context, sound: String?) {
        val edit = prefs(context).edit()
        if (sound == null) edit.remove(KEY_SOUND) else edit.putString(KEY_SOUND, sound)
        edit.commit()
        id(context)
    }

    /** How an Occurrence's sound plays: as a notification event. */
    val attributes: AudioAttributes
        get() = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_NOTIFICATION_EVENT)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build()

    /**
     * The id of the channel ringing with the chosen sound, made first
     * where it is not yet; below Android O there are no channels and the
     * notification carries the sound itself.
     */
    @Synchronized
    fun id(context: Context): String {
        val prefs = prefs(context)
        val made = if (prefs.contains(KEY_VERSION)) {
            Made(prefs.getInt(KEY_VERSION, 1), prefs.getString(KEY_MADE, "") ?: "")
        } else {
            null
        }
        val sound = sound(context)
        val next = next(made, sound)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val manager = context.getSystemService(NotificationManager::class.java)
            // Unchanged, it is made again all the same: that only renames
            // it into the reader's language, or brings it back if it is gone.
            val id = next?.id ?: replaced(made)
            val old = next?.let { manager.getNotificationChannel(replaced(made)) }
            grant(context, sound)
            manager.createNotificationChannel(
                NotificationChannel(
                    id,
                    Scheduler.label(context, "channel"),
                    old?.importance ?: NotificationManager.IMPORTANCE_HIGH,
                ).apply {
                    setSound(Uri.parse(sound), attributes)
                    if (old != null) {
                        // The pattern first: setting it turns vibration on.
                        vibrationPattern = old.vibrationPattern
                        enableVibration(old.shouldVibrate())
                        enableLights(old.shouldShowLights())
                        lightColor = old.lightColor
                        lockscreenVisibility = old.lockscreenVisibility
                        setShowBadge(old.canShowBadge())
                    }
                },
            )
            if (old != null) manager.deleteNotificationChannel(old.id)
        }
        if (next == null) return replaced(made)
        prefs.edit().putInt(KEY_VERSION, next.version).putString(KEY_MADE, next.sound).commit()
        return next.id
    }

    /**
     * Lets the system UI, which plays a notification's sound, read a
     * sound another app provides.
     */
    fun grant(context: Context, sound: String) {
        val uri = Uri.parse(sound)
        if (uri.scheme != "content") return
        try {
            context.grantUriPermission("com.android.systemui", uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        } catch (_: SecurityException) {
            // The provider allows no grant; most sounds need none.
        } catch (_: IllegalArgumentException) {
            // No such provider: the system plays its default instead.
        }
    }

    private const val PREFS = "oldbrainfix_sound"
    private const val KEY_SOUND = "sound"
    private const val KEY_VERSION = "version"
    private const val KEY_MADE = "made"

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
