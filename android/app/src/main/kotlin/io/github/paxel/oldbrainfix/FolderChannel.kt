package io.github.paxel.oldbrainfix

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.DocumentsContract
import androidx.documentfile.provider.DocumentFile
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.util.concurrent.Executors

/**
 * The shared sync folder through the document tree the folder picker
 * granted (Storage Access Framework), adapted from Catlog: a persistable
 * tree URI, good for shared storage, a USB stick and cloud providers
 * alike, none of which is a path an app may read. Dart sees the folder
 * as `oldbrainfix-sync` with `keys` and `blobs` inside, addressed by
 * name; Backups go straight into the folder picked for them ([put]),
 * and the one Auto Backup may take lies in the app's own files
 * ([AutoBackup]). An Import lists every file of the picked folder and
 * its folders ([files]), reads them one by one ([readDoc]) and gives up
 * the folder's lasting grant afterwards ([releases]).
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
        if (call.method !in setOf("name", "list", "read", "write", "put", "delete", "ensure", "autoBackupDir", "autoBackup", "files", "readDoc", "release")) {
            return false
        }
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
        // The app's own directory whose Backup Auto Backup may take, and
        // the user's opt-in to it ([AutoBackup]).
        "autoBackupDir" -> AutoBackup.dir(context).apply { mkdirs() }.absolutePath
        "autoBackup" -> {
            AutoBackup.setOptedIn(context, call.argument<Boolean>("on") == true)
            null
        }
        "name" -> tree(context, call)?.name
        "files" -> files(context, call)
        "release" -> {
            val tree = call.argument<String>("tree") ?: throw IllegalArgumentException("no tree")
            if (releases(tree, call.argument<List<String>>("keep") ?: emptyList())) {
                try {
                    context.contentResolver.releasePersistableUriPermission(
                        Uri.parse(tree),
                        Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                    )
                } catch (_: SecurityException) {
                    // Nothing persisted for it: nothing to give up.
                }
            }
            null
        }
        "readDoc" -> {
            val doc = Uri.parse(call.argument<String>("doc") ?: throw IllegalArgumentException("no doc"))
            val limit = call.argument<Int>("limit") ?: throw IllegalArgumentException("no limit")
            // Null only for a file larger than the limit; one that cannot
            // be opened fails the Import.
            val input = context.contentResolver.openInputStream(doc)
                ?: throw IllegalStateException("cannot read $doc")
            input.use { readAtMost(it, limit) }
        }
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

    /**
     * Every file of the tree and of its folders but hidden ones, for an
     * Import: its path inside the tree, its document to read it by, when
     * it was last changed (0 when the provider cannot tell) and its size
     * (-1 when it cannot). One query per folder, not one per file.
     */
    private fun files(context: Context, call: MethodCall): List<Map<String, Any>> {
        val tree = Uri.parse(call.argument<String>("tree") ?: throw IllegalArgumentException("no tree"))
        val found = mutableListOf<Map<String, Any>>()
        val folders = ArrayDeque(listOf(DocumentsContract.getTreeDocumentId(tree) to ""))
        while (folders.isNotEmpty()) {
            val (id, path) = folders.removeFirst()
            val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, id)
            context.contentResolver.query(children, columns, null, null, null)?.use { row ->
                while (row.moveToNext()) {
                    val child = row.getString(0) ?: continue
                    val name = row.getString(1) ?: continue
                    val inside = path(path, name)
                    if (row.getString(2) == DocumentsContract.Document.MIME_TYPE_DIR) {
                        if (descends(name)) folders.addLast(child to inside)
                        continue
                    }
                    found += mapOf(
                        "path" to inside,
                        "doc" to DocumentsContract.buildDocumentUriUsingTree(tree, child).toString(),
                        "modified" to (if (row.isNull(3)) 0L else row.getLong(3)),
                        "size" to (if (row.isNull(4)) -1L else row.getLong(4)),
                    )
                }
            }
        }
        return found
    }

    private val columns = arrayOf(
        DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        DocumentsContract.Document.COLUMN_MIME_TYPE,
        DocumentsContract.Document.COLUMN_LAST_MODIFIED,
        DocumentsContract.Document.COLUMN_SIZE,
    )

    /**
     * Whether the lasting grant of the imported [tree] is given up: always,
     * unless it is a folder the app keeps, the sync or the Backup folder in
     * [keep], which the picker granted the same way.
     */
    fun releases(tree: String, keep: List<String>): Boolean = tree !in keep

    /** The path of [name] in the folder at [parent], "" for the tree itself. */
    fun path(parent: String, name: String): String = if (parent.isEmpty()) name else "$parent/$name"

    /** Whether an Import takes the files of the folder [name]: not of a hidden one, such as `.git` or `.obsidian`. */
    fun descends(name: String): Boolean = !name.startsWith(".")

    /** The bytes of [input], or null as soon as there are more than [limit]: a larger file is never read whole. */
    fun readAtMost(input: InputStream, limit: Int): ByteArray? {
        val out = ByteArrayOutputStream()
        val buffer = ByteArray(8192)
        while (true) {
            val read = input.read(buffer)
            if (read < 0) return out.toByteArray()
            if (out.size() + read > limit) return null
            out.write(buffer, 0, read)
        }
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
