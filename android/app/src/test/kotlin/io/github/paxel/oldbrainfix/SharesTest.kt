package io.github.paxel.oldbrainfix

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
}
