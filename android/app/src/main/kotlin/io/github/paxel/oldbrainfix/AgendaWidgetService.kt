package io.github.paxel.oldbrainfix

import android.content.Context
import android.content.Intent
import android.widget.RemoteViews
import android.widget.RemoteViewsService

/**
 * Fills the Agenda widget's list ([AgendaWidget]), which scrolls when its
 * rows are taller than the widget. Asked off the main thread, so it may
 * read the calendar.
 */
class AgendaWidgetService : RemoteViewsService() {
    override fun onGetViewFactory(intent: Intent): RemoteViewsFactory = Rows(applicationContext)

    private class Rows(private val context: Context) : RemoteViewsFactory {
        private var snapshot: AgendaSnapshot? = null
        private var rows: List<WidgetRow> = emptyList()

        override fun onCreate() {}

        override fun onDataSetChanged() {
            val (snapshot, rows) = AgendaWidget.rows(context)
            this.snapshot = snapshot
            this.rows = rows
        }

        override fun onDestroy() {
            rows = emptyList()
        }

        override fun getCount(): Int = rows.size

        override fun getViewAt(position: Int): RemoteViews =
            rows.getOrNull(position)?.let { AgendaWidget.rowViews(context, snapshot, it) }
                ?: RemoteViews(context.packageName, R.layout.widget_agenda_text)

        override fun getLoadingView(): RemoteViews? = null

        /** A row with its box, a line of text, and the Unsorted count. */
        override fun getViewTypeCount(): Int = 3

        override fun getItemId(position: Int): Long = position.toLong()

        override fun hasStableIds(): Boolean = false
    }
}
