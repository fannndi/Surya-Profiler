package com.suryaprofiler.core

import org.json.JSONArray
import org.json.JSONObject

/**
 * Parsed shapes of the `surya-prof` JSON protocol.
 *
 * Responsibility: faithful org.json parsing (every optional field tolerated).
 * Non-goals: IO, state, UI.
 */

data class DeviceInfo(
    val device: String,
    val model: String,
    val rom: String,
    val socId: String,
    val kernel: String,
)

data class FrameworkEvidence(
    val miThermald: String? = null,
    val perfHal: String? = null,
    val perfservice: String? = null,
    val thermalSconfig: String? = null,
    val inputBoost: String? = null,
    val schedBoost: String? = null,
    val gameCpuset: String? = null,
)

data class CatalogInfo(
    val total: Int,
    val free: Int,
    val baseline: Int,
    val present: Int,
)

data class SnapshotInfo(
    val created: Long,
    val keys: Int,
)

data class Status(
    val active: String?,
    val updated: Long,
    val snapshot: SnapshotInfo?,
    val profiles: List<String>,
    val device: DeviceInfo,
    val framework: FrameworkEvidence,
    val root: Boolean,
    val catalog: CatalogInfo,
)

data class OpStatus(val kind: String, val reason: String?) {
    val isLocked: Boolean get() = kind == "locked"
    val isUnchanged: Boolean get() = kind == "unchanged"
}

data class Op(
    val key: String,
    val path: String,
    val tier: String,
    val wanted: String,
    val resolved: String,
    val current: String?,
    val status: OpStatus,
)

data class Plan(
    val profileId: String,
    val label: String,
    val ok: Boolean,
    val errors: List<String>,
    val ops: List<Op>,
) {
    val locked: List<Op> get() = ops.filter { it.status.isLocked }
    val pending: Int get() = ops.count { it.status.kind == "ok" }
    val inSync: Int get() = ops.count { it.status.isUnchanged }
}

data class LockedKey(val key: String, val reason: String)

data class WriteResult(
    val key: String,
    val resolved: String,
    val error: String?,
    val written: Boolean,
    val verified: Boolean,
)

data class ApplyReport(
    val mode: String,
    val profile: String?,
    val wrote: Int,
    val unchanged: Int,
    val verified: Int,
    val failed: Int,
    val locked: List<LockedKey>,
    val results: List<WriteResult>,
    val errors: List<String> = emptyList(),
    val ok: Boolean,
    val snapshotCreated: Boolean,
    val active: String?,
)

data class Profile(
    val id: String,
    val label: String,
    val desc: String,
    /** One-line row subtitle from profiles.json (single source of truth). */
    val short: String = "",
    /** Hidden from the home cards (e.g. the screen-off "sleep" profile). */
    val hidden: Boolean = false,
    val params: Map<String, String>,
)

data class ProfilesFile(
    val profiles: List<Profile>,
)

private fun JSONObject.optNullableString(key: String): String? =
    if (isNull(key)) null else optString(key).ifEmpty { null }

fun parseStatus(json: JSONObject): Status {
    val device = json.optJSONObject("device") ?: JSONObject()
    val fw = json.optJSONObject("framework") ?: JSONObject()
    val catalog = json.optJSONObject("catalog") ?: JSONObject()
    val snapshot = json.optJSONObject("snapshot")
    return Status(
        active = json.optNullableString("active"),
        updated = json.optLong("updated"),
        snapshot = snapshot?.let { SnapshotInfo(it.optLong("created"), it.optInt("keys")) },
        profiles = json.optJSONArray("profiles")?.toStringList() ?: emptyList(),
        device = DeviceInfo(
            device = device.optString("device", "?"),
            model = device.optString("model", "?"),
            rom = device.optString("rom", "?"),
            socId = device.optString("soc_id", "?"),
            kernel = device.optString("kernel", "?"),
        ),
        framework = FrameworkEvidence(
            miThermald = fw.optNullableString("mi_thermald"),
            perfHal = fw.optNullableString("perf_hal"),
            perfservice = fw.optNullableString("perfservice"),
            thermalSconfig = fw.optNullableString("thermal_sconfig"),
            inputBoost = fw.optNullableString("input_boost"),
            schedBoost = fw.optNullableString("sched_boost"),
            gameCpuset = fw.optNullableString("game_cpuset"),
        ),
        root = json.optBoolean("root"),
        catalog = CatalogInfo(
            total = catalog.optInt("total"),
            free = catalog.optInt("free"),
            baseline = catalog.optInt("baseline"),
            present = catalog.optInt("present"),
        ),
    )
}

