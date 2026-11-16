package io.github.paxel.oldbrainfix

/**
 * A tap on an Occurrence's notification opens the editor of its Note
 * (#270): the notification's content intent carries the Routine and the
 * Occurrence, and the app asks for them once it shows.
 */
object OccurrenceOpen {
    const val EXTRA_ROUTINE = "oldbrainfix.open.routine"
    const val EXTRA_OCCURRENCE = "oldbrainfix.open.occurrence"

    /** The extras of the content intent of [alarm]'s notification. */
    fun extras(alarm: Alarm): Map<String, String> = mapOf(
        EXTRA_ROUTINE to alarm.routine,
        EXTRA_OCCURRENCE to alarm.occurrence,
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
