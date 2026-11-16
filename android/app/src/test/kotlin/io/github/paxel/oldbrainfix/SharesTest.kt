package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SharesTest {
    @Test
    fun `sound files are told by their MIME type, Ogg by its container too`() {
        assertTrue(Shares.isAudio("audio/ogg"))
        assertTrue(Shares.isAudio("audio/ogg; codecs=opus"))
        assertTrue(Shares.isAudio("audio/mp4"))
        assertTrue(Shares.isAudio("audio/mpeg"))
        assertTrue(Shares.isAudio("audio/*"))
        assertTrue(Shares.isAudio("application/ogg"))
    }

    @Test
    fun `pictures, text and unknown files are no sound`() {
        assertFalse(Shares.isAudio("image/jpeg"))
        assertFalse(Shares.isAudio("text/plain"))
        assertFalse(Shares.isAudio("application/octet-stream"))
        assertFalse(Shares.isAudio("video/mp4"))
        assertFalse(Shares.isAudio(null))
        assertTrue(Shares.isImage("image/png"))
        assertFalse(Shares.isImage("audio/ogg"))
    }

    @Test
    fun `a file is sound by its own type, else by the share's unless it is a picture`() {
        assertTrue(Shares.isSound("audio/ogg", "image/*"))
        assertTrue(Shares.isSound("application/octet-stream", "audio/ogg"))
        assertTrue(Shares.isSound(null, "audio/*"))
        assertFalse(Shares.isSound("image/jpeg", "audio/*"))
        assertFalse(Shares.isSound(null, "image/*"))
        assertFalse(Shares.isSound("application/octet-stream", "image/*"))
    }

    @Test
    fun `a shared file is sound, a picture or an Attachment, never a PDF taken for a picture`() {
        assertEquals(Shares.Kind.SOUND, Shares.kindOf("audio/ogg", "*/*"))
        assertEquals(Shares.Kind.SOUND, Shares.kindOf("application/octet-stream", "audio/ogg"))
        assertEquals(Shares.Kind.PICTURE, Shares.kindOf("image/jpeg", "*/*"))
        // A provider that knows only "some file" in a share of pictures.
        assertEquals(Shares.Kind.PICTURE, Shares.kindOf(null, "image/*"))
        assertEquals(Shares.Kind.PICTURE, Shares.kindOf("application/octet-stream", "image/*"))
        assertEquals(Shares.Kind.FILE, Shares.kindOf("application/pdf", "image/*"))
        assertEquals(Shares.Kind.FILE, Shares.kindOf("application/pdf", "*/*"))
        assertEquals(Shares.Kind.FILE, Shares.kindOf("application/zip", "application/zip"))
        assertEquals(Shares.Kind.FILE, Shares.kindOf(null, "*/*"))
        assertEquals(Shares.Kind.FILE, Shares.kindOf("text/plain", "text/plain"))
    }

    @Test
    fun `a file of the app's own shared to the share target opens as with Open with`() {
        assertTrue(Shares.isOwnFile("application/vnd.oldbrainfix.pack", "file"))
        assertTrue(Shares.isOwnFile("application/octet-stream", "Trip.OBFPACK"))
        assertTrue(Shares.isOwnFile(null, "Ann.obfcard"))
        assertTrue(Shares.isOwnFile(null, "changes.obfsync"))
        assertTrue(Shares.isOwnFile(null, "Note.oldbrain"))
        assertFalse(Shares.isOwnFile("application/pdf", "Lease.pdf"))
        assertFalse(Shares.isOwnFile("application/zip", "obfpack.zip"))
    }
}
