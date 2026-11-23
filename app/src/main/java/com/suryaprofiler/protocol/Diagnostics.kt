package com.suryaprofiler.protocol

import org.json.JSONArray
import org.json.JSONObject

/**
 * Diagnostics data shapes for the daemon's read-only telemetry.
 *
 * Parsing lives here (not in the service) so the JSON contract is greppable
 * in one file; the daemon is the source of truth for every value.
 */

/** One environment sample from the daemon's `env` event. */
data class EnvSnapshot(
    val batteryPct: Int?,
    val charging: Boolean?,
    val batteryTempC: Float?,
    val cpuTempC: Float?,
    val gpuTempC: Float?,
    val gpuBusyPct: Int?,
    /** Panel frame rate from the read-only DRM `measured_fps` node. */
    val screenFps: Float? = null,
    /** Battery current µA: negative = charging, positive = discharging. */
    val batteryCurrentUa: Long? = null,
    val batteryVoltageUv: Long? = null,
    val littleFreqMhz: Int? = null,
    val bigFreqMhz: Int? = null,
    val gpuFreqMhz: Int? = null,
    /** F2FS userdata lifetime write counter (KB, read-only node). */
    val storageWrittenKb: Long? = null,
    /** Battery full-charge capacity in mAh (`charge_full`), health readout. */
    val chargeFullMah: Int? = null,
    /** Battery cycle count (`cycle_count`), when the fuel gauge exposes it. */
    val cycleCount: Int? = null,
    /** MIUI thermal scenario selector (`thermal_message/sconfig`). */
    val thermalSconfig: Int? = null,
    /** Resolved config name from the ROM's `thermal-map.conf`. */
    val thermalMode: String? = null,
    /** Full MIUI scenario label decoded from PowerKeeper's ID table. */
    val thermalScenario: String? = null,
    /** MIUI performance-mode flag (`thermal-global-mode`). */
    val thermalGlobalMode: Int? = null,
    /** MIUI performance-mode property (`persist.sys.thermal.config`). */
    val thermalPerfConfig: Int? = null,
    /** MIUI thermal severity state (`thermal_message/temp_state`). */
    val thermalTempState: Int? = null,
    /** MIUI screen state mirrored into thermal_message (0 off, 1 on). */
    val thermalScreenState: Int? = null,
    /** Quiet-therm skin temperature. */
    val quietTempC: Float? = null,
    /** NVM thermistor virtual sensor used by the CPU SS algorithm. */
    val nvmTempC: Float? = null,
    /** CPU6/CPU7 cpufreq cooling states (0 = no cap). */
    val cpu6Cooling: Int? = null,
    val cpu6CoolingMax: Int? = null,
    val cpu7Cooling: Int? = null,
    val cpu7CoolingMax: Int? = null,
    /** Effective policy max frequencies after thermal/perf clamping (MHz). */
    val littleMaxMhz: Int? = null,
    val bigMaxMhz: Int? = null,
)

/** Auto-revive (watchdog) totals from the last `stats` reply. */
data class HealsInfo(
    val total: Long = 0,
    val lastT: Long = 0,
    val lastKeys: Int = 0,
)

/** One transition from the daemon's `stats` reply. */
data class StatEntry(
    val t: Long,
    val from: String?,
    val to: String,
    val reason: String,
    val batteryPct: Int?,
    val tempC: Float?,
)

/** One `mi_thermald` event from the daemon's `diag` reply. */
data class ThermalEvent(
    val time: String,
    val algo: String,
    val sensor: String?,
    val sensorValue: String?,
    val device: String?,
    val deviceValue: String?,
    val raw: String,
)

/** Daemon health snapshot from the `diag` reply. */
data class DiagInfo(
    val pid: Int,
    val uptimeS: Long,
    val active: String?,
    val reason: String?,
    val foreground: String?,
    val screenOn: Boolean,
    val locked: Boolean,
    val multiWindow: Boolean,
    val watcherFg: Boolean,
    val saverHeld: Boolean,
    val inputSuspendHeld: Boolean,
    val thermalHeld: Boolean,
    val thermalSavedSconfig: Int?,
    val thermalProfile: String?,
    val thermalYield: Int?,
    val statsLen: Int,
    val configEnabled: Boolean,
    val configDynamic: Boolean,
    val baseProfile: String,
    /** Newest MIUI thermal events (read-only). */
    val thermalEvents: List<ThermalEvent> = emptyList(),
    /** REA 2026-11-23: PowerKeeper's −2 °C SP-game virtual-sensor
     *  compensation is likely active for [foreground] (null = unknown fg). */
    val miuiSpGame: Boolean? = null,
    /** Newest daemon log lines (ring buffer, oldest first). */
    val recentLogs: List<String> = emptyList(),
)

object DiagnosticsParse {

    fun env(ev: JSONObject): EnvSnapshot =
        envObject(ev.optJSONObject("env") ?: JSONObject())

