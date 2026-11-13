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
 * pictures, sound files and other files are copied at once, while the
 * sharing app's permission to read them lasts; pictures are made into
 * photos when the app takes them, sound files are kept as they are as
 * voice recordings, and any other file becomes an Attachment (#267).
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
        val attachments: List<Files.Copied> = emptyList(),
    )

    /** What a shared file becomes. */
    enum class Kind { PICTURE, SOUND, FILE }

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
     * What a shared file of MIME type [fileType], as its provider says,
     * in a share of [shareType] becomes: sound by [isSound]; a picture by
     * its own type, or by the share's while its own says no more than
     * "some file"; any other file an Attachment (#267), so a PDF shared
     * with pictures is never taken for one.
     */
    fun kindOf(fileType: String?, shareType: String?): Kind = when {
        isSound(fileType, shareType) -> Kind.SOUND
        isImage(fileType) -> Kind.PICTURE
        (fileType == null || fileType == "application/octet-stream") && isImage(shareType) -> Kind.PICTURE
        else -> Kind.FILE
    }

    /** The endings of the app's own files: a bundle, a Sent Item, a Profile card, a Pack. */
    private val ownEndings = listOf(".obfsync", ".oldbrain", ".obfcard", ".obfpack")

    /**
     * Whether a file of MIME [type] named [name] is one of the app's own,
     * by its type or its ending in any case: shared to the share target
     * instead of "Open with", it still opens as "Open with" opens it
     * rather than becoming an Attachment.
     */
    fun isOwnFile(type: String?, name: String): Boolean =
        type?.startsWith("application/vnd.oldbrainfix.") == true ||
            ownEndings.any { name.endsWith(it, ignoreCase = true) }

    /**
     * Keeps what [intent] shares: its text, subject, pictures, sound
     * files and other files, from [source], each file by [kindOf]; a file
     * of the app's own ([isOwnFile]) goes to [Bundles] as if opened with
     * the app. Whether one did, so the app is told.
     */
    fun receive(context: Context, intent: Intent, source: Pair<String, String?>? = null): Boolean {
        val streams: List<Uri> = when (intent.action) {
            Intent.ACTION_SEND ->
                listOfNotNull(IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java))
            Intent.ACTION_SEND_MULTIPLE ->
                IntentCompat.getParcelableArrayListExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
                    .orEmpty()
            else -> return false
        }
        val types = streams.associateWith { uri ->
            runCatching { context.contentResolver.getType(uri) }.getOrNull()
        }
        val kinds = streams.groupBy { uri -> kindOf(types[uri], intent.type) }
        val (own, others) = kinds[Kind.FILE].orEmpty().partition { uri ->
            isOwnFile(types[uri], Files.nameOf(context, uri))
        }
        val opened = own.count { Bundles.keep(context, it) } > 0
        val files = copy(context, kinds[Kind.PICTURE].orEmpty())
        val soundFiles = copy(context, kinds[Kind.SOUND].orEmpty())
        val attachments = Files.copy(context, others)
        val text = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
        val subject = intent.getStringExtra(Intent.EXTRA_SUBJECT)
        if (text.isNullOrBlank() && subject.isNullOrBlank() && files.isEmpty() && soundFiles.isEmpty() &&
            attachments.isEmpty()
        ) {
            return opened
        }
        synchronized(waiting) {
            waiting.addLast(Share(subject, text, files, source, soundFiles, attachments))
        }
        return opened
    }

    /**
     * Keeps the files [uris] picked with the file picker while the app was
     * gone, as with [keepPicked]: they come back as a share of their own.
     */
    fun keepPickedFiles(context: Context, uris: List<Uri>) {
        val attachments = Files.copy(context, uris)
        if (attachments.isEmpty()) return
        synchronized(waiting) { waiting.addLast(Share(null, null, emptyList(), attachments = attachments)) }
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
     * ([Speech.problem]); its other files as their copies' `path`, `name`
     * and `type`, which the app lets go of once attached. Slow: decodes
     * pictures.
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
            "files" to Files.handed(share.attachments),
            "sourcePackage" to share.source?.first,
            "sourceLabel" to share.source?.second,
        )
    }
}

/**
 * Where other apps share images, sound files, any other file, text and links to. No screen of its
 * own: it keeps the share and hands over to the app's one running
 * [MainActivity], so the share never starts a second app inside the
 * sharing app.
 */
class ShareActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Only this activity knows who shared: MainActivity's referrer
        // is the app itself.
        val opened = savedInstanceState == null && Shares.receive(this, intent, SourceApps.of(this))
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_SHARED, true)
                .putExtra(MainActivity.EXTRA_BUNDLE, opened)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
        finish()
    }
}
