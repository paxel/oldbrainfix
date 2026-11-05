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
import java.io.File
import java.io.InputStream
import java.io.OutputStream
import java.util.concurrent.Executors
import java.util.zip.ZipInputStream

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
 * the folder's lasting grant afterwards ([releases]). The import from
 * Google Keep takes a Takeout zip as well: it lists the zip's files
 * ([zipNames]), then copies those the core reads into the app's cache,
 * each at most its limit and one byte more ([unzip]), read from there as
 * any other file and deleted afterwards ("dropUnzipped").
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
        if (call.method !in setOf("name", "list", "read", "write", "put", "delete", "ensure", "autoBackupDir", "autoBackup", "files", "readDoc", "release", "zipNames", "unzip", "dropUnzipped", "fileApp", "openFile")) {
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
        // A file link (#261): the app that opens the file, "" when the
        // system asks which, or null when the tree holds no such file.
        "fileApp" -> fileIn(context, call)?.let { appFor(context, viewOf(it)) }
        "openFile" -> fileIn(context, call)?.let {
            try {
                context.startActivity(viewOf(it))
                true
            } catch (_: android.content.ActivityNotFoundException) {
                false
            }
        } ?: false
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
        "zipNames" -> zipStream(context, call).use { zipNames(it) }
        "unzip" -> {
            val wanted = call.argument<Map<String, Int>>("wanted") ?: throw IllegalArgumentException("no wanted")
            val into = unzipped(context)
            into.deleteRecursively()
            if (!into.mkdirs()) throw IllegalStateException("cannot make $into")
            zipStream(context, call).use { unzip(it, wanted, into) }.map { (path, file, size) ->
                mapOf("path" to path, "doc" to Uri.fromFile(file).toString(), "size" to size)
            }
        }
        "dropUnzipped" -> {
            unzipped(context).deleteRecursively()
            null
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

    /**
     * The file at the path inside the tree, its names separated by `/`,
     * found name by name below the tree, so it never lies outside it; null
     * when there is none. The cores refuse `..`, `.` and empty names.
     */
    private fun fileIn(context: Context, call: MethodCall): DocumentFile? {
        val tree = call.argument<String>("tree") ?: return null
        val path = call.argument<String>("path") ?: return null
        var at = DocumentFile.fromTreeUri(context, Uri.parse(tree)) ?: return null
        for (name in path.split('/')) {
            if (name.isEmpty() || name == "." || name == "..") return null
            at = at.findFile(name) ?: return null
        }
        return at.takeIf { it.isFile }
    }

    /** Viewing [file] in the app the system has for its type, allowed to read it. */
    private fun viewOf(file: DocumentFile): Intent =
        Intent(Intent.ACTION_VIEW)
            .setDataAndType(file.uri, file.type ?: "*/*")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)

    /** The name of the app [view] opens in, "" when the system asks which or tells none. */
    private fun appFor(context: Context, view: Intent): String {
        val pm = context.packageManager
        val found = pm.resolveActivity(view, android.content.pm.PackageManager.MATCH_DEFAULT_ONLY)
            ?: return ""
        // The system's chooser, as when no app is the default.
        if (found.activityInfo.packageName == "android") return ""
        return found.loadLabel(pm).toString()
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

    /** The zip document the call names, opened; one that cannot be opened fails the Import. */
    private fun zipStream(context: Context, call: MethodCall): InputStream {
        val doc = Uri.parse(call.argument<String>("zip") ?: throw IllegalArgumentException("no zip"))
        return context.contentResolver.openInputStream(doc) ?: throw IllegalStateException("cannot read $doc")
    }

    /** Where [unzip] puts the files it copies out of a zip, in the app's cache. */
    private fun unzipped(context: Context): File = File(context.cacheDir, "import-zip")

    /** The paths of the files inside the zip [input], folders left out, in the zip's order. */
    fun zipNames(input: InputStream): List<String> {
        val names = mutableListOf<String>()
        ZipInputStream(input).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                if (!entry.isDirectory) names += entry.name
            }
        }
        return names
    }

    /**
     * Copies each file of the zip [input] whose path [wanted] names into
     * [into], as a file named by its number, so no path from inside the
     * zip is ever one on the phone; of each at most its limit and one
     * byte more, so a larger file is never copied whole. Answers each
     * copy's path inside the zip, its file and its size.
     */
    fun unzip(input: InputStream, wanted: Map<String, Int>, into: File): List<Triple<String, File, Long>> {
        val copied = mutableListOf<Triple<String, File, Long>>()
        ZipInputStream(input).use { zip ->
            while (true) {
                val entry = zip.nextEntry ?: break
                val limit = wanted[entry.name] ?: continue
                if (entry.isDirectory) continue
                val file = File(into, copied.size.toString())
                val size = file.outputStream().use { copyAtMost(zip, it, limit + 1L) }
                copied += Triple(entry.name, file, size)
            }
        }
        return copied
    }

    /** Copies [input] into [out] until it ends or [most] bytes are copied; answers how many were. */
    fun copyAtMost(input: InputStream, out: OutputStream, most: Long): Long {
        val buffer = ByteArray(8192)
        var copied = 0L
        while (copied < most) {
            val read = input.read(buffer, 0, minOf(buffer.size.toLong(), most - copied).toInt())
            if (read < 0) break
            out.write(buffer, 0, read)
            copied += read
        }
        return copied
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
