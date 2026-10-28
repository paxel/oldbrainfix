package io.github.paxel.oldbrainfix

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Handler
import android.os.Looper

/**
 * Tells the network about this Device's paired host (ADR 0003) with
 * Android's NsdManager (no Google service), as the desktop does: the
 * service type `_oldbrainfix._tcp`, the device id as the instance name
 * and as the only TXT entry ([NsdRecords.own]), on the host's port.
 * Paired Devices check the certificate against the one they pinned, so
 * the record only says where to try.
 */
class Advertiser(context: Context) {
    private val nsd = context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val main = Handler(Looper.getMainLooper())
    private var listener: NsdManager.RegistrationListener? = null
    private var advertised: Pair<String, Int>? = null

    /** Advertises [device]'s host on [port], in place of what was advertised before. */
    fun start(device: String, port: Int) {
        if (listener != null && advertised == device to port) return
        stop()
        val txt = NsdRecords.own(device, port) ?: return
        val info = NsdServiceInfo().apply {
            serviceName = device
            serviceType = NsdRecords.SERVICE_TYPE
            setPort(port)
            txt.forEach { (key, value) -> setAttribute(key, value) }
        }
        val registering = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {}
            override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                main.post {
                    if (listener === this) {
                        listener = null
                        advertised = null
                    }
                }
            }
            override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {}
            override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {}
        }
        listener = registering
        advertised = device to port
        try {
            nsd.registerService(info, NsdManager.PROTOCOL_DNS_SD, registering)
        } catch (_: RuntimeException) {
            listener = null
            advertised = null
        }
    }

    /** Takes the record off the network. */
    fun stop() {
        val running = listener ?: return
        listener = null
        advertised = null
        try {
            nsd.unregisterService(running)
        } catch (_: RuntimeException) {
            // Never registered, or already gone.
        }
    }
}
