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
 * pictures and sound files are copied at once, while the sharing app's
 * permission to read them lasts; pictures are made into photos when the
 * app takes them, sound files are kept as they are as voice recordings.
 */
object Shares {
    /**
     * One share: its pictures [files] and its sound files [sounds];
     * [source] is the package and label of the app it came from, when
     * Android said.
     */
    class Share(
        val subject: String?,
        val text: String?,
        val files: List<File>,
        val source: Pair<String, String?>? = null,
        val sounds: List<File> = emptyList(),
    )

    private val waiting = ArrayDeque<Share>()

    /** Whether a share of MIME [type] carries pictures. */
    fun isImage(type: String?): Boolean = type?.startsWith("image/") == true

    /**
     * Whether a share of MIME [type] carries sound, such as a messenger's
     * voice message: `audio/…`, or an Ogg file some apps name by its
     * container alone.
     */
    fun isAudio(type: String?): Boolean =
        type?.startsWith("audio/") == true || type == "application/ogg"

    /**
     * Whether a shared file of MIME type [fileType], as its provider
     * says, in a share of [shareType] is sound: by its own type, or by
     * the share's unless its own says picture. A provider that only
     * knows "some file" never makes a voice message a picture.
     */
    fun isSound(fileType: String?, shareType: String?): Boolean =
        isAudio(fileType) || (isAudio(shareType) && !isImage(fileType))

    /**
     * Keeps what [intent] shares: its text, subject, pictures and sound
     * files, from [source]. Each file goes by [isSound]; what is not
     * sound is taken for a picture.
     */
    fun receive(context: Context, intent: Intent, source: Pair<String, String?>? = null) {
        val streams: List<Uri> = when (intent.action) {
            Intent.ACTION_SEND ->
                if (isImage(intent.type) || isAudio(intent.type)) {
                    listOfNotNull(IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java))
                } else {
                    emptyList()
                }
            Intent.ACTION_SEND_MULTIPLE ->
                IntentCompat.getParcelableArrayListExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
                    .orEmpty()
            else -> return
        }
        val (sounds, pictures) = streams.partition { uri ->
            isSound(runCatching { context.contentResolver.getType(uri) }.getOrNull(), intent.type)
        }
        val files = copy(context, pictures)
        val soundFiles = copy(context, sounds)
        val text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
        val subject = intent.getStringExtra(Intent.EXTRA_SUBJECT)
        if (text.isNullOrBlank() && subject.isNullOrBlank() && files.isEmpty() && soundFiles.isEmpty()) return
        synchronized(waiting) { waiting.addLast(Share(subject, text, files, source, soundFiles)) }
    }

    /** Copies the files behind [uris] to the side, while the permission to read them lasts. */
    private fun copy(context: Context, uris: List<Uri>): List<File> {
        val dir = File(context.cacheDir, "shared").apply { mkdirs() }
        return uris.mapNotNull { uri ->
            runCatching {
                val file = File(dir, UUID.randomUUID().toString())
                context.contentResolver.openInputStream(uri)?.use { input ->
                    file.outputStream().use { input.copyTo(it) }
                } ?: return@runCatching null
                file
            }.getOrNull()
        }
    }

    /**
     * Keeps the pictures [uris] picked from the gallery while the app was
     * gone, as with [keepPhoto]: they come back as a share of their own.
     */
    fun keepPicked(context: Context, uris: List<Uri>) {
        val files = copy(context, uris)
        if (files.isEmpty()) return
        synchronized(waiting) { waiting.addLast(Share(null, null, files)) }
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
     * its pictures made into photos, or null. Its sound files come as
     * they are, each as a map of its `bytes` and, when this phone cannot
     * decode it to transcribe it, the `problem` and the `format`
     * ([Speech.problem]). Slow: decodes pictures.
     */
    fun take(): Map<String, Any?>? {
        val share = synchronized(waiting) { waiting.removeFirstOrNull() } ?: return null
        val photos = share.files.mapNotNull { file ->
            Photos.normalize { file.inputStream() }.also { file.delete() }
        }
        val recordings = share.sounds.mapNotNull { file ->
            try {
                val bytes = file.readBytes()
                if (bytes.isEmpty()) return@mapNotNull null
                val problem = Speech.problem(file)
                mapOf("bytes" to bytes, "problem" to problem?.kind, "format" to problem?.format)
            } catch (e: Exception) {
                null
            } finally {
                file.delete()
            }
        }
        return mapOf(
            "subject" to share.subject,
            "text" to share.text,
            "photos" to photos,
            "recordings" to recordings,
            "sourcePackage" to share.source?.first,
            "sourceLabel" to share.source?.second,
        )
    }
}

/**
 * Where other apps share images, sound files, text and links to. No screen of its
 * own: it keeps the share and hands over to the app's one running
 * [MainActivity], so the share never starts a second app inside the
 * sharing app.
 */
class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Only this activity knows who shared: MainActivity's referrer
        // is the app itself.
        if (savedInstanceState == null) Shares.receive(this, intent, SourceApps.of(this))
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_SHARED, true)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
        finish()
    }
}
