package io.github.paxel.oldbrainfix

/**
 * A tap on an Occurrence's notification shows it on the Agenda, marked
 * (obf-1039): the notification's content intent carries the Routine and
 * the Occurrence, and the app asks for them once it shows.
 */
object OccurrenceOpen {
    const val EXTRA_ROUTINE = "oldbrainfix.open.routine"
    const val EXTRA_OCCURRENCE = "oldbrainfix.open.occurrence"

    /**
     * The extras of the content intent of [alarm]'s notification, its
     * Heads-up's when [headsUp], which the trace of the taps tells apart
     * ([NotificationTaps]).
     */
    fun extras(alarm: Alarm, headsUp: Boolean = false): Map<String, String> = mapOf(
        EXTRA_ROUTINE to alarm.routine,
        EXTRA_OCCURRENCE to alarm.occurrence,
        NotificationTaps.EXTRA_KIND to if (headsUp) NotificationTaps.HEADS_UP else NotificationTaps.OCCURRENCE,
    )

    /**
     * The Occurrence [extra] names, as the app reads it ("routine",
     * "occurrence"); null for an intent without both, or one brought
     * back from the recent apps ([fromHistory]).
     */
    fun of(extra: (String) -> String?, fromHistory: Boolean = false): Map<String, String>? {
        if (fromHistory) return null
        val routine = extra(EXTRA_ROUTINE)?.takeIf { it.isNotEmpty() } ?: return null
        val occurrence = extra(EXTRA_OCCURRENCE)?.takeIf { it.isNotEmpty() } ?: return null
        return mapOf("routine" to routine, "occurrence" to occurrence)
    }
}
