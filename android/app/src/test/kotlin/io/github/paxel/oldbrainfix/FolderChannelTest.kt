package io.github.paxel.oldbrainfix

import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.nio.file.Files
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class FolderChannelTest {
    @Test
    fun theFolderIsFoundBelowItsRoot() {
        assertEquals(listOf("oldbrainfix-sync"), FolderChannel.segments("oldbrainfix-sync", ""))
        assertEquals(listOf("oldbrainfix-sync", "keys"), FolderChannel.segments("oldbrainfix-sync", "keys"))
        assertEquals(listOf("oldbrainfix-sync", "a", "b"), FolderChannel.segments("oldbrainfix-sync", "a//b/"))
    }

    @Test
    fun namesNeverLeaveTheirFolder() {
        assertEquals(listOf("oldbrainfix-sync", "keys"), FolderChannel.segments("oldbrainfix-sync", "../keys/."))
        assertTrue(FolderChannel.safeName("abc.1.seg"))
        assertFalse(FolderChannel.safeName(".."))
        assertFalse(FolderChannel.safeName("a/b"))
        assertFalse(FolderChannel.safeName("a\\b"))
        assertFalse(FolderChannel.safeName(""))
    }

    @Test
    fun anImportFindsFilesByTheirPathAndSkipsHiddenFolders() {
        assertEquals("Shopping.md", FolderChannel.path("", "Shopping.md"))
        assertEquals("Work/Plans/Q3.md", FolderChannel.path("Work/Plans", "Q3.md"))
        assertTrue(FolderChannel.descends("Work"))
        assertFalse(FolderChannel.descends(".obsidian"))
        assertFalse(FolderChannel.descends(".git"))
    }

    @Test
    fun aFileLargerThanTheLimitIsNeverReadWhole() {
        val bytes = ByteArray(20_000) { (it % 251).toByte() }
        assertArrayEquals(bytes, FolderChannel.readAtMost(ByteArrayInputStream(bytes), 20_000))
        assertNull(FolderChannel.readAtMost(ByteArrayInputStream(bytes), 19_999))
        assertArrayEquals(ByteArray(0), FolderChannel.readAtMost(ByteArrayInputStream(ByteArray(0)), 0))
    }

    @Test
    fun anImportedFolderGivesUpItsGrantUnlessTheAppKeepsIt() {
        val notes = "content://tree/primary%3ANotes"
        val sync = "content://tree/primary%3ASync"
        assertTrue(FolderChannel.releases(notes, emptyList()))
        assertTrue(FolderChannel.releases(notes, listOf(sync)))
        assertFalse(FolderChannel.releases(sync, listOf(sync)))
        assertFalse(FolderChannel.releases(notes, listOf(sync, notes)))
    }

    /** A zip of [files], by their paths, and a folder entry. */
    private fun zip(vararg files: Pair<String, ByteArray>): ByteArray {
        val out = ByteArrayOutputStream()
        ZipOutputStream(out).use { zip ->
            zip.putNextEntry(ZipEntry("Takeout/"))
            zip.closeEntry()
            for ((path, bytes) in files) {
                zip.putNextEntry(ZipEntry(path))
                zip.write(bytes)
                zip.closeEntry()
            }
        }
        return out.toByteArray()
    }

    @Test
    fun aZipListsItsFilesByTheirPathsWithoutItsFolders() {
        val bytes = zip(
            "Takeout/Keep/Shopping.json" to "{}".toByteArray(),
            "Takeout/Keep/Shopping.html" to "<html>".toByteArray(),
        )
        assertEquals(
            listOf("Takeout/Keep/Shopping.json", "Takeout/Keep/Shopping.html"),
            FolderChannel.zipNames(ByteArrayInputStream(bytes)),
        )
    }

    @Test
    fun anImportUnzipsOnlyTheFilesWantedAndNoneLargerThanItsLimitAndOneByte() {
        val note = "{\"title\":\"Shopping\"}".toByteArray()
        val photo = ByteArray(5_000) { (it % 251).toByte() }
        val bytes = zip(
            "Takeout/Keep/Shopping.json" to note,
            "Takeout/Keep/Shopping.html" to "<html>".toByteArray(),
            "Takeout/Keep/garden.jpg" to photo,
            // Android 14 on rejects such a zip outright, failing the
            // Import; elsewhere its copy still stays in the folder.
            "Takeout/Keep/../../evil.json" to note,
        )
        val into = Files.createTempDirectory("unzip").toFile()
        try {
            val copied = FolderChannel.unzip(
                ByteArrayInputStream(bytes),
                mapOf(
                    "Takeout/Keep/Shopping.json" to 1_000,
                    "Takeout/Keep/garden.jpg" to 4_000,
                    "Takeout/Keep/../../evil.json" to 1_000,
                ),
                into,
            )
            assertEquals(
                listOf("Takeout/Keep/Shopping.json", "Takeout/Keep/garden.jpg", "Takeout/Keep/../../evil.json"),
                copied.map { it.first },
            )
            assertArrayEquals(note, copied[0].second.readBytes())
            assertEquals(note.size.toLong(), copied[0].third)
            // The photo over its limit is copied as its limit and one byte.
            assertEquals(4_001L, copied[1].third)
            assertArrayEquals(photo.copyOf(4_001), copied[1].second.readBytes())
            // No path from inside the zip leaves the folder.
            for ((_, file, _) in copied) assertEquals(into, file.parentFile)
            assertEquals(3, into.listFiles()!!.size)
        } finally {
            into.deleteRecursively()
        }
    }

    @Test
    fun aCopyStopsAtTheMostBytesAsked() {
        val bytes = ByteArray(20_000) { (it % 251).toByte() }
        val out = ByteArrayOutputStream()
        assertEquals(12_345L, FolderChannel.copyAtMost(ByteArrayInputStream(bytes), out, 12_345L))
        assertArrayEquals(bytes.copyOf(12_345), out.toByteArray())
        val all = ByteArrayOutputStream()
        assertEquals(20_000L, FolderChannel.copyAtMost(ByteArrayInputStream(bytes), all, 30_000L))
        assertArrayEquals(bytes, all.toByteArray())
    }
}
