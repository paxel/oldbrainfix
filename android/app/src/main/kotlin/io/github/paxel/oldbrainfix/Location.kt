package io.github.paxel.oldbrainfix

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationListener
import android.location.LocationManager
import android.os.Build
import android.os.Bundle
import android.os.Looper
import android.os.SystemClock

/**
 * Where the phone is, from Android's own LocationManager: no Google
 * library. Nothing here ever asks the user; asking is [MainActivity]'s.
 */
object Location {
    val permissions = arrayOf(
        Manifest.permission.ACCESS_FINE_LOCATION,
        Manifest.permission.ACCESS_COARSE_LOCATION,
    )

    /** A last known fix younger than this is taken as the current one. */
    private const val FRESH_NANOS = 2L * 60 * 1_000_000_000

    /** Whether the user allowed a precise or an approximate location. */
    fun granted(context: Context): Boolean =
        permissions.any { context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }

    /**
     * Hands [done] the position as [latitude, longitude], or null when
     * access is missing, location is off or no provider answers. [done]
     * runs at most once, on the main thread.
     */
    @SuppressLint("MissingPermission")
    fun current(context: Context, done: (List<Double>?) -> Unit) {
        var answered = false
        val once = { fix: android.location.Location? ->
            if (!answered) {
                answered = true
                done(fix?.let { listOf(it.latitude, it.longitude) })
            }
        }
        val manager = context.getSystemService(LocationManager::class.java)
        if (!granted(context) || manager == null) return once(null)
        val providers = buildList {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) add(LocationManager.FUSED_PROVIDER)
            add(LocationManager.NETWORK_PROVIDER)
            add(LocationManager.GPS_PROVIDER)
        }.filter { runCatching { manager.isProviderEnabled(it) }.getOrDefault(false) }
        val fresh = providers
            .mapNotNull { runCatching { manager.getLastKnownLocation(it) }.getOrNull() }
            .filter { SystemClock.elapsedRealtimeNanos() - it.elapsedRealtimeNanos < FRESH_NANOS }
            .minByOrNull { it.accuracy }
        if (fresh != null) return once(fresh)
        val provider = providers.firstOrNull() ?: return once(null)
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                manager.getCurrentLocation(provider, null, context.mainExecutor) { once(it) }
            } else {
                // Older Androids call every method of the listener.
                val listener = object : LocationListener {
                    override fun onLocationChanged(location: android.location.Location) = once(location)

                    @Deprecated("Deprecated in Android")
                    override fun onStatusChanged(provider: String?, status: Int, extras: Bundle?) {}

                    override fun onProviderEnabled(provider: String) {}

                    override fun onProviderDisabled(provider: String) = once(null)
                }
                @Suppress("DEPRECATION")
                manager.requestSingleUpdate(provider, listener, Looper.getMainLooper())
            }
        } catch (e: RuntimeException) {
            // A provider the granted permission does not cover.
            once(null)
        }
    }
}
