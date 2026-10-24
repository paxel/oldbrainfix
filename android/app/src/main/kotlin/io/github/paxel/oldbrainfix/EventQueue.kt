package io.github.paxel.oldbrainfix

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

/**
 * Done and snoozes tapped on notifications, kept until the app has
 * applied them to the log (adapted from Chaos Alert). Survives the
 * process dying; the app reads, applies, then acknowledges, so a crash in
 * between loses nothing.
 */
object EventQueue {
    private const val PREFS = "oldbrainfix_actions"
    private const val KEY = "actions"
    private const val KEY_SEQ = "seq"

    @Synchronized
    fun add(context: Context, kind: String, alarm: Alarm, until: Long = 0) {
        val prefs = prefs(context)
        val seq = prefs.getLong(KEY_SEQ, 0) + 1
        val actions = JSONArray(prefs.getString(KEY, "[]") ?: "[]")
        actions.put(
            JSONObject()
                .put("seq", seq)
                .put("kind", kind)
                .put("routine", alarm.routine)
                .put("occurrence", alarm.occurrence)
                .put("at", System.currentTimeMillis())
                .put("until", until)
                .put("title", alarm.title)
                .put("body", alarm.body),
        )
        prefs.edit().putString(KEY, actions.toString()).putLong(KEY_SEQ, seq).commit()
    }

    /** Every action not acknowledged yet, oldest first. */
    @Synchronized
    fun pending(context: Context): List<Action> {
        val actions = JSONArray(prefs(context).getString(KEY, "[]") ?: "[]")
        return (0 until actions.length()).map { i ->
            val a = actions.getJSONObject(i)
            Action(
                seq = a.getLong("seq"),
                kind = a.getString("kind"),
                routine = a.getString("routine"),
                occurrence = a.getString("occurrence"),
                at = a.getLong("at"),
                until = a.optLong("until"),
                title = a.optString("title"),
                body = a.optString("body"),
            )
        }
    }

    /** Drops the actions up to [seq]: the app has applied them. */
    @Synchronized
    fun ack(context: Context, seq: Long) {
        val prefs = prefs(context)
        val actions = JSONArray(prefs.getString(KEY, "[]") ?: "[]")
        val kept = JSONArray()
        for (i in 0 until actions.length()) {
            val a = actions.getJSONObject(i)
            if (a.getLong("seq") > seq) kept.put(a)
        }
        prefs.edit().putString(KEY, kept.toString()).commit()
    }

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
