package io.github.paxel.oldbrainfix

import org.junit.Assert.assertEquals
import org.junit.Test

class ClipboardsTest {
    @Test
    fun `a paste takes every item's text, one per line, blank ones left out`() {
        val (text, pictures) = Clipboards.taken(
            listOf(
                Clipboards.Item<String>("call the vet", null, null),
                Clipboards.Item<String>("  ", null, null),
                Clipboards.Item<String>(null, null, null),
                Clipboards.Item<String>("at nine", null, null),
            ),
        )
        assertEquals("call the vet\nat nine", text)
        assertEquals(emptyList<String>(), pictures)
    }

    @Test
    fun `a paste takes pictures and unknown files, not other files`() {
        val (text, pictures) = Clipboards.taken(
            listOf(
                Clipboards.Item("the old flat", "content://a", "image/png"),
                Clipboards.Item(null, "content://b", null),
                Clipboards.Item(null, "content://c", "text/x-vcard"),
                Clipboards.Item(null, "content://d", "image/jpeg"),
            ),
        )
        assertEquals("the old flat", text)
        assertEquals(listOf("content://a", "content://b", "content://d"), pictures)
    }

    @Test
    fun `an empty clipboard gives nothing`() {
        val (text, pictures) = Clipboards.taken(emptyList<Clipboards.Item<String>>())
        assertEquals("", text)
        assertEquals(emptyList<String>(), pictures)
    }
}
