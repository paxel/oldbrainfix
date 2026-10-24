package io.github.paxel.oldbrainfix

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream

class SpeechTest {
    @Test
    fun `16-bit samples become floats from -1 to 1`() {
        assertArrayEquals(
            floatArrayOf(0f, 0.5f, -0.5f, -1f, 32767f / 32768f),
            Speech.pcm16ToFloat(shortArrayOf(0, 16384, -16384, Short.MIN_VALUE, Short.MAX_VALUE)),
            0f,
        )
    }

    @Test
    fun `stereo is averaged to mono and a cut-off frame dropped`() {
        assertArrayEquals(
            floatArrayOf(0.5f, 0f),
            Speech.downmix(floatArrayOf(1f, 0f, 0.5f, -0.5f, 0.25f), 2),
            1e-6f,
        )
    }

    @Test
    fun `mono stays as it is`() {
        val mono = floatArrayOf(0.1f, 0.2f)
        assertSame(mono, Speech.downmix(mono, 1))
    }

    @Test
    fun `a recording at Whisper's rate is not resampled`() {
        val samples = floatArrayOf(0.1f, 0.2f, 0.3f)
        assertSame(samples, Speech.toWhisper(samples, 1, 16_000))
    }

    @Test
    fun `twice the rate keeps every other sample`() {
        assertArrayEquals(
            floatArrayOf(0f, 2f, 4f),
            Speech.resample(floatArrayOf(0f, 1f, 2f, 3f, 4f, 5f), 32_000, 16_000),
            1e-6f,
        )
    }

    @Test
    fun `half the rate interpolates between samples`() {
        assertArrayEquals(
            floatArrayOf(0f, 0.5f, 1f, 1f),
            Speech.resample(floatArrayOf(0f, 1f), 8_000, 16_000),
            1e-6f,
        )
    }

    @Test
    fun `44,1 kHz stereo comes out as 16 kHz mono`() {
        val second = FloatArray(44_100 * 2) { if (it % 2 == 0) 0.5f else -0.5f }
        val whisper = Speech.toWhisper(second, 2, 44_100)
        assertEquals(16_000, whisper.size)
        assertEquals(0f, whisper.maxOf { kotlin.math.abs(it) }, 1e-6f)
    }

    @Test
    fun `Whisper gets one core less than there are, at most four`() {
        assertEquals(1, Speech.threadsFor(1))
        assertEquals(1, Speech.threadsFor(2))
        assertEquals(3, Speech.threadsFor(4))
        assertEquals(4, Speech.threadsFor(8))
    }

    @Test
    fun `copying hashes what was copied`() {
        val out = ByteArrayOutputStream()
        val hash = Speech.copyHashing(ByteArrayInputStream("abc".toByteArray()), out)
        assertEquals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad", hash)
        assertArrayEquals("abc".toByteArray(), out.toByteArray())
    }

    @Test
    fun `nothing copied hashes as the empty input`() {
        val hash = Speech.copyHashing(ByteArrayInputStream(ByteArray(0)), ByteArrayOutputStream())
        assertEquals("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", hash)
    }

    @Test
    fun `progress comes about every megabyte and at the end`() {
        val told = mutableListOf<Long>()
        val size = 3 * (1 shl 20) + 5
        Speech.copyHashing(ByteArrayInputStream(ByteArray(size)), ByteArrayOutputStream(), copied = { told += it })
        assertEquals(listOf(1L shl 20, 2L shl 20, 3L shl 20, size.toLong()), told)
    }

    @Test
    fun `a stopped copy has no hash`() {
        val hash = Speech.copyHashing(ByteArrayInputStream(ByteArray(10)), ByteArrayOutputStream(), stop = { true })
        assertNull(hash)
    }

    @Test
    fun `hex is lowercase, two digits a byte`() {
        assertEquals("00ff0a80", Speech.hex(byteArrayOf(0, -1, 10, -128)))
    }
}
