package io.github.paxel.oldbrainfix

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequest
import androidx.work.WorkManager
import java.util.concurrent.TimeUnit

/**
 * The background sync's periodic work (ADR 0003): about every 15
 * minutes, Android's shortest period, while some paired Device syncs on
 * its own; no persistent connection. It waits for a network; the
 * worker itself goes on only on Wi-Fi or Ethernet, where a computer at
 * home can be, so a Wi-Fi marked metered still syncs and mobile data
 * never wakes the phone for nothing.
 */
object SyncWork {
    private const val NAME = "sync"
    const val PERIOD_MINUTES = 15L

    /** Turns the periodic work on, keeping its rhythm when it already runs, or off. */
    fun schedule(context: Context, on: Boolean) {
        val work = WorkManager.getInstance(context)
        if (!on) {
            work.cancelUniqueWork(NAME)
            return
        }
        val constraints = Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
        val request = PeriodicWorkRequest.Builder(SyncWorker::class.java, PERIOD_MINUTES, TimeUnit.MINUTES)
            .setConstraints(constraints)
            .build()
        work.enqueueUniquePeriodicWork(NAME, ExistingPeriodicWorkPolicy.KEEP, request)
    }

    /** Whether the phone is on a local network now: Wi-Fi or Ethernet. */
    fun onLocalNetwork(context: Context): Boolean {
        val connectivity = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val capabilities = connectivity.getNetworkCapabilities(connectivity.activeNetwork) ?: return false
        return capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
    }
}
