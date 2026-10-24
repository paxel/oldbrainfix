package io.github.paxel.oldbrainfix

import android.app.AlarmManager
import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.text.SpannableString
import android.text.Spanned
import android.text.style.StrikethroughSpan
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import java.util.TimeZone

/**
 * The home-screen Agenda widget: today's Occurrences in Agenda order,
 * each checked off as Done with its box, and the Unsorted count, which
 * opens the Unsorted list. It never reads the log: it shows the snapshot
 * the app hands over whenever it syncs the alarms ([replace]), and a Done
 * tapped here goes the way of one tapped on a notification, into the
 * [EventQueue] for the app to apply. The snapshot holds tomorrow too, so
 * the widget moves on at local midnight without the app.
 */
class AgendaWidget : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) =
        render(context, manager, ids)

    override fun onAppWidgetOptionsChanged(context: Context, manager: AppWidgetManager, id: Int, options: Bundle) =
        render(context, manager, intArrayOf(id))

    override fun onDisabled(context: Context) {
        context.getSystemService(AlarmManager::class.java).cancel(midnight(context))
    }

    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            ACTION_DONE -> AlarmReceiver.done(
                context,
                Alarm(
                    id = "",
                    routine = intent.getStringExtra(AlarmReceiver.EXTRA_ROUTINE) ?: return,
                    occurrence = intent.getStringExtra(AlarmReceiver.EXTRA_OCCURRENCE) ?: return,
                    local = null,
                    at = null,
                    title = intent.getStringExtra(AlarmReceiver.EXTRA_TITLE) ?: "",
                    body = intent.getStringExtra(AlarmReceiver.EXTRA_BODY) ?: "",
                ),
            )
            ACTION_MIDNIGHT -> update(context)
            else -> super.onReceive(context, intent)
        }
    }

    companion object {
        private const val PREFS = "oldbrainfix_widget"
        private const val KEY_SNAPSHOT = "agenda"
        private const val ACTION_DONE = "io.github.paxel.oldbrainfix.WIDGET_DONE"
        private const val ACTION_MIDNIGHT = "io.github.paxel.oldbrainfix.WIDGET_MIDNIGHT"

        /** Request codes of the widget's taps that open the app; apart from Capture's 0 to 2. */
        private const val REQUEST_OPEN = 100
        private const val REQUEST_UNSORTED = 101

        /** Keeps the app's new [snapshot] and shows it. */
        fun replace(context: Context, snapshot: JSONObject) {
            prefs(context).edit().putString(KEY_SNAPSHOT, snapshot.toString()).commit()
            update(context)
        }

        /**
         * Shows the snapshot again, as of now and with the Done tapped
         * meanwhile; nothing when no Agenda widget is on the home screen.
         */
        fun update(context: Context) {
            val manager = AppWidgetManager.getInstance(context) ?: return
            val ids = manager.getAppWidgetIds(ComponentName(context, AgendaWidget::class.java))
            if (ids.isNotEmpty()) render(context, manager, ids)
        }

        private fun render(context: Context, manager: AppWidgetManager, ids: IntArray) {
            val now = System.currentTimeMillis()
            val zone = TimeZone.getDefault()
            val snapshot = snapshot(context)
            val today = AgendaWidgetPlan.dayKey(now, zone)
            val pending = EventQueue.pending(context)
            for (id in ids) {
                val height = manager.getAppWidgetOptions(id).getInt(AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT)
                val view = AgendaWidgetPlan.view(snapshot, today, pending, AgendaWidgetPlan.room(height))
                manager.updateAppWidget(id, views(context, snapshot, view))
            }
            // Inexact is enough: the day moves on soon after midnight.
            context.getSystemService(AlarmManager::class.java)
                .set(AlarmManager.RTC, AgendaWidgetPlan.nextMidnight(now, zone), midnight(context))
        }

        private fun views(context: Context, snapshot: AgendaSnapshot?, view: AgendaView): RemoteViews {
            val views = RemoteViews(context.packageName, R.layout.widget_agenda)
            val open = PendingIntent.getActivity(
                context, REQUEST_OPEN,
                Intent(context, MainActivity::class.java)
                    .setAction(Intent.ACTION_MAIN)
                    .addCategory(Intent.CATEGORY_LAUNCHER)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
            views.setOnClickPendingIntent(R.id.agenda, open)
            if (snapshot != null) views.setTextViewText(R.id.agenda_heading, snapshot.today)
            if (snapshot != null && snapshot.unsorted > 0) {
                views.setViewVisibility(R.id.agenda_unsorted, View.VISIBLE)
                views.setTextViewText(R.id.agenda_unsorted, snapshot.unsortedText)
                views.setOnClickPendingIntent(
                    R.id.agenda_unsorted,
                    PendingIntent.getActivity(
                        context, REQUEST_UNSORTED,
                        Intent(context, MainActivity::class.java)
                            .putExtra(MainActivity.EXTRA_UNSORTED, true)
                            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
                        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
                    ),
                )
            } else {
                views.setViewVisibility(R.id.agenda_unsorted, View.GONE)
            }
            views.removeAllViews(R.id.agenda_rows)
            for (row in view.rows) views.addView(R.id.agenda_rows, row(context, snapshot, row, open))
            val message = when {
                view.stale -> context.getString(R.string.agenda_widget_open_app)
                view.rows.isEmpty() -> snapshot?.empty
                else -> null
            }
            views.setViewVisibility(R.id.agenda_message, if (message == null) View.GONE else View.VISIBLE)
            if (message != null) views.setTextViewText(R.id.agenda_message, message)
            views.setViewVisibility(R.id.agenda_more, if (view.more > 0) View.VISIBLE else View.GONE)
            if (snapshot != null && view.more > 0) {
                views.setTextViewText(R.id.agenda_more, AgendaWidgetPlan.moreText(snapshot.more, view.more))
            }
            return views
        }

        /**
         * One Occurrence; its box checks it off, the rest opens the app.
         * A Done one is struck through and has nothing to tap: it is
         * unchecked in the app.
         */
        private fun row(context: Context, snapshot: AgendaSnapshot?, row: WidgetRow, open: PendingIntent): RemoteViews {
            val views = RemoteViews(context.packageName, R.layout.widget_agenda_row)
            views.setTextViewText(R.id.agenda_time, row.time)
            views.setOnClickPendingIntent(R.id.agenda_row, open)
            views.setContentDescription(R.id.agenda_check, snapshot?.done ?: "")
            if (row.done) {
                views.setImageViewResource(R.id.agenda_check, R.drawable.ic_widget_checked)
                views.setTextViewText(
                    R.id.agenda_title,
                    SpannableString(row.title).apply {
                        setSpan(StrikethroughSpan(), 0, length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                    },
                )
                views.setTextColor(R.id.agenda_title, context.getColor(R.color.agenda_widget_text_dim))
                views.setOnClickPendingIntent(R.id.agenda_check, open)
                return views
            }
            views.setImageViewResource(R.id.agenda_check, R.drawable.ic_widget_check)
            views.setTextViewText(R.id.agenda_title, row.title)
            val done = Intent(context, AgendaWidget::class.java)
                .setAction(ACTION_DONE)
                // Tells the rows' taps apart; extras alone do not.
                .setData(
                    Uri.Builder().scheme("oldbrainfix").authority("widget")
                        .appendPath(row.routine).appendPath(row.occurrence).build(),
                )
                .putExtra(AlarmReceiver.EXTRA_ROUTINE, row.routine)
                .putExtra(AlarmReceiver.EXTRA_OCCURRENCE, row.occurrence)
                .putExtra(AlarmReceiver.EXTRA_TITLE, row.title)
                .putExtra(AlarmReceiver.EXTRA_BODY, row.time)
            views.setOnClickPendingIntent(
                R.id.agenda_check,
                PendingIntent.getBroadcast(
                    context, 0, done,
                    PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
                ),
            )
            return views
        }

        /** The snapshot the app handed over last, or null before the first. */
        private fun snapshot(context: Context): AgendaSnapshot? {
            val text = prefs(context).getString(KEY_SNAPSHOT, null) ?: return null
            return runCatching { parse(JSONObject(text)) }.getOrNull()
        }

        private fun parse(json: JSONObject): AgendaSnapshot {
            val days = json.getJSONObject("days")
            val labels = json.getJSONObject("labels")
            return AgendaSnapshot(
                days = days.keys().asSequence().associateWith { day ->
                    val rows = days.getJSONArray(day)
                    (0 until rows.length()).map {
                        val row = rows.getJSONObject(it)
                        WidgetRow(
                            routine = row.getString("routine"),
                            occurrence = row.getString("occurrence"),
                            title = row.optString("title"),
                            time = row.optString("time"),
                            done = row.optBoolean("done"),
                        )
                    }
                },
                unsorted = json.optInt("unsorted"),
                today = labels.optString("today"),
                empty = labels.optString("empty"),
                unsortedText = labels.optString("unsorted"),
                more = labels.optString("more"),
                done = labels.optString("done"),
            )
        }

        /** The inexact alarm that moves the widget on to the new day. */
        private fun midnight(context: Context): PendingIntent = PendingIntent.getBroadcast(
            context, 0,
            Intent(context, AgendaWidget::class.java).setAction(ACTION_MIDNIGHT),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    }
}
