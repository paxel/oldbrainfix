package io.github.paxel.oldbrainfix

import android.content.Intent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CaptureModesTest {
    @Test
    fun `each mode's action asks for that mode`() {
        for (mode in listOf(CaptureModes.TEXT, CaptureModes.VOICE, CaptureModes.PHOTO)) {
            assertEquals(mode, CaptureModes.of(CaptureModes.action(mode), 0))
        }
    }

    @Test
    fun `the actions differ per mode`() {
        assertEquals("io.github.paxel.oldbrainfix.action.CAPTURE_VOICE", CaptureModes.action(CaptureModes.VOICE))
        assertEquals(
            3,
            listOf(CaptureModes.TEXT, CaptureModes.VOICE, CaptureModes.PHOTO).map(CaptureModes::action).toSet().size,
        )
    }

    @Test
    fun `other launches ask for no capture`() {
        assertNull(CaptureModes.of(null, 0))
        assertNull(CaptureModes.of(Intent.ACTION_MAIN, 0))
        assertNull(CaptureModes.of("io.github.paxel.oldbrainfix.action.CAPTURE_", 0))
        assertNull(CaptureModes.of("io.github.paxel.oldbrainfix.action.CAPTURE_VIDEO", 0))
    }

    @Test
    fun `reopening from the recent apps asks for no capture again`() {
        val action = CaptureModes.action(CaptureModes.PHOTO)
        assertNull(CaptureModes.of(action, Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY))
        assertEquals(
            CaptureModes.PHOTO,
            CaptureModes.of(action, Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
    }
}