    /** Parses an env object from either an `env` event or a `diag` reply. */
    fun envObject(e: JSONObject): EnvSnapshot {
        return EnvSnapshot(
            batteryPct = e.intOrNull("battery_pct"),
            charging = e.boolOrNull("charging"),
            batteryTempC = e.floatOrNull("battery_temp_c"),
            cpuTempC = e.floatOrNull("cpu_temp_c"),
            gpuTempC = e.floatOrNull("gpu_temp_c"),
            gpuBusyPct = e.intOrNull("gpu_busy_pct"),
            screenFps = e.floatOrNull("screen_fps"),
            batteryCurrentUa = e.longOrNull("battery_current_ua"),
            batteryVoltageUv = e.longOrNull("battery_voltage_uv"),
            littleFreqMhz = e.intOrNull("little_freq_mhz"),
            bigFreqMhz = e.intOrNull("big_freq_mhz"),
            gpuFreqMhz = e.intOrNull("gpu_freq_mhz"),
            storageWrittenKb = e.longOrNull("storage_written_kb"),
            chargeFullMah = e.intOrNull("charge_full_mah"),
            cycleCount = e.intOrNull("cycle_count"),
            thermalSconfig = e.intOrNull("thermal_sconfig"),
            thermalMode = e.strOrNull("thermal_mode"),
            thermalScenario = e.strOrNull("thermal_scenario"),
            thermalGlobalMode = e.intOrNull("thermal_global_mode"),
            thermalPerfConfig = e.intOrNull("thermal_perf_config"),
            thermalTempState = e.intOrNull("thermal_temp_state"),
            thermalScreenState = e.intOrNull("thermal_screen_state"),
            quietTempC = e.floatOrNull("quiet_temp_c"),
            nvmTempC = e.floatOrNull("nvm_temp_c"),
            cpu6Cooling = e.intOrNull("cpu6_cooling"),
            cpu6CoolingMax = e.intOrNull("cpu6_cooling_max"),
            cpu7Cooling = e.intOrNull("cpu7_cooling"),
            cpu7CoolingMax = e.intOrNull("cpu7_cooling_max"),
            littleMaxMhz = e.intOrNull("little_max_mhz"),
            bigMaxMhz = e.intOrNull("big_max_mhz"),
        )
    }

    /** Auto-revive totals carried by the `stats` reply. */
    fun heals(ev: JSONObject): HealsInfo = HealsInfo(
        total = ev.optLong("heals_total"),
        lastT = ev.optLong("heals_last_t"),
        lastKeys = ev.optInt("heals_last_keys"),
    )

    fun stats(ev: JSONObject): List<StatEntry> {
        val arr: JSONArray = ev.optJSONArray("entries") ?: return emptyList()
        return (0 until arr.length()).mapNotNull { i ->
            val o = arr.optJSONObject(i) ?: return@mapNotNull null
            StatEntry(
                t = o.optLong("t"),
                from = o.strOrNull("from"),
                to = o.optString("to", "?"),
                reason = o.optString("reason", "?"),
                batteryPct = o.intOrNull("battery"),
                tempC = o.floatOrNull("temp_c"),
            )
        }
    }

    fun diag(ev: JSONObject): DiagInfo? {
        val d = ev.optJSONObject("diag") ?: return null
        val cfg = d.optJSONObject("config") ?: JSONObject()
        val w = d.optJSONObject("watchers") ?: JSONObject()
        val h = d.optJSONObject("holds") ?: JSONObject()
        return DiagInfo(
            pid = d.optInt("pid"),
            uptimeS = d.optLong("uptime_s"),
            active = d.strOrNull("active"),
            reason = d.strOrNull("reason"),
            foreground = d.strOrNull("foreground"),
            screenOn = d.optBoolean("screen_on"),
            locked = d.optBoolean("locked"),
            multiWindow = d.optBoolean("multi_window"),
            watcherFg = w.optBoolean("fg"),
            saverHeld = h.optBoolean("saver_held"),
            inputSuspendHeld = h.optBoolean("input_suspend_held"),
            thermalHeld = h.optBoolean("thermal_held"),
            thermalSavedSconfig = h.intOrNull("thermal_saved_sconfig"),
            thermalProfile = h.strOrNull("thermal_profile"),
            thermalYield = h.intOrNull("thermal_yield"),
            statsLen = d.optInt("stats_len"),
            configEnabled = cfg.optBoolean("enabled"),
            configDynamic = cfg.optBoolean("dynamic"),
            baseProfile = cfg.optString("base_profile", "?"),
            thermalEvents = thermalEvents(d),
            miuiSpGame = d.boolOrNull("miui_sp_game"),
            recentLogs = d.optJSONArray("recent_logs")?.let { arr ->
                (0 until arr.length()).map { i -> arr.optString(i) }
            } ?: emptyList(),
        )
    }

    /** Parses the `thermal_events` array from a `diag` reply. */
    fun thermalEvents(d: JSONObject): List<ThermalEvent> {
        val arr = d.optJSONArray("thermal_events") ?: return emptyList()
        return (0 until arr.length()).mapNotNull { i ->
            val o = arr.optJSONObject(i) ?: return@mapNotNull null
            ThermalEvent(
                time = o.optString("time"),
                algo = o.optString("algo"),
                sensor = o.strOrNull("sensor"),
                sensorValue = o.strOrNull("sensor_value"),
                device = o.strOrNull("device"),
                deviceValue = o.strOrNull("device_value"),
                raw = o.optString("raw"),
            )
        }
    }
}
