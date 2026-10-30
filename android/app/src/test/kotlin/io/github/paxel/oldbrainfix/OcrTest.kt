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
     * The model files bundled for RustO are exactly those of RustO's
     * release models-rten-v0.3.2 (RustO-Models-PPOCRv6-Tiny.zip), as
     * named in assets/rusto/NOTICE.
     */
    @Test
    fun `the bundled models are the pinned ones`() {
        val pinned = mapOf(
            Ocr.DETECTION to "ca1f0196d2a2da2b485e2cda6aaf5ba02258ac71127d577c8a77a75e89b67605",
            Ocr.RECOGNITION to "6c94c6b42c43a39d11cf3dd1267199ba04a11afe9de43632daa6f3983b365a9b",
            Ocr.CHARACTERS to "c5cbe34ef40c29c4df07ed012bf96569cb69a2d2a01a07027e9f13cb832bd9cd",
        )
        for ((name, sha256) in pinned) {
            val bytes = File("src/main/assets/${Ocr.ASSETS}/$name").readBytes()
            val hash = MessageDigest.getInstance("SHA-256").digest(bytes)
                .joinToString("") { "%02x".format(it) }
            assertEquals(name, sha256, hash)
        }
    }
}
