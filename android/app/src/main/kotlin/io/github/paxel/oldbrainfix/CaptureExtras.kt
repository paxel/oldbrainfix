package io.github.paxel.oldbrainfix

import android.Manifest
import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.content.ContentUris
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.provider.CalendarContract
import java.util.TimeZone

/**
 * The app a share came from: best effort, as Android only sometimes
 * says. Its label is read at once, so a Note keeps the name the app had.
 */
object SourceApps {
    /** The chooser apps that pass a share on; never a source. */
    private val passers = setOf("android", "com.android.intentresolver", "com.android.internal.app")

    /**
     * The package of the first of [candidates] that names another app:
     * an `android-app://package` referrer or a plain package name. Web
     * referrers, the chooser and [own] are skipped.
     */
    fun packageOf(candidates: List<String?>, own: String): String? = candidates.asSequence()
        .mapNotNull { it?.trim()?.takeIf(String::isNotEmpty) }
        .mapNotNull { candidate ->
            when {
                candidate.startsWith(APP_SCHEME) ->
                    candidate.removePrefix(APP_SCHEME).substringBefore('/').takeIf(String::isNotEmpty)
                candidate.contains(':') || candidate.contains('/') -> null
                else -> candidate
            }
        }
        .firstOrNull { it != own && it !in passers }

    /** The package and label of the app that started [activity] with a share, or null. */
    fun of(activity: Activity): Pair<String, String?>? {
        val intent = activity.intent
        val launchedFrom = if (Build.VERSION.SDK_INT >= 34) activity.launchedFromPackage else null
        val pkg = packageOf(
            listOf(
                activity.referrer?.toString(),
                intent?.getStringExtra(Intent.EXTRA_REFERRER_NAME),
                activity.callingPackage,
                launchedFrom,
            ),
            activity.packageName,
        ) ?: return null
        val pm = activity.packageManager
        val label = runCatching {
            pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString()
        }.getOrNull()
        return pkg to label
    }

    private const val APP_SCHEME = "android-app://"
}

/**
 * The calendar events running now, from Android's calendar provider. Read
 * only when the user switched it on and allowed it.
 */
object Calendar {
    const val permission = Manifest.permission.READ_CALENDAR

    /** One occurrence of an event as the provider lists it. */
    data class Instance(
        val title: String?,
        val allDay: Boolean,
        val begin: Long,
        val end: Long,
        val declined: Boolean,
    )

    private const val DAY = 24L * 60 * 60 * 1000

    fun granted(context: Context): Boolean =
        context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    /**
     * The [instances] running at [now] that the user did not decline. An
     * all-day event's times are midnight UTC, so it runs while the local
     * date, [now] shifted by [utcOffset], lies inside them.
     */
    fun running(instances: List<Instance>, now: Long, utcOffset: Long): List<Instance> =
        instances.filter {
            val at = if (it.allDay) now + utcOffset else now
            !it.declined && !it.title.isNullOrBlank() && it.begin <= at && at < it.end
        }

    /** The events running now as the app's channel hands them over, or null. Slow: queries. */
    fun current(context: Context): List<Map<String, Any>>? {
        if (!granted(context)) return null
        val now = System.currentTimeMillis()
        val uri = CalendarContract.Instances.CONTENT_URI.buildUpon().also {
            ContentUris.appendId(it, now - DAY)
            ContentUris.appendId(it, now + DAY)
        }.build()
        val projection = arrayOf(
            CalendarContract.Instances.TITLE,
            CalendarContract.Instances.ALL_DAY,
            CalendarContract.Instances.BEGIN,
            CalendarContract.Instances.END,
            CalendarContract.Instances.SELF_ATTENDEE_STATUS,
        )
        val instances = context.contentResolver.query(
            uri,
            projection,
            "${CalendarContract.Instances.VISIBLE} = 1",
            null,
            null,
        )?.use { cursor ->
            buildList<Instance> {
                while (cursor.moveToNext()) {
                    add(
                        Instance(
                            title = cursor.getString(0),
                            allDay = cursor.getInt(1) != 0,
                            begin = cursor.getLong(2),
                            end = cursor.getLong(3),
                            declined = cursor.getInt(4) == CalendarContract.Attendees.ATTENDEE_STATUS_DECLINED,
                        ),
                    )
                }
            }
        } ?: return null
        return running(instances, now, TimeZone.getDefault().getOffset(now).toLong())
            .map { mapOf("title" to it.title.orEmpty(), "allDay" to it.allDay) }
    }
}

/**
 * The Bluetooth audio devices connected now, such as a car or headphones,
 * through the system's profile proxies. Never scans for others.
 */
object Bluetooth {
    /** Until Android 12 the names need no runtime permission. */
    val permission: String?
        get() = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) Manifest.permission.BLUETOOTH_CONNECT else null

    /** How long the profiles may take to answer. */
    private const val LIMIT_MS = 3_000L

    private const val LE_AUDIO = 22

    fun granted(context: Context): Boolean =
        permission?.let { context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED } ?: true

    /** The [names] worth keeping: trimmed, not blank, each once. */
    fun names(names: List<String?>): List<String> =
        names.mapNotNull { it?.trim()?.takeIf(String::isNotEmpty) }.distinct()

    /**
     * Hands [done] the names of the connected audio devices, or null when
     * access is missing or there is no Bluetooth. [done] runs at most
     * once, on the main thread.
     */
    @SuppressLint("MissingPermission")
    fun current(context: Context, done: (List<String>?) -> Unit) {
        val adapter = context.getSystemService(BluetoothManager::class.java)?.adapter
        if (!granted(context) || adapter == null) return done(null)
        if (!runCatching { adapter.isEnabled }.getOrDefault(false)) return done(emptyList())
        val profiles = buildList {
            add(BluetoothProfile.A2DP)
            add(BluetoothProfile.HEADSET)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) add(LE_AUDIO)
        }
        val found = mutableListOf<String?>()
        var waiting = profiles.size
        var answered = false
        val main = Handler(Looper.getMainLooper())
        val finish = Runnable {
            if (!answered) {
                answered = true
                done(names(found))
            }
        }
        main.postDelayed(finish, LIMIT_MS)
        val answer = Runnable {
            waiting--
            if (waiting == 0) {
                main.removeCallbacks(finish)
                finish.run()
            }
        }
        for (profile in profiles) {
            val listener = object : BluetoothProfile.ServiceListener {
                override fun onServiceConnected(profile: Int, proxy: BluetoothProfile) {
                    val devices = runCatching { proxy.connectedDevices }.getOrDefault(emptyList())
                    found += devices.map(::nameOf)
                    runCatching { adapter.closeProfileProxy(profile, proxy) }
                    main.post(answer)
                }

                override fun onServiceDisconnected(profile: Int) {}
            }
            val asked = runCatching { adapter.getProfileProxy(context, listener, profile) }.getOrDefault(false)
            if (!asked) main.post(answer)
        }
    }

    /** The name the user gave [device], else its own; null without access. */
    @SuppressLint("MissingPermission")
    private fun nameOf(device: BluetoothDevice): String? = runCatching {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) device.alias ?: device.name else device.name
    }.getOrNull()
}
