package io.github.paxel.oldbrainfix

import android.content.Context
import android.os.Build
import android.provider.Settings

/**
 * What the user's own Profile starts with on a phone (ADR 0017): the
 * name the phone was given in its settings, which needs no permission,
 * else its model. Nothing is asked.
 */
object Profiles {
    /** The phone's own name, else its model. */
    fun defaultName(context: Context): String = defaultName(
        runCatching { Settings.Global.getString(context.contentResolver, Settings.Global.DEVICE_NAME) }
            .getOrNull(),
        Build.MODEL,
    )

    /** [deviceName] when it holds more than blanks, else [model], else nothing. */
    fun defaultName(deviceName: String?, model: String?): String = when {
        !deviceName.isNullOrBlank() -> deviceName.trim()
        !model.isNullOrBlank() -> model.trim()
        else -> ""
    }
}
