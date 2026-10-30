package io.github.paxel.oldbrainfix

import android.content.SharedPreferences
import org.json.JSONArray
import org.json.JSONObject

/**
 * Reading what the app keeps in SharedPreferences: the alarm plan, its
 * labels, the Nudge plans, the taps queued for the app and the Agenda
 * widget's snapshot. A value that is missing, of another type or not
 * readable reads as none, so neither a receiver nor the widget crashes
 * over it. Each JSON object kept there carries the version of its
 * structure by the rule of ADR 0019: a missing `v` means 1, and 1 is
 * never written; only the whole number 1 is 1 (ADR 0034). One of a later
 * version is left as it is and not used here.
 */
object Stored {
    /** The text under [key], or null when there is none or it is no text. */
    fun text(prefs: SharedPreferences, key: String): String? =
        runCatching { prefs.getString(key, null) }.getOrNull()

    /** The number under [key], or [fallback] when there is none or it is no number. */
    fun long(prefs: SharedPreferences, key: String, fallback: Long = 0): Long =
        runCatching { prefs.getLong(key, fallback) }.getOrDefault(fallback)

    /** Whether [json] is of version 1. */
    fun current(json: JSONObject): Boolean {
        if (!json.has("v")) return true
        val v = json.opt("v")
        return (v is Int && v == 1) || (v is Long && v == 1L)
    }

    /** The array in [text], or null when there is none or it is no array. */
    fun array(text: String?): JSONArray? = text?.let { runCatching { JSONArray(it) }.getOrNull() }

    /**
     * The object in [text] read with [read]; null when there is none, it
     * is no object, it is of a later version or [read] fails on it.
     */
    fun <T> objectOf(text: String?, read: (JSONObject) -> T): T? {
        val json = text?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return null
        if (!current(json)) return null
        return runCatching { read(json) }.getOrNull()
    }

    /** The element [i] of [array] read with [read]; null as for [objectOf]. */
    fun <T> elementOf(array: JSONArray, i: Int, read: (JSONObject) -> T): T? {
        val json = array.optJSONObject(i) ?: return null
        if (!current(json)) return null
        return runCatching { read(json) }.getOrNull()
    }

    /** The objects of [array] read with [read], in order, leaving out each [elementOf] gives none for. */
    fun <T> elementsOf(array: JSONArray, read: (JSONObject) -> T): List<T> =
        (0 until array.length()).mapNotNull { elementOf(array, it, read) }

    /** The objects of the array in [text], as [elementsOf] reads them; none without one. */
    fun <T> listOf(text: String?, read: (JSONObject) -> T): List<T> =
        array(text)?.let { elementsOf(it, read) } ?: emptyList()
}
