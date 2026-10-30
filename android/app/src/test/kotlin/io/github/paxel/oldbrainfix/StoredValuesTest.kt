package io.github.paxel.oldbrainfix

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * SharedPreferences in memory, failing as the real ones do when a value
 * is of another type than asked for.
 */
class FakePrefs : SharedPreferences {
    val values = mutableMapOf<String, Any?>()

    override fun getAll(): Map<String, *> = values.toMap()
    override fun getString(key: String, defValue: String?): String? =
        if (key in values) values[key] as String? else defValue
    override fun getStringSet(key: String, defValues: Set<String>?): Set<String>? = defValues
    override fun getInt(key: String, defValue: Int): Int = if (key in values) values[key] as Int else defValue
    override fun getLong(key: String, defValue: Long): Long = if (key in values) values[key] as Long else defValue
    override fun getFloat(key: String, defValue: Float): Float = if (key in values) values[key] as Float else defValue
    override fun getBoolean(key: String, defValue: Boolean): Boolean =
        if (key in values) values[key] as Boolean else defValue
    override fun contains(key: String): Boolean = key in values
    override fun edit(): SharedPreferences.Editor = Edits()
    override fun registerOnSharedPreferenceChangeListener(l: SharedPreferences.OnSharedPreferenceChangeListener?) {}
    override fun unregisterOnSharedPreferenceChangeListener(l: SharedPreferences.OnSharedPreferenceChangeListener?) {}

    inner class Edits : SharedPreferences.Editor {
        private val put = mutableMapOf<String, Any?>()
        private val removed = mutableSetOf<String>()

        override fun putString(key: String, value: String?) = apply { put[key] = value }
        override fun putStringSet(key: String, values: Set<String>?) = apply { put[key] = values }
        override fun putInt(key: String, value: Int) = apply { put[key] = value }
        override fun putLong(key: String, value: Long) = apply { put[key] = value }
        override fun putFloat(key: String, value: Float) = apply { put[key] = value }
        override fun putBoolean(key: String, value: Boolean) = apply { put[key] = value }
        override fun remove(key: String) = apply { removed += key }
        override fun clear() = apply { removed += values.keys }
        override fun commit(): Boolean {
            values.keys.removeAll(removed)
            values.putAll(put)
            return true
        }
        override fun apply() {
            commit()
        }
    }
}

/**
 * Removing or garbling each value kept in SharedPreferences crashes
 * nothing that reads it (AlarmReceiver, RearmReceiver and the Agenda
 * widget read through these), a value of a later version is not used,
 * and no tap queued for the app is lost (#132).
 */
class StoredValuesTest {
    /** Values no stored text is: not JSON, the wrong JSON, empty. */
    private val garbled = listOf("", "garbage", "{", "[", "null", "42", "\"text\"", "[1, 2]", "{\"x\": 1}", "[]", "{}")

    /** Versions that are not 1 (ADR 0034). */
    private val later = listOf("2", "0", "1.0", "\"1\"", "\"01\"", "null", "-1")

    private fun alarmJson(id: String) = JSONObject()
        .put("id", id)
        .put("routine", "routine:$id")
        .put("occurrence", "2026-10-24T08:00")
        .put("local", JSONArray(listOf(2026, 10, 24, 8, 0)))
        .put("title", "Pills")
        .put("body", "8:00")

    private fun nudgeJson() = JSONObject()
        .put("anchors", JSONArray().put(JSONArray(listOf(1_000L, 500L))))
        .put("repeat", 60)
        .put("title", "Unsorted")
        .put("body", "2 Notes")

    private fun ofVersion(json: JSONObject, version: String): String =
        json.toString().replaceFirst("{", "{\"v\":$version,")

    @Test
    fun aMissingOrForeignTypedValueReadsAsNone() {
        val prefs = FakePrefs()
        assertNull(Stored.text(prefs, "plan"))
        assertEquals(0L, Stored.long(prefs, "lastFired"))
        prefs.values["plan"] = 7L
        prefs.values["lastFired"] = "yesterday"
        assertNull(Stored.text(prefs, "plan"))
        assertEquals(0L, Stored.long(prefs, "lastFired"))
        prefs.values["plan"] = "[]"
        prefs.values["lastFired"] = 5L
        assertEquals("[]", Stored.text(prefs, "plan"))
        assertEquals(5L, Stored.long(prefs, "lastFired"))
    }

    @Test
    fun onlyNoVersionOrTheWholeNumberOneIsVersionOne() {
        assertTrue(Stored.current(JSONObject("{}")))
        assertTrue(Stored.current(JSONObject("{\"v\":1}")))
        for (version in later) {
            assertTrue(version, !Stored.current(JSONObject("{\"v\":$version}")))
        }
    }

    @Test
    fun thePlanReadsWhatItCanAndLeavesOutTheRest() {
        assertEquals(emptyList<Alarm>(), Scheduler.planOf(null))
        for (text in garbled) assertEquals(text, emptyList<Alarm>(), Scheduler.planOf(text))
        val plan = JSONArray()
            .put(alarmJson("a"))
            .put("not an alarm")
            .put(JSONObject().put("id", "no routine"))
            .put(alarmJson("b").put("local", JSONArray(listOf("x"))))
            .put(JSONObject(ofVersion(alarmJson("c"), "2")))
            .put(alarmJson("d").put("v", 1))
        assertEquals(listOf("a", "d"), Scheduler.planOf(plan.toString()).map { it.id })
    }

