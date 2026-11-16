package io.github.paxel.oldbrainfix

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context

/**
 * What Paste takes from the clipboard: its text and its pictures. The
 * clipboard is read only when Paste is tapped, never on its own, as
 * Android shows a notice for every read. And what Copy puts there with
 * HTML beside it (#324).
 */
object Clipboards {
    /**
     * Puts [text] on the clipboard with [html] beside it, one item of
     * both: an app that takes formatted text takes the HTML, any other
     * the text (#324). Whether it is there.
     */
    fun putHtml(context: Context, text: String, html: String): Boolean = runCatching {
        val clipboard = context.getSystemService(ClipboardManager::class.java) ?: return false
        clipboard.setPrimaryClip(ClipData.newHtmlText("", text, html))
        true
    }.getOrDefault(false)

    /**
     * One item of the clipboard: its [text], and the content [uri] it
     * points to with the MIME [type] its provider says, when it does.
     */
    class Item<U>(val text: CharSequence?, val uri: U?, val type: String?)

    /**
     * The text of [items], one item's text per line and blank ones left
     * out, and the URIs of their pictures: those of a picture type, and
     * those whose type is not known, which only count once Android
     * decodes them. Other files, such as a contact, are left out.
     */
    fun <U> taken(items: List<Item<U>>): Pair<String, List<U>> {
        val text = items.mapNotNull { it.text?.toString()?.takeIf(String::isNotBlank) }
            .joinToString("\n")
        val pictures = items.mapNotNull { item ->
            item.uri?.takeIf { item.type == null || Shares.isImage(item.type) }
        }
        return text to pictures
    }
}
