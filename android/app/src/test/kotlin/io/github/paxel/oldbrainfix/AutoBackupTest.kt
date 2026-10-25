package io.github.paxel.oldbrainfix

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AutoBackupTest {
    private fun files(): File {
        val root = Files.createTempDirectory("autobackup").toFile()
        root.deleteOnExit()
        return root
    }

    @Test
    fun nothingIsBackedUpUntilTheUserOptedIn() {
        val root = files()
        val dir = File(root, AutoBackup.DIR).apply { mkdirs() }
        File(dir, "oldbrainfix-20261024-090000-abcdef01.obfbackup").writeText("sealed")
        File(root, "oldbrainfix.db").writeText("plain")
        assertEquals(emptyList<File>(), AutoBackup.files(false, dir, Long.MAX_VALUE))
    }

    @Test
    fun optedInOnlyTheBackupFileGoes() {
        val root = files()
        val dir = File(root, AutoBackup.DIR).apply { mkdirs() }
        val backup = File(dir, "oldbrainfix-20261024-090000-abcdef01.obfbackup").apply { writeText("sealed") }
        File(dir, "oldbrainfix-20261024-090000-abcdef01.obfbackup.tmp").writeText("half")
        File(dir, "notes.txt").writeText("other")
        File(dir, "sub.obfbackup").mkdirs()
        File(root, "oldbrainfix.db").writeText("plain")
        assertEquals(listOf(backup), AutoBackup.files(true, dir, Long.MAX_VALUE))
        // None at all where there is no directory yet.
        assertEquals(emptyList<File>(), AutoBackup.files(true, File(root, "none"), Long.MAX_VALUE))
    }

    @Test
    fun aBackupLargerThanTheQuotaIsLeftOut() {
        val dir = File(files(), AutoBackup.DIR).apply { mkdirs() }
        val backup = File(dir, "b.obfbackup").apply { writeBytes(ByteArray(100)) }
        assertEquals(emptyList<File>(), AutoBackup.files(true, dir, 99))
        assertEquals(listOf(backup), AutoBackup.files(true, dir, 100))
    }

    @Test
    fun onlyABackupFileInItsDirectoryComesBack() {
        val root = files()
        val dir = File(root, AutoBackup.DIR)
        assertTrue(AutoBackup.restores(dir, File(dir, "b.obfbackup")))
        assertFalse(AutoBackup.restores(dir, File(dir, "b.db")))
        assertFalse(AutoBackup.restores(dir, File(root, "b.obfbackup")))
        assertFalse(AutoBackup.restores(dir, File(File(dir, "x"), "b.obfbackup")))
        assertFalse(AutoBackup.restores(dir, null))
    }
}
