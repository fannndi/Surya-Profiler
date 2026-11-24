package com.suryaprofiler.ui

import com.suryaprofiler.protocol.EnvSnapshot
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/** Display formatters shared by the Diagnostics cards. */

internal fun alive(ok: Boolean) = if (ok) "alive" else "down"

internal fun fmtTemp(t: Float?): String =
    t?.let { String.format(Locale.US, "%.1f °C", it) } ?: "—"

internal fun fmtCooling(cur: Int?, max: Int?): String = when {
    cur == null -> "—"
    max == null -> "$cur"
    else -> "$cur / $max"
}

internal fun fmtMhz(mhz: Int?): String =
    mhz?.let { String.format(Locale.US, "%.2f GHz", it / 1000.0) } ?: "—"

/** µA + µV -> "-0.58 A (chg)" style; sign: negative = charging. */
internal fun fmtCurrent(ua: Long?, uv: Long?): String {
    if (ua == null && uv == null) return "—"
    val i = ua?.let {
        val a = Math.abs(it) / 1_000_000.0
        val dir = if (it < 0) "chg" else "dis"
        if (a >= 0.05) String.format(Locale.US, "%.2f A (%s)", a, dir)
        else String.format(Locale.US, "%.0f mA (%s)", it / 1000.0, dir)
    } ?: "—"
    val v = uv?.let { String.format(Locale.US, "%.2f V", it / 1_000_000.0) } ?: "—"
    return "$i · $v"
}

internal fun fmtPairMhz(l: Int?, b: Int?): String =
    if (l == null || b == null) "—" else "$l · $b"

/**
 * "5008 mAh · 97% of 5160 · 562 cycles" — `charge_full_design` is broken on
 * this fuel gauge (negative), so the wear estimate uses the device spec.
 */
internal fun fmtBatteryHealth(env: EnvSnapshot): String {
    val parts = mutableListOf<String>()
    env.chargeFullMah?.let {
        parts += "$it mAh"
        parts += "${it * 100 / 5160}% of 5160"
    }
    env.cycleCount?.let { parts += "$it cycles" }
    return parts.joinToString(" · ").ifEmpty { "—" }
}
internal fun fmtUptime(s: Long): String = when {
    s < 60 -> "${s}s"
    s < 3600 -> "${s / 60}m ${s % 60}s"
    else -> "${s / 3600}h ${(s % 3600) / 60}m"
}

/** UFS raw hex -> "used 50–60% · eol <80%". Unknown/missing stays "—". */
internal fun fmtStorageHealth(life: String?, eol: String?): String {
    val lifeLabels = arrayOf(
        "unknown", "0–10%", "10–20%", "20–30%", "30–40%", "40–50%",
        "50–60%", "60–70%", "70–80%", "80–90%", "90–100%",
    )
    val lifeText = life?.removePrefix("0x")?.toIntOrNull(16)?.let { v ->
        when {
            v in 1..10 -> lifeLabels[v]
            v >= 0x0B -> "exceeded"
            else -> "unknown"
        }
    } ?: "—"
    val eolText = eol?.removePrefix("0x")?.toIntOrNull(16)?.let { v ->
        when (v) {
            1 -> "<80% reserved"
            2 -> "≈80% reserved"
            3 -> ">90% reserved"
            else -> "unknown"
        }
    } ?: "—"
    return "used $lifeText · eol $eolText"
}

private val timeFormat = ThreadLocal.withInitial {
    SimpleDateFormat("HH:mm", Locale.US)
}

internal fun fmtTime(t: Long): String =
    timeFormat.get()!!.format(Date(t * 1000))
