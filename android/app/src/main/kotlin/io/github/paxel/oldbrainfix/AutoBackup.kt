package io.github.paxel.oldbrainfix

import android.app.backup.BackupAgent
import android.app.backup.BackupDataInput
import android.app.backup.BackupDataOutput
import android.app.backup.FullBackupDataOutput
import android.content.Context
import android.os.Build
import android.os.ParcelFileDescriptor
import java.io.File

/**
 * What Android's Auto Backup may take to the user's Google account (ADR
 * 0010): nothing until the user opted in on the Backups page, then only
 * the encrypted Backup file the app keeps in [DIR], never the database,
 * the settings or anything else. The opt-in is a flag the agent reads
 * without Flutter; the app sets it, and it never travels itself.
 */
object AutoBackup {
    /** The directory under the app's files the Backup for Google lies in. */
    const val DIR = "google-backup"
    private const val PREFS = "autobackup"
    private const val OPTED_IN = "optedIn"
    private const val EXTENSION = ".obfbackup"

    fun dir(context: Context): File = File(context.filesDir, DIR)

    fun optedIn(context: Context): Boolean =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(OPTED_IN, false)

    fun setOptedIn(context: Context, on: Boolean) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putBoolean(OPTED_IN, on).apply()
    }

    /**
     * The files a backup takes: none unless [optedIn], else the Backup
     * files lying in [dir], each at most [quota] bytes, since Google takes
     * nothing of an app that asks for more.
     */
    fun files(optedIn: Boolean, dir: File, quota: Long): List<File> {
        if (!optedIn) return emptyList()
        return dir.listFiles()
            ?.filter { it.isFile && it.name.endsWith(EXTENSION) && it.length() <= quota }
            ?.sortedBy { it.name }
            ?: emptyList()
    }

    /** Whether a file coming back from a backup goes to [destination]: only a Backup file in [dir]. */
    fun restores(dir: File, destination: File?): Boolean =
        destination != null &&
            destination.name.endsWith(EXTENSION) &&
            destination.parentFile?.canonicalPath == dir.canonicalPath
}

/**
 * The app's backup agent: full backups only (`fullBackupOnly`), and of
 * those only what [AutoBackup.files] says. The framework's own full
 * backup, which would take every file, is never called.
 */
class AutoBackupAgent : BackupAgent() {
    // Key-value backup is not used: fullBackupOnly in the manifest.
    override fun onBackup(oldState: ParcelFileDescriptor?, data: BackupDataOutput?, newState: ParcelFileDescriptor?) = Unit

    override fun onRestore(data: BackupDataInput?, appVersionCode: Int, newState: ParcelFileDescriptor?) = Unit

    override fun onFullBackup(data: FullBackupDataOutput) {
        // The quota is told from Android 8 on; before, Google's 25 MB.
        val quota = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) data.quota else QUOTA
        for (file in AutoBackup.files(AutoBackup.optedIn(this), AutoBackup.dir(this), quota)) {
            fullBackupFile(file, data)
        }
    }

    private companion object {
        const val QUOTA = 25L * 1024 * 1024
    }

    override fun onRestoreFile(
        data: ParcelFileDescriptor,
        size: Long,
        destination: File?,
        type: Int,
        mode: Long,
        mtime: Long,
    ) {
        // A null destination passes the bytes over.
        val keep = if (AutoBackup.restores(AutoBackup.dir(this), destination)) destination else null
        super.onRestoreFile(data, size, keep, type, mode, mtime)
    }
}
