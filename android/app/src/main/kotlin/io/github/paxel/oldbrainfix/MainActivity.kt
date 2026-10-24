package io.github.paxel.oldbrainfix

import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject

class MainActivity : FlutterActivity() {
    /** The answer to "request", until the permission dialog comes back. */
    private var permissionAnswer: MethodChannel.Result? = null

    private var alarmChannel: MethodChannel? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        if (intent?.getBooleanExtra(EXTRA_RING, false) == true) showOverLockScreen(true)
        super.onCreate(savedInstanceState)
    }

    /** The full-screen alarm opened the running app: the app shows it. */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
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
        super.onDestroy()
    }

    private fun request(result: MethodChannel.Result) {
        if (Location.granted(this)) return result.success(true)
        permissionAnswer?.success(false)
        permissionAnswer = result
        requestPermissions(Location.permissions, REQUEST_LOCATION)
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
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
        private const val REQUEST_LOCATION = 1
        private var active: MainActivity? = null

        /** Marks the launch from the full-screen alarm's notification. */
        const val EXTRA_RING = "oldbrainfix.ring"

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
