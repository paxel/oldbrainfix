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
 * its own or a shared folder is chosen; no persistent connection.
 * Without a shared folder it waits for a network, and the worker itself
 * goes on only on Wi-Fi or Ethernet, where a computer at home can be,
 * so a Wi-Fi marked metered still syncs and mobile data never wakes the
 * phone for nothing. A shared folder needs no network of the app's own:
 * Syncthing, a cloud app or a USB stick carries it.
 */
object SyncWork {
    private const val NAME = "sync"
    private const val PREFS = "sync"
    private const val FOLDER = "folder"
    const val PERIOD_MINUTES = 15L

    /**
     * Turns the periodic work on, keeping its rhythm when it already
     * runs, or off; with a shared [folder] it runs without a network too.
     */
    fun schedule(context: Context, on: Boolean, folder: Boolean) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putBoolean(FOLDER, on && folder).apply()
        val work = WorkManager.getInstance(context)
        if (!on) {
            work.cancelUniqueWork(NAME)
            return
        }
        val request = PeriodicWorkRequest.Builder(SyncWorker::class.java, PERIOD_MINUTES, TimeUnit.MINUTES)
            .setConstraints(constraints(folder))
            .build()
        work.enqueueUniquePeriodicWork(NAME, ExistingPeriodicWorkPolicy.UPDATE, request)
    }

    /** What the work waits for: a network, unless a shared folder is chosen. */
    fun constraints(folder: Boolean): Constraints = Constraints.Builder()
        .setRequiredNetworkType(if (folder) NetworkType.NOT_REQUIRED else NetworkType.CONNECTED)
        .build()

    /** Whether a shared folder is chosen, as the app last said. */
    fun folderOn(context: Context): Boolean =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(FOLDER, false)

    /** Whether the phone is on a local network now: Wi-Fi or Ethernet. */
    fun onLocalNetwork(context: Context): Boolean {
        val connectivity = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val capabilities = connectivity.getNetworkCapabilities(connectivity.activeNetwork) ?: return false
        return capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) ||
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)
    }
}
