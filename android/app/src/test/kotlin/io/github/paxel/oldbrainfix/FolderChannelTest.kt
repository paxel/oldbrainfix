package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
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
}
