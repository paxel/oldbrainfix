package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import androidx.core.content.IntentCompat
import java.io.File
import java.util.UUID

/**
 * What other apps shared in and the app has not taken yet. The shared
 * pictures are copied at once, while the sharing app's permission to read
 * them lasts; they are made into photos when the app takes them.
 */
object Shares {
    class Share(val subject: String?, val text: String?, val files: List<File>)

    private val waiting = ArrayDeque<Share>()

    /** Whether a share of MIME [type] carries pictures. */
    fun isImage(type: String?): Boolean = type?.startsWith("image/") == true

    /** Keeps what [intent] shares: its text, subject and pictures. */
    fun receive(context: Context, intent: Intent) {
        val streams: List<Uri> = when (intent.action) {
            Intent.ACTION_SEND ->
                if (isImage(intent.type)) {
                    listOfNotNull(IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java))
                } else {
                    emptyList()
                }
            Intent.ACTION_SEND_MULTIPLE ->
                IntentCompat.getParcelableArrayListExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
                    .orEmpty()
            else -> return
        }
        val dir = File(context.cacheDir, "shared").apply { mkdirs() }
        val files = streams.mapNotNull { uri ->
            runCatching {
                val file = File(dir, UUID.randomUUID().toString())
                context.contentResolver.openInputStream(uri)?.use { input ->
                    file.outputStream().use { input.copyTo(it) }
                } ?: return@runCatching null
                file
            }.getOrNull()
        }
        val text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
        val subject = intent.getStringExtra(Intent.EXTRA_SUBJECT)
        if (text.isNullOrBlank() && subject.isNullOrBlank() && files.isEmpty()) return
        synchronized(waiting) { waiting.addLast(Share(subject, text, files)) }
    }

    /**
     * Keeps the photo [shot] the camera took while the app was gone, as
     * on a low-memory phone that dropped it meanwhile: it comes back as
     * a share of its own instead of being lost.
     */
    fun keepPhoto(context: Context, shot: File) {
        val file = File(File(context.cacheDir, "shared").apply { mkdirs() }, UUID.randomUUID().toString())
        if (!shot.renameTo(file)) return
        synchronized(waiting) { waiting.addLast(Share(null, null, listOf(file))) }
    }

    /**
     * The oldest share not taken yet as the app's channel hands it over,
     * its pictures made into photos, or null. Slow: decodes pictures.
     */
    fun take(): Map<String, Any?>? {
        val share = synchronized(waiting) { waiting.removeFirstOrNull() } ?: return null
        val photos = share.files.mapNotNull { file ->
            Photos.normalize { file.inputStream() }.also { file.delete() }
        }
        return mapOf("subject" to share.subject, "text" to share.text, "photos" to photos)
    }
}

/**
 * Where other apps share images, text and links to. No screen of its
 * own: it keeps the share and hands over to the app's one running
 * [MainActivity], so the share never starts a second app inside the
 * sharing app.
 */
class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) Shares.receive(this, intent)
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_SHARED, true)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
        finish()
    }
}
