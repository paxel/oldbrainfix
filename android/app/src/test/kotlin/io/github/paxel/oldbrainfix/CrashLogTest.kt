package io.github.paxel.oldbrainfix

import java.io.File
import java.nio.file.Files
import java.util.Calendar
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class CrashLogTest {
    private fun deepCrash(): Throwable {
        fun dig(depth: Int): Nothing =
            if (depth == 0) throw IllegalStateException("the root cause") else dig(depth - 1)
        return try {
            dig(60)
        } catch (root: Throwable) {
            RuntimeException("outer " + "x".repeat(500), IllegalArgumentException("middle", root))
        }
    }

    @Test
    fun theHeaderHasTheDartShape() {
        val at = Calendar.getInstance().apply { set(2026, Calendar.OCTOBER, 26, 16, 53, 33); set(Calendar.MILLISECOND, 7) }.time
        val lines = CrashLog.header("0.1.0+1", "android 14 (API 34, Maker Phone)", "de_DE", at).split("\n")
        assertEquals(
            listOf("Old Brain Fix 0.1.0+1", "android 14 (API 34, Maker Phone)", "locale de_DE", "2026-10-26T16:53:33.007"),
            lines,
        )
    }

    @Test
    fun theMailKeepsTheRootCauseWithinTheLimit() {
        val error = deepCrash()
        assertTrue(CrashLog.full("head", error).length > CrashLog.MAIL_LIMIT)
        val mail = CrashLog.compact("head", error, CrashLog.MAIL_LIMIT)
        assertTrue(mail.length <= CrashLog.MAIL_LIMIT)
        assertTrue(mail.startsWith("head\n\njava.lang.RuntimeException: outer"))
        assertTrue(mail.contains("Caused by: java.lang.IllegalArgumentException: middle"))
        assertTrue(mail.contains("Caused by: java.lang.IllegalStateException: the root cause"))
        assertTrue(mail.contains("\tat "))
    }

    @Test
    fun aShortReportStaysWhole() {
        val error = IllegalStateException("small")
        error.stackTrace = error.stackTrace.take(3).toTypedArray()
        val mail = CrashLog.compact("head", error, CrashLog.MAIL_LIMIT)
        assertEquals(CrashLog.full("head", error).trimEnd().lines().size, mail.lines().size)
    }

    @Test
    fun anUncaughtExceptionIsWrittenForTheDartSideAndPassedOn() {
        val dir = Files.createTempDirectory("crashlog").toFile().apply { deleteOnExit() }
        val before = Thread.getDefaultUncaughtExceptionHandler()
        var passedOn: Throwable? = null
        var shown: String? = null
        Thread.setDefaultUncaughtExceptionHandler { _, e -> passedOn = e }
        try {
            CrashLog.install(dir, { "Old Brain Fix 0.1.0+1\nandroid\nlocale en\nnow" }) { report, _ ->
                shown = report
                false
            }
            val error = IllegalStateException("ring")
            Thread.getDefaultUncaughtExceptionHandler()!!.uncaughtException(Thread.currentThread(), error)
            val written = File(dir, CrashLog.FILE).readText()
            assertTrue(written.startsWith("Old Brain Fix 0.1.0+1\nandroid\nlocale en\nnow\n\n"))
            assertTrue(written.contains("java.lang.IllegalStateException: ring"))
            assertTrue("the running marker says the run died", File(dir, CrashLog.MARKER).exists())
            assertTrue(shown!!.contains("IllegalStateException: ring"))
            assertSame("the handler before still ends the process", error, passedOn)
        } finally {
            Thread.setDefaultUncaughtExceptionHandler(before)
        }
    }
}
