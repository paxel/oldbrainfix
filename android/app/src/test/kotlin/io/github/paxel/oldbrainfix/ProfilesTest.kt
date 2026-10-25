package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Test

class ProfilesTest {
    @Test
    fun `the phone's own name comes first`() {
        assertEquals("Ada's Pixel", Profiles.defaultName(" Ada's Pixel ", "Pixel 9"))
    }

    @Test
    fun `without a name the model stands in`() {
        assertEquals("Pixel 9", Profiles.defaultName(null, "Pixel 9"))
        assertEquals("Pixel 9", Profiles.defaultName("  ", "Pixel 9"))
    }

    @Test
    fun `without either the name is empty`() {
        assertEquals("", Profiles.defaultName(null, null))
        assertEquals("", Profiles.defaultName("", " "))
    }
}
