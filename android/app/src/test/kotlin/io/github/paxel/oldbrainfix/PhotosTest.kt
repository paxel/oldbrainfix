package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PhotosTest {
    @Test
    fun `a photo is decoded small but never below the kept size`() {
        assertEquals(1, Photos.sampleSize(2048, 1536, 2048))
        assertEquals(1, Photos.sampleSize(4000, 3000, 2048))
        assertEquals(2, Photos.sampleSize(4096, 3072, 2048))
        assertEquals(4, Photos.sampleSize(3000, 9000, 2048))
        assertEquals(1, Photos.sampleSize(800, 600, 2048))
    }

    @Test
    fun `the long edge is scaled down to the kept size, never up`() {
        assertEquals(1f, Photos.scale(800, 600, 2048), 0f)
        assertEquals(0.5f, Photos.scale(4096, 1000, 2048), 0f)
        assertEquals(0.5f, Photos.scale(1000, 4096, 2048), 0f)
    }

    @Test
    fun `the Exif orientation turns the photo upright`() {
        assertEquals(0, Photos.rotation(1))
        assertEquals(90, Photos.rotation(6))
        assertEquals(180, Photos.rotation(3))
        assertEquals(270, Photos.rotation(8))
        assertEquals(0, Photos.rotation(0))
    }

    @Test
    fun `only images are taken as pictures from a share`() {
        assertTrue(Shares.isImage("image/jpeg"))
        assertTrue(Shares.isImage("image/*"))
        assertFalse(Shares.isImage("text/plain"))
        assertFalse(Shares.isImage(null))
    }
}
