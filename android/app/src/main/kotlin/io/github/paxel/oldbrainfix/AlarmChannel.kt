package io.github.paxel.oldbrainfix

import android.content.Context
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import org.json.JSONArray
import org.json.JSONObject

/**
 * The alarm channel's calls that need no Activity: arming the plan, the
 * taps queued meanwhile, the ringing alarm, the Agenda widget and the
 * permissions. The app answers them, and so does the background sync's
 * engine (SyncWorker), which arms the plan anew after a sync.
 */
object AlarmChannel {
    /** Answers [call] and returns true, or false when it is not one of these. */
    fun handle(context: Context, call: MethodCall, result: MethodChannel.Result): Boolean {
        when (call.method) {
            "arm" -> {
                val args = call.arguments as Map<*, *>
                Scheduler.replace(
                    context,
                    channelJson(args["alarms"]) as JSONArray,
                    channelJson(args["labels"]) as JSONObject,
                    channelJson(args["done"]) as JSONArray,
                    channelJson(args["nudge"]) as JSONObject?,
                )
                result.success(null)
            }
            "pending" -> result.success(
                EventQueue.pending(context).map {
                    mapOf(
                        "seq" to it.seq,
                        "kind" to it.kind,
                        "routine" to it.routine,
                        "occurrence" to it.occurrence,
                        "at" to it.at,
                        "until" to it.until,
                    )
                },
            )
            "ack" -> {
                EventQueue.ack(context, (call.arguments as Number).toLong())
                Scheduler.arm(context)
                result.success(null)
            }
            "ringing" -> result.success(
                RingService.ringing?.let {
                    mapOf(
                        "routine" to it.routine,
                        "occurrence" to it.occurrence,
                        "title" to it.title,
                        "body" to it.body,
                    )
                },
            )
            "answerRing" -> {
                val args = call.arguments as Map<*, *>
                val alarm = Alarm(
                    id = "",
                    routine = args["routine"] as String,
                    occurrence = args["occurrence"] as String,
                    local = null,
                    at = null,
                    title = args["title"] as String,
                    body = args["body"] as String,
                )
                val minutes = (args["snooze"] as Number?)?.toInt()
                if (minutes == null) {
                    AlarmReceiver.done(context, alarm)
                } else {
                    AlarmReceiver.snooze(context, alarm, minutes)
                }
                result.success(null)
            }
            "agenda" -> {
                AgendaWidget.replace(context, channelJson(call.arguments) as JSONObject)
                result.success(null)
            }
            "permissions" -> result.success(Permissions.granted(context))
            else -> return false
        }
        return true
    }
}

/** Channel arguments (maps, lists, numbers) as org.json values. */
fun channelJson(value: Any?): Any? = when (value) {
    is Map<*, *> -> JSONObject().apply {
        for ((k, v) in value) put(k.toString(), channelJson(v))
    }
    is List<*> -> JSONArray().apply { for (v in value) put(channelJson(v)) }
    else -> value
}
