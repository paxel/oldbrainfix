package io.github.paxel.oldbrainfix

import android.app.Activity
import android.app.ActivityManager
import android.app.ApplicationExitInfo
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.media.RingtoneManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.MediaStore
import android.view.WindowManager
import androidx.core.content.FileProvider
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.util.TimeZone
import java.util.concurrent.CountDownLatch

class MainActivity : FlutterActivity() {
    /** The answer to "request", until the permission dialog comes back. */
    private var permissionAnswer: MethodChannel.Result? = null

    private var alarmChannel: MethodChannel? = null

    private var photoChannel: MethodChannel? = null

    /** The answers to the calendar's, the calendar mirror's and Bluetooth's "request", by request code. */
    private val contextAnswers = mutableMapOf<Int, MethodChannel.Result>()

    /** The answer to "take", until the camera app comes back. */
    private var photoAnswer: MethodChannel.Result? = null

    /** The answer to Capture's "pick", until the gallery comes back. */
    private var galleryAnswer: MethodChannel.Result? = null

    /** The answer to the Profile's "pick", until the picture picker comes back. */
    private var pictureAnswer: MethodChannel.Result? = null

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

    /** RustO and its bundled models for the Photo Texts of photos. */
    private val ocr by lazy { Ocr(this) }

    private var syncChannel: MethodChannel? = null

    private var profileChannel: MethodChannel? = null

    /** The answer to the shared folder's "pickTree", until the picker comes back. */
    private var folderAnswer: MethodChannel.Result? = null

    private var bundleChannel: MethodChannel? = null

    /** The answer to the bundle's "pick", until the file picker comes back. */
    private var bundleAnswer: MethodChannel.Result? = null

    /** The answer to the alarm sound's "pickSound", until the sound picker comes back. */
    private var soundAnswer: MethodChannel.Result? = null

    /** The answer to the bundle's "share", until the user is back from the share chooser. */
    private var shareAnswer: MethodChannel.Result? = null

    /** Whether the app went out of sight since the share chooser was opened. */
    private var shareLeft = false

    /** Paired hosts on the network while the app shows, told to the app as they come. */
    private val discovery by lazy {
        Discovery(this) { found -> syncChannel?.invokeMethod("found", found.toMap()) }
    }

    /** This Device's own paired host on the network, while the app hosts. */
    private val advertiser by lazy { Advertiser(this) }

