package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.provider.MediaStore
import android.view.WindowManager
import androidx.core.content.FileProvider
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject
import java.util.TimeZone

class MainActivity : FlutterActivity() {
    /** The answer to "request", until the permission dialog comes back. */
    private var permissionAnswer: MethodChannel.Result? = null

    private var alarmChannel: MethodChannel? = null

    private var photoChannel: MethodChannel? = null

    /** The answers to the calendar's, the calendar mirror's and Bluetooth's "request", by request code. */
    private val contextAnswers = mutableMapOf<Int, MethodChannel.Result>()

    /** The answer to "take", until the camera app comes back. */
    private var photoAnswer: MethodChannel.Result? = null

    private var voiceChannel: MethodChannel? = null

    private var captureChannel: MethodChannel? = null

    /** The microphone and the player for voice Notes. */
    private val voice by lazy { Voice(this) }

    /** The answer to the microphone's "request", until the dialog comes back. */
    private var microphoneAnswer: MethodChannel.Result? = null

    private var speechChannel: MethodChannel? = null

    /** Whisper and its model for the Transcripts of voice Notes. */
    private val speech by lazy { Speech(this) }

    /** The answer to the speech model's "pick", until the file picker comes back. */
    private var modelAnswer: MethodChannel.Result? = null

    /** ocrs and its bundled models for the Photo Texts of photos. */
    private val ocr by lazy { Ocr(this) }

    override fun onCreate(savedInstanceState: Bundle?) {
        if (intent?.getBooleanExtra(EXTRA_RING, false) == true) showOverLockScreen(true)
        if (savedInstanceState == null && intent?.getBooleanExtra(EXTRA_NUDGE, false) == true) nudgeOpened = true
        if (savedInstanceState == null && opensUnsorted(intent)) unsortedOpened = true
        if (savedInstanceState == null) {
            CaptureModes.of(intent?.action, intent?.flags ?: 0)?.let { captureRequested = it }
        }
        super.onCreate(savedInstanceState)
    }

