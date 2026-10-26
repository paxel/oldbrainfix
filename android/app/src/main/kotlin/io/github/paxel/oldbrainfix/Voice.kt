package io.github.paxel.oldbrainfix

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.MediaPlayer
import android.media.MediaRecorder
import android.os.Build
import java.io.File

/**
 * Voice Notes with Android's own MediaRecorder and MediaPlayer: no
 * Google library. Nothing here ever asks the user; asking is
 * [MainActivity]'s.
 *
 * A recording is AAC in an MPEG-4 (.m4a) file, mono, at 16 kHz and about
 * 64 kbit/s: half a megabyte a minute to keep and sync, and already the
 * sample rate and channel count Whisper listens at, so transcribing only
 * has to decode it, never resample it. The desktop records WAV at the
 * same rate instead; such a recording plays and transcribes here too.
 */
class Voice(private val context: Context) {
    private var recorder: MediaRecorder? = null
    private var player: MediaPlayer? = null

    /** How fast recordings play: 1 is normal speed. */
    private var speed = 1f

    /** Where the recording running is written until it is handed over. */
    private val recordingFile: File
        get() = File(context.cacheDir, "voice").apply { mkdirs() }.let { File(it, "recording.m4a") }

    /** Where the recording [bytes] being played are written for the player. */
    private fun playingFile(bytes: ByteArray): File =
        File(context.cacheDir, "voice").apply { mkdirs() }.let { File(it, "playing.${extension(bytes)}") }

    /** Starts recording from the microphone; whether it did. */
    fun start(): Boolean {
        stopPlaying()
        discard()
        val file = recordingFile.apply { delete() }
        val recorder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            MediaRecorder(context)
        } else {
            @Suppress("DEPRECATION")
            MediaRecorder()
        }
        // Never setLocation: a kept recording says nothing about where it
        // was made; the Note's own Capture Context does that.
        return try {
            recorder.setAudioSource(MediaRecorder.AudioSource.MIC)
            recorder.setOutputFormat(MediaRecorder.OutputFormat.MPEG_4)
            recorder.setAudioEncoder(MediaRecorder.AudioEncoder.AAC)
            recorder.setAudioChannels(CHANNELS)
            recorder.setAudioSamplingRate(SAMPLE_RATE)
            recorder.setAudioEncodingBitRate(BIT_RATE)
            recorder.setOutputFile(file.path)
            recorder.prepare()
            recorder.start()
            this.recorder = recorder
            true
        } catch (e: Exception) {
            // No permission, or the microphone is busy elsewhere.
            recorder.release()
            file.delete()
            false
        }
    }

    /** A recording ended: its [file], and whether it ended [clean]ly. */
    class Stopped(val file: File, val clean: Boolean)

    /**
     * Ends the recording; the file it was written to, or null when none
     * ran or nothing was written. Stopping right after the start throws:
     * then it did not end [Stopped.clean]ly, and only a [complete] file is
     * worth keeping. The caller reads the file and deletes it.
     */
    fun stop(): Stopped? {
        val recorder = recorder ?: return null
        this.recorder = null
        val clean = runCatching { recorder.stop() }.isSuccess
        recorder.release()
        val file = recordingFile
        if (file.length() > 0L) return Stopped(file, clean)
        file.delete()
        return null
    }

    /** Ends a recording without keeping it: the app is gone anyway. */
    private fun discard() {
        stop()?.file?.delete()
    }

    /**
     * Plays the recording [bytes], stopping whatever played before;
     * whether it plays. [ended] runs on the main thread when it played
     * to its end.
     */
    fun play(bytes: ByteArray, ended: () -> Unit): Boolean {
        stopPlaying()
        val player = MediaPlayer()
        return try {
            val file = playingFile(bytes).apply { writeBytes(bytes) }
            player.setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
                    .build(),
            )
            player.setDataSource(file.path)
            player.setOnCompletionListener {
                if (this.player === it) stopPlaying()
                ended()
            }
            player.prepare()
            player.start()
            // After the start: on a player not started yet, a speed
            // starts it.
            applySpeed(player)
            this.player = player
            true
        } catch (e: Exception) {
            player.release()
            false
        }
    }

    /**
     * Plays recordings at [speed], 1 being normal speed: the one playing
     * at once, and every one after it.
     */
    fun setSpeed(speed: Float) {
        this.speed = speed
        player?.let(::applySpeed)
    }

    /** Lets [player] play at [speed]; a player that cannot keeps its own. */
    private fun applySpeed(player: MediaPlayer) {
        runCatching { player.playbackParams = player.playbackParams.setSpeed(speed) }
    }

    /** Stops playing, if anything plays. */
    fun stopPlaying() {
        player?.release()
        player = null
    }

    /** Lets go of the microphone and the player. */
    fun release() {
        discard()
        stopPlaying()
    }

    companion object {
        const val permission = Manifest.permission.RECORD_AUDIO

        /** Whisper's own rate: transcribing never resamples. */
        const val SAMPLE_RATE = 16_000
        const val CHANNELS = 1
        const val BIT_RATE = 64_000

        /**
         * The file extension of the recording [bytes]: `wav` for a WAV
         * file (`RIFF`…`WAVE`), as the desktop records, else `m4a`, as
         * this phone records. Players and extractors read the bytes, the
         * name only keeps them from guessing wrong.
         */
        fun extension(bytes: ByteArray): String {
            fun tag(at: Int) = if (bytes.size < at + 4) "" else String(bytes, at, 4, Charsets.US_ASCII)
            return if (tag(0) == "RIFF" && tag(8) == "WAVE") "wav" else "m4a"
        }

        /** Whether the user allowed the microphone. */
        fun granted(context: Context): Boolean =
            context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

        /**
         * Whether [bytes] are a whole MPEG-4 file: boxes that run exactly
         * to the end, starting with `ftyp` and holding the `moov` index a
         * player needs. A recording whose stop failed is kept only when
         * it is whole anyway; without its index it cannot be played.
         */
        fun complete(bytes: ByteArray): Boolean {
            var at = 0L
            var first = true
            var index = false
            val end = bytes.size.toLong()
            while (at < end) {
                if (end - at < 8) return false
                var size = uint32(bytes, at.toInt())
                val type = String(bytes, at.toInt() + 4, 4, Charsets.US_ASCII)
                var header = 8L
                if (size == 1L) {
                    if (end - at < 16) return false
                    size = (uint32(bytes, at.toInt() + 8) shl 32) or uint32(bytes, at.toInt() + 12)
                    header = 16L
                } else if (size == 0L) {
                    size = end - at // the last box runs to the end
                }
                if (size < header || size > end - at) return false
                if (first && type != "ftyp") return false
                first = false
                if (type == "moov") index = true
                at += size
            }
            return index
        }

        private fun uint32(bytes: ByteArray, at: Int): Long =
            ((bytes[at].toLong() and 0xFF) shl 24) or
                ((bytes[at + 1].toLong() and 0xFF) shl 16) or
                ((bytes[at + 2].toLong() and 0xFF) shl 8) or
                (bytes[at + 3].toLong() and 0xFF)
    }
}
