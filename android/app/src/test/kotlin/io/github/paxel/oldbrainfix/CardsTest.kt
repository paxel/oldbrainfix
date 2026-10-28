package io.github.paxel.oldbrainfix

import android.content.Intent
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CardsTest {
    private val card = "https://paxel.github.io/oldbrainfix/p#eyJmb3JtYXQiOjF9.c2ln"

    @Test
    fun aCardLinkOpenedWithTheAppIsKept() {
        assertEquals(card, Cards.linkOf(Intent.ACTION_VIEW, card))
        assertEquals(
            "HTTPS://Paxel.GitHub.io/oldbrainfix/p#x.y",
            Cards.linkOf(Intent.ACTION_VIEW, "HTTPS://Paxel.GitHub.io/oldbrainfix/p#x.y"),
        )
    }

    @Test
    fun everyOtherLinkIsLeftOut() {
        assertNull(Cards.linkOf(Intent.ACTION_SEND, card))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, null))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, "http://paxel.github.io/oldbrainfix/p#x.y"))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, "https://evil.example/oldbrainfix/p#x.y"))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, "https://paxel.github.io/oldbrainfix/privacy-policy.html"))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, "https://paxel.github.io/oldbrainfix/p/x#x.y"))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, "https://paxel.github.io/oldbrainfix/p#" + "a".repeat(Cards.MAX_LENGTH)))
        assertNull(Cards.linkOf(Intent.ACTION_VIEW, "https://paxel.github.io/oldbrainfix/p#a b"))
    }
}
