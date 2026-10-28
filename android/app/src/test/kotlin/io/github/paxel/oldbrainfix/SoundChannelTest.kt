package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SoundChannelTest {
    private val own = "android.resource://io.github.paxel.oldbrainfix/raw/obf_alarm"
    private val bell = "content://media/internal/audio/media/42"

    @Test
    fun theFirstChannelReplacesTheOneOf011WithTheOwnSound() {
        val first = SoundChannel.next(null, own)
        assertEquals(SoundChannel.Made(1, own), first)
        assertEquals("occurrences-1", first?.id)
        assertEquals("occurrences", SoundChannel.replaced(null))
    }

    @Test
    fun theSameSoundKeepsItsChannel() {
        val made = SoundChannel.Made(3, bell)
        assertNull(SoundChannel.next(made, bell))
        assertEquals("occurrences-3", SoundChannel.replaced(made))
    }

    @Test
    fun anotherSoundGetsAChannelNeverUsedBefore() {
        val first = SoundChannel.Made(1, own)
        val second = SoundChannel.next(first, bell)!!
        assertEquals(SoundChannel.Made(2, bell), second)
        assertEquals("occurrences-1", SoundChannel.replaced(first))
        // Back to the own sound: not channel 1 again, which would come
        // back with its old settings.
        val third = SoundChannel.next(second, own)!!
        assertEquals("occurrences-3", third.id)
        assertEquals("occurrences-2", SoundChannel.replaced(second))
    }

    @Test
    fun thePickerLeftChoosesNothing() {
        assertNull(SoundChannel.picked(null, own))
    }

    @Test
    fun theDefaultEntryIsTheOwnSound() {
        assertEquals(mapOf("sound" to null), SoundChannel.picked(own, own))
    }

    @Test
    fun anotherSoundIsKeptByItsUri() {
        assertEquals(mapOf("sound" to bell), SoundChannel.picked(bell, own))
    }
}
