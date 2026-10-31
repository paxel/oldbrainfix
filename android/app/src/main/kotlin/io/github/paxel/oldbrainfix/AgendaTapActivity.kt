package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.Intent
import android.os.Bundle

/** What a tap on a row of the Agenda widget's list does. */
sealed interface AgendaTap {
    /** A box checked: [kind] queued for [alarm], with the [line] it checks. */
    data class Check(val kind: String, val alarm: Alarm, val line: String) : AgendaTap

    /** The app opens on the Agenda. */
    data object Open : AgendaTap

    /** The app opens on the Unsorted list. */
    data object Unsorted : AgendaTap

    companion object {
        const val EXTRA_TAP = "oldbrainfix.tap"
        const val TAP_CHECK = "check"
        const val TAP_OPEN = "open"
        const val TAP_UNSORTED = "unsorted"
        const val EXTRA_KIND = "oldbrainfix.kind"
        const val EXTRA_LINE = "oldbrainfix.line"

        /** The kinds a box queues besides "done". */
        private val CHECKS = setOf(EventQueue.KIND_LINE, EventQueue.KIND_FROG, EventQueue.KIND_SHOPPING)

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
                    EventQueue.KIND_SHOPPING -> line.isNotEmpty()
                    else -> false
                }
                if (kind != null && reads && (kind == "done" || kind in CHECKS)) Check(kind, alarm, line) else Open
            }
            TAP_UNSORTED -> Unsorted
            else -> Open
        }
    }
}

/**
 * Where every tap on the Agenda widget's list lands (#153). Started by the
 * user's own tap, it may open the app, which a broadcast receiver may
 * not on later Androids; it shows nothing itself. A box checked goes
 * into the queue as before and the widget shows it, without opening
 * anything; any other tap opens the app, or its Unsorted list.
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
                AgendaTap.Open -> startActivity(AgendaWidget.openIntent(this))
            }
        }
        finish()
    }
}
