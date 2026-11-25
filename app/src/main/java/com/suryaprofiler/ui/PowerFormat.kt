package com.suryaprofiler.ui

import com.suryaprofiler.protocol.PowerSample
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlin.math.abs

/**
 * Pure power formatting/semantics shared by the Power screen and its tests.
 * No Android imports: the JVM test suite exercises every function.
 */

internal const val DESIGN_CAPACITY_MAH = 5160
private const val IDLE_UA = 20_000L

/** Signed battery-side power in mW: negative = charging, positive = usage. */
internal fun powerMw(currentUa: Long?, voltageUv: Long?): Double? {
    val ua = currentUa ?: return null
    val uv = voltageUv ?: return null
    if (uv <= 0) return null
    return ua.toDouble() * uv.toDouble() / 1_000_000_000.0
}

internal fun formatCurrent(ua: Long?): String {
    val v = ua ?: return "—"
    val ma = abs(v) / 1000.0
    return if (ma >= 1000) {
        String.format(Locale.US, "%.2f A", ma / 1000.0)
    } else {
        String.format(Locale.US, "%.0f mA", ma)
    }
}

internal fun formatPower(mw: Double?): String {
    val v = mw ?: return "—"
    val a = abs(v)
    return if (a >= 1000) {
        String.format(Locale.US, "%.2f W", a / 1000.0)
    } else {
        String.format(Locale.US, "%.0f mW", a)
    }
}

internal fun formatVoltage(uv: Long?): String =
    uv?.let { String.format(Locale.US, "%.2f V", it / 1_000_000.0) } ?: "—"

internal fun formatPowerTemp(c: Float?): String =
    c?.let { String.format(Locale.US, "%.1f °C", it) } ?: "—"

internal fun statusLabel(status: String?): String = when (status) {
    "charging" -> "Charging"
    "discharging" -> "Discharging"
    "full" -> "Full"
    "not_charging" -> "Not charging"
    "unknown" -> "Unknown"
    else -> "—"
}

/** Direction from the verified sign, not the status string. */
internal fun powerDirection(sample: PowerSample?): String {
    val ua = sample?.currentUa ?: return "idle"
    return when {
        ua < -IDLE_UA -> "charging"
        ua > IDLE_UA -> "usage"
        else -> "idle"
    }
}

internal fun powerWarnings(sample: PowerSample?): List<String> {
    if (sample == null) return emptyList()
    val out = mutableListOf<String>()
    if (sample.inputSuspend == true) {
        out += "Charging bypass active (R&D input_suspend=1)"
    }
    if (sample.chargeEnabled == false) {
        out += "Charge guard off (battery_charging_enabled=0)"
    }
    if (sample.status == "charging" && (sample.currentUa ?: 0) > IDLE_UA) {
        out += "Status says charging but current flows out — charge guard or charger limit"
    }
    if (sample.currentUa == null || sample.voltageUv == null) {
        out += "Battery current/voltage node unavailable"
    }
    return out
}

/** Rough time to full from charge_counter (or pct × design capacity). */
internal fun estimateToFull(sample: PowerSample?): String? {
    if (sample == null || powerDirection(sample) != "charging") return null
    val pct = sample.pct ?: return null
    if (pct >= 99) return null
    val currentUa = sample.currentUa ?: return null
    val rateMa = abs(currentUa) / 1000.0
    if (rateMa < 100) return null
    val remainingUah = sample.chargeCounterUah
        ?.let { counter ->
            sample.chargeFullMah?.let { full -> full * 1000L - counter }
        }
        ?: ((100 - pct) / 100.0 * DESIGN_CAPACITY_MAH * 1000).toLong()
    if (remainingUah <= 0) return null
    val seconds = (remainingUah / 1000.0 / rateMa * 3600).toLong()
    return fmtDuration(seconds)
}

/** Rough time to empty at the current battery-side load. */
internal fun estimateToEmpty(sample: PowerSample?): String? {
    if (sample == null || powerDirection(sample) != "usage") return null
    val currentUa = sample.currentUa ?: return null
    val rateMa = abs(currentUa) / 1000.0
    if (rateMa < 50) return null
    val remainingMah = sample.chargeCounterUah
        ?.let { it / 1000.0 }
        ?: ((sample.pct ?: return null) / 100.0 * DESIGN_CAPACITY_MAH)
    if (remainingMah <= 0) return null
    return fmtDuration((remainingMah / rateMa * 3600).toLong())
}

internal fun batteryHealthLabel(sample: PowerSample?): String {
    val parts = mutableListOf<String>()
    sample?.chargeFullMah?.let {
        parts += "$it mAh"
        parts += "${it * 100 / DESIGN_CAPACITY_MAH}% of $DESIGN_CAPACITY_MAH"
    }
    sample?.cycleCount?.let { parts += "$it cycles" }
    return parts.joinToString(" · ").ifEmpty { "—" }
}

private val clock = ThreadLocal.withInitial {
    SimpleDateFormat("HH:mm", Locale.US)
}
private val dateTime = ThreadLocal.withInitial {
    SimpleDateFormat("MMM d, HH:mm", Locale.US)
}

internal fun fmtClock(tMs: Long): String = clock.get()!!.format(Date(tMs))

/** Today -> HH:mm, older -> "Nov 24, 14:05". */
internal fun fmtWhen(tMs: Long): String {
    val now = System.currentTimeMillis()
    val sameDay = now - tMs < 24 * 3_600_000L && now >= tMs
    return (if (sameDay) clock else dateTime).get()!!.format(Date(tMs))
}

internal fun fmtAxisW(mw: Double): String {
    val w = mw / 1000.0
    return if (abs(w) >= 0.1) {
        String.format(Locale.US, "%.1f W", w)
    } else {
        String.format(Locale.US, "%.0f mW", mw)
    }
}
