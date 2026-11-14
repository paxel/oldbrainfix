package io.github.paxel.oldbrainfix

import java.io.File
import java.io.IOException
import java.nio.file.Files as NioFiles
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Print to Old Brain Fix (#293): the name a print goes by, the printer's
 * capabilities, whether the service is on, and a job's PDF kept until
 * the app takes it as a share's file.
 */
class PrintInTest {
    private val dir: File = NioFiles.createTempDirectory("printed").toFile()
    private val shared: File = NioFiles.createTempDirectory("shared").toFile()

    @After
    fun cleanUp() {
        dir.deleteRecursively()
        shared.deleteRecursively()
    }

    @Test
    fun `a print goes by its document's name, else the job's, as a PDF`() {
        assertEquals("Kontoauszug März.pdf", PrintIn.nameOf("Kontoauszug März.pdf", "Bank"))
        assertEquals("Kontoauszug März.pdf", PrintIn.nameOf("Kontoauszug März", "Bank"))
        assertEquals("Scan.PDF", PrintIn.nameOf("Scan.PDF", null))
        assertEquals("Bank.pdf", PrintIn.nameOf(" ", "Bank"))
        assertEquals("print.pdf", PrintIn.nameOf(null, ""))
    }

    @Test
    fun `the printer offers A4 and Letter, Letter first where the region prints on it`() {
        val germany = PrintIn.capabilities("DE")
        assertEquals(listOf(PrintIn.A4, PrintIn.LETTER), germany.media)
        assertEquals(PrintIn.A4, germany.defaultMedia)
        assertEquals(PrintIn.LETTER, PrintIn.capabilities("us").defaultMedia)
        assertEquals(PrintIn.LETTER, PrintIn.capabilities("CA").defaultMedia)
        assertEquals(PrintIn.A4, PrintIn.capabilities(null).defaultMedia)
        assertEquals(PrintIn.A4, PrintIn.capabilities("").defaultMedia)
    }

    @Test
    fun `the service is on unless Android lists it as turned off`() {
        val own = PrintIn.ComponentNames(
            "io.github.paxel.oldbrainfix",
            "io.github.paxel.oldbrainfix.PrintInService",
        )
        // Never turned off: on, as Android 7 and later start every
        // installed print service.
        assertEquals(PrintIn.State.ON, PrintIn.state(null, true, own))
        assertEquals(PrintIn.State.ON, PrintIn.state("", true, own))
        assertEquals(PrintIn.State.ON, PrintIn.state("com.other/.PrintService", true, own))
        // Turned off, in short or full form, among others.
        assertEquals(
            PrintIn.State.OFF,
            PrintIn.state("com.other/.Print:io.github.paxel.oldbrainfix/.PrintInService", true, own),
        )
        assertEquals(
            PrintIn.State.OFF,
            PrintIn.state("io.github.paxel.oldbrainfix/io.github.paxel.oldbrainfix.PrintInService", true, own),
        )
        // Another app's service of the same class name is another.
        assertEquals(PrintIn.State.ON, PrintIn.state("io.github.other/.PrintInService", true, own))
        // Not readable: unknown, never a wrong "off".
        assertEquals(PrintIn.State.UNKNOWN, PrintIn.state(null, false, own))
    }

    @Test
    fun `a job's PDF is kept with its name and taken once as a share's file`() {
        val pdf = PrintIn.keep(dir, "Kontoauszug März.pdf") { it.write("%PDF-1.7".toByteArray()) }!!
        assertEquals("%PDF-1.7", pdf.readText())
        val taken = PrintIn.take(dir, shared).single()
        assertEquals("Kontoauszug März.pdf", taken.name)
        assertEquals("application/pdf", taken.type)
        assertEquals("Kontoauszug März.pdf", Files.handed(listOf(taken)).single()["name"])
        // Moved where a share's copies wait, for the app to attach; it
        // never comes twice, and nothing stays in the app's files.
        assertEquals(shared, taken.file.parentFile)
        assertEquals("%PDF-1.7", taken.file.readText())
        assertFalse(pdf.exists())
        assertTrue(PrintIn.take(dir, shared).isEmpty())
        assertTrue(dir.listFiles().orEmpty().isEmpty())
    }

    @Test
    fun `prints come oldest first`() {
        val first = PrintIn.keep(dir, "a.pdf") { it.write(1) }!!
        File(dir, first.nameWithoutExtension + ".name").setLastModified(1_000)
        PrintIn.keep(dir, "b.pdf") { it.write(2) }!!
        assertEquals(listOf("a.pdf", "b.pdf"), PrintIn.take(dir, shared).map { it.name })
    }

    @Test
    fun `a print that could not be written is not kept, nor half of it`() {
        assertNull(PrintIn.keep(dir, "broken.pdf") { throw IOException("pipe closed") })
        assertTrue(dir.listFiles().orEmpty().isEmpty())
        assertTrue(PrintIn.take(dir, shared).isEmpty())
    }

    @Test
    fun `a print still being written does not come yet, and a dropped one not at all`() {
        File(dir, "x.part").writeText("%PDF")
        assertTrue(PrintIn.take(dir, shared).isEmpty())
        val pdf = PrintIn.keep(dir, "gone.pdf") { it.write(1) }!!
        PrintIn.drop(pdf)
        assertFalse(pdf.exists())
        assertTrue(PrintIn.take(dir, shared).isEmpty())
    }
}
