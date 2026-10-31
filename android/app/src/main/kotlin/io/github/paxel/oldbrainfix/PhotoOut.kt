package io.github.paxel.oldbrainfix

import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentValues
import android.content.Context
import android.os.Build
import android.provider.MediaStore
import androidx.annotation.ChecksSdkIntAtLeast
import androidx.core.content.FileProvider
import java.io.File

/**
 * A photo of a Note handed out of it from the full-screen viewer (#149):
 * saved into the phone's Pictures, or put on the clipboard as a picture.
 * Both take the file the app wrote into the exports the FileProvider
 * serves, and nothing else; neither needs a permission.
 */
object PhotoOut {
    /** Where a saved photo goes, as the gallery shows it: an album of its own. */
    const val FOLDER = "Pictures/Old Brain Fix"

    /** What [save] answers: saved, this Android too old, or not written. */
    const val SAVED = "saved"
    const val OLD = "old"
    const val FAILED = "failed"

    /**
     * Whether Android [sdk] saves into Pictures without a permission:
     * from Android 10 an app adds its own files to MediaStore freely;
     * before it, that needs the storage permission the app never asks for.
     */
    fun savesWithoutPermission(sdk: Int): Boolean = sdk >= Build.VERSION_CODES.Q

    /** [savesWithoutPermission] on this phone. */
    @ChecksSdkIntAtLeast(api = Build.VERSION_CODES.Q)
    fun savesHere(): Boolean = savesWithoutPermission(Build.VERSION.SDK_INT)

    /** The name a photo is saved under: [name] without any directory part. */
    fun displayName(name: String): String = FileShare.safeName(name)

    /**
     * Saves the photo at [path], of MIME [type], into [FOLDER] as [name];
     * [SAVED], [OLD] before Android 10, or [FAILED]. A photo cut off on
     * the way is removed again, never left half in the gallery.
     */
    fun save(context: Context, path: String, name: String, type: String): String {
        if (!savesHere()) return OLD
        val file = exported(context, path) ?: return FAILED
        val resolver = context.contentResolver
        val values = ContentValues().apply {
            put(MediaStore.MediaColumns.DISPLAY_NAME, displayName(name))
            put(MediaStore.MediaColumns.MIME_TYPE, type)
            put(MediaStore.MediaColumns.RELATIVE_PATH, FOLDER)
            put(MediaStore.MediaColumns.IS_PENDING, 1)
        }
        val uri = runCatching {
            resolver.insert(MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY), values)
        }.getOrNull() ?: return FAILED
        return try {
            resolver.openOutputStream(uri)?.use { out -> file.inputStream().use { it.copyTo(out) } }
                ?: throw IllegalStateException("no stream to write to")
            resolver.update(uri, ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) }, null, null)
            SAVED
        } catch (_: Exception) {
            runCatching { resolver.delete(uri, null, null) }
            FAILED
        }
    }

    /**
     * Puts the photo at [path], of MIME [type], on the clipboard as a
     * picture, through the FileProvider: the app that pastes it is let
     * read it. Whether it is there.
     */
    fun copy(context: Context, path: String, type: String): Boolean {
        val file = exported(context, path) ?: return false
        return runCatching {
            val uri = FileProvider.getUriForFile(context, "${context.packageName}.photos", file)
            val clip = ClipData("", arrayOf(type), ClipData.Item(uri))
            val clipboard = context.getSystemService(ClipboardManager::class.java) ?: return false
            clipboard.setPrimaryClip(clip)
            true
        }.getOrDefault(false)
    }

    /** The file at [path] when it lies in the exports and is there; null otherwise. */
    private fun exported(context: Context, path: String): File? {
        if (!Bundles.inside(FileShare.dir(context), path)) return null
        return File(path).takeIf { it.isFile }
    }
}
