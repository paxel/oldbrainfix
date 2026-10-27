package io.github.paxel.oldbrainfix

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FilterInputStream
import java.io.InputStream
import java.nio.file.Files

class SpeechTest {
    @Test
    fun `a transcribe request carries the recording and both prompts`() {
        val bytes = byteArrayOf(1, 2, 3)
        val request = Speech.request(
            mapOf("bytes" to bytes, "english" to "Work, Garten.", "german" to "Notiz. Work, Garten."),
        )!!
        assertSame(bytes, request.bytes)
        assertEquals(Speech.Prompts("Work, Garten.", "Notiz. Work, Garten."), request.prompts)
    }

    @Test
    fun `a request without prompts has none, one without bytes is none`() {
        assertEquals(Speech.Prompts(), Speech.request(mapOf("bytes" to byteArrayOf(1)))!!.prompts)
        assertNull(Speech.request(mapOf("english" to "Work.")))
        assertNull(Speech.request(byteArrayOf(1)))
    }

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
    fun `48 kHz stereo, as Opus voice messages decode, comes out as 16 kHz mono`() {
        // A 440 Hz tone, the same on both channels, for one second.
        fun tone(i: Int, rate: Int) = 0.5f * kotlin.math.sin(2 * Math.PI * 440 * i / rate).toFloat()
        val stereo = FloatArray(48_000 * 2) { tone(it / 2, 48_000) }
        val whisper = Speech.toWhisper(stereo, 2, 48_000)
        assertEquals(16_000, whisper.size)
        // Still the same tone, at Whisper's rate.
        for (i in whisper.indices) assertEquals(tone(i, 16_000), whisper[i], 1e-3f)
    }

    @Test
    fun `48 kHz mono keeps every third sample`() {
        val mono = FloatArray(9) { it.toFloat() }
        assertArrayEquals(floatArrayOf(0f, 3f, 6f), Speech.toWhisper(mono, 1, 48_000), 1e-6f)
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

    /** A model file with "abc" in it, its record beside it, and the bytes read from it. */
    private class Model {
        val dir: File = Files.createTempDirectory("whisper").toFile().apply { deleteOnExit() }
        val file = File(dir, "ggml-base.bin").apply { writeText("abc") }
        val record = File(dir, "ggml-base.bin.checked")
        var read = 0L

        fun hash(): String? = Speech.checkedHash(file, record) { counting(it.inputStream()) }

        private fun counting(input: InputStream) = object : FilterInputStream(input) {
            override fun read(b: ByteArray, off: Int, len: Int): Int =
                super.read(b, off, len).also { if (it > 0) read += it }
        }
    }

    private val abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

    @Test
    fun `a model without a record is hashed once and then recorded`() {
        val model = Model()
        assertEquals(abc, model.hash())
        assertEquals(3L, model.read)
        assertEquals(Speech.ModelRecord(abc, 3, model.file.lastModified()), Speech.readRecord(model.record))
        assertEquals(abc, model.hash())
        assertEquals(3L, model.read)
    }

    @Test
    fun `a model kept with its hash is not read again`() {
        val model = Model()
        val incoming = File(model.dir, "ggml-base.bin.part").apply { writeText("abc") }
        model.file.delete()
        assertTrue(Speech.keep(incoming, model.file, model.record, abc))
        assertEquals(abc, model.hash())
        assertEquals(0L, model.read)
    }

    @Test
    fun `a changed modification time hashes the model again`() {
        val model = Model()
        model.hash()
        assertTrue(model.file.setLastModified(model.file.lastModified() - 60_000))
        assertEquals(abc, model.hash())
        assertEquals(6L, model.read)
    }

    @Test
    fun `a changed size hashes the model again`() {
        val model = Model()
        val modified = model.file.lastModified()
        model.hash()
        model.file.writeText("abcd")
        model.file.setLastModified(modified)
        assertEquals("88d4266fd4e6338d13b845fcf289579d209c897823b9217da3e161936f031589", model.hash())
        assertEquals(7L, model.read)
    }

    @Test
    fun `a record that is not one is no record`() {
        val model = Model()
        for (text in listOf("", "ba78 3 1", "$abc 3", "$abc x 1", "${abc.uppercase()} 3 1", "$abc 3 1 1")) {
            model.record.writeText(text)
            assertNull(text, Speech.readRecord(model.record))
        }
        model.record.writeText("${abc.dropLast(1)} 3 ${model.file.lastModified()}")
        assertEquals(abc, model.hash())
        assertEquals(3L, model.read)
    }

    @Test
    fun `no model has no hash and reads nothing`() {
        val model = Model()
        model.file.delete()
        assertNull(model.hash())
        assertEquals(0L, model.read)
    }

    @Test
    fun `keeping a model drops the record of the one before`() {
        val model = Model()
        model.hash()
        val incoming = File(model.dir, "ggml-base.bin.part").apply { writeText("abcd") }
        assertTrue(Speech.keep(incoming, model.file, model.record, null))
        assertFalse(model.record.exists())
        assertFalse(Speech.keep(incoming, model.file, model.record, abc))
    }
}
