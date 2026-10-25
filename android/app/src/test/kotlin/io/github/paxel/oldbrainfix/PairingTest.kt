package io.github.paxel.oldbrainfix

import com.google.zxing.BarcodeFormat
import com.google.zxing.qrcode.QRCodeWriter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PairingTest {
    private val code = "r2m02_5xgnw_0aeg8_3188h_g7s65_mt3pg_j9a1b_nwsbc_edx83_24fjt_et9ax_jq70c_fkpnv_g"

    /** [text] as a QR, [scale] pixels per module, dark on light or the other way round. */
    private fun picture(text: String, size: Int, dark: Int, light: Int): IntArray {
        val matrix = QRCodeWriter().encode(text, BarcodeFormat.QR_CODE, size, size)
        return IntArray(size * size) { i -> if (matrix.get(i % size, i / size)) dark else light }
    }

    @Test
    fun readsTheCodeOffAScreen() {
        val pixels = picture(code, 300, 0xFF000000.toInt(), 0xFFFFFFFF.toInt())
        assertEquals(code, Pairing.readQr(pixels, 300, 300))
    }

    @Test
    fun readsALightCodeOnADarkScreen() {
        val pixels = picture(code, 300, 0xFFFFFFFF.toInt(), 0xFF202020.toInt())
        assertEquals(code, Pairing.readQr(pixels, 300, 300))
    }

    @Test
    fun aPictureWithoutACodeReadsAsNone() {
        val grey = IntArray(200 * 200) { 0xFF808080.toInt() }
        assertNull(Pairing.readQr(grey, 200, 200))
    }

    @Test
    fun theNameIsMakerAndModelOnce() {
        assertEquals("Google Pixel 8", Pairing.deviceName("Google", "Pixel 8"))
        assertEquals("Samsung SM-A515F", Pairing.deviceName("samsung", "SM-A515F"))
        assertEquals("OnePlus 9", Pairing.deviceName("OnePlus", "OnePlus 9"))
        assertEquals("Fairphone", Pairing.deviceName("Fairphone", ""))
    }
}
