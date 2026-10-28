package io.github.paxel.oldbrainfix

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Handler
import android.os.Looper
import java.net.Inet4Address

/** A paired Device's host as its record names it: its device id and where it answers. */
data class Found(val device: String, val host: String, val port: Int) {
    fun toMap(): Map<String, Any> = mapOf("device" to device, "host" to host, "port" to port)
}

/**
 * What an mDNS/DNS-SD record of a paired host says (the desktop core's
 * `discovery`, and [Advertiser] here): the service type, and the device
 * id as the instance name and its only TXT entry. Plain values, so the
 * JVM tests can read and make records.
 */
object NsdRecords {
    /** The service type as NsdManager takes it. */
    const val SERVICE_TYPE = "_oldbrainfix._tcp"

    const val ID_KEY = "id"

    /** Whether [type], as NsdManager reports it ("._oldbrainfix._tcp." and the like), is ours. */
    fun isOurs(type: String?): Boolean = type?.trim('.')?.removeSuffix(".local")?.trim('.') == SERVICE_TYPE

    /** The device id in the TXT [attributes]: 32 lowercase hex digits, or null. */
    fun deviceId(attributes: Map<String, ByteArray?>): String? {
        val raw = attributes.entries.firstOrNull { it.key.equals(ID_KEY, ignoreCase = true) }?.value ?: return null
        val id = String(raw, Charsets.US_ASCII)
        return id.takeIf { ID.matches(it) }
    }

    /**
     * The TXT entries of this Device's own record: its [device] id only,
     * no name, since anyone on the network hears it. Null when [device]
     * is no device id or [port] no real port: then nothing is advertised.
     */
    fun own(device: String, port: Int): Map<String, String>? =
        if (ID.matches(device) && port in 1..65535) mapOf(ID_KEY to device) else null

    /** The host the record names, when it gives a device id, an IPv4 [host] and a [port]. */
    fun found(attributes: Map<String, ByteArray?>, host: String?, port: Int): Found? {
        val device = deviceId(attributes) ?: return null
        if (host == null || !IPV4.matches(host) || port !in 1..65535) return null
        return Found(device, host, port)
    }

    private val ID = Regex("[0-9a-f]{32}")
    private val IPV4 = Regex("""\d{1,3}(\.\d{1,3}){3}""")
}

/**
 * Looks for paired hosts on the network with Android's NsdManager (no
 * Google service): each record found is resolved, one at a time as older
 * Androids need it, and what it names is handed to [onFound] on the main
 * thread. The phone checks the host's certificate against the one it
 * pinned before it sends anything, so a record only says where to try.
 */
class Discovery(context: Context, private val onFound: (Found) -> Unit) {
    private val nsd = context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val main = Handler(Looper.getMainLooper())
    private var listener: NsdManager.DiscoveryListener? = null
    private val waiting = ArrayDeque<NsdServiceInfo>()
    private var resolving = false

    /** Starts looking, unless it already does. */
    fun start() {
        if (listener != null) return
        val started = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(serviceType: String) {}
            override fun onDiscoveryStopped(serviceType: String) {}
            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                main.post { if (listener === this) listener = null }
            }
            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {}
            override fun onServiceFound(service: NsdServiceInfo) {
                if (!NsdRecords.isOurs(service.serviceType)) return
                main.post { resolve(service) }
            }
            override fun onServiceLost(service: NsdServiceInfo) {}
        }
        listener = started
        try {
            nsd.discoverServices(NsdRecords.SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, started)
        } catch (_: RuntimeException) {
            listener = null
        }
    }

    /** Stops looking. */
    fun stop() {
        val running = listener ?: return
        listener = null
        waiting.clear()
        try {
            nsd.stopServiceDiscovery(running)
        } catch (_: RuntimeException) {
            // Never started, or already gone.
        }
    }

    private fun resolve(service: NsdServiceInfo) {
        if (listener == null) return
        if (resolving) {
            waiting.addLast(service)
            return
        }
        resolving = true
        @Suppress("DEPRECATION")
        nsd.resolveService(
            service,
            object : NsdManager.ResolveListener {
                override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                    main.post { next() }
                }

                override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                    @Suppress("DEPRECATION")
                    val address = serviceInfo.host as? Inet4Address
                    val found = NsdRecords.found(serviceInfo.attributes, address?.hostAddress, serviceInfo.port)
                    main.post {
                        if (found != null && listener != null) onFound(found)
                        next()
                    }
                }
            },
        )
    }

    private fun next() {
        resolving = false
        waiting.removeFirstOrNull()?.let { resolve(it) }
    }
}
