package io.github.paxel.oldbrainfix

import android.app.Activity
import android.content.Intent
import android.graphics.Typeface
import android.net.Uri
import android.os.Bundle
import android.util.TypedValue
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.io.File

/**
 * What a crash in the Kotlin code shows, in a process of its own and
 * without Flutter, so it shows even when the app dies before Flutter
 * starts: what happened, the report, and a mail to the developer. Once
 * shown, the report is not offered again at the next start.
 */
class CrashActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val report = intent.getStringExtra(EXTRA_REPORT) ?: ""
        val mail = intent.getStringExtra(EXTRA_MAIL) ?: report.take(CrashLog.MAIL_LIMIT)
        // Shown here, so the Flutter side does not ask about it again.
        File(filesDir, CrashLog.FILE).delete()
        File(filesDir, CrashLog.MARKER).delete()

        val pad = dp(24)
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad, pad, pad)
        }
        column.addView(TextView(this).apply {
            setText(R.string.crash_title)
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 22f)
        })
        column.addView(TextView(this).apply {
            setText(R.string.crash_body)
            setPadding(0, dp(8), 0, dp(16))
        })
        column.addView(Button(this).apply {
            setText(R.string.crash_send_report)
            setOnClickListener { send(mail) }
        })
        column.addView(Button(this).apply {
            setText(R.string.crash_close)
            setOnClickListener { finish() }
        })
        column.addView(TextView(this).apply {
            text = report
            typeface = Typeface.MONOSPACE
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 12f)
            setTextIsSelectable(true)
            setPadding(0, dp(16), 0, 0)
        })
        setContentView(
            ScrollView(this).apply {
                fitsSystemWindows = true
                addView(column)
            },
        )
    }

    private fun send(body: String) {
        val uri = Uri.parse(
            "mailto:${CrashLog.MAIL}?subject=${Uri.encode(CrashLog.MAIL_SUBJECT)}&body=${Uri.encode(body)}",
        )
        try {
            startActivity(Intent(Intent.ACTION_SENDTO, uri))
        } catch (_: Exception) {
            // No mail app; the report stays on screen to copy.
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    companion object {
        /** The process the crash screen runs in, apart from the one that died. */
        const val PROCESS = ":crash"
        const val EXTRA_REPORT = "report"
        const val EXTRA_MAIL = "mail"
    }
}
