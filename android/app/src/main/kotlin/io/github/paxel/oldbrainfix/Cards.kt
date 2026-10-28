package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.os.Bundle
import java.net.URI

/**
 * Profile cards (ADR 0017, #107) on the phone's side: the card links
 * tapped in another app, kept until the app takes them, and the own
 * card handed to the app the user picks as plain text. The app opens
 * each link; nothing here reads the card.
 */
object Cards {
    /** The only address a card link has; the card follows its `#`. */
    const val HOST = "paxel.github.io"
    const val PATH = "/oldbrainfix/p"

    /** More than any card holds; a longer link is no card and is left out. */
    const val MAX_LENGTH = 64 * 1024

    private val waiting = ArrayDeque<String>()

    /** The card link an "open with" ([Intent.ACTION_VIEW]) of [link] names, or null for any other. */
    fun linkOf(action: String?, link: String?): String? {
        if (action != Intent.ACTION_VIEW || link == null || link.length > MAX_LENGTH) return null
        val uri = runCatching { URI(link) }.getOrNull() ?: return null
        val ours = uri.scheme.equals("https", ignoreCase = true) &&
            uri.host.equals(HOST, ignoreCase = true) &&
            uri.rawPath == PATH
        return if (ours) link else null
    }

    /** Keeps the card link [intent] opens, to be taken by the app. */
    fun receive(intent: Intent) {
        val link = linkOf(intent.action, intent.dataString) ?: return
        synchronized(waiting) { waiting.addLast(link) }
    }

    /** The links not taken yet, oldest first; each is handed out once. */
    fun take(): List<String> = synchronized(waiting) {
        waiting.toList().also { waiting.clear() }
    }

    /** Opens the share chooser for [text]; false when nothing can take it. No subject: some apps put it before the text, and the link goes last. */
    fun share(context: Context, text: String): Boolean {
        val send = Intent(Intent.ACTION_SEND)
            .setType("text/plain")
            .putExtra(Intent.EXTRA_TEXT, text)
        return try {
            context.startActivity(Intent.createChooser(send, null))
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }
}

/**
 * Where a Profile card link tapped in another app arrives (the App Link
 * of `https://paxel.github.io/oldbrainfix/p`). No screen of its own: it
 * keeps the link and hands over to the app's one running
 * [MainActivity], which opens the card.
 */
class CardActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) Cards.receive(intent)
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_CARD, true)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
        finish()
    }
}
