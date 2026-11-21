package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class VoiceStartTest {
    @Test
    fun `a recording that starts keeps the screen on and says so`() {
        var screenOn = 0
        assertTrue(VoiceStart.run({ true }, { screenOn++ }))
        assertEquals(1, screenOn)
    }

    @Test
    fun `a recording that does not start leaves the screen alone`() {
        var screenOn = 0
        assertFalse(VoiceStart.run({ false }, { screenOn++ }))
        assertEquals(0, screenOn)
    }
}
