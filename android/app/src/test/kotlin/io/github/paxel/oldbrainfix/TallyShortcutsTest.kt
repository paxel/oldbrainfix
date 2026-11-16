package io.github.paxel.oldbrainfix

import android.content.Intent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** A Tally's shortcut on the home screen (#284). */
class TallyShortcutsTest {
    @Test
    fun `each Tally has a shortcut of its own, the same each time`() {
        assertEquals(TallyShortcuts.id("n1"), TallyShortcuts.id("n1"))
        assertNotEquals(TallyShortcuts.id("n1"), TallyShortcuts.id("n2"))
    }

    @Test
    fun `a shortcut's launch asks for its Tally`() {
        assertEquals("n1", TallyShortcuts.noteOf("n1", 0))
        assertEquals(
            "n1",
            TallyShortcuts.noteOf("n1", Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK),
        )
    }

    @Test
    fun `a launch without a Tally asks for none`() {
        assertNull(TallyShortcuts.noteOf(null, 0))
        assertNull(TallyShortcuts.noteOf("", 0))
        assertNull(TallyShortcuts.noteOf("  ", 0))
    }

    @Test
    fun `reopening from the recent apps asks for no entry page again`() {
        assertNull(TallyShortcuts.noteOf("n1", Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY))
    }

    @Test
    fun `a Tally without a name is named Tally`() {
        assertEquals("Weight", TallyShortcuts.label(" Weight ", "Tally"))
        assertEquals("Tally", TallyShortcuts.label("", "Tally"))
        assertEquals("Tally", TallyShortcuts.label(null, "Tally"))
    }
}
