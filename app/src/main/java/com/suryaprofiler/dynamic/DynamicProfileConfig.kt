package com.suryaprofiler.dynamic

import android.content.Context
import android.util.Log
import com.suryaprofiler.config.AppConfig
import com.suryaprofiler.config.ConfigCodec
import com.suryaprofiler.protocol.ProfileId
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.json.JSONObject
import java.io.File

/**
 * App-owned user config (`filesDir/config.json`), exposed as StateFlows.
 *
 * Schema 2 is deliberately tiny: service flags, the universal base, the
 * per-app Device-Profile mapping, the automatic thermal takeover, the R&D
 * input-suspend switch, the opt-in Game thread placement, per-app render
 * scale, the optional global display preset and the crash-recovery
 * `restore_pending` marker.
 *
 * Ownership: the app is the SINGLE writer; the Rust daemon reads the file
 * (start, `config_changed` hint, mtime fallback). Writes are atomic
 * (tmp + rename). [ConfigCodec] owns schema/migration; this class owns IO and
 * the observable StateFlows.
 */
class DynamicProfileConfig private constructor(context: Context) {

    companion object {
        private const val TAG = "SuryaProfiler"
        const val FILE = "config.json"
        const val DEFAULT_BASE = "balance"

        @Volatile
        private var instance: DynamicProfileConfig? = null

        fun get(context: Context): DynamicProfileConfig =
            instance ?: synchronized(this) {
                instance ?: DynamicProfileConfig(context.applicationContext).also { instance = it }
            }
    }

    private val file = File(context.filesDir, FILE)
    internal val lock = Any()

    private val _enabled = MutableStateFlow(true)
    val enabledFlow: StateFlow<Boolean> = _enabled
    var enabled: Boolean
        get() = _enabled.value
        set(v) { put { it.put("enabled", v) } }

    private val _dynamicEnabled = MutableStateFlow(true)
    val dynamicEnabledFlow: StateFlow<Boolean> = _dynamicEnabled
    var dynamicEnabled: Boolean
        get() = _dynamicEnabled.value
        set(v) { put { it.put("dynamic", v) } }

    /** Take over MIUI thermal per Device Profile (stock map is the fallback). */
    private val _customThermal = MutableStateFlow(true)
    val customThermalFlow: StateFlow<Boolean> = _customThermal
    var customThermal: Boolean
        get() = _customThermal.value
        set(v) { put { it.put("custom_thermal", v) } }

    /** R&D only: suspend charger input while the service is on. */
    private val _rndInputSuspend = MutableStateFlow(false)
    val rndInputSuspendFlow: StateFlow<Boolean> = _rndInputSuspend
    var rndInputSuspend: Boolean
        get() = _rndInputSuspend.value
        set(v) { put { it.put("rnd_input_suspend", v) } }

    /** Freq floor audit: re-assert the profile's CPU floor on drift. */
    private val _freqFloorAudit = MutableStateFlow(true)
    val freqFloorAuditFlow: StateFlow<Boolean> = _freqFloorAudit
    var freqFloorAudit: Boolean
        get() = _freqFloorAudit.value
        set(v) { put { it.put("freq_floor_audit", v) } }

    /** Opt-in thread placement for apps mapped to Game. */
    private val _threadAffinity = MutableStateFlow(false)
    val threadAffinityFlow: StateFlow<Boolean> = _threadAffinity
    var threadAffinity: Boolean
        get() = _threadAffinity.value
        set(v) { put { it.put("thread_affinity", v) } }

    /** Per-app render scale (canonical ratio strings). */
    private val _appDownscale = MutableStateFlow<Map<String, String>>(emptyMap())
    val appDownscaleFlow: StateFlow<Map<String, String>> = _appDownscale

    /** null/blank ratio removes the per-app downscale. */
    fun setAppDownscale(pkg: String, ratio: String?): Boolean = put { json ->
        val map = json.optJSONObject("app_downscale") ?: JSONObject()
        if (ratio.isNullOrBlank()) map.remove(pkg) else map.put(pkg, ratio)
        json.put("app_downscale", map)
    }

    /** Global display preset (`off`, `hd`, `80`, ...). */
    private val _displayScale = MutableStateFlow("off")
    val displayScaleFlow: StateFlow<String> = _displayScale
    var displayScale: String
        get() = _displayScale.value
        set(v) { put { it.put("display_scale", v) } }

    /** A Service OFF was started but not confirmed yet (crash recovery). */
    private val _restorePending = MutableStateFlow(false)
    val restorePending: Boolean get() = _restorePending.value

    private val _appMap = MutableStateFlow<Map<String, String>>(emptyMap())
    val appMapFlow: StateFlow<Map<String, String>> = _appMap

    private val _baseProfile = MutableStateFlow(DEFAULT_BASE)
    val baseProfileFlow: StateFlow<String> = _baseProfile

    /** The universal base: the last manually selected profile. */
    var baseProfile: String
        get() = _baseProfile.value
        set(v) { put { it.put("base_profile", v) } }

