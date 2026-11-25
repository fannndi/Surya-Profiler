package com.suryaprofiler.dynamic

import android.content.Context
import android.util.Log
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.protocol.DaemonEvent
import org.json.JSONObject

/**
 * Daemon event -> app state reducer.
 *
 * Responsibility: parse a typed event, update [DynamicProfileState] and ask
 * the service to refresh its notification. No lifecycle, process or tuning
 * logic lives here.
 */
class DaemonEventRouter(
    private val context: Context,
    private val onNotify: (profile: String, reason: String) -> Unit,
) {
    companion object {
        private const val TAG = "SuryaProfiler"
    }

    private val labelCache = java.util.concurrent.ConcurrentHashMap<String, String>()

    fun handle(ev: JSONObject) {
        when (val event = DaemonEvent.parse(ev)) {
            is DaemonEvent.Hello -> {
                Log.d(TAG, "daemon: hello v${event.version} pid=${event.pid}")
                if (event.version != DaemonCommands.PROTOCOL_VERSION) {
                    Log.w(
                        TAG,
                        "daemon protocol ${event.version} != app ${DaemonCommands.PROTOCOL_VERSION}",
                    )
                }
            }

            is DaemonEvent.State -> {
                val st = event.state
                DynamicProfileState.appliedProfile.value = st.active
                val reason = st.reason?.let { displayReason(it, st.srcPkg) }
                DynamicProfileState.reason.value = reason
                onNotify(st.active ?: "stock", reason ?: "base")
            }

            is DaemonEvent.Applied -> {
                val reason = displayReason(event.reason, event.srcPkg)
                if (event.ok) {
                    DynamicProfileState.appliedProfile.value = event.profile
                    DynamicProfileState.reason.value = reason
                    onNotify(event.profile, reason)
                } else {
                    DynamicProfileState.reason.value = "apply failed"
                    onNotify(event.profile, "apply failed — open the app")
                }
                DynamicProfileState.pushApplied(
                    AppliedEvent(
                        seq = 0,
                        profile = event.profile,
                        reason = reason,
                        srcPkg = event.srcPkg,
                        ok = event.ok,
                        applied = event.applied,
                        wrote = event.wrote,
                        verified = event.verified,
                        failed = event.failed,
                        ms = event.ms,
                        settleMs = event.settleMs,
                    ),
                )
            }

            is DaemonEvent.Env -> DynamicProfileState.env.value = event.env

            is DaemonEvent.Power -> DynamicProfileState.onPowerSnapshot(event.snapshot)

            is DaemonEvent.PowerLive ->
                DynamicProfileState.onPowerLive(event.sample, event.bucket)

            is DaemonEvent.Diag -> {
                event.diag?.let {
                    DynamicProfileState.diag.value = it
                    // Seed the log view from the daemon's durable ring when
                    // the in-app relay has nothing yet.
                    DynamicProfileState.seedLogs(it.recentLogs)
                }
                // The 30 s env cadence can be stale when Diagnostics opens;
                // the diag reply carries a fresh sample, so mirror it.
                event.env?.let { DynamicProfileState.env.value = it }
            }

            is DaemonEvent.Stats -> {
                DynamicProfileState.stats.value = event.entries
                DynamicProfileState.heals.value = event.heals
            }

            is DaemonEvent.Restored -> DynamicProfileState.pushRestored(
                RestoredEvent(
                    seq = 0,
                    ok = event.ok,
                    wrote = event.wrote,
                    verified = event.verified,
                    failed = event.failed,
                    affinityOk = event.affinityOk,
                    downscaleOk = event.downscaleOk,
                    displayOk = event.displayOk,
                ),
            )

            is DaemonEvent.Error -> Log.w(TAG, "daemon error: ${event.msg}")
            is DaemonEvent.Pong, is DaemonEvent.Bye ->
                Log.d(TAG, "daemon: ${ev.optString("event")}")
            is DaemonEvent.Decision, is DaemonEvent.Unknown -> Unit
        }
    }

    /** "app" resolves to the app label; everything else passes through. */
    private fun displayReason(raw: String, srcPkg: String?): String = when (raw) {
        "app" -> srcPkg?.let { labelFor(it) } ?: "app"
        else -> raw
    }

    private fun labelFor(pkg: String): String {
        labelCache[pkg]?.let { return it }
        val label = runCatching {
            val pm = context.packageManager
            pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString()
        }.getOrDefault(pkg)
        labelCache[pkg] = label
        if (labelCache.size > 128) labelCache.keys.firstOrNull()?.let { labelCache.remove(it) }
        return label
    }
}
