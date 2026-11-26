package com.suryaprofiler.protocol

import org.json.JSONArray
import org.json.JSONObject

/**
 * Power usage telemetry (`power` / `power_live` events).
 *
 * Units are the daemon's raw units: current µA (device sign: negative =
 * charging, positive = discharging), voltage µV. Formatting/watt math lives
 * in the UI model, not in the wire parser.
 */

data class PowerInput(
    val name: String,
    val online: Boolean?,
    val voltageUv: Long?,
    val currentUa: Long?,
    /** USB_CDP / USB_PD / Main / … when the rail reports it. */
    val kind: String? = null,
)

data class PowerSample(
    val tMs: Long,
    val pct: Int?,
    val status: String?,
    val currentUa: Long?,
    val voltageUv: Long?,
    val tempC: Float?,
    val chargeCounterUah: Long?,
    val chargeFullMah: Int?,
    val cycleCount: Int?,
    val inputSuspend: Boolean?,
    val chargeEnabled: Boolean?,
    val input: PowerInput?,
    val screenOn: Boolean,
    val pkg: String?,
    val profile: String?,
)

data class PowerBucket(
    val tMs: Long,
    val pct: Int?,
    val avgUa: Long,
    val minUa: Long,
    val maxUa: Long,
    val avgUv: Long,
    val avgTempC: Float?,
    val maxTempC: Float?,
    val screenOn: Boolean,
    val durMs: Long,
    val samples: Int,
    val pkg: String?,
    val profile: String?,
    val startMs: Long? = null,
    val endMs: Long? = null,
    val energyMwh: Double? = null,
    val segmentId: String? = null,
)

data class PowerPeak(
    val ua: Long,
    val uv: Long,
    val tMs: Long,
    val pct: Int?,
)

data class PowerTempPeak(
    val c: Float,
    val tMs: Long,
    val pct: Int?,
)

data class PowerPeaks(
    val charge: PowerPeak? = null,
    val discharge: PowerPeak? = null,
    val temp: PowerTempPeak? = null,
)

data class PowerSnapshot(
    val windowS: Long,
    val live: PowerSample?,
    val currentBucket: PowerBucket?,
    val history: List<PowerBucket>,
    val peaks: PowerPeaks,
    /** False for a malformed/missing snapshot; reducers must ignore it. */
    val isValid: Boolean = true,
)

object PowerParse {
    fun snapshot(root: JSONObject): PowerSnapshot {
        val s = root.optJSONObject("snapshot") ?: return invalidSnapshot()
        val windowS = s.requiredLong("window_s")?.takeIf { it >= 0 } ?: return invalidSnapshot()
        val history = s.optJSONArray("history") ?: return invalidSnapshot()
        return PowerSnapshot(
            windowS = windowS,
            live = s.optJSONObject("live")?.let(::sample),
            currentBucket = s.optJSONObject("current_bucket")?.let(::bucket),
            history = buckets(history),
            peaks = peaks(s.optJSONObject("peaks")),
        )
    }

    private fun invalidSnapshot() = PowerSnapshot(
        windowS = 0,
        live = null,
        currentBucket = null,
        history = emptyList(),
        peaks = PowerPeaks(),
        isValid = false,
    )

    fun sample(o: JSONObject): PowerSample? {
        val tMs = o.requiredLong("t_ms")?.takeIf { it >= 0 } ?: return null
        val screenOn = o.requiredBoolean("screen_on") ?: return null
        return PowerSample(
            tMs = tMs,
            pct = o.intOrNull("pct"),
            status = o.strOrNull("status"),
            currentUa = o.longOrNull("current_ua"),
            voltageUv = o.longOrNull("voltage_uv"),
            tempC = o.floatOrNull("temp_c"),
            chargeCounterUah = o.longOrNull("charge_counter_uah"),
            chargeFullMah = o.intOrNull("charge_full_mah"),
            cycleCount = o.intOrNull("cycle_count"),
            inputSuspend = o.boolOrNull("input_suspend"),
            chargeEnabled = o.boolOrNull("charge_enabled"),
            input = o.optJSONObject("input")?.let(::input),
            screenOn = screenOn,
            pkg = o.strOrNull("pkg"),
            profile = o.strOrNull("profile"),
        )
    }

