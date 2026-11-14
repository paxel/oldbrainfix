package io.github.paxel.oldbrainfix

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Attachments (#267): the name a picked or shared file goes by, a
 * PDF's page as the native reader hands it over (android/ocr/src/pdf.rs),
 * and the size a page is drawn at for the viewer (#286).
 */
class FilesTest {
    @Test
    fun `a file goes by its display name, else the end of its address, else file`() {
        assertEquals("Lease 2026.pdf", Files.nameOf("Lease 2026.pdf", "document/primary:Lease.pdf"))
        assertEquals("Lease.pdf", Files.nameOf(null, "primary:Download/Lease.pdf"))
        assertEquals("Lease.pdf", Files.nameOf(" ", "Lease.pdf"))
        assertEquals("file", Files.nameOf(null, null))
        assertEquals("file", Files.nameOf("", "folder/"))
    }

    @Test
    fun `a page is its text's length, its text and its scan`() {
        val text = "Grüße".toByteArray(Charsets.UTF_8)
        val plain = byteArrayOf(0, 0, 0, text.size.toByte()) + text
        val page = PdfNative.decode(plain)!!
        assertEquals("Grüße", page["text"])
        assertNull(page["scan"])

        val scanned = PdfNative.decode(byteArrayOf(0, 0, 0, 0, -1, -40, -1))!!
        assertEquals("", scanned["text"])
        assertArrayEquals(byteArrayOf(-1, -40, -1), scanned["scan"] as ByteArray)
    }

    @Test
    fun `bytes that are no page are none`() {
        assertNull(PdfNative.decode(byteArrayOf(0, 0)))
        assertNull(PdfNative.decode(byteArrayOf(0, 0, 0, 9, 65)))
        assertNull(PdfNative.decode(byteArrayOf(-1, -1, -1, -1, 65)))
    }

    @Test
    fun `a page is drawn as wide as asked, in its proportions, within the caps`() {
        // A4 at 595 by 842 points, on a phone 1080 pixels wide.
        assertEquals(1080 to 1528, Files.renderSize(595, 842, 1080))
        // Zoomed six times it stays within 8 Mi pixels and its proportions.
        val (wide, high) = Files.renderSize(595, 842, 6480)!!
        assertTrue(wide.toLong() * high <= Files.MAX_RENDER_PIXELS)
        assertEquals(595.0 / 842, wide.toDouble() / high, 0.01)
        // A strip of a receipt stays within the longest side.
        assertEquals(Files.MAX_RENDER_SIDE, Files.renderSize(100, 10_000, 1080)!!.second)
        assertEquals(1 to 1, Files.renderSize(595, 842, 0))
        assertNull(Files.renderSize(0, 842, 1080))
    }
}
