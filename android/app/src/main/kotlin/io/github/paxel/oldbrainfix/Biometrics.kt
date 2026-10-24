package io.github.paxel.oldbrainfix

import android.annotation.SuppressLint
import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.CancellationSignal

/**
 * Unlocking the App Lock and the Area Locks with biometrics, or the
 * Device's own screen lock as fallback, through the framework's
 * BiometricPrompt: no library, no Google Play Services. From Android 10
 * on; before that only the app's own PIN unlocks.
 */
object Biometrics {
    /** How the prompt is told to take the screen lock as well. */
    enum class Mode {
        /** No prompt with the screen lock: before Android 10. */
        NONE,

        /** Android 10: biometrics, with the screen lock allowed. */
        CREDENTIAL_ALLOWED,

        /** Android 11 on: the authenticators named. */
        AUTHENTICATORS,
    }

    /** Any biometric, weak ones too, or the screen lock: nothing is encrypted with it. */
    const val ALLOWED =
        BiometricManager.Authenticators.BIOMETRIC_WEAK or BiometricManager.Authenticators.DEVICE_CREDENTIAL

    /** The [Mode] on Android [sdk]. */
    fun mode(sdk: Int): Mode = when {
        sdk >= Build.VERSION_CODES.R -> Mode.AUTHENTICATORS
        sdk >= Build.VERSION_CODES.Q -> Mode.CREDENTIAL_ALLOWED
        else -> Mode.NONE
    }

    /**
     * Whether the prompt can unlock in [mode]: on Android 11 on when the
     * system says the [ALLOWED] authenticators can ([canAuthenticate]),
     * on Android 10 when the Device has a screen lock ([deviceSecure]),
     * as the prompt falls back to it.
     */
    fun usable(mode: Mode, canAuthenticate: Boolean, deviceSecure: Boolean): Boolean = when (mode) {
        Mode.NONE -> false
        Mode.CREDENTIAL_ALLOWED -> deviceSecure
        Mode.AUTHENTICATORS -> canAuthenticate
    }

    /** Whether this Device can unlock with biometrics or its screen lock. */
    @SuppressLint("NewApi")
    fun available(context: Context): Boolean {
        val mode = mode(Build.VERSION.SDK_INT)
        if (mode == Mode.NONE) return false
        val keyguard = context.getSystemService(KeyguardManager::class.java)
        val manager = context.getSystemService(BiometricManager::class.java)
        val can = mode == Mode.AUTHENTICATORS &&
            manager?.canAuthenticate(ALLOWED) == BiometricManager.BIOMETRIC_SUCCESS
        return usable(mode, can, keyguard?.isDeviceSecure == true)
    }

    /** The prompt showing, so a second one is turned down. */
    private var showing: CancellationSignal? = null

    /**
     * Shows the prompt with [title] over [activity]; [answer] gets whether
     * the user unlocked. The screen lock is offered in it, so it needs no
     * button of its own to give up.
     */
    @SuppressLint("NewApi")
    @Suppress("DEPRECATION")
    fun authenticate(activity: Activity, title: String, answer: (Boolean) -> Unit) {
        val mode = mode(Build.VERSION.SDK_INT)
        if (mode == Mode.NONE || showing != null) return answer(false)
        val builder = BiometricPrompt.Builder(activity).setTitle(title)
        if (mode == Mode.AUTHENTICATORS) {
            builder.setAllowedAuthenticators(ALLOWED)
        } else {
            builder.setDeviceCredentialAllowed(true)
        }
        val signal = CancellationSignal()
        showing = signal
        var answered = false
        fun done(ok: Boolean) {
            if (answered) return
            answered = true
            showing = null
            answer(ok)
        }
        try {
            builder.build().authenticate(
                signal,
                activity.mainExecutor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) = done(true)

                    // Given up, locked out or none enrolled; one finger
                    // that did not match leaves the prompt open.
                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) = done(false)
                },
            )
        } catch (_: RuntimeException) {
            done(false)
        }
    }
}