    /**
     * The full-screen alarm, the Nudge or the Agenda widget's Unsorted
     * count opened the running app: the app shows it.
     */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (intent.getBooleanExtra(EXTRA_NUDGE, false)) {
            nudgeOpened = true
            actionsArrived()
        }
        if (opensUnsorted(intent)) {
            unsortedOpened = true
            actionsArrived()
        }
        // Something was shared in while the app runs: it takes it now.
        if (intent.getBooleanExtra(EXTRA_SHARED, false)) {
            photoChannel?.invokeMethod("shared", null)
        }
        // A shortcut, tile or widget asked for Capture while the app runs.
        CaptureModes.of(intent.action, intent.flags)?.let {
            captureRequested = it
            captureChannel?.invokeMethod("requested", null)
        }
        if (!intent.getBooleanExtra(EXTRA_RING, false)) return
        showOverLockScreen(true)
        actionsArrived()
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        active = this
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, LOCATION_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "granted" -> result.success(Location.granted(this))
                    "request" -> request(result)
                    "current" -> Location.current(this) { result.success(it) }
                    else -> result.notImplemented()
                }
            }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CONTEXT_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "granted" -> result.success(
                        when (call.arguments) {
                            "calendar" -> Calendar.granted(this)
                            "bluetooth" -> Bluetooth.granted(this)
                            else -> false
                        },
                    )
                    "request" -> requestContext(call.arguments as String?, result)
                    "calendar" -> inBackground(result) { Calendar.current(this) }
                    "bluetooth" -> Bluetooth.current(this) { result.success(it) }
                    else -> result.notImplemented()
                }
            }
        photoChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, PHOTO_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "take" -> takePhoto(result)
                    "shared" -> inBackground(result) { Shares.take() }
                    else -> result.notImplemented()
                }
            }
        }
        voiceChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, VOICE_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "granted" -> result.success(Voice.granted(this@MainActivity))
                    "request" -> requestMicrophone(result)
                    "start" -> result.success(voice.start())
                    "stop" -> {
                        val stopped = voice.stop() ?: return@setMethodCallHandler result.success(null)
                        inBackground(result) {
                            stopped.file.readBytes()
                                .takeIf { stopped.clean || Voice.complete(it) }
                                .also { stopped.file.delete() }
                        }
                    }
                    "play" -> result.success(
                        voice.play(call.arguments as ByteArray) {
                            voiceChannel?.invokeMethod("played", null)
                        },
                    )
                    "stopPlaying" -> {
                        voice.stopPlaying()
                        result.success(true)
                    }
                    else -> result.notImplemented()
                }
            }
        }
        speechChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, SPEECH_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "supported" -> result.success(WhisperNative.loaded)
                    "modelHash" -> inBackground(result) { speech.modelHash() }
                    "download" -> {
                        val url = call.arguments as String
                        inBackground(result) {
                            speech.download(url) { received, total ->
                                runOnUiThread {
                                    speechChannel?.invokeMethod(
                                        "progress",
                                        mapOf("received" to received, "total" to total),
                                    )
                                }
                            }
                        }
                    }
                    "pick" -> pickModel(result)
                    "cancel" -> {
                        speech.cancel()
                        result.success(null)
                    }
                    "install" -> inBackground(result) { speech.install() }
                    "discard" -> inBackground(result) {
                        speech.discard()
                        true
                    }
                    "remove" -> inBackground(result) {
                        speech.remove()
                        true
                    }
                    "transcribe" -> speech.transcribe(call.arguments as ByteArray) {
                        runOnUiThread { result.success(it) }
                    }
                    "idle" -> {
                        speech.idle()
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, OCR_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "supported" -> result.success(OcrNative.loaded)
                    "read" -> ocr.read(call.arguments as ByteArray) {
                        runOnUiThread { result.success(it) }
                    }
                    "idle" -> {
                        ocr.idle()
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CALENDAR_CHANNEL)
            .setMethodCallHandler { call, result ->
                val args = call.arguments as? Map<*, *> ?: emptyMap<Any, Any>()
                when (call.method) {
                    "mirrorGranted" -> result.success(CalendarMirror.granted(this))
                    "mirrorRequest" -> requestMirror(result)
                    "zone" -> result.success(TimeZone.getDefault().id)
                    "calendars" -> inBackground(result) { CalendarMirror.calendars(this) }
                    "marked" -> inBackground(result) {
                        CalendarMirror.marked(this, (args["calendar"] as String).toLong())
                    }
                    "apply" -> inBackground(result) {
                        CalendarMirror.apply(
                            this,
                            calendarId = (args["calendar"] as String).toLong(),
                            creates = (args["create"] as List<*>).map { it as Map<*, *> },
                            updates = (args["update"] as List<*>).map { it as Map<*, *> },
                            deletes = (args["delete"] as List<*>).map { it as String },
                            minutes = (args["minutes"] as Number).toInt(),
                        )
                    }
                    "insert" -> result.success(
                        CalendarInsert.open(
                            this,
                            CalendarInsert.of(
                                title = args["title"] as String,
                                description = args["description"] as String? ?: "",
                                start = args["start"] as String?,
                                rule = args["rule"] as String?,
                                minutes = (args["minutes"] as Number).toInt(),
                                zone = TimeZone.getDefault(),
                            ),
                        ),
                    )
                    "share" -> result.success(
                        FileShare.share(
                            this,
                            args["name"] as String,
                            args["bytes"] as ByteArray,
                            args["type"] as String,
                        ),
                    )
                    else -> result.notImplemented()
                }
            }
        captureChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CAPTURE_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "take" -> {
                        result.success(captureRequested)
                        captureRequested = null
                    }
                    else -> result.notImplemented()
                }
            }
        }
        alarmChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, ALARM_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "arm" -> {
                        val args = call.arguments as Map<*, *>
                        Scheduler.replace(
                            this@MainActivity,
                            json(args["alarms"]) as JSONArray,
                            json(args["labels"]) as JSONObject,
                            json(args["done"]) as JSONArray,
                            json(args["nudge"]) as JSONObject?,
                        )
                        result.success(null)
                    }
                    "pending" -> result.success(
                        EventQueue.pending(this@MainActivity).map {
                            mapOf(
                                "seq" to it.seq,
                                "kind" to it.kind,
                                "routine" to it.routine,
                                "occurrence" to it.occurrence,
                                "at" to it.at,
                                "until" to it.until,
                            )
                        },
                    )
                    "ack" -> {
                        EventQueue.ack(this@MainActivity, (call.arguments as Number).toLong())
                        Scheduler.arm(this@MainActivity)
                        result.success(null)
                    }
                    "ringing" -> result.success(
                        RingService.ringing?.let {
                            mapOf(
                                "routine" to it.routine,
                                "occurrence" to it.occurrence,
                                "title" to it.title,
                                "body" to it.body,
                            )
                        },
                    )
                    "answerRing" -> {
                        val args = call.arguments as Map<*, *>
                        val alarm = Alarm(
                            id = "",
                            routine = args["routine"] as String,
                            occurrence = args["occurrence"] as String,
                            local = null,
                            at = null,
                            title = args["title"] as String,
                            body = args["body"] as String,
                        )
                        val minutes = (args["snooze"] as Number?)?.toInt()
                        if (minutes == null) {
                            AlarmReceiver.done(this@MainActivity, alarm)
                        } else {
                            AlarmReceiver.snooze(this@MainActivity, alarm, minutes)
                        }
                        result.success(null)
                    }
                    "nudgeOpened" -> {
                        result.success(nudgeOpened)
                        nudgeOpened = false
                    }
                    "agenda" -> {
                        AgendaWidget.replace(this@MainActivity, json(call.arguments) as JSONObject)
                        result.success(null)
                    }
                    "unsortedOpened" -> {
                        result.success(unsortedOpened)
                        unsortedOpened = false
                    }
                    "permissions" -> result.success(Permissions.granted(this@MainActivity))
                    "request" -> {
                        Permissions.request(this@MainActivity, call.arguments as String)
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        }
    }

    override fun onDestroy() {
        if (active === this) active = null
        voice.release()
        speech.release()
        ocr.release()
        super.onDestroy()
    }

    /**
     * Opens the Device's camera app to take a photo into [cameraFile]
     * through our FileProvider. Not the CAMERA permission: the camera app
     * takes the photo, and declaring it would make this intent need it.
     */
    private fun takePhoto(result: MethodChannel.Result) {
        val file = Photos.cameraFile(this).apply { delete() }
        val uri = FileProvider.getUriForFile(this, "$packageName.photos", file)
        val intent = Intent(MediaStore.ACTION_IMAGE_CAPTURE)
            .putExtra(MediaStore.EXTRA_OUTPUT, uri)
            .addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
        intent.clipData = ClipData.newRawUri("", uri)
        try {
            startActivityForResult(intent, REQUEST_PHOTO)
        } catch (_: ActivityNotFoundException) {
            return result.success(null)
        }
        photoAnswer?.success(null)
        photoAnswer = result
    }

    /**
     * Lets the user pick a speech model file of their own with the
     * system's file picker, which needs no storage permission, and copies
     * it to the side; answers its SHA-256, or null when none was picked.
     */
    private fun pickModel(result: MethodChannel.Result) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("*/*")
        try {
            startActivityForResult(intent, REQUEST_MODEL)
        } catch (_: ActivityNotFoundException) {
            return result.success(null)
        }
        modelAnswer?.success(null)
        modelAnswer = result
    }

    @Deprecated("The Flutter activity has no Activity Result API.")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_MODEL) {
            val answer = modelAnswer ?: return
            modelAnswer = null
            val uri = data?.data
            if (resultCode != Activity.RESULT_OK || uri == null) return answer.success(null)
            return inBackground(answer) { speech.takeIn { contentResolver.openInputStream(uri) } }
        }
        if (requestCode != REQUEST_PHOTO) return
        val file = Photos.cameraFile(this)
        val answer = photoAnswer
        photoAnswer = null
        if (answer == null) {
            // The app was gone while the camera was open: the photo
            // still reaches a Note, through Capture like a share.
            if (resultCode == Activity.RESULT_OK && file.length() > 0L) {
                Shares.keepPhoto(this, file)
                photoChannel?.invokeMethod("shared", null)
            }
            return
        }
        if (resultCode != Activity.RESULT_OK || file.length() == 0L) {
            file.delete()
            return answer.success(null)
        }
        inBackground(answer) { Photos.normalize { file.inputStream() }.also { file.delete() } }
    }

    /** Answers [result] with what [work] makes, off the main thread. */
    private fun inBackground(result: MethodChannel.Result, work: () -> Any?) {
        Thread {
            val value = runCatching(work).getOrNull()
            runOnUiThread { result.success(value) }
        }.start()
    }

    private fun request(result: MethodChannel.Result) {
        if (Location.granted(this)) return result.success(true)
        permissionAnswer?.success(false)
        permissionAnswer = result
        requestPermissions(Location.permissions, REQUEST_LOCATION)
    }

    /** Asks for the microphone; answers whether it is allowed now. */
    private fun requestMicrophone(result: MethodChannel.Result) {
        if (Voice.granted(this)) return result.success(true)
        microphoneAnswer?.success(false)
        microphoneAnswer = result
        requestPermissions(arrayOf(Voice.permission), REQUEST_MICROPHONE)
    }

    /** Asks for the calendar or Bluetooth [kind]; answers whether it is allowed now. */
    private fun requestContext(kind: String?, result: MethodChannel.Result) {
        val (code, permission, granted) = when (kind) {
            "calendar" -> Triple(REQUEST_CALENDAR, Calendar.permission, Calendar.granted(this))
            "bluetooth" -> Triple(REQUEST_BLUETOOTH, Bluetooth.permission, Bluetooth.granted(this))
            else -> return result.success(false)
        }
        if (granted || permission == null) return result.success(granted)
        contextAnswers.remove(code)?.success(false)
        contextAnswers[code] = result
        requestPermissions(arrayOf(permission), code)
    }

    /** Asks to read and write calendars for the mirror; answers whether both are allowed now. */
    private fun requestMirror(result: MethodChannel.Result) {
        if (CalendarMirror.granted(this)) return result.success(true)
        contextAnswers.remove(REQUEST_CALENDAR_MIRROR)?.success(false)
        contextAnswers[REQUEST_CALENDAR_MIRROR] = result
        requestPermissions(CalendarMirror.permissions, REQUEST_CALENDAR_MIRROR)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        when (requestCode) {
            REQUEST_CALENDAR -> contextAnswers.remove(requestCode)?.success(Calendar.granted(this))
            REQUEST_BLUETOOTH -> contextAnswers.remove(requestCode)?.success(Bluetooth.granted(this))
            REQUEST_CALENDAR_MIRROR -> contextAnswers.remove(requestCode)?.success(CalendarMirror.granted(this))
            REQUEST_MICROPHONE -> {
                microphoneAnswer?.success(Voice.granted(this))
                microphoneAnswer = null
            }
        }
        if (requestCode != REQUEST_LOCATION) return
        val answer = permissionAnswer ?: return
        permissionAnswer = null
        answer.success(Location.granted(this))
    }

    /**
     * Shows the app over the lock screen and wakes the screen, only while
     * a full-screen alarm rings: the Notes stay behind the lock otherwise.
     */
    private fun showOverLockScreen(on: Boolean) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
            setShowWhenLocked(on)
            setTurnScreenOn(on)
        } else {
            @Suppress("DEPRECATION")
            val flags = WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED or
                WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON
            if (on) window.addFlags(flags) else window.clearFlags(flags)
        }
    }

    /**
     * Whether [intent] is the Agenda widget's Unsorted count, not a
     * reopening from the recent apps with it.
     */
    private fun opensUnsorted(intent: Intent?): Boolean =
        intent?.getBooleanExtra(EXTRA_UNSORTED, false) == true &&
            intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY == 0

    /** Channel arguments (maps, lists, numbers) as org.json values. */
    private fun json(value: Any?): Any? = when (value) {
        is Map<*, *> -> JSONObject().apply {
            for ((k, v) in value) put(k.toString(), json(v))
        }
        is List<*> -> JSONArray().apply { for (v in value) put(json(v)) }
        else -> value
    }

    companion object {
        private const val LOCATION_CHANNEL = "io.github.paxel.oldbrainfix/location"
        private const val ALARM_CHANNEL = "io.github.paxel.oldbrainfix/alarms"
        private const val PHOTO_CHANNEL = "io.github.paxel.oldbrainfix/photos"
        private const val CONTEXT_CHANNEL = "io.github.paxel.oldbrainfix/context"
        private const val VOICE_CHANNEL = "io.github.paxel.oldbrainfix/voice"
        private const val CAPTURE_CHANNEL = "io.github.paxel.oldbrainfix/capture"
        private const val SPEECH_CHANNEL = "io.github.paxel.oldbrainfix/speech"
        private const val OCR_CHANNEL = "io.github.paxel.oldbrainfix/ocr"
        private const val CALENDAR_CHANNEL = "io.github.paxel.oldbrainfix/calendar"
        private const val REQUEST_LOCATION = 1
        // Not 2: Permissions.REQUEST_NOTIFICATIONS.
        private const val REQUEST_PHOTO = 3
        private const val REQUEST_MODEL = 4
        private const val REQUEST_CALENDAR = 10
        private const val REQUEST_BLUETOOTH = 11
        private const val REQUEST_MICROPHONE = 12
        private const val REQUEST_CALENDAR_MIRROR = 13
        private var active: MainActivity? = null

        /** Marks the launch from the full-screen alarm's notification. */
        const val EXTRA_RING = "oldbrainfix.ring"

        /** Marks the launch from the Nudge's notification. */
        const val EXTRA_NUDGE = "oldbrainfix.nudge"

        /** Marks the launch from the Agenda widget's Unsorted count. */
        const val EXTRA_UNSORTED = "oldbrainfix.unsorted"

        /** Marks the launch by a share from another app ([ShareActivity]). */
        const val EXTRA_SHARED = "oldbrainfix.shared"

        /** The Nudge was tapped and the app has not shown the Nudge screen yet. */
        private var nudgeOpened = false

        /** The Agenda widget's Unsorted count was tapped and the app has not shown the list yet. */
        private var unsortedOpened = false

        /** The capture screen an entry point asked for and the app has not opened yet. */
        private var captureRequested: String? = null

        /**
         * The full-screen alarm stopped: the app no longer shows over the
         * lock screen, and a running app closes the alarm screen.
         */
        fun ringStopped() {
            val activity = active ?: return
            activity.runOnUiThread { activity.showOverLockScreen(false) }
            actionsArrived()
        }

        /**
         * Tells a running app that a notification was tapped, or a
         * full-screen alarm started or stopped, so it looks again now.
         */
        fun actionsArrived() {
            val activity = active ?: return
            activity.runOnUiThread { activity.alarmChannel?.invokeMethod("actions", null) }
        }
    }
}
