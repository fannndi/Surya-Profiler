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
)

object PowerParse {
    fun snapshot(root: JSONObject): PowerSnapshot {
        val s = root.optJSONObject("snapshot") ?: JSONObject()
        return PowerSnapshot(
            windowS = s.optLong("window_s", 86_400),
            live = s.optJSONObject("live")?.let(::sample),
            currentBucket = s.optJSONObject("current_bucket")?.let(::bucket),
            history = buckets(s.optJSONArray("history")),
            peaks = peaks(s.optJSONObject("peaks")),
        )
    }

    fun sample(o: JSONObject): PowerSample = PowerSample(
        tMs = o.optLong("t_ms"),
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
        screenOn = o.optBoolean("screen_on"),
        pkg = o.strOrNull("pkg"),
        profile = o.strOrNull("profile"),
    )

    fun bucket(o: JSONObject): PowerBucket = PowerBucket(
        tMs = o.optLong("t_ms"),
        pct = o.intOrNull("pct"),
        avgUa = o.optLong("avg_ua"),
        minUa = o.optLong("min_ua"),
        maxUa = o.optLong("max_ua"),
        avgUv = o.optLong("avg_uv"),
        avgTempC = o.floatOrNull("avg_temp_c"),
        maxTempC = o.floatOrNull("max_temp_c"),
        screenOn = o.optBoolean("screen_on"),
        durMs = o.optLong("dur_ms"),
        samples = o.optInt("samples"),
        pkg = o.strOrNull("pkg"),
        profile = o.strOrNull("profile"),
    )

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
