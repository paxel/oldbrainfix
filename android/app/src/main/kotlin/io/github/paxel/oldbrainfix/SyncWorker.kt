package io.github.paxel.oldbrainfix

import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.work.Worker
import androidx.work.WorkerParameters
import io.flutter.FlutterInjector
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.embedding.engine.dart.DartExecutor
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

/**
 * One background sync while the app is closed: the store and the sync
 * are Dart's, so the worker starts a Flutter engine of its own without
 * any screen and runs `backgroundSyncMain` (lib/main.dart) in it, which
 * applies what was tapped on notifications, finds the paired hosts here
 * ([Discovery]), syncs and arms the alarms anew. The engine is gone when
 * Dart says it finished, or after [LIMIT_MINUTES] at the latest. While
 * the app is open it syncs itself instead.
 */
class SyncWorker(context: Context, params: WorkerParameters) : Worker(context, params) {
    private val main = Handler(Looper.getMainLooper())
    private val over = CountDownLatch(1)
    private var engine: FlutterEngine? = null
    private var discovery: Discovery? = null

    override fun doWork(): Result {
        if (!SyncWork.onLocalNetwork(applicationContext) || MainActivity.syncInApp()) return Result.success()
        main.post { start() }
        over.await(LIMIT_MINUTES, TimeUnit.MINUTES)
        main.post { finish() }
        return Result.success()
    }

    override fun onStopped() {
        main.post { finish() }
    }

    private fun start() {
        if (over.count == 0L) return
        val context = applicationContext
        val started = try {
            FlutterEngine(context)
        } catch (_: RuntimeException) {
            return finish()
        }
        engine = started
        val messenger = started.dartExecutor.binaryMessenger
        MethodChannel(messenger, MainActivity.SYNC_CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                "discover" -> discover((call.arguments as? Number)?.toLong() ?: LOOK_MS, result)
                "finished" -> {
                    result.success(null)
                    finish()
                }
                "start", "stop", "background" -> result.success(null)
                else -> result.notImplemented()
            }
        }
        MethodChannel(messenger, ALARM_CHANNEL).setMethodCallHandler { call, result ->
            when (call.method) {
                // Nothing was tapped to open: there is no screen.
                "nudgeOpened", "unsortedOpened" -> result.success(false)
                else -> if (!AlarmChannel.handle(context, call, result)) result.notImplemented()
            }
        }
        MethodChannel(messenger, PAIRING_CHANNEL).setMethodCallHandler { call, result ->
            if (call.method == "deviceName") result.success(Pairing.deviceName()) else result.notImplemented()
        }
        val bundle = FlutterInjector.instance().flutterLoader().findAppBundlePath()
        started.dartExecutor.executeDartEntrypoint(DartExecutor.DartEntrypoint(bundle, ENTRYPOINT))
    }

    /** Looks for paired hosts for [millis] and answers with what was found. */
    private fun discover(millis: Long, result: MethodChannel.Result) {
        val found = linkedMapOf<String, Found>()
        val looking = Discovery(applicationContext) { found[it.device] = it }
        discovery = looking
        looking.start()
        main.postDelayed({
            looking.stop()
            if (discovery === looking) discovery = null
            result.success(found.values.map { it.toMap() })
        }, millis)
    }

    private fun finish() {
        discovery?.stop()
        discovery = null
        engine?.destroy()
        engine = null
        over.countDown()
    }

    companion object {
        /** WorkManager stops a worker after 10 minutes; this one ends before. */
        const val LIMIT_MINUTES = 9L
        const val LOOK_MS = 8_000L
        const val ENTRYPOINT = "backgroundSyncMain"
        private const val ALARM_CHANNEL = "io.github.paxel.oldbrainfix/alarms"
        private const val PAIRING_CHANNEL = "io.github.paxel.oldbrainfix/pairing"
    }
}
