package io.github.paxel.oldbrainfix

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Attachments (#267): the name a picked or shared file goes by, and a
 * PDF's page as the native reader hands it over (android/ocr/src/pdf.rs).
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
}
