package io.github.paxel.oldbrainfix

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    /** The answer to "request", until the permission dialog comes back. */
    private var permissionAnswer: MethodChannel.Result? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, LOCATION_CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "granted" -> result.success(Location.granted(this))
                    "request" -> request(result)
                    "current" -> Location.current(this) { result.success(it) }
                    else -> result.notImplemented()
                }
            }
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

    companion object {
        private const val LOCATION_CHANNEL = "io.github.paxel.oldbrainfix/location"
        private const val REQUEST_LOCATION = 1
    }
}
