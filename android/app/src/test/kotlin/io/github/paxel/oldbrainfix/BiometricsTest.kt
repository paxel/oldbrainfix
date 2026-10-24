package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BiometricsTest {
    @Test
    fun `before Android 10 only the PIN unlocks`() {
        assertEquals(Biometrics.Mode.NONE, Biometrics.mode(24))
        assertEquals(Biometrics.Mode.NONE, Biometrics.mode(28))
        assertFalse(Biometrics.usable(Biometrics.Mode.NONE, canAuthenticate = true, deviceSecure = true))
    }

    @Test
    fun `Android 10 allows the screen lock in the prompt`() {
        assertEquals(Biometrics.Mode.CREDENTIAL_ALLOWED, Biometrics.mode(29))
        assertTrue(Biometrics.usable(Biometrics.Mode.CREDENTIAL_ALLOWED, canAuthenticate = false, deviceSecure = true))
        assertFalse(Biometrics.usable(Biometrics.Mode.CREDENTIAL_ALLOWED, canAuthenticate = false, deviceSecure = false))
    }

    @Test
    fun `from Android 11 the system says whether biometrics or the screen lock can`() {
        assertEquals(Biometrics.Mode.AUTHENTICATORS, Biometrics.mode(30))
        assertEquals(Biometrics.Mode.AUTHENTICATORS, Biometrics.mode(35))
        assertTrue(Biometrics.usable(Biometrics.Mode.AUTHENTICATORS, canAuthenticate = true, deviceSecure = false))
        assertFalse(Biometrics.usable(Biometrics.Mode.AUTHENTICATORS, canAuthenticate = false, deviceSecure = true))
    }

    @Test
    fun `weak biometrics and the screen lock are both allowed`() {
        // BIOMETRIC_WEAK (0xff) | DEVICE_CREDENTIAL (1 shl 15)
        assertEquals(0xff or (1 shl 15), Biometrics.ALLOWED)
    }
}
