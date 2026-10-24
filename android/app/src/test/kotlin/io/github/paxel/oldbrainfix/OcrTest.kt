package io.github.paxel.oldbrainfix

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Test
import java.io.File
import java.security.MessageDigest

class OcrTest {
    @Test
    fun `ARGB pixels become RGBA bytes in the same order`() {
        assertArrayEquals(
            byteArrayOf(
                0x11, 0x22, 0x33, 0xFF.toByte(),
                0xFF.toByte(), 0x00, 0x80.toByte(), 0x7F,
            ),
            Ocr.rgba(intArrayOf(0xFF112233.toInt(), 0x7FFF0080)),
        )
    }

    @Test
    fun `no pixels are no bytes`() {
        assertEquals(0, Ocr.rgba(IntArray(0)).size)
    }

    /**
     * The models bundled for ocrs are exactly those from ocrs' own model
     * host (ocrs-models.s3-accelerate.amazonaws.com, January 2024), as
     * named in assets/ocrs/NOTICE.
     */
    @Test
    fun `the bundled models are the pinned ones`() {
        val pinned = mapOf(
            Ocr.DETECTION to "f15cfb56bd02c4bf478a20343986504a1f01e1665c2b3a0ad66340f054b1b5ca",
            Ocr.RECOGNITION to "e484866d4cce403175bd8d00b128feb08ab42e208de30e42cd9889d8f1735a6e",
        )
        for ((name, sha256) in pinned) {
            val bytes = File("src/main/assets/${Ocr.ASSETS}/$name").readBytes()
            val hash = MessageDigest.getInstance("SHA-256").digest(bytes)
                .joinToString("") { "%02x".format(it) }
            assertEquals(name, sha256, hash)
        }
    }
}
