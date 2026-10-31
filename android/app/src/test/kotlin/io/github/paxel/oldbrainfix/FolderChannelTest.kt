package io.github.paxel.oldbrainfix

import java.io.ByteArrayInputStream
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
}
