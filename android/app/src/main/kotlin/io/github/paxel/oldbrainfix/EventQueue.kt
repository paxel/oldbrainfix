package io.github.paxel.oldbrainfix

import android.content.Context
import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

/**
 * Done and snoozes tapped on notifications, kept until the app has
 * applied them to the log (adapted from Chaos Alert). Survives the
 * process dying; the app reads, applies, then acknowledges, so a crash in
 * between loses nothing.
 *
 * Nothing queued is dropped before the app applied it: an action that
 * does not read, of a kind not known or of a later version ([Stored]) is
 * not handed to the app and stays queued, and a queue that does not read
 * at all is kept aside, not written over, while a new tap starts another.
 */
object EventQueue {
    private const val PREFS = "oldbrainfix_actions"
    private const val KEY = "actions"
    private const val KEY_SEQ = "seq"

    /** Queues that did not read, one per line, as they were. */
    private const val KEY_UNREAD = "unread"

    /** The kinds of action this version applies. */
    private val KINDS = setOf("done", "snooze", NudgePlan.KIND_SNOOZE)

    @Synchronized
    fun add(context: Context, kind: String, alarm: Alarm, until: Long = 0) =
        add(prefs(context), kind, alarm, until, System.currentTimeMillis())

    internal fun add(prefs: SharedPreferences, kind: String, alarm: Alarm, until: Long, now: Long) {
        val stored = Stored.text(prefs, KEY)
        val read = Stored.array(stored)
        val edit = prefs.edit()
        if (stored != null && read == null) {
            val unread = Stored.text(prefs, KEY_UNREAD)
            edit.putString(KEY_UNREAD, if (unread == null) stored else "$unread\n$stored")
        }
        val actions = read ?: JSONArray()
        // Past every seq queued too, so an acknowledgement never takes a
        // new tap along with older ones.
        val seq = maxOf(Stored.long(prefs, KEY_SEQ), lastSeq(actions)) + 1
        actions.put(
            JSONObject()
                .put("seq", seq)
                .put("kind", kind)
                .put("routine", alarm.routine)
                .put("occurrence", alarm.occurrence)
                .put("at", now)
                .put("until", until)
                .put("title", alarm.title)
                .put("body", alarm.body),
        )
        edit.putString(KEY, actions.toString()).putLong(KEY_SEQ, seq).commit()
    }

    /** Every action not acknowledged yet, oldest first. */
    @Synchronized
    fun pending(context: Context): List<Action> = pending(prefs(context))

    internal fun pending(prefs: SharedPreferences): List<Action> = Stored.listOf(Stored.text(prefs, KEY), ::action)

    /** Drops the actions up to [seq]: the app has applied them. */
    @Synchronized
    fun ack(context: Context, seq: Long) = ack(prefs(context), seq)

    internal fun ack(prefs: SharedPreferences, seq: Long) {
        val actions = Stored.array(Stored.text(prefs, KEY)) ?: return
        val kept = JSONArray()
        for (i in 0 until actions.length()) {
            val applied = Stored.elementOf(actions, i, ::action)?.let { it.seq <= seq } ?: false
            if (!applied) kept.put(actions.opt(i))
        }
        prefs.edit().putString(KEY, kept.toString()).commit()
    }

    /** The action [json] holds, failing on one this version does not apply. */
    private fun action(json: JSONObject): Action {
        val kind = json.getString("kind")
        require(kind in KINDS) { kind }
        return Action(
            seq = json.getLong("seq"),
            kind = kind,
            routine = json.getString("routine"),
            occurrence = json.getString("occurrence"),
            at = json.getLong("at"),
            until = json.optLong("until"),
            title = json.optString("title"),
            body = json.optString("body"),
        )
    }

    /** The highest seq in [actions], of those that do not read too; 0 without one. */
    private fun lastSeq(actions: JSONArray): Long =
        (0 until actions.length()).maxOfOrNull { actions.optJSONObject(it)?.optLong("seq") ?: 0 } ?: 0

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
