package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.Intent
import android.os.Bundle

/** What a tap on a row of the Agenda widget's list does. */
sealed interface AgendaTap {
    /** The fill-in extras that say this tap, as [of] reads them. */
    fun extras(): Map<String, String>

    /** A box checked: [kind] queued for [alarm], with the [line] it checks. */
    data class Check(val kind: String, val alarm: Alarm, val line: String) : AgendaTap {
        override fun extras() = mapOf(
            EXTRA_TAP to TAP_CHECK,
            EXTRA_KIND to kind,
            AlarmReceiver.EXTRA_ROUTINE to alarm.routine,
            AlarmReceiver.EXTRA_OCCURRENCE to alarm.occurrence,
            EXTRA_LINE to line,
            AlarmReceiver.EXTRA_TITLE to alarm.title,
            AlarmReceiver.EXTRA_BODY to alarm.body,
        )
    }

    /** The app opens on the entry form of the pick of the Tally [note] for [occurrence] of [session] (#209). */
    data class Pick(val session: String, val occurrence: String, val note: String) : AgendaTap {
        override fun extras() = mapOf(
            EXTRA_TAP to TAP_PICK,
            AlarmReceiver.EXTRA_ROUTINE to session,
            AlarmReceiver.EXTRA_OCCURRENCE to occurrence,
            EXTRA_NOTE to note,
        )
    }

    /** The app opens on the Agenda. */
    data object Open : AgendaTap {
        override fun extras() = mapOf(EXTRA_TAP to TAP_OPEN)
    }

    /** The app opens on the Unsorted list. */
    data object Unsorted : AgendaTap {
        override fun extras() = mapOf(EXTRA_TAP to TAP_UNSORTED)
    }

    /** Nothing happens: a Session's box only shows Done (#209). */
    data object None : AgendaTap {
        override fun extras() = mapOf(EXTRA_TAP to TAP_NONE)
    }

    companion object {
        const val EXTRA_TAP = "oldbrainfix.tap"
        const val TAP_CHECK = "check"
        const val TAP_OPEN = "open"
        const val TAP_UNSORTED = "unsorted"
        const val TAP_PICK = "pick"
        const val TAP_NONE = "none"
        const val EXTRA_KIND = "oldbrainfix.kind"
        const val EXTRA_LINE = "oldbrainfix.line"
        const val EXTRA_NOTE = "oldbrainfix.note"

        /** The kinds a box queues besides "done". */
        private val CHECKS = setOf(EventQueue.KIND_LINE, EventQueue.KIND_FROG, EventQueue.KIND_SHOPPING, EventQueue.KIND_CHECKLIST)

        /**
         * The tap the fill-in extras say, free of Android so the JVM tests
         * cover it ([extra] reads one by name). A box whose check does not
         * read, such as a Done without its Occurrence, opens the app
         * rather than queue something the app cannot apply.
         */
        fun of(extra: (String) -> String?): AgendaTap = when (extra(EXTRA_TAP)) {
            TAP_CHECK -> {
                val kind = extra(EXTRA_KIND)
                val alarm = Alarm(
                    id = "",
                    routine = extra(AlarmReceiver.EXTRA_ROUTINE) ?: "",
                    occurrence = extra(AlarmReceiver.EXTRA_OCCURRENCE) ?: "",
                    local = null,
                    at = null,
                    title = extra(AlarmReceiver.EXTRA_TITLE) ?: "",
                    body = extra(AlarmReceiver.EXTRA_BODY) ?: "",
                )
                val line = extra(EXTRA_LINE) ?: ""
                val reads = when (kind) {
                    "done" -> alarm.routine.isNotEmpty() && alarm.occurrence.isNotEmpty()
                    EventQueue.KIND_LINE -> alarm.routine.isNotEmpty() && alarm.occurrence.isNotEmpty() && line.isNotEmpty()
                    EventQueue.KIND_FROG -> alarm.routine.isNotEmpty()
                    EventQueue.KIND_SHOPPING, EventQueue.KIND_CHECKLIST -> line.isNotEmpty()
                    else -> false
                }
                if (kind != null && reads && (kind == "done" || kind in CHECKS)) Check(kind, alarm, line) else Open
            }
            TAP_PICK -> {
                val session = extra(AlarmReceiver.EXTRA_ROUTINE) ?: ""
                val occurrence = extra(AlarmReceiver.EXTRA_OCCURRENCE) ?: ""
                val note = extra(EXTRA_NOTE) ?: ""
                if (session.isNotEmpty() && occurrence.isNotEmpty() && note.isNotEmpty()) {
                    Pick(session, occurrence, note)
                } else {
                    Open
                }
            }
            TAP_UNSORTED -> Unsorted
            TAP_NONE -> None
            else -> Open
        }
    }
}

/**
 * Where every tap on the Agenda widget's list lands (#153). Started by the
 * user's own tap, it may open the app, which a broadcast receiver may
 * not on later Androids; it shows nothing itself. A box checked goes
 * into the queue as before and the widget shows it, without opening
 * anything; a Session's box does nothing; a pick opens the app on its
 * entry form (#209); any other tap opens the app, or its Unsorted list.
 */
class AgendaTapActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) {
            when (val tap = AgendaTap.of { intent.getStringExtra(it) }) {
                is AgendaTap.Check -> AgendaWidget.check(this, tap)
                AgendaTap.Unsorted -> startActivity(
                    Intent(this, MainActivity::class.java)
                        .putExtra(MainActivity.EXTRA_UNSORTED, true)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
                )
                is AgendaTap.Pick -> startActivity(
                    Intent(this, MainActivity::class.java)
                        .putExtra(MainActivity.EXTRA_PICK_SESSION, tap.session)
                        .putExtra(MainActivity.EXTRA_PICK_OCCURRENCE, tap.occurrence)
                        .putExtra(MainActivity.EXTRA_PICK_NOTE, tap.note)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
                )
                AgendaTap.Open -> startActivity(AgendaWidget.openIntent(this))
                AgendaTap.None -> {}
            }
        }
        finish()
    }
}