fun JSONArray.toStringList(): List<String> =
    (0 until length()).map { optString(it) }

private fun parseOpStatus(raw: Any): OpStatus = when (raw) {
    is String -> OpStatus(raw, null)
    is JSONObject -> {
        // Rust serializes a one-key object (`{"locked":"reason"}`), but do
        // not rely on JSON key order: prefer the known status keys first.
        val kind = listOf("ok", "unchanged", "locked")
            .firstOrNull { raw.has(it) }
            ?: raw.keys().nextOrNull()
            ?: "unknown"
        OpStatus(kind, raw.optNullableString(kind))
    }
    else -> OpStatus("unknown", null)
}

private fun Iterator<String>.nextOrNull(): String? = if (hasNext()) next() else null

fun parseOp(json: JSONObject): Op = Op(
    key = json.optString("key"),
    path = json.optString("path"),
    tier = json.optString("tier"),
    wanted = json.optString("wanted"),
    resolved = json.optString("resolved"),
    current = if (json.has("current") && !json.isNull("current")) json.optString("current") else null,
    status = parseOpStatus(json.opt("status") ?: "unknown"),
)

fun parsePlan(json: JSONObject): Plan = Plan(
    profileId = json.optString("profile_id"),
    label = json.optString("label"),
    ok = json.optBoolean("ok"),
    errors = json.optJSONArray("errors")?.toStringList() ?: emptyList(),
    ops = json.optJSONArray("ops")?.let { arr ->
        (0 until arr.length()).mapNotNull { i ->
            arr.optJSONObject(i)?.let(::parseOp)
        }
    } ?: emptyList(),
)

fun parseReport(json: JSONObject): ApplyReport = ApplyReport(
    mode = json.optString("mode"),
    profile = json.optNullableString("profile"),
    wrote = json.optInt("wrote"),
    unchanged = json.optInt("unchanged"),
    verified = json.optInt("verified"),
    failed = json.optInt("failed"),
    locked = json.optJSONArray("locked")?.let { arr ->
        (0 until arr.length()).mapNotNull { i ->
            arr.optJSONObject(i)?.let {
                LockedKey(it.optString("key"), it.optString("reason"))
            }
        }
    } ?: emptyList(),
    results = json.optJSONArray("results")?.let { arr ->
        (0 until arr.length()).mapNotNull { i ->
            arr.optJSONObject(i)?.let { o ->
                WriteResult(
                    key = o.optString("key"),
                    resolved = o.optString("resolved"),
                    error = if (o.has("error")) o.optString("error") else null,
                    written = o.optBoolean("written"),
                    verified = o.optBoolean("verified"),
                )
            }
        }
    } ?: emptyList(),
    errors = json.optJSONArray("errors")?.toStringList() ?: emptyList(),
    ok = json.optBoolean("ok"),
    snapshotCreated = json.optBoolean("snapshot_created"),
    active = json.optNullableString("active"),
)

fun parseProfiles(json: JSONObject): ProfilesFile {
    val arr = json.optJSONArray("profiles") ?: JSONArray()
    return ProfilesFile((0 until arr.length()).mapNotNull { i ->
        val p = arr.optJSONObject(i) ?: return@mapNotNull null
        val paramsArr = p.optJSONObject("params") ?: JSONObject()
        val params = paramsArr.keys().asSequence()
            .mapNotNull { k -> paramsArr.optString(k).takeIf { it.isNotEmpty() }?.let { k to it } }
            .toMap()
        Profile(
            id = p.optString("id"),
            label = p.optString("label"),
            desc = p.optString("desc", ""),
            short = p.optString("short", ""),
            hidden = p.optBoolean("hidden", false),
            params = params,
        )
    })
}