    fun bucket(o: JSONObject): PowerBucket? {
        val tMs = o.requiredLong("t_ms")?.takeIf { it >= 0 } ?: return null
        val avgUa = o.requiredLong("avg_ua") ?: return null
        val minUa = o.requiredLong("min_ua") ?: return null
        val maxUa = o.requiredLong("max_ua") ?: return null
        val avgUv = o.requiredLong("avg_uv")?.takeIf { it > 0 } ?: return null
        val screenOn = o.requiredBoolean("screen_on") ?: return null
        val durMs = o.requiredLong("dur_ms")?.takeIf { it >= 0 } ?: return null
        val samples = o.requiredLong("samples")
            ?.takeIf { it in 0..Int.MAX_VALUE.toLong() }
            ?.toInt()
            ?: return null
        if (minUa > maxUa || avgUa !in minUa..maxUa) return null
        val startMs = o.optionalLong("start_ms")
        val endMs = o.optionalLong("end_ms")
        if (startMs != null && endMs != null && startMs > endMs) return null
        return PowerBucket(
            tMs = tMs,
            pct = o.intOrNull("pct"),
            avgUa = avgUa,
            minUa = minUa,
            maxUa = maxUa,
            avgUv = avgUv,
            avgTempC = o.floatOrNull("avg_temp_c"),
            maxTempC = o.floatOrNull("max_temp_c"),
            screenOn = screenOn,
            durMs = durMs,
            samples = samples,
            pkg = o.strOrNull("pkg"),
            profile = o.strOrNull("profile"),
            startMs = startMs,
            endMs = endMs,
            energyMwh = o.optionalDouble("energy_mwh"),
            segmentId = o.strOrNull("segment_id"),
        )
    }

    private fun JSONObject.requiredLong(key: String): Long? {
        val value = opt(key) as? Number ?: return null
        return if (value.toDouble().isFinite()) value.toLong() else null
    }

    private fun JSONObject.requiredBoolean(key: String): Boolean? = opt(key) as? Boolean

    private fun JSONObject.optionalLong(key: String): Long? {
        if (!has(key) || isNull(key)) return null
        val value = opt(key) as? Number ?: return null
        return if (value.toDouble().isFinite()) value.toLong() else null
    }

    private fun JSONObject.optionalDouble(key: String): Double? {
        if (!has(key) || isNull(key)) return null
        val value = opt(key) as? Number ?: return null
        return value.toDouble().takeIf { it.isFinite() }
    }

    private fun input(o: JSONObject): PowerInput = PowerInput(
        name = o.optString("name", "input"),
        online = o.boolOrNull("online"),
        voltageUv = o.longOrNull("voltage_uv"),
        currentUa = o.longOrNull("current_ua"),
        kind = o.strOrNull("kind"),
    )

    private fun buckets(arr: JSONArray?): List<PowerBucket> {
        if (arr == null) return emptyList()
        return (0 until arr.length()).mapNotNull { i ->
            arr.optJSONObject(i)?.let(::bucket)
        }
    }

    private fun peaks(o: JSONObject?): PowerPeaks {
        if (o == null) return PowerPeaks()
        return PowerPeaks(
            charge = o.optJSONObject("charge")?.let(::peak),
            discharge = o.optJSONObject("discharge")?.let(::peak),
            temp = o.optJSONObject("temp")?.let { t ->
                PowerTempPeak(
                    c = t.optDouble("c").toFloat(),
                    tMs = t.optLong("t_ms"),
                    pct = t.intOrNull("pct"),
                )
            },
        )
    }

    private fun peak(o: JSONObject): PowerPeak = PowerPeak(
        ua = o.optLong("ua"),
        uv = o.optLong("uv"),
        tMs = o.optLong("t_ms"),
        pct = o.intOrNull("pct"),
    )
}
