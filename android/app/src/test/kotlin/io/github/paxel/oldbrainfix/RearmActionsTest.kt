package io.github.paxel.oldbrainfix

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The exported RearmReceiver arms the plan again only for the system's broadcasts. */
class RearmActionsTest {
    @Test
    fun theSystemsBroadcastsArmThePlanAgain() {
        assertTrue(RearmActions.rearms("android.intent.action.BOOT_COMPLETED"))
        assertTrue(RearmActions.rearms("android.intent.action.MY_PACKAGE_REPLACED"))
        assertTrue(RearmActions.rearms("android.intent.action.TIME_SET"))
        assertTrue(RearmActions.rearms("android.intent.action.TIMEZONE_CHANGED"))
        assertTrue(RearmActions.rearms("android.app.action.SCHEDULE_EXACT_ALARM_PERMISSION_STATE_CHANGED"))
    }

    @Test
    fun anotherAppsExplicitIntentIsIgnored() {
        assertFalse(RearmActions.rearms(null))
        assertFalse(RearmActions.rearms(""))
        assertFalse(RearmActions.rearms("com.example.REARM"))
        assertFalse(RearmActions.rearms("android.intent.action.BOOT_COMPLETED "))
    }
}
