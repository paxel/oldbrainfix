package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VoiceTest {
    /** One MPEG-4 box: its 32-bit size, its type, then [body]. */
    private fun box(type: String, body: Int = 0): ByteArray {
        val size = 8 + body
        return byteArrayOf(
            (size ushr 24).toByte(),
            (size ushr 16).toByte(),
            (size ushr 8).toByte(),
            size.toByte(),
        ) + type.toByteArray(Charsets.US_ASCII) + ByteArray(body)
    }

    /** A box with the 64-bit size form: size 1, the type, then the size. */
    private fun largeBox(type: String, body: Int): ByteArray {
        val size = 16L + body
        val large = ByteArray(8) { ((size ushr (56 - 8 * it)) and 0xFF).toByte() }
        return byteArrayOf(0, 0, 0, 1) + type.toByteArray(Charsets.US_ASCII) + large + ByteArray(body)
    }

    @Test
    fun `a recording with its index is complete`() {
        assertTrue(Voice.complete(box("ftyp", 16) + box("mdat", 100) + box("moov", 40)))
        assertTrue(Voice.complete(box("ftyp", 16) + box("moov", 40) + box("mdat", 100)))
    }

    @Test
    fun `the 64-bit and the to-the-end box sizes are understood`() {
        assertTrue(Voice.complete(box("ftyp", 16) + largeBox("mdat", 30) + box("moov", 8)))
        val toTheEnd = box("ftyp", 16) + box("moov", 8) + byteArrayOf(0, 0, 0, 0) +
            "mdat".toByteArray(Charsets.US_ASCII) + ByteArray(50)
        assertTrue(Voice.complete(toTheEnd))
    }

    @Test
    fun `a recording cut off before its index is not`() {
        assertFalse(Voice.complete(box("ftyp", 16) + box("mdat", 100)))
        // The data box says it is longer than what is there.
        assertFalse(Voice.complete(box("ftyp", 16) + box("mdat", 100).copyOf(60)))
        // A few stray bytes after the last box.
        assertFalse(Voice.complete(box("ftyp", 16) + box("moov", 8) + byteArrayOf(1, 2, 3)))
    }

    @Test
    fun `anything that is no MPEG-4 file is not`() {
        assertFalse(Voice.complete(ByteArray(0)))
        assertFalse(Voice.complete(box("moov", 8) + box("ftyp", 8)))
        assertFalse(Voice.complete("RIFF....WAVEfmt ".toByteArray(Charsets.US_ASCII)))
        // A size smaller than its own header.
        assertFalse(Voice.complete(byteArrayOf(0, 0, 0, 4) + "ftyp".toByteArray(Charsets.US_ASCII)))
    }

    @Test
    fun `a desktop WAV recording is named wav and anything else m4a`() {
        val wav = "RIFF".toByteArray(Charsets.US_ASCII) + ByteArray(4) +
            "WAVEfmt ".toByteArray(Charsets.US_ASCII)
        assertEquals("wav", Voice.extension(wav))
        assertEquals("m4a", Voice.extension(box("ftyp", 16) + box("moov", 8)))
        assertEquals("m4a", Voice.extension("RIFF".toByteArray(Charsets.US_ASCII)))
        assertEquals("m4a", Voice.extension(ByteArray(0)))
    }

    @Test
    fun `a shared Ogg voice message is named ogg`() {
        // test/resources/voice-message.ogg: 0.2 s of a 440 Hz tone, 16 kHz
        // mono, encoded with libopus into an Ogg file, as messengers send.
        val message = javaClass.getResourceAsStream("/voice-message.ogg")!!.use { it.readBytes() }
        assertEquals("OpusHead", String(message, 28, 8, Charsets.US_ASCII))
        assertEquals("ogg", Voice.extension(message))
    }

    @Test
    fun `an MP3 file is named mp3, by its tag or its first frame`() {
        assertEquals("mp3", Voice.extension("ID3".toByteArray(Charsets.US_ASCII) + byteArrayOf(4, 0, 0, 0)))
        // MPEG-1 layer III, the usual MP3 frame header.
        assertEquals("mp3", Voice.extension(byteArrayOf(0xFF.toByte(), 0xFB.toByte(), 0x90.toByte(), 0)))
        // ADTS AAC shares the sync bits but has no layer: not MP3.
        assertEquals("m4a", Voice.extension(byteArrayOf(0xFF.toByte(), 0xF1.toByte(), 0x50, 0x80.toByte())))
    }
}
