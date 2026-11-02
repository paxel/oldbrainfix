package io.github.paxel.oldbrainfix

import android.app.AlarmManager
import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.graphics.Typeface
import android.net.Uri
import android.os.Binder
import android.os.Build
import android.text.SpannableString
import android.text.Spanned
import android.text.style.StrikethroughSpan
import android.text.style.StyleSpan
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import java.util.TimeZone

/**
 * The home-screen Agenda widget: what the Agenda shows for today, in its
 * order (#153), then the coming days the Agenda shows, each under its
 * heading (#194), as a list that scrolls when it is taller than the
 * widget, its groups drawn as the Agenda's (#209): a heading, then its
 * entries indented under it, each with its box. A row's box of today or
 * the Missed Summary checks it as the Agenda's does, a coming day's
 * opens the app, a Session's only shows Done, a pick opens its
 * entry form, the rest of a row opens the app, and the Unsorted count
 * opens the Unsorted list; every tap on the list goes through
 * [AgendaTapActivity]. It
 * never reads the log: it shows the snapshot the app hands over whenever
 * it syncs the alarms ([replace]), and a box checked here goes the way of
 * a Done tapped on a notification, into the [EventQueue] for the app to
 * apply. The calendar's appointments it reads itself as it shows, as
 * nothing of them is kept. The snapshot holds the coming days, so the
 * widget moves on at local midnight without the app.
 */
class AgendaWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) {
        render(context, manager, ids)
        manager.notifyAppWidgetViewDataChanged(ids, R.id.agenda_list)
    }

    override fun onDisabled(context: Context) {
        context.getSystemService(AlarmManager::class.java).cancel(midnight(context))
    }

    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            ACTION_MIDNIGHT -> update(context)
            else -> super.onReceive(context, intent)
        }
    }

    companion object {
        private const val PREFS = "oldbrainfix_widget"
        private const val KEY_SNAPSHOT = "agenda"
        private const val ACTION_MIDNIGHT = "io.github.paxel.oldbrainfix.WIDGET_MIDNIGHT"

        /** How opaque a coming day's box is, of 255 (#194). */
        private const val AHEAD_ALPHA = 140

        /** Request codes of the widget's taps; apart from Capture's 0 to 2. */
        private const val REQUEST_OPEN = 100
        private const val REQUEST_ROWS = 102

        /** Keeps the app's new [snapshot] and shows it. */
        fun replace(context: Context, snapshot: JSONObject) {
            prefs(context).edit().putString(KEY_SNAPSHOT, snapshot.toString()).commit()
            update(context)
        }

        /**
         * Shows the snapshot again, as of now and with the boxes checked
         * meanwhile; nothing when no Agenda widget is on the home screen.
         */
        fun update(context: Context) {
            val manager = AppWidgetManager.getInstance(context) ?: return
            val ids = manager.getAppWidgetIds(ComponentName(context, AgendaWidget::class.java))
            if (ids.isEmpty()) return
            render(context, manager, ids)
            manager.notifyAppWidgetViewDataChanged(ids, R.id.agenda_list)
        }

        private fun render(context: Context, manager: AppWidgetManager, ids: IntArray) {
            for (id in ids) manager.updateAppWidget(id, views(context, id))
            // Inexact is enough: the day moves on soon after midnight.
            val now = System.currentTimeMillis()
            context.getSystemService(AlarmManager::class.java)
                .set(AlarmManager.RTC, AgendaWidgetPlan.nextMidnight(now, TimeZone.getDefault()), midnight(context))
        }

        /** The widget [id]: the list of rows, filled by [AgendaWidgetService]. */
        @Suppress("DEPRECATION")
        private fun views(context: Context, id: Int): RemoteViews {
            val views = RemoteViews(context.packageName, R.layout.widget_agenda)
            val open = PendingIntent.getActivity(
                context, REQUEST_OPEN, openIntent(context),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            views.setOnClickPendingIntent(R.id.agenda, open)
            views.setOnClickPendingIntent(R.id.agenda_message, open)
            val adapter = Intent(context, AgendaWidgetService::class.java)
                .putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, id)
            // One adapter per widget; extras alone do not tell them apart.
            adapter.data = Uri.parse(adapter.toUri(Intent.URI_INTENT_SCHEME))
            views.setRemoteAdapter(R.id.agenda_list, adapter)
            views.setEmptyView(R.id.agenda_list, R.id.agenda_message)
            // The rows fill in what each tap does. An activity, started by
            // the tap itself, may open the app where a receiver may not.
            val mutable = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
            views.setPendingIntentTemplate(
                R.id.agenda_list,
                PendingIntent.getActivity(
                    context, REQUEST_ROWS,
                    Intent(context, AgendaTapActivity::class.java)
                        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_NO_ANIMATION),
                    mutable or PendingIntent.FLAG_UPDATE_CURRENT,
                ),
            )
            return views
        }

        /** The rows the widget shows now, with the calendar's appointments of each day it shows. */
        fun rows(context: Context): Pair<AgendaSnapshot?, List<WidgetRow>> {
            val now = System.currentTimeMillis()
            val zone = TimeZone.getDefault()
            val snapshot = snapshot(context)
            val today = AgendaWidgetPlan.dayKey(now, zone)
            val appointments = if (snapshot == null) emptyMap() else appointments(context, snapshot, today, now, zone)
            val view = AgendaWidgetPlan.view(
                snapshot, today, EventQueue.pending(context), appointments,
                context.getString(R.string.agenda_widget_open_app),
            )
            return snapshot to view.rows
        }

        /**
         * The appointments of the calendars the Agenda shows on each day
         * the widget shows ([AgendaWidgetPlan.shownDays]), read now and
         * kept nowhere; none while that is off or not allowed.
         */
        private fun appointments(
            context: Context,
            snapshot: AgendaSnapshot,
            today: String,
            now: Long,
            zone: TimeZone,
        ): Map<String, List<WidgetRow>> {
            val days = AgendaWidgetPlan.shownDays(snapshot, today)
            if (snapshot.calendars.isEmpty() || days.isEmpty()) return emptyMap()
            // Read as this app, not as the launcher asking for the rows.
            val identity = Binder.clearCallingIdentity()
            try {
                val start = AgendaWidgetPlan.midnightOf(now, zone)
                val read = runCatching {
                    val colours = Appointments.calendars(context).associate { it["id"] as String to it["colour"] as Int }
                    // A day on either side, for the clock changes between.
                    Appointments.between(context, start - 86_400_000L, start + (days.size + 1) * 86_400_000L, snapshot.calendars)
                        ?.map {
                            CalendarEntry(
                                title = it["title"] as String,
                                begin = it["begin"] as Long,
                                end = it["end"] as Long,
                                allDay = it["allDay"] as Boolean,
                                colour = colours[it["calendar"] as String] ?: 0,
                            )
                        }
                }.getOrNull() ?: return emptyMap()
                return days.associateWith { AgendaWidgetPlan.appointmentRows(snapshot, read, it, zone) }
            } finally {
                Binder.restoreCallingIdentity(identity)
            }
        }

        /**
         * Every layout a row of the list is drawn with; the list's view
         * types ([AgendaWidgetService]) count them.
         */
        val ROW_LAYOUTS = listOf(R.layout.widget_agenda_row, R.layout.widget_agenda_text, R.layout.widget_agenda_unsorted)

        /** The layout a row of [kind] is drawn with, one of [ROW_LAYOUTS]. */
        fun layoutOf(kind: String): Int = when (kind) {
            WidgetRow.UNSORTED -> R.layout.widget_agenda_unsorted
            WidgetRow.HEADING, WidgetRow.TEXT, WidgetRow.MESSAGE -> R.layout.widget_agenda_text
            else -> R.layout.widget_agenda_row
        }

        /**
         * The views of [row]: its box does [WidgetRow.onBox], the rest
         * [WidgetRow.onRow]; the Unsorted count opens the Unsorted list.
         */
        fun rowViews(context: Context, snapshot: AgendaSnapshot?, row: WidgetRow): RemoteViews = when (layoutOf(row.kind)) {
            R.layout.widget_agenda_unsorted -> RemoteViews(context.packageName, R.layout.widget_agenda_unsorted).apply {
                setTextViewText(R.id.agenda_unsorted, row.title)
                setOnClickFillInIntent(R.id.agenda_unsorted, fillIn(AgendaTap.Unsorted))
            }
            R.layout.widget_agenda_text ->
                RemoteViews(context.packageName, R.layout.widget_agenda_text).apply {
                    val text = SpannableString(row.title)
                    if (row.kind == WidgetRow.HEADING) {
                        text.setSpan(StyleSpan(Typeface.BOLD), 0, text.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                    }
                    if (row.done) strike(text)
                    setTextViewText(R.id.agenda_text, text)
                    setTextColor(
                        R.id.agenda_text,
                        context.getColor(if (row.kind == WidgetRow.HEADING) R.color.agenda_widget_text else R.color.agenda_widget_text_dim),
                    )
                    // Under a heading its words start where the heading's do.
                    val start = maxOf(8, AgendaWidgetPlan.indentDp(row.indent))
                    setViewPadding(R.id.agenda_text, dp(context, start), dp(context, 4), dp(context, 8), dp(context, 4))
                    setOnClickFillInIntent(R.id.agenda_text, fillIn(AgendaTap.Open))
                }
            else -> RemoteViews(context.packageName, R.layout.widget_agenda_row).apply {
                setViewPadding(R.id.agenda_row, dp(context, AgendaWidgetPlan.indentDp(row.indent)), 0, 0, dp(context, 4))
                setTextViewText(R.id.agenda_time, row.time)
                setViewVisibility(R.id.agenda_time, if (row.time.isEmpty()) View.GONE else View.VISIBLE)
                setOnClickFillInIntent(R.id.agenda_row, fillIn(row.onRow))
                if (row.kind == WidgetRow.APPOINTMENT) {
                    setViewVisibility(R.id.agenda_strip, View.VISIBLE)
                    if (row.colour != 0) setInt(R.id.agenda_strip, "setBackgroundColor", row.colour or 0xFF000000.toInt())
                } else {
                    setViewVisibility(R.id.agenda_strip, View.GONE)
                }
                setViewVisibility(R.id.agenda_check, if (row.box) View.VISIBLE else View.GONE)
                setViewPadding(R.id.agenda_time, dp(context, if (row.box) 0 else 12), 0, dp(context, 8), 0)
                setContentDescription(R.id.agenda_check, snapshot?.done ?: "")
                val title = SpannableString(row.title)
                if (row.done) strike(title)
                setImageViewResource(R.id.agenda_check, if (row.done) R.drawable.ic_widget_checked else R.drawable.ic_widget_check)
                // A coming day is dimmed, as on the Agenda, its box too (#194).
                setTextColor(
                    R.id.agenda_title,
                    context.getColor(if (row.done || row.ahead) R.color.agenda_widget_text_dim else R.color.agenda_widget_text),
                )
                setInt(R.id.agenda_check, "setImageAlpha", if (row.ahead) AHEAD_ALPHA else 255)
                setTextViewText(R.id.agenda_title, title)
                // A list's heading has no box; its words start where a
                // box's heading's do, over its entries' boxes (#209).
                val titleStart = when {
                    row.box || row.time.isNotEmpty() -> 0
                    row.heading -> AgendaWidgetPlan.BOX_DP
                    else -> 12
                }
                setViewPadding(R.id.agenda_title, dp(context, titleStart), 0, dp(context, 8), 0)
                setOnClickFillInIntent(R.id.agenda_check, fillIn(row.onBox))
            }
        }

        /** The fill-in intent that tells [AgendaTapActivity] to do [tap]. */
        private fun fillIn(tap: AgendaTap): Intent =
            Intent().apply { for ((name, value) in tap.extras()) putExtra(name, value) }

        private fun strike(text: SpannableString) =
            text.setSpan(StrikethroughSpan(), 0, text.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)

        private fun dp(context: Context, dp: Int): Int = (dp * context.resources.displayMetrics.density).toInt()

        /**
         * A box checked in the widget ([AgendaTapActivity]): it goes into
         * the queue as the Agenda's box would change the log, a Done the
         * way of a notification's, and shows checked at once.
         */
        fun check(context: Context, tap: AgendaTap.Check) {
            if (tap.kind == "done") return AlarmReceiver.done(context, tap.alarm)
            EventQueue.add(context, tap.kind, tap.alarm, line = tap.line)
            update(context)
            MainActivity.actionsArrived()
        }

        /** Opens the app on the Agenda. */
        fun openIntent(context: Context): Intent = Intent(context, MainActivity::class.java)
            .setAction(Intent.ACTION_MAIN)
            .addCategory(Intent.CATEGORY_LAUNCHER)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)

        /** The snapshot the app handed over last, or null before the first. */
        private fun snapshot(context: Context): AgendaSnapshot? =
            AgendaWidgetPlan.snapshotOf(Stored.text(prefs(context), KEY_SNAPSHOT))

        /** The inexact alarm that moves the widget on to the new day. */
        private fun midnight(context: Context): PendingIntent = PendingIntent.getBroadcast(
            context, 0,
            Intent(context, AgendaWidget::class.java).setAction(ACTION_MIDNIGHT),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    }
}
