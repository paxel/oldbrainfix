package io.github.paxel.oldbrainfix

import android.content.Context
import android.net.Uri
import android.os.Handler
import android.os.Looper
import androidx.documentfile.provider.DocumentFile
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.Executors

/**
 * The shared sync folder through the document tree the folder picker
 * granted (Storage Access Framework), adapted from Catlog: a persistable
 * tree URI, good for shared storage, a USB stick and cloud providers
 * alike, none of which is a path an app may read. Dart sees the folder
 * as `oldbrainfix-sync` with `keys` and `blobs` inside, addressed by
 * name; Backups go straight into the folder picked for them ([put]).
 * Everything here needs only a [Context], so the background
 * sync's engine (SyncWorker) answers it as the app does; picking the
 * tree is the Activity's ([MainActivity]). The files are read and
 * written off the main thread, one call after the other.
 */
object FolderChannel {
    const val CHANNEL = "io.github.paxel.oldbrainfix/folder"
    private const val ROOT = "oldbrainfix-sync"

    private val io by lazy { Executors.newSingleThreadExecutor() }
    private val main by lazy { Handler(Looper.getMainLooper()) }

    /** Answers [call] and returns true, or false when it is not one of these. */
    fun handle(context: Context, call: MethodCall, result: MethodChannel.Result): Boolean {
        if (call.method !in setOf("name", "list", "read", "write", "put", "delete", "ensure")) return false
        val app = context.applicationContext
        io.execute {
            val answer = try {
                Result.success(answer(app, call))
            } catch (e: Exception) {
                Result.failure(e)
            }
            main.post {
                answer.fold(
                    { result.success(it) },
                    { result.error("folder", it.message ?: it.toString(), null) },
                )
            }
        }
        return true
    }

    private fun answer(context: Context, call: MethodCall): Any? = when (call.method) {
        "name" -> tree(context, call)?.name
        "list" -> dir(context, call, create = false)?.listFiles()
            ?.filter { it.isFile }?.mapNotNull { it.name } ?: emptyList<String>()
        "read" -> dir(context, call, create = false)?.findFile(name(call))
            ?.takeIf { it.isFile }
            ?.let { file -> context.contentResolver.openInputStream(file.uri)?.use { it.readBytes() } }
        "write" -> {
            val dir = dir(context, call, create = true) ?: throw IllegalStateException("no folder")
            val name = name(call)
            // octet-stream keeps the name as given; a known type would make
            // the provider force its extension on it.
            val file = dir.findFile(name)
                ?: dir.createFile("application/octet-stream", name)
                ?: throw IllegalStateException("cannot create $name")
            val bytes = call.argument<ByteArray>("bytes") ?: ByteArray(0)
            // "wt": truncate, so a shorter file leaves no old tail.
            val out = context.contentResolver.openOutputStream(file.uri, "wt")
                ?: throw IllegalStateException("cannot write $name")
            out.use { it.write(bytes) }
            null
        }
        "put" -> {
            put(context, call)
            null
        }
        "delete" -> {
            dir(context, call, create = false)?.findFile(name(call))?.delete()
            null
        }
        else -> {
            dir(context, call, create = true)
            null
        }
    }

    /**
     * Copies the app's own file at `path` into the folder as `name`, a
     * Backup too big to cross the channel: streamed in beside, then
     * renamed, so a cloud client never takes half a file for the whole;
     * where the provider cannot rename, written in place.
     */
    private fun put(context: Context, call: MethodCall) {
        val dir = dir(context, call, create = true) ?: throw IllegalStateException("no folder")
        val name = name(call)
        val from = java.io.File(call.argument<String>("path") ?: throw IllegalArgumentException("no path"))
        val tmp = "$name.tmp"
        dir.findFile(tmp)?.delete()
        val beside = dir.createFile("application/octet-stream", tmp)
            ?: throw IllegalStateException("cannot create $tmp")
        copy(context, from, beside)
        dir.findFile(name)?.delete()
        if (!beside.renameTo(name)) {
            beside.delete()
            val file = dir.createFile("application/octet-stream", name)
                ?: throw IllegalStateException("cannot create $name")
            copy(context, from, file)
        }
    }

    private fun copy(context: Context, from: java.io.File, to: DocumentFile) {
        val out = context.contentResolver.openOutputStream(to.uri, "wt")
            ?: throw IllegalStateException("cannot write ${to.name}")
        out.use { sink -> from.inputStream().use { it.copyTo(sink) } }
    }

    private fun tree(context: Context, call: MethodCall): DocumentFile? {
        val tree = call.argument<String>("tree") ?: return null
        return DocumentFile.fromTreeUri(context, Uri.parse(tree))
    }

    private fun name(call: MethodCall): String {
        val name = call.argument<String>("name") ?: throw IllegalArgumentException("no name")
        if (!safeName(name)) throw IllegalArgumentException("not a folder file: $name")
        return name
    }

    /** `oldbrainfix-sync` (or the `root` the call names), or a folder in it; made on the way when [create]. */
    private fun dir(context: Context, call: MethodCall, create: Boolean): DocumentFile? {
        var dir = tree(context, call) ?: return null
        val path = segments(call.argument<String>("root") ?: ROOT, call.argument<String>("dir") ?: "")
        for (segment in path) {
            val next = dir.findFile(segment)
            dir = when {
                next != null && next.isDirectory -> next
                create -> dir.createDirectory(segment)
                    ?: throw IllegalStateException("cannot create $segment")
                else -> return null
            }
        }
        return dir
    }

    /** The folders from the tree down to [dir] inside [root]. */
    fun segments(root: String, dir: String): List<String> =
        (listOf(root) + dir.split('/')).filter { it.isNotEmpty() && safeName(it) }

    /** A name that stays in its folder. */
    fun safeName(name: String): Boolean =
        name.isNotEmpty() && name != "." && name != ".." && '/' !in name && '\\' !in name
}
