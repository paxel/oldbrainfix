package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class NsdRecordsTest {
    private val id = "3f9a0c1d2e4b5a6c7d8e9f0a1b2c3d4e"

    private fun txt(vararg entries: Pair<String, String?>): Map<String, ByteArray?> =
        entries.associate { (k, v) -> k to v?.toByteArray(Charsets.US_ASCII) }

    @Test
    fun aRecordNamesTheDeviceAndWhereItAnswers() {
        assertEquals(Found(id, "192.168.1.5", 45231), NsdRecords.found(txt("id" to id), "192.168.1.5", 45231))
        assertEquals(mapOf("device" to id, "host" to "192.168.1.5", "port" to 45231), Found(id, "192.168.1.5", 45231).toMap())
    }

    @Test
    fun aRecordWithoutAProperDeviceIdIsNone() {
        assertNull(NsdRecords.deviceId(txt()))
        assertNull(NsdRecords.deviceId(txt("id" to null)))
        assertNull(NsdRecords.deviceId(txt("id" to "Patrick's laptop")))
        assertNull(NsdRecords.deviceId(txt("id" to id.uppercase())))
        assertNull(NsdRecords.deviceId(txt("id" to id + "0")))
        assertEquals(id, NsdRecords.deviceId(txt("ID" to id, "other" to "x")))
    }

    @Test
    fun onlyAnIpv4AddressAndARealPortCount() {
        assertNull(NsdRecords.found(txt("id" to id), null, 45231))
        assertNull(NsdRecords.found(txt("id" to id), "fe80::1", 45231))
        assertNull(NsdRecords.found(txt("id" to id), "192.168.1.5", 0))
        assertNull(NsdRecords.found(txt("id" to id), "192.168.1.5", 70000))
    }

    @Test
    fun theOwnRecordCarriesTheDeviceIdOnly() {
        assertEquals(mapOf("id" to id), NsdRecords.own(id, 45231))
        val heard = NsdRecords.own(id, 45231)!!.mapValues { it.value.toByteArray(Charsets.US_ASCII) }
        assertEquals(Found(id, "192.168.1.5", 45231), NsdRecords.found(heard, "192.168.1.5", 45231))
    }

    @Test
    fun nothingIsAdvertisedWithoutADeviceIdAndARealPort() {
        assertNull(NsdRecords.own("Patrick's phone", 45231))
        assertNull(NsdRecords.own(id.uppercase(), 45231))
        assertNull(NsdRecords.own(id, 0))
        assertNull(NsdRecords.own(id, 70000))
    }

    @Test
    fun theServiceTypeIsReadAsNsdManagerReportsIt() {
        assertTrue(NsdRecords.isOurs("_oldbrainfix._tcp"))
        assertTrue(NsdRecords.isOurs("._oldbrainfix._tcp."))
        assertTrue(NsdRecords.isOurs("_oldbrainfix._tcp.local."))
        assertFalse(NsdRecords.isOurs("_catlog._tcp."))
        assertFalse(NsdRecords.isOurs(null))
    }
}
