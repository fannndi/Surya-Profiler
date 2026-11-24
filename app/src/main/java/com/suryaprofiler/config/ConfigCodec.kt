package com.suryaprofiler.config

import com.suryaprofiler.protocol.ProfileId
import com.suryaprofiler.protocol.strOrNull
import org.json.JSONObject

/** Pure schema-2 config model. No Android imports, so it is JVM-testable. */
data class AppConfig(
    val enabled: Boolean = true,
    val dynamic: Boolean = true,
    val customThermal: Boolean = true,
    val baseProfile: ProfileId = ProfileId.BALANCE,
    val appMap: Map<String, ProfileId> = emptyMap(),
    val rndInputSuspend: Boolean = false,
    val restorePending: Boolean = false,
    /** Freq floor audit sentinel (auto re-assert of a drifted CPU floor). */
    val freqFloorAudit: Boolean = true,
    /** Opt-in thread placement for apps mapped to Game (default off). */
    val threadAffinity: Boolean = false,
    /** Daemon log level (error|warn|info|debug); null = env/default. */
    val logLevel: String? = null,
)

/**
 * The single encoder/decoder for `config.json`.
 *
 * Compatibility:
 * - schema-1 `app_profiles[pkg].profile` is folded into `app_map`;
 * - unknown keys are dropped on write;
 * - unknown profile ids are ignored (never reach the engine).
 */
object ConfigCodec {
    const val SCHEMA = 2

    fun decode(json: JSONObject): AppConfig {
        val base = ProfileId.from(json.optString("base_profile"))
            ?.takeIf { it.isUser } ?: ProfileId.BALANCE

        val map = linkedMapOf<String, ProfileId>()
        json.optJSONObject("app_map")?.let { obj ->
            obj.keys().forEach { pkg ->
                ProfileId.from(obj.optString(pkg))
                    ?.takeIf { it.isUser }
                    ?.let { map[pkg] = it }
            }
        }
        // Legacy schema-1 mirror: the extended source wins for the mapping.
        json.optJSONObject("app_profiles")?.let { obj ->
            obj.keys().forEach { pkg ->
                ProfileId.from(obj.optJSONObject(pkg)?.optString("profile"))
                    ?.takeIf { it.isUser }
                    ?.let { map[pkg] = it }
            }
        }

        return AppConfig(
            enabled = json.optBoolean("enabled", true),
            dynamic = json.optBoolean("dynamic", true),
            customThermal = json.optBoolean("custom_thermal", true),
            baseProfile = base,
            appMap = map,
            rndInputSuspend = json.optBoolean("rnd_input_suspend", false),
            restorePending = json.optBoolean("restore_pending", false),
            freqFloorAudit = json.optBoolean("freq_floor_audit", true),
            threadAffinity = json.optBoolean("thread_affinity", false),
            logLevel = json.strOrNull("log_level"),
        )
    }

    fun encode(config: AppConfig): JSONObject = JSONObject()
        .put("schema", SCHEMA)
        .put("enabled", config.enabled)
        .put("dynamic", config.dynamic)
        .put("custom_thermal", config.customThermal)
        .put("base_profile", config.baseProfile.wire)
        .put("app_map", JSONObject().also { out ->
            for ((pkg, profile) in config.appMap) out.put(pkg, profile.wire)
        })
        .put("rnd_input_suspend", config.rndInputSuspend)
        .put("restore_pending", config.restorePending)
        .put("freq_floor_audit", config.freqFloorAudit)
        .put("thread_affinity", config.threadAffinity)
        .putOpt("log_level", config.logLevel)
}
