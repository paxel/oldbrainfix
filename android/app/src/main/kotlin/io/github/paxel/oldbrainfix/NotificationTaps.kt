package io.github.paxel.oldbrainfix

/**
 * The trace of the notification taps (obf-1039): which kind of
 * notification opened the app, whether the app was running, in the
 * background or started by it, and when. The app asks for it once and
 * notes where the tap took it; nothing of a Note's text is in it.
 */
object NotificationTaps {
    const val RUNNING = "running"
    const val BACKGROUND = "background"
    const val COLD = "cold"

    /** The kind an Occurrence's content intent carries: its ring's or its Heads-up's. */
    const val EXTRA_KIND = "oldbrainfix.open.kind"
    const val OCCURRENCE = "occurrence"
    const val HEADS_UP = "headsUp"

    /**
     * How the app was when the tap came: [cold] when it started the
     * activity, the background when the activity was [stopped] since it
     * last resumed, else running (also with the shade pulled down over
     * it).
     */
    fun from(cold: Boolean, stopped: Boolean): String = when {
        cold -> COLD
        stopped -> BACKGROUND
        else -> RUNNING
    }

    /**
     * The kind of notification an intent with the extras [flag] and
     * [extra] comes from, as the app reads it ("ring", "nudge",
     * "changes", "catchUp", "occurrence", "headsUp"); null for any other
     * intent, or one brought back from the recent apps ([fromHistory]).
     */
    fun kindOf(flag: (String) -> Boolean, extra: (String) -> String?, fromHistory: Boolean = false): String? {
        if (fromHistory) return null
        return when {
            flag(MainActivity.EXTRA_RING) -> "ring"
            flag(MainActivity.EXTRA_NUDGE) -> "nudge"
            flag(MainActivity.EXTRA_CHANGES) -> "changes"
            flag(MainActivity.EXTRA_CATCH_UP) -> "catchUp"
            OccurrenceOpen.of(extra) != null -> if (extra(EXTRA_KIND) == HEADS_UP) HEADS_UP else OCCURRENCE
            else -> null
        }
    }

    /** The tap as the app reads it: its [kind], how the app was ([from]) and when ([at], ms). */
    fun arrival(kind: String, from: String, at: Long): Map<String, Any> =
        mapOf("kind" to kind, "from" to from, "at" to at)
}
