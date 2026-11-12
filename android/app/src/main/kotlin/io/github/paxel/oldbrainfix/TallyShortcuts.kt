package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.core.content.pm.ShortcutInfoCompat
import androidx.core.content.pm.ShortcutManagerCompat
import androidx.core.graphics.drawable.IconCompat

/**
 * A Tally's shortcut on the home screen (#284): "Add to home screen" in
 * its card's ⋮ pins one, and a tap on it opens the Tally's entry page.
 * The shortcut starts [TallyEntryActivity], which no other app may
 * start, with the Tally's Note id; that hands it to the app's one
 * [MainActivity] in the process, never in an intent, so no other app
 * can open an entry page.
 */
object TallyShortcuts {
    /** The Tally's Note id in the shortcut's intent. */
    const val EXTRA_NOTE = "oldbrainfix.tally.note"

    /** The action [TallyEntryActivity] starts [MainActivity] with. */
    const val ACTION = "io.github.paxel.oldbrainfix.action.TALLY_ENTRY"

    private const val PREFIX = "tally:"

    /** The shortcut's id: one per Tally, so pinning it again replaces it. */
    fun id(note: String): String = PREFIX + note

    /** The label as Android shows it: the Tally's name, "Tally" without one. */
    fun label(label: String?, fallback: String): String =
        label?.trim()?.takeIf { it.isNotEmpty() } ?: fallback

    /**
     * The Tally's Note id a shortcut's launch asks for, or null: none
     * without one, and none when Android reopens the app from the recent
     * apps with the intent that once started it.
     */
    fun noteOf(extra: String?, flags: Int): String? {
        if (flags and Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY != 0) return null
        return extra?.trim()?.takeIf { it.isNotEmpty() }
    }

    /**
     * Asks the home screen to place the shortcut to the Tally of [note],
     * named [label], with Android's own prompt; false where the home
     * screen takes no shortcuts.
     */
    fun pin(context: Context, note: String, label: String): Boolean {
        if (!ShortcutManagerCompat.isRequestPinShortcutSupported(context)) return false
        val name = label(label, context.getString(R.string.tally_shortcut))
        val shortcut = ShortcutInfoCompat.Builder(context, id(note))
            .setShortLabel(name)
            .setLongLabel(name)
            .setIcon(IconCompat.createWithResource(context, R.drawable.ic_shortcut_tally))
            .setIntent(
                Intent(context, TallyEntryActivity::class.java)
                    .setAction(Intent.ACTION_VIEW)
                    .putExtra(EXTRA_NOTE, note),
            )
            .build()
        return try {
            ShortcutManagerCompat.requestPinShortcut(context, shortcut, null)
        } catch (_: IllegalStateException) {
            // Android takes the request only while the app is in front.
            false
        }
    }
}

/**
 * Where a Tally's shortcut lands. No screen of its own: it tells the
 * running [MainActivity], or the one it starts, which Tally was asked
 * for, like [CaptureActivity], so a launcher that clears the task it
 * starts does not close what the app had open.
 */
class TallyEntryActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) {
            TallyShortcuts.noteOf(intent.getStringExtra(TallyShortcuts.EXTRA_NOTE), intent.flags)?.let {
                MainActivity.tallyAsked(it)
                startActivity(
                    Intent(this, MainActivity::class.java)
                        .setAction(TallyShortcuts.ACTION)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
                )
            }
        }
        finish()
    }
}