    init {
        val loaded = loadStrict()
        if (loaded.isFailure) {
            Log.w(TAG, "config parse failed; failing closed", loaded.exceptionOrNull())
            applyConfig(
                AppConfig(enabled = false, dynamic = false, customThermal = false),
            )
        } else {
            val raw = loaded.getOrNull() ?: JSONObject()
            val schema = raw.optInt("schema", ConfigCodec.SCHEMA)
            if (schema > ConfigCodec.SCHEMA) {
                // Never downgrade a future config. Fail closed until the app
                // that understands it runs.
                Log.w(TAG, "config schema $schema is newer; refusing to rewrite")
                applyConfig(
                    AppConfig(
                        enabled = false,
                        dynamic = false,
                        customThermal = false,
                    ),
                )
            } else {
                var cfg = ConfigCodec.decode(raw)
                // Service OFF means no intervention: a persisted R&D suspend
                // flag from a crashed/old session must not survive.
                if (!cfg.enabled && cfg.rndInputSuspend) cfg = cfg.copy(rndInputSuspend = false)
                applyConfig(cfg)
                // Always normalize: removes every schema-1 key and folds the
                // old `app_profiles[*].profile` mirror into `app_map`.
                writeAtomic(ConfigCodec.encode(cfg).toString())
                    .onFailure { Log.w(TAG, "config normalize failed: $it") }
            }
        }
    }

    fun appMap(): Map<String, String> = _appMap.value

    /**
     * Starts Service OFF. `restore_pending` is persisted *before* the restore
     * runs, so a process death mid-restore is retried on the next launch.
     */
    fun beginServiceOff(): Boolean = put {
        it.put("enabled", false)
        it.put("restore_pending", true)
    }

    /** Stock restore verified: keep the service disabled and clear the flag. */
    fun confirmServiceOff(): Boolean = put {
        it.put("enabled", false)
        it.put("restore_pending", false)
    }

    /** Restore failed: keep the service ON so the user can retry. */
    fun abortServiceOff(): Boolean = put {
        it.put("enabled", true)
        it.put("restore_pending", false)
    }

    /** null profileId removes the mapping; an unknown id is rejected. */
    fun setAppProfile(pkg: String, profileId: String?): Boolean {
        if (profileId != null && ProfileId.from(profileId)?.isUser != true) {
            Log.w(TAG, "setAppProfile: rejected profile '$profileId'")
            return false
        }
        return put { json ->
            val map = json.optJSONObject("app_map") ?: JSONObject()
            val profile = ProfileId.from(profileId)?.takeIf { it.isUser }
            if (profile == null) map.remove(pkg) else map.put(pkg, profile.wire)
            json.put("app_map", map)
        }
    }

    /** Missing file = success(null); corrupt file = failure (never overwrite). */
    internal fun loadStrict(): Result<JSONObject?> {
        if (!file.exists()) return Result.success(null)
        return runCatching { JSONObject(file.readText()) }
    }

    internal fun applyConfig(cfg: AppConfig) {
        _enabled.value = cfg.enabled
        _dynamicEnabled.value = cfg.dynamic
        _customThermal.value = cfg.customThermal
        _baseProfile.value = cfg.baseProfile.wire
        _appMap.value = cfg.appMap.mapValues { it.value.wire }
        _rndInputSuspend.value = cfg.rndInputSuspend
        _restorePending.value = cfg.restorePending
        _freqFloorAudit.value = cfg.freqFloorAudit
        _threadAffinity.value = cfg.threadAffinity
        _appDownscale.value = cfg.appDownscale
        _displayScale.value = cfg.displayScale
    }

    /**
     * Mutates one key, persists atomically, then refreshes flows and hints the
     * daemon. Returns false on a read/parse/write failure; the in-memory
     * StateFlows are not advanced in that case.
     */
    private fun put(mutate: (JSONObject) -> Unit): Boolean {
        val written = synchronized(lock) {
            val loaded = loadStrict().getOrElse {
                Log.w(TAG, "config write refused: unreadable file")
                return@synchronized false
            }
            val json = loaded ?: JSONObject()
            if (json.optInt("schema", ConfigCodec.SCHEMA) > ConfigCodec.SCHEMA) {
                Log.w(TAG, "config write refused: newer schema on disk")
                return@synchronized false
            }
            mutate(json)
            val cfg = ConfigCodec.decode(json)
            val result = writeAtomic(ConfigCodec.encode(cfg).toString())
            if (result.isFailure) {
                Log.w(TAG, "config write failed: ${result.exceptionOrNull()}")
                false
            } else {
                applyConfig(cfg)
                true
            }
        }
        if (written) DaemonLink.configChanged()
        return written
    }

    internal fun writeAtomic(body: String): Result<Unit> = runCatching {
        val tmp = File(file.parentFile, "$FILE.tmp")
        tmp.writeText(body)
        if (!tmp.renameTo(file)) {
            // rename failed (unexpected, same filesystem) — surface the
            // error instead of silently writing the target non-atomically
            tmp.delete()
            throw IllegalStateException("atomic rename failed")
        }
    }
}
