package io.github.paxel.oldbrainfix

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import androidx.core.content.FileProvider
import androidx.core.content.IntentCompat
import java.io.File
import java.util.UUID

/**
 * Sync bundle files (ADR 0003) and Sent Items (ADR 0009) on the phone's
 * side: each file another app opened with this one, or the user picked,
 * is copied at once, while the permission to read it lasts, into the
 * app's cache; the app imports the copy, or previews a Sent Item, and
 * removes it. A file to share is written into the exports the
 * FileProvider serves.
 */
object Bundles {
    private val waiting = ArrayDeque<String>()

    /** Where the copies wait to be imported. */
    fun dir(context: Context): File = File(context.cacheDir, "bundles").apply { mkdirs() }

    /** Copies the bundle at [uri]; its path, or null when it could not be read. */
    fun copy(context: Context, uri: Uri): String? = runCatching {
        val file = File(dir(context), "${UUID.randomUUID()}.obfsync")
        context.contentResolver.openInputStream(uri)?.use { input ->
            file.outputStream().use { input.copyTo(it) }
        } ?: return@runCatching null
        file.path
    }.getOrNull()

    /** Keeps the bundle [intent] opens or sends, to be taken by the app. */
    fun receive(context: Context, intent: Intent) {
        val uri = uriOf(intent) ?: return
        val path = copy(context, uri) ?: return
        synchronized(waiting) { waiting.addLast(path) }
    }

    /** The bundle an "open with" ([Intent.ACTION_VIEW]) or a share names. */
    fun uriOf(intent: Intent): Uri? = when (intent.action) {
        Intent.ACTION_VIEW -> intent.data
        Intent.ACTION_SEND -> IntentCompat.getParcelableExtra(intent, Intent.EXTRA_STREAM, Uri::class.java)
        else -> null
    }

    /** The copies not taken yet, oldest first; each is handed out once. */
    fun take(): List<String> = synchronized(waiting) {
        waiting.toList().also { waiting.clear() }
    }

    /** Where a bundle named [name] is written to be shared. */
    fun exportPath(context: Context, name: String): String =
        File(FileShare.dir(context), FileShare.safeName(name)).path

    /** Whether [path] lies directly in [dir], so nothing else is ever shared. */
    fun inside(dir: File, path: String): Boolean {
        val file = File(path)
        return file.parentFile?.canonicalPath == dir.canonicalPath && file.name.isNotEmpty()
    }

    /**
     * Whether an app was picked on the share chooser opened last: the
     * chooser tells [ShareChosenReceiver], and nothing when it is left
     * without one.
     */
    @Volatile
    var chosen = false

    /**
     * The share chooser for the bundle at [path] as [type], telling
     * [ShareChosenReceiver] which app is picked; null when there is no
     * such file to share.
     */
    fun chooser(context: Context, path: String, type: String): Intent? {
        if (!inside(FileShare.dir(context), path)) return null
        val file = File(path)
        if (!file.isFile) return null
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.photos", file)
        val send = Intent(Intent.ACTION_SEND)
            .setType(type)
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        send.clipData = ClipData.newRawUri("", uri)
        // Mutable, so the chooser can put in the app picked; explicit, so
        // only this app's receiver hears it.
        val picked = PendingIntent.getBroadcast(
            context, 0, Intent(context, ShareChosenReceiver::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or
                (if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0),
        )
        chosen = false
        return Intent.createChooser(send, null, picked.intentSender)
    }
}

/** Hears from the share chooser that an app was picked ([Intent.EXTRA_CHOSEN_COMPONENT]). */
class ShareChosenReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        Bundles.chosen = true
    }
}

/**
 * Where other apps open or share a sync bundle or a Sent Item with this
 * one. No screen of its own: it keeps a copy and hands over to the app's
 * one running [MainActivity], which imports or previews it.
 */
class BundleActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) Bundles.receive(this, intent)
        startActivity(
            Intent(this, MainActivity::class.java)
                .putExtra(MainActivity.EXTRA_BUNDLE, true)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
        finish()
    }
}
