package io.github.paxel.oldbrainfix

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.w3c.dom.Element

/**
 * The manifest's "open with" filters: a Profile card file (#144) comes
 * in as a bundle or a Sent Item does, and no card link opens the app.
 */
class ManifestTest {
    private val android = "http://schemas.android.com/apk/res/android"

    private val manifest = DocumentBuilderFactory.newInstance()
        .apply { isNamespaceAware = true }
        .newDocumentBuilder()
        .parse(File("src/main/AndroidManifest.xml"))

    private fun elements(parent: Element, name: String): List<Element> {
        val nodes = parent.getElementsByTagName(name)
        return (0 until nodes.length).map { nodes.item(it) as Element }
    }

    private fun activity(name: String): Element =
        elements(manifest.documentElement, "activity").single { it.getAttributeNS(android, "name") == name }

    private fun dataOf(parent: Element, attribute: String): List<String> =
        elements(parent, "data").map { it.getAttributeNS(android, attribute) }.filter { it.isNotEmpty() }

    @Test
    fun aCardFileOpensWithTheAppAsBundlesAndSentItemsDo() {
        val types = dataOf(activity(".BundleActivity"), "mimeType")
        assertTrue(types.contains("application/vnd.oldbrainfix.card"))
        assertTrue(types.contains("application/vnd.oldbrainfix.item"))
        assertTrue(types.contains("application/vnd.oldbrainfix.sync"))
        // Messengers often hand a file over without its own type.
        assertTrue(types.contains("application/octet-stream"))
    }

    @Test
    fun noCardLinkOpensTheApp() {
        val paths = dataOf(manifest.documentElement, "path") +
            dataOf(manifest.documentElement, "pathPrefix") +
            dataOf(manifest.documentElement, "pathPattern")
        assertFalse(paths.any { it.startsWith("/oldbrainfix/p") })
        assertFalse(dataOf(manifest.documentElement, "host").contains("paxel.github.io"))
        assertTrue(elements(manifest.documentElement, "activity").none { it.getAttributeNS(android, "name") == ".CardActivity" })
    }
}
