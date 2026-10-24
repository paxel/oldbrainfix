package io.github.paxel.oldbrainfix

import android.app.Activity
import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.widget.RemoteViews

/**
 * The quick entry points into Capture: app shortcuts, Quick Settings
 * tiles, widgets and hardware-button mappers. Each asks the app's one
 * [MainActivity] for the capture screen of one mode, by the intent's
 * action; the app opens it at once, as from "+".
 */
object CaptureModes {
    const val TEXT = "text"
    const val VOICE = "voice"
    const val PHOTO = "photo"
    private val all = listOf(TEXT, VOICE, PHOTO)
    private const val PREFIX = "io.github.paxel.oldbrainfix.action.CAPTURE_"

    /** The intent action that asks for [mode]'s capture screen. */
    fun action(mode: String): String = PREFIX + mode.uppercase()

    /**
     * The mode an intent with [action] and [flags] asks for, or null: none
     * for other actions, and none when Android reopens the app from the
     * recent apps with the intent that once started it.
     */
    fun of(action: String?, flags: Int): String? {
        if (flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0) return null
        if (action == null || !action.startsWith(PREFIX)) return null
        return action.removePrefix(PREFIX).lowercase().takeIf { it in all }
    }

    /** Opens the running app, or starts it, on [mode]'s capture screen. */
    fun intent(context: Context, mode: String): Intent =
        Intent(context, MainActivity::class.java)
            .setAction(action(mode))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP)

    /** [intent] for a widget's button or a tile, one per mode. */
    fun pendingIntent(context: Context, mode: String): PendingIntent =
        PendingIntent.getActivity(
            context,
            all.indexOf(mode),
            intent(context, mode),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
}

/**
 * Where an app shortcut or a hardware-button mapper starts Capture. No
 * screen of its own: it hands over to the running [MainActivity], like
 * [ShareActivity], so a launcher that clears the task it starts does not
 * close what the app had open.
 */
abstract class CaptureActivity(private val mode: String) : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        startActivity(CaptureModes.intent(this, mode))
        finish()
    }
}

class TextCaptureActivity : CaptureActivity(CaptureModes.TEXT)

class VoiceCaptureActivity : CaptureActivity(CaptureModes.VOICE)

class PhotoCaptureActivity : CaptureActivity(CaptureModes.PHOTO)

/**
 * A Quick Settings tile that opens one capture screen. On the lock screen
 * Android asks to unlock the Device first: the capture screen is part of
 * the app, whose Notes stay behind the Device's lock.
 */
abstract class CaptureTile(private val mode: String) : TileService() {
    override fun onStartListening() {
        super.onStartListening()
        val tile = qsTile ?: return
        tile.state = Tile.STATE_INACTIVE
        tile.updateTile()
    }

    override fun onClick() {
        super.onClick()
        if (isLocked) unlockAndRun { open() } else open()
    }

    private fun open() {
        if (Build.VERSION.SDK_INT >= 34) {
            startActivityAndCollapse(CaptureModes.pendingIntent(this, mode))
        } else {
            @Suppress("DEPRECATION")
            startActivityAndCollapse(CaptureModes.intent(this, mode))
        }
    }
}

class TextCaptureTile : CaptureTile(CaptureModes.TEXT)

class VoiceCaptureTile : CaptureTile(CaptureModes.VOICE)

class PhotoCaptureTile : CaptureTile(CaptureModes.PHOTO)

/**
 * A home-screen widget of capture buttons, [buttons] being each button's
 * view in [layout] and the mode it opens.
 */
abstract class CaptureWidget(
    private val layout: Int,
    private val buttons: List<Pair<Int, String>>,
) : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) {
        val views = RemoteViews(context.packageName, layout)
        for ((view, mode) in buttons) {
            views.setOnClickPendingIntent(view, CaptureModes.pendingIntent(context, mode))
        }
        manager.updateAppWidget(ids, views)
    }
}

class TextCaptureWidget :
    CaptureWidget(R.layout.widget_capture_text, listOf(R.id.capture_text to CaptureModes.TEXT))

class VoiceCaptureWidget :
    CaptureWidget(R.layout.widget_capture_voice, listOf(R.id.capture_voice to CaptureModes.VOICE))

class PhotoCaptureWidget :
    CaptureWidget(R.layout.widget_capture_photo, listOf(R.id.capture_photo to CaptureModes.PHOTO))

/** Voice, text and photo side by side. */
class CaptureBarWidget :
    CaptureWidget(
        R.layout.widget_capture_bar,
        listOf(
            R.id.capture_voice to CaptureModes.VOICE,
            R.id.capture_text to CaptureModes.TEXT,
            R.id.capture_photo to CaptureModes.PHOTO,
        ),
    )
