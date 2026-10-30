package io.github.paxel.oldbrainfix

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class BundlesTest {
    @Test
    fun onlyBundlesInTheExportsAreShared() {
        val exports = Files.createTempDirectory("exports").toFile()
        try {
            assertTrue(Bundles.inside(exports, File(exports, "oldbrainfix-2026-10-24.obfsync").path))
            assertFalse(Bundles.inside(exports, File(exports, "../oldbrainfix.db").path))
            assertFalse(Bundles.inside(exports, File(File(exports, "sub"), "a.obfsync").path))
            assertFalse(Bundles.inside(exports, "/data/data/x/oldbrainfix.db"))
        } finally {
            exports.deleteRecursively()
        }
    }

    @Test
    fun aSharedNoteGoesAsTextOrAsTheTypeItsFilesHave() {
        assertEquals("text/plain", Bundles.typeOf(emptyList()))
        assertEquals("image/jpeg", Bundles.typeOf(listOf("image/jpeg", "image/jpeg")))
        assertEquals("audio/*", Bundles.typeOf(listOf("audio/mp4", "audio/ogg")))
        assertEquals("*/*", Bundles.typeOf(listOf("image/jpeg", "audio/mp4")))
    }
}
