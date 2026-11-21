package com.suryaprofiler.protocol

import org.json.JSONObject

/** Null-safe JSON readers shared by every protocol parser. */

internal fun JSONObject.strOrNull(key: String): String? =
    if (has(key) && !isNull(key)) optString(key).ifEmpty { null } else null

internal fun JSONObject.intOrNull(key: String): Int? =
    if (has(key) && !isNull(key)) optInt(key) else null

internal fun JSONObject.floatOrNull(key: String): Float? =
    if (has(key) && !isNull(key)) optDouble(key).toFloat() else null

internal fun JSONObject.longOrNull(key: String): Long? =
    if (has(key) && !isNull(key)) optLong(key) else null

internal fun JSONObject.boolOrNull(key: String): Boolean? =
    if (has(key) && !isNull(key)) optBoolean(key) else null
