package io.github.paxel.oldbrainfix

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.w3c.dom.Element

/**
 * The Agenda widget's rows inflate on the home screen (#181): its
 * layouts hold only the view classes RemoteViews allows on every
 * Android the app runs on, and the list's view types cover every layout
 * a row is drawn with. A layout off this list shows as "Couldn't add
 * widget." instead of its row.
 */
class AgendaWidgetLayoutTest {
    /** The view classes RemoteViews inflates since API 21; CheckBox and the like only from 31. */
    private val allowed = setOf(
        "FrameLayout", "LinearLayout", "RelativeLayout", "GridLayout",
        "AnalogClock", "Button", "Chronometer", "ImageButton", "ImageView", "ProgressBar", "TextView",
        "ViewFlipper", "ListView", "GridView", "StackView", "AdapterViewFlipper", "ViewStub",
    )

    private val kinds = listOf(
        WidgetRow.OCCURRENCE, WidgetRow.LINE, WidgetRow.SHOPPING, WidgetRow.CHECKLIST, WidgetRow.FROG,
        WidgetRow.MISSED, WidgetRow.HEADING, WidgetRow.TEXT, WidgetRow.UNSORTED, WidgetRow.APPOINTMENT,
        WidgetRow.MESSAGE,
    )

    private fun widgetLayouts(): List<File> =
        File("src/main/res/layout").listFiles { file -> file.name.startsWith("widget_") }!!.sortedBy { it.name }

    private fun views(file: File): List<String> {
        val document = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file)
        val nodes = document.getElementsByTagName("*")
        return (0 until nodes.length).map { (nodes.item(it) as Element).tagName }
    }

    @Test
    fun theWidgetLayoutsHoldOnlyViewsRemoteViewsInflates() {
        val layouts = widgetLayouts()
        assertTrue(layouts.map { it.name }.containsAll(listOf("widget_agenda_row.xml", "widget_agenda_text.xml", "widget_agenda_unsorted.xml")))
        for (layout in layouts) {
            val refused = views(layout).filterNot { it in allowed }
            assertEquals("${layout.name} holds views a widget cannot show", emptyList<String>(), refused)
        }
    }

    @Test
    fun theListsViewTypesCoverEveryLayoutARowIsDrawnWith() {
        val layouts = AgendaWidget.ROW_LAYOUTS
        assertEquals(layouts.size, layouts.toSet().size)
        for (kind in kinds) assertTrue(kind, AgendaWidget.layoutOf(kind) in layouts)
        assertEquals(layouts.toSet(), kinds.map { AgendaWidget.layoutOf(it) }.toSet())
    }
}
