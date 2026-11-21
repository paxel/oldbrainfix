package io.github.paxel.oldbrainfix

/**
 * The voice channel's start (#350, #368): the recorder is asked to start,
 * and only a recording that runs keeps the screen on.
 */
object VoiceStart {
    /** Starts with [start]; when it runs, [running] follows. Its answer. */
    fun run(start: () -> Boolean, running: () -> Unit): Boolean {
        val started = start()
        if (started) running()
        return started
    }
}