    @Test
    fun theLabelsFallBackToTheirNames() {
        val labels = JSONObject().put("done", "Erledigt").put("snooze", 5)
        assertEquals("Erledigt", Scheduler.labelOf(labels.toString(), "done"))
        assertEquals("snooze", Scheduler.labelOf(labels.toString(), "snooze"))
        assertEquals("ringChannel", Scheduler.labelOf(labels.toString(), "ringChannel"))
        assertEquals("done", Scheduler.labelOf(null, "done"))
        for (text in garbled) assertEquals(text, "done", Scheduler.labelOf(text, "done"))
        for (version in later) assertEquals("done", Scheduler.labelOf(ofVersion(labels, version), "done"))
    }

    @Test
    fun theNudgesReadAsNoneWhenTheyDoNotRead() {
        assertNotNull(Scheduler.nudgeOf(nudgeJson().toString()))
        assertEquals("Locked", Scheduler.changesOf(nudgeJson().put("public", "Locked").toString())?.second)
        assertNull(Scheduler.nudgeOf(null))
        assertNull(Scheduler.changesOf(null))
        for (text in garbled) {
            assertNull(text, Scheduler.nudgeOf(text))
            assertNull(text, Scheduler.changesOf(text))
        }
        val broken = nudgeJson().put("anchors", JSONArray().put("x")).toString()
        assertNull(Scheduler.nudgeOf(broken))
        assertNull(Scheduler.changesOf(broken))
        for (version in later) {
            assertNull(Scheduler.nudgeOf(ofVersion(nudgeJson(), version)))
            assertNull(Scheduler.changesOf(ofVersion(nudgeJson(), version)))
        }
    }

    @Test
    fun theWidgetSnapshotReadsWhatItCan() {
        val row = JSONObject().put("routine", "routine:a").put("occurrence", "2026-10-24T08:00").put("title", "Pills")
        val snapshot = JSONObject()
            .put("days", JSONObject().put("2026-10-24", JSONArray().put(row).put("x").put(JSONObject().put("v", 2))))
            .put("labels", JSONObject().put("today", "Today"))
            .put("unsorted", 2)
        val read = AgendaWidgetPlan.snapshotOf(snapshot.toString())
        assertEquals(listOf("Pills"), read?.days?.get("2026-10-24")?.map { it.title })
        assertEquals("Today", read?.today)
        assertNull(AgendaWidgetPlan.snapshotOf(null))
        for (text in garbled) assertNull(text, AgendaWidgetPlan.snapshotOf(text))
        for (version in later) assertNull(AgendaWidgetPlan.snapshotOf(ofVersion(snapshot, version)))
        // Without a snapshot the widget asks to open the app.
        assertTrue(AgendaWidgetPlan.view(null, "2026-10-24", emptyList(), 3).stale)
    }

    private val pills = Alarm(id = "a", routine = "routine:a", occurrence = "2026-10-24T08:00", local = null, at = null, title = "Pills", body = "")

    @Test
    fun aDoneTapIsQueuedWhateverTheQueueHeld() {
        for (text in garbled + listOf<String?>(null)) {
            val prefs = FakePrefs()
            if (text != null) prefs.values["actions"] = text
            EventQueue.add(prefs, "done", pills, 0, 1_000)
            val pending = EventQueue.pending(prefs)
            assertEquals("$text", listOf("done"), pending.map { it.kind })
            // A queue that did not read is kept aside as it was.
            val array = runCatching { JSONArray(text) }.isSuccess
            assertEquals("$text", if (text == null || array) null else text, prefs.values["unread"])
        }
        val prefs = FakePrefs()
        prefs.values["actions"] = 3L
        prefs.values["seq"] = "seven"
        EventQueue.add(prefs, "done", pills, 0, 1_000)
        assertEquals(listOf(1L), EventQueue.pending(prefs).map { it.seq })
    }

    @Test
    fun whatTheAppCannotApplyStaysQueuedPastItsAcknowledgement() {
        val prefs = FakePrefs()
        val queued = JSONArray()
            .put(JSONObject().put("seq", 1).put("kind", "done").put("routine", "r").put("occurrence", "o").put("at", 5))
            .put(JSONObject().put("seq", 2).put("kind", "laterKind").put("routine", "r").put("occurrence", "o").put("at", 5))
            .put(JSONObject().put("seq", 3).put("kind", "done").put("routine", "r").put("occurrence", "o").put("at", 5).put("v", 2))
            .put(JSONObject().put("seq", 4).put("kind", "done"))
            .put("garbage")
        prefs.values["actions"] = queued.toString()
        prefs.values["seq"] = 4L
        EventQueue.add(prefs, "snooze", pills, 9_000, 1_000)
        val pending = EventQueue.pending(prefs)
        assertEquals(listOf(1L, 5L), pending.map { it.seq })
        EventQueue.ack(prefs, pending.maxOf { it.seq })
        assertEquals(emptyList<Action>(), EventQueue.pending(prefs))
        val left = JSONArray(prefs.values["actions"] as String)
        assertEquals(4, left.length())
        assertEquals(listOf(2L, 3L, 4L), (0..2).map { left.getJSONObject(it).getLong("seq") })
        assertEquals("garbage", left.getString(3))
        // The next tap counts on past every one of them.
        prefs.values["seq"] = 0L
        EventQueue.add(prefs, "done", pills, 0, 2_000)
        assertEquals(listOf(5L), EventQueue.pending(prefs).map { it.seq })
    }

    @Test
    fun anAcknowledgementLeavesAQueueThatDoesNotReadAsItWas() {
        for (text in garbled.filter { runCatching { JSONArray(it) }.isFailure }) {
            val prefs = FakePrefs()
            prefs.values["actions"] = text
            EventQueue.ack(prefs, 100)
            assertEquals(text, prefs.values["actions"])
        }
        val prefs = FakePrefs()
        prefs.values["actions"] = 7L
        EventQueue.ack(prefs, 100)
        assertEquals(7L, prefs.values["actions"])
    }
}
