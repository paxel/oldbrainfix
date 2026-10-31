package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PhotoOutTest {
    @Test
    fun `a photo is saved without a permission from Android 10 on`() {
        assertFalse(PhotoOut.savesWithoutPermission(24))
        assertFalse(PhotoOut.savesWithoutPermission(28))
        assertTrue(PhotoOut.savesWithoutPermission(29))
        assertTrue(PhotoOut.savesWithoutPermission(36))
    }

    @Test
    fun `a saved photo goes into an album of its own under Pictures`() {
        assertEquals("Pictures/Old Brain Fix", PhotoOut.FOLDER)
    }

    @Test
    fun `a saved photo keeps its name without any folder in it`() {
        assertEquals("photo-0123456789ab.jpg", PhotoOut.displayName("photo-0123456789ab.jpg"))
        assertEquals("photo-x.png", PhotoOut.displayName("../../photo-x.png"))
        assertEquals("export", PhotoOut.displayName(""))
    }
}
