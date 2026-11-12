package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Test

/** The buttons of an Occurrence's notification (#269): three, as Android shows at most. */
class NotificationButtonsTest {
    @Test
    fun anOccurrenceOffersDoneSnoozeAndNotToday() {
        assertEquals(
            listOf(
                AlarmReceiver.ACTION_DONE to "done",
                AlarmReceiver.ACTION_SNOOZE to "snooze",
                AlarmReceiver.ACTION_NOT_TODAY to "notToday",
            ),
            Notifications.buttons(),
        )
    }

    @Test
    fun notTodayReplacesThemWithSkipAndTomorrow() {
        assertEquals(
            listOf(AlarmReceiver.ACTION_SKIP to "skip", AlarmReceiver.ACTION_TOMORROW to "tomorrow"),
            Notifications.notTodayChoices("2026-10-24T08:00", "2026-10-24"),
        )
    }

    @Test
    fun anOccurrenceOfAnEarlierDayOffersSkipAlone() {
        assertEquals(
            listOf(AlarmReceiver.ACTION_SKIP to "skip"),
            Notifications.notTodayChoices("2026-10-23T22:00", "2026-10-24"),
        )
    }

    @Test
    fun skipAndTomorrowAreQueuedForTheApp() {
        val prefs = FakePrefs()
        val pills = Alarm(id = "a", routine = "routine:a", occurrence = "2026-10-24T08:00", local = null, at = null, title = "Pills", body = "")
        EventQueue.add(prefs, EventQueue.KIND_SKIP, pills, 0, 1_000)
        EventQueue.add(prefs, EventQueue.KIND_TOMORROW, pills, 0, 2_000)
        assertEquals(listOf("skip", "tomorrow"), EventQueue.pending(prefs).map { it.kind })
    }
}