    /** The periodic sync the app runs for the worker, until the app says it is over. */
    private var inAppSync: CountDownLatch? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        if (intent?.getBooleanExtra(EXTRA_RING, false) == true) showOverLockScreen(true)
        if (savedInstanceState == null && intent?.getBooleanExtra(EXTRA_NUDGE, false) == true) nudgeOpened = true
        if (savedInstanceState == null && intent?.getBooleanExtra(EXTRA_CHANGES, false) == true) changesOpened = true
        if (savedInstanceState == null && opensUnsorted(intent)) unsortedOpened = true
        if (savedInstanceState == null) {
            CaptureModes.of(intent?.action, intent?.flags ?: 0)?.let { captureRequested = it }
        }
        super.onCreate(savedInstanceState)
    }

    /**
     * The full-screen alarm, the Nudge, the Changes Nudge or the Agenda
     * widget's Unsorted count opened the running app: the app shows it.
     */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (intent.getBooleanExtra(EXTRA_NUDGE, false)) {
            nudgeOpened = true
            actionsArrived()
        }
        if (intent.getBooleanExtra(EXTRA_CHANGES, false)) {
            changesOpened = true
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
        // A bundle was opened with the running app: it imports it now.
        if (intent.getBooleanExtra(EXTRA_BUNDLE, false)) {
            bundleChannel?.invokeMethod("opened", null)
        }
        // A Profile card link was tapped while the app runs: it opens it now.
        if (intent.getBooleanExtra(EXTRA_CARD, false)) {
            profileChannel?.invokeMethod("cardOpened", null)
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
                    "pick" -> pickPhotos(result)
                    "paste" -> paste(result)
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
                    "speed" -> {
                        voice.setSpeed((call.arguments as Number).toFloat())
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
                    "onMobileData" -> result.success(speech.onMobileData())
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
                    "transcribe" -> {
                        val request = Speech.request(call.arguments)
                        if (request == null) {
                            result.success(null)
                        } else {
                            speech.transcribe(request.bytes, request.prompts) {
                                runOnUiThread { result.success(it) }
                            }
                        }
                    }
                    "idle" -> {
                        speech.idle()
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        }
        profileChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, PROFILE_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "deviceName" -> result.success(Profiles.defaultName(this@MainActivity))
                    "pick" -> pickPicture(result)
                    "share" -> result.success(Cards.share(this@MainActivity, call.arguments as? String ?: ""))
                    "takeCards" -> result.success(Cards.take())
                    else -> result.notImplemented()
                }
            }
        }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, PAIRING_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "readQr" -> {
                        val photo = call.arguments as? ByteArray
                            ?: return@setMethodCallHandler result.success(null)
                        inBackground(result) { Pairing.readQr(photo) }
                    }
                    "deviceName" -> result.success(Pairing.deviceName())
                    else -> result.notImplemented()
                }
            }
        syncChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, SYNC_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                when (call.method) {
                    "start" -> {
                        discovery.start()
                        result.success(null)
                    }
                    "stop" -> {
                        discovery.stop()
                        result.success(null)
                    }
                    "background" -> {
                        val args = call.arguments as? Map<*, *>
                        SyncWork.schedule(this@MainActivity, args?.get("on") == true, args?.get("folder") == true)
                        result.success(null)
                    }
                    "localNetwork" -> result.success(SyncWork.onLocalNetwork(this@MainActivity))
                    "advertise" -> {
                        val args = call.arguments as? Map<*, *>
                        val device = args?.get("device") as? String
                        val port = (args?.get("port") as? Number)?.toInt()
                        if (device != null && port != null) advertiser.start(device, port)
                        result.success(null)
                    }
                    "unadvertise" -> {
                        advertiser.stop()
                        result.success(null)
                    }
                    "finished" -> {
                        inAppSync?.countDown()
                        inAppSync = null
                        result.success(null)
                    }
                    else -> result.notImplemented()
                }
            }
        }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, FolderChannel.CHANNEL)
            .setMethodCallHandler { call, result ->
                when {
                    call.method == "pickTree" -> pickFolder(result)
                    !FolderChannel.handle(this, call, result) -> result.notImplemented()
                }
            }
        bundleChannel = MethodChannel(flutterEngine.dartExecutor.binaryMessenger, BUNDLE_CHANNEL).apply {
            setMethodCallHandler { call, result ->
                val args = call.arguments as? Map<*, *> ?: emptyMap<Any, Any>()
                when (call.method) {
                    "exportPath" -> result.success(
                        Bundles.exportPath(this@MainActivity, args["name"] as? String ?: ""),
                    )
                    "share" -> shareBundle(
                        result,
                        args["path"] as? String ?: "",
                        args["type"] as? String ?: "application/octet-stream",
                    )
                    "shareText" -> shareWith(
                        result,
                        Bundles.textChooser(
                            this@MainActivity,
                            args["text"] as? String ?: "",
                            args["html"] as? String,
                            (args["paths"] as? List<*>).orEmpty().map { it as? String ?: "" },
                            (args["types"] as? List<*>).orEmpty().map { it as? String ?: "" },
                        ),
                    )
                    "pick" -> pickBundle(result)
                    "take" -> result.success(Bundles.take())
                    "print" -> result.success(
                        Printing.print(
                            this@MainActivity,
                            args["path"] as? String ?: "",
                            args["name"] as? String ?: "",
                        ),
                    )
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
                    "readCalendars" -> inBackground(result) { Appointments.calendars(this) }
                    "appointments" -> inBackground(result) {
                        Appointments.between(
                            this,
                            from = (args["from"] as Number).toLong(),
                            until = (args["until"] as Number).toLong(),
                            calendarIds = (args["calendars"] as List<*>).map { it as String },
                        )
                    }
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
        // Why the process died last time, as Android 11+ remembers it: a
        // battery saver's kill is not a crash and gets no report.
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, EXIT_CHANNEL)
            .setMethodCallHandler { call, result ->
                if (call.method == "lastExit") result.success(lastExit()) else result.notImplemented()
            }
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, LOCK_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "available" -> result.success(Biometrics.available(this))
                    "authenticate" -> {
                        val args = call.arguments as? Map<*, *> ?: emptyMap<Any, Any>()
                        Biometrics.authenticate(this, args["title"] as String? ?: "") { result.success(it) }
                    }
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
                    "nudgeOpened" -> {
                        result.success(nudgeOpened)
                        nudgeOpened = false
                    }
                    "changesOpened" -> {
                        result.success(changesOpened)
                        changesOpened = false
                    }
                    "unsortedOpened" -> {
                        result.success(unsortedOpened)
                        unsortedOpened = false
                    }
                    "request" -> {
                        Permissions.request(this@MainActivity, call.arguments as String)
                        result.success(null)
                    }
                    "pickSound" -> {
                        val args = call.arguments as Map<*, *>
                        pickSound(result, args["current"] as String?, args["title"] as String)
                    }
                    else -> if (!AlarmChannel.handle(this@MainActivity, call, result)) result.notImplemented()
                }
            }
        }
    }

    /**
     * Opens the share chooser for the file at [path]; the answer comes
     * once the user is back: "taken" when an app was picked, "dismissed"
     * when the chooser was left, "none" when nothing can take it.
     */
    private fun shareBundle(result: MethodChannel.Result, path: String, type: String) =
        shareWith(result, Bundles.chooser(this, path, type))

    /**
     * Opens the share [chooser], answering as [shareBundle] does; "none"
     * when there is no chooser.
     */
    private fun shareWith(result: MethodChannel.Result, chooser: Intent?) {
        if (chooser == null) return result.success("none")
        try {
            startActivity(chooser)
        } catch (_: ActivityNotFoundException) {
            return result.success("none")
        }
        shareAnswer?.success("dismissed")
        shareAnswer = result
        shareLeft = false
    }

    override fun onPause() {
        super.onPause()
        if (shareAnswer != null) shareLeft = true
    }

    override fun onResume() {
        super.onResume()
        if (!shareLeft) return
        val answer = shareAnswer ?: return
        shareAnswer = null
        shareLeft = false
        // The chooser tells of the app picked by a broadcast, which may
        // come just after the app is back.
        window.decorView.postDelayed(
            { answer.success(if (Bundles.chosen) "taken" else "dismissed") },
            SHARE_SETTLE_MS,
        )
    }

    override fun onDestroy() {
        shareAnswer?.success(if (Bundles.chosen) "taken" else "dismissed")
        shareAnswer = null
        if (active === this) active = null
        voice.release()
        speech.release()
        ocr.release()
        discovery.stop()
        advertiser.stop()
        inAppSync?.countDown()
        inAppSync = null
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
     * Reads the clipboard for Paste in Capture, only when it was tapped:
     * its text and its pictures, made into photos as the gallery's are.
     * The clip is read here, while the app has the focus; its pictures
     * are read and decoded off the main thread.
     */
    private fun paste(result: MethodChannel.Result) {
        val clip = runCatching { getSystemService(ClipboardManager::class.java)?.primaryClip }
            .getOrNull()
        val items = clip?.let { (0 until it.itemCount).map(it::getItemAt) }.orEmpty()
        inBackground(result) {
            val (text, pictures) = Clipboards.taken(
                items.map { item ->
                    val type = item.uri?.let { runCatching { contentResolver.getType(it) }.getOrNull() }
                    Clipboards.Item(item.text, item.uri, type)
                },
            )
            mapOf(
                "text" to text,
                "photos" to pictures.mapNotNull { uri ->
                    Photos.normalize { contentResolver.openInputStream(uri) }
                },
            )
        }
    }

    /**
     * Opens the system's picture picker for Capture, several pictures at
     * once, which needs no storage permission; they come back as photos
     * are kept, upright and without metadata, in [onActivityResult].
     */
    private fun pickPhotos(result: MethodChannel.Result) {
        val intent = Intent(Intent.ACTION_GET_CONTENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("image/*")
            .putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_GALLERY)
        } catch (_: ActivityNotFoundException) {
            return result.success(emptyList<ByteArray>())
        }
        galleryAnswer?.success(emptyList<ByteArray>())
        galleryAnswer = result
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

    /**
     * Opens the system's picture picker for the Profile picture, which
     * needs no storage permission; the picture comes back as photos are
     * kept, upright and without metadata, in [onActivityResult].
     */
    private fun pickPicture(result: MethodChannel.Result) {
        val intent = Intent(Intent.ACTION_GET_CONTENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("image/*")
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_PICTURE)
        } catch (_: ActivityNotFoundException) {
            return result.success(null)
        }
        pictureAnswer?.success(null)
        pictureAnswer = result
    }

    /**
     * Opens the system's sound picker for the alarm sound (#102), with
     * ringtones, alarm and notification sounds, the app's own as its
     * default, and no silence; the answer comes in [onActivityResult].
     */
    private fun pickSound(result: MethodChannel.Result, current: String?, title: String) {
        val own = Uri.parse(SoundChannel.ownSound(this))
        val intent = Intent(RingtoneManager.ACTION_RINGTONE_PICKER)
            .putExtra(RingtoneManager.EXTRA_RINGTONE_TYPE, RingtoneManager.TYPE_ALL)
            .putExtra(RingtoneManager.EXTRA_RINGTONE_SHOW_DEFAULT, true)
            .putExtra(RingtoneManager.EXTRA_RINGTONE_DEFAULT_URI, own)
            .putExtra(RingtoneManager.EXTRA_RINGTONE_SHOW_SILENT, false)
            .putExtra(RingtoneManager.EXTRA_RINGTONE_EXISTING_URI, current?.let(Uri::parse) ?: own)
            .putExtra(RingtoneManager.EXTRA_RINGTONE_TITLE, title)
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_SOUND)
        } catch (_: ActivityNotFoundException) {
            return result.success(null)
        }
        soundAnswer?.success(null)
        soundAnswer = result
    }

    /** Opens the system's file picker for a bundle to import; the answer comes in [onActivityResult]. */
    private fun pickBundle(result: MethodChannel.Result) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("*/*")
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_BUNDLE)
        } catch (_: ActivityNotFoundException) {
            return result.success(null)
        }
        bundleAnswer?.success(null)
        bundleAnswer = result
    }

    /** Opens the system's folder picker for the shared folder; the answer comes in [onActivityResult]. */
    private fun pickFolder(result: MethodChannel.Result) {
        folderAnswer?.success(null)
        folderAnswer = result
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).addFlags(
            Intent.FLAG_GRANT_READ_URI_PERMISSION or
                Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
                Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION,
        )
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_FOLDER)
        } catch (_: ActivityNotFoundException) {
            folderAnswer = null
            result.success(null)
        }
    }

    @Deprecated("The Flutter activity has no Activity Result API.")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_FOLDER) {
            val answer = folderAnswer ?: return
            folderAnswer = null
            val uri = if (resultCode == Activity.RESULT_OK) data?.data else null
            if (uri != null) {
                try {
                    // Kept across restarts, for the background sync too.
                    contentResolver.takePersistableUriPermission(
                        uri,
                        Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                    )
                } catch (_: SecurityException) {
                    // A provider granting nothing persistable: the folder
                    // works while the app runs.
                }
            }
            return answer.success(uri?.toString())
        }
        if (requestCode == REQUEST_BUNDLE) {
            val answer = bundleAnswer ?: return
            bundleAnswer = null
            val uri = data?.data
            if (resultCode != Activity.RESULT_OK || uri == null) return answer.success(null)
            return inBackground(answer) { Bundles.copy(this, uri) }
        }
        if (requestCode == REQUEST_SOUND) {
            val answer = soundAnswer ?: return
            soundAnswer = null
            val picked = if (resultCode == Activity.RESULT_OK && data != null) {
                @Suppress("DEPRECATION")
                data.getParcelableExtra<Uri>(RingtoneManager.EXTRA_RINGTONE_PICKED_URI)?.toString()
            } else {
                null
            }
            return answer.success(SoundChannel.picked(picked, SoundChannel.ownSound(this)))
        }
        if (requestCode == REQUEST_PICTURE) {
            val answer = pictureAnswer ?: return
            pictureAnswer = null
            val uri = data?.data
            if (resultCode != Activity.RESULT_OK || uri == null) return answer.success(null)
            return inBackground(answer) { Photos.normalize { contentResolver.openInputStream(uri) } }
        }
        if (requestCode == REQUEST_GALLERY) {
            val answer = galleryAnswer
            galleryAnswer = null
            val clips = data?.clipData
            val uris = Photos.picked(
                resultCode == Activity.RESULT_OK,
                clips?.let { (0 until it.itemCount).mapNotNull { i -> it.getItemAt(i).uri } },
                data?.data,
            )
            if (answer == null) {
                // The app was gone while the gallery was open: the
                // pictures still reach a Note, through Capture like a share.
                if (uris.isNotEmpty()) {
                    Thread {
                        Shares.keepPicked(this, uris)
                        runOnUiThread { photoChannel?.invokeMethod("shared", null) }
                    }.start()
                }
                return
            }
            if (uris.isEmpty()) return answer.success(emptyList<ByteArray>())
            return inBackground(answer) {
                uris.mapNotNull { uri -> Photos.normalize { contentResolver.openInputStream(uri) } }
            }
        }
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
    /**
     * The newest exit record of this app: the reason in a word, whether
     * the app was on screen, when, and the system's own description.
     * Null before Android 11 or when the system holds no record.
     */
    private fun lastExit(): Map<String, Any?>? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return null
        return try {
            val manager = getSystemService(ACTIVITY_SERVICE) as ActivityManager
            val info = manager.getHistoricalProcessExitReasons(packageName, 0, 1)
                .firstOrNull() ?: return null
            val reason = when (info.reason) {
                ApplicationExitInfo.REASON_CRASH -> "crash"
                ApplicationExitInfo.REASON_CRASH_NATIVE -> "native"
                ApplicationExitInfo.REASON_ANR -> "anr"
                ApplicationExitInfo.REASON_LOW_MEMORY -> "low_memory"
                else -> "other"
            }
            mapOf(
                "reason" to reason,
                "foreground" to
                    (info.importance <= ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND),
                "timestamp" to info.timestamp,
                "description" to (info.description ?: ""),
            )
        } catch (_: Exception) {
            null
        }
    }

    private fun opensUnsorted(intent: Intent?): Boolean =
        intent?.getBooleanExtra(EXTRA_UNSORTED, false) == true &&
            intent.flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY == 0

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
        private const val LOCK_CHANNEL = "io.github.paxel.oldbrainfix/lock"
        private const val EXIT_CHANNEL = "io.github.paxel.oldbrainfix/exit"
        private const val PAIRING_CHANNEL = "io.github.paxel.oldbrainfix/pairing"
        const val SYNC_CHANNEL = "io.github.paxel.oldbrainfix/sync"
        private const val BUNDLE_CHANNEL = "io.github.paxel.oldbrainfix/bundle"
        private const val PROFILE_CHANNEL = "io.github.paxel.oldbrainfix/profile"
        private const val REQUEST_LOCATION = 1
        // Not 2: Permissions.REQUEST_NOTIFICATIONS.
        private const val REQUEST_PHOTO = 3
        private const val REQUEST_MODEL = 4
        private const val REQUEST_FOLDER = 5
        private const val REQUEST_BUNDLE = 6
        private const val REQUEST_PICTURE = 7
        private const val REQUEST_GALLERY = 8
        private const val REQUEST_CALENDAR = 10
        private const val REQUEST_BLUETOOTH = 11
        private const val REQUEST_MICROPHONE = 12
        private const val REQUEST_CALENDAR_MIRROR = 13
        private const val REQUEST_SOUND = 14
        private var active: MainActivity? = null

        /** Marks the launch from the full-screen alarm's notification. */
        const val EXTRA_RING = "oldbrainfix.ring"

        /** Marks the launch from the Nudge's notification. */
        const val EXTRA_NUDGE = "oldbrainfix.nudge"

        /** Marks the launch from the Changes Nudge's notification. */
        const val EXTRA_CHANGES = "oldbrainfix.changes"

        /** How long the app waits, back from the share chooser, for it to tell of the app picked. */
        private const val SHARE_SETTLE_MS = 500L

        /** Marks the launch from the Agenda widget's Unsorted count. */
        const val EXTRA_UNSORTED = "oldbrainfix.unsorted"

        /** Marks the launch by a share from another app ([ShareActivity]). */
        const val EXTRA_SHARED = "oldbrainfix.shared"

        /** Marks the launch by a bundle another app opened with this one ([BundleActivity]). */
        const val EXTRA_BUNDLE = "oldbrainfix.bundle"

        /** Marks the launch by a Profile card link tapped in another app ([CardActivity]). */
        const val EXTRA_CARD = "oldbrainfix.card"

        /** The Nudge was tapped and the app has not shown the Nudge screen yet. */
        private var nudgeOpened = false

        /** The Changes Nudge was tapped and the app has not sent the changes yet. */
        private var changesOpened = false

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
         * The periodic sync came due: a running app syncs itself, with
         * its own store, and out of sight hosts for a while too. The
         * latch opens when the app says it is over; null when there was
         * no app to tell.
         */
        fun syncInApp(): CountDownLatch? {
            val activity = active ?: return null
            val over = CountDownLatch(1)
            activity.runOnUiThread {
                // One at a time: an earlier one still waiting is over.
                activity.inAppSync?.countDown()
                val channel = activity.syncChannel
                if (channel == null) {
                    over.countDown()
                    return@runOnUiThread
                }
                activity.inAppSync = over
                channel.invokeMethod("wanted", null)
            }
            return over
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
