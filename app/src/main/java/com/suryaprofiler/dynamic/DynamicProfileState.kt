package com.suryaprofiler.dynamic

import com.suryaprofiler.protocol.DiagInfo
import com.suryaprofiler.protocol.EnvSnapshot
import com.suryaprofiler.protocol.HealsInfo
import com.suryaprofiler.protocol.PowerBucket
import com.suryaprofiler.protocol.PowerSample
import com.suryaprofiler.protocol.PowerSnapshot
import com.suryaprofiler.protocol.StatEntry

import kotlinx.coroutines.flow.MutableStateFlow
import java.util.concurrent.atomic.AtomicLong

/** One `applied` event from the daemon (UI waits on seq for dialog flows). */
data class AppliedEvent(
    val seq: Long,
    val profile: String,
    val reason: String,
    val srcPkg: String?,
    val ok: Boolean,
    /** At least one key was written/verified (or already in sync). */
    val applied: Boolean = true,
    val wrote: Int,
    val verified: Int,
    val failed: Int,
    val ms: Long,
    val settleMs: Long,
)

/** One `restored` event from the daemon (service-off stock restore). */
data class RestoredEvent(
    val seq: Long,
    val ok: Boolean,
    val wrote: Int,
    val verified: Int,
    val failed: Int,
    /** Opt-in thread placement restore succeeded (additive, default true). */
    val affinityOk: Boolean = true,
    /** Per-app render-scale restore succeeded. */
    val downscaleOk: Boolean = true,
    /** Global display preset restore succeeded. */
    val displayOk: Boolean = true,
)

/**
 * Process-wide dynamic profile runtime state (daemon writes via the service,
 * UI observes).
 *
 * Responsibility: live status for the UI.
 * Non-goals: persistence (DynamicProfileConfig), decisions (Rust daemon).
 */
object DynamicProfileState {

    /** True while [DynamicProfileService] is alive. */
    val running = MutableStateFlow(false)

    /** True only after the current daemon completes its hello handshake. */
    val daemonConnected = MutableStateFlow(false)

    /**
     * Display text of the current decision:
     * "base" | "screen off" | "multi-window" | app label | "MIUI saver".
     */
    val reason = MutableStateFlow<String?>(null)

    /** Profile the daemon last applied (null until first decision). */
    val appliedProfile = MutableStateFlow<String?>(null)

    // --- diagnostics ---------------------------------------------------------

    /** Latest environment sample (battery / thermal / GPU busy). */
    val env = MutableStateFlow<EnvSnapshot?>(null)

    /** Latest `diag` reply (null until requested). */
    val diag = MutableStateFlow<DiagInfo?>(null)

    /** Transition history (oldest first; the screen renders it reversed). */
    val stats = MutableStateFlow<List<StatEntry>>(emptyList())

    /** Auto-revive watchdog totals (from the same `stats` reply). */
    val heals = MutableStateFlow(HealsInfo())

    /** Latest `power` reply (history window + peaks). */
    val powerSnapshot = MutableStateFlow<PowerSnapshot?>(null)

    /** Latest live power sample while the Power screen is open. */
    val powerLive = MutableStateFlow<PowerSample?>(null)

    /** In-progress buckets received live since the last snapshot. */
    val powerLiveBuckets = MutableStateFlow<List<PowerBucket>>(emptyList())

    /**
     * Replaces the snapshot; live buckets restart at the daemon's current
     * partial bucket (completed history is already in the snapshot).
     */
    fun onPowerSnapshot(snap: PowerSnapshot) {
        if (!snap.isValid) return
        powerSnapshot.value = snap
        powerLive.value = snap.live
        powerLiveBuckets.value = listOfNotNull(snap.currentBucket)
    }

    /** Service/daemon lifecycle hook; a disconnect invalidates live power data. */
    fun setDaemonConnected(connected: Boolean) {
        daemonConnected.value = connected
        if (!connected) resetPower()
    }

    /** Merges one live bucket by segment (or minute for legacy rows). */
    fun onPowerLive(sample: PowerSample, bucket: PowerBucket?) {
        powerLive.value = sample
        if (bucket == null) return
        val list = powerLiveBuckets.value
        val i = list.indexOfLast { previous ->
            when {
                previous.segmentId != null && bucket.segmentId != null ->
                    previous.segmentId == bucket.segmentId
                else -> previous.segmentId == null && bucket.segmentId == null && previous.tMs == bucket.tMs
            }
        }
        powerLiveBuckets.value = if (i >= 0) {
            // Live rows are cumulative for one segment: replace, never add durations.
            list.toMutableList().also { it[i] = bucket }
        } else {
            (list + bucket).takeLast(2880)
        }
    }

    /** Stop showing a live sample after watch=false (Home row must not stale). */
    fun clearPowerLive() {
        powerLive.value = null
    }

    /** Local reset for Service OFF / Clear before the next daemon reply. */
    fun resetPower() {
        powerSnapshot.value = null
        powerLive.value = null
        powerLiveBuckets.value = emptyList()
    }

    /** In-app relay of the daemon's stderr log (capped, timestamped). */
    val logs = MutableStateFlow<List<String>>(emptyList())

    private val logTime = ThreadLocal.withInitial {
        java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.US)
    }

    fun pushLog(line: String) {
        val time = logTime.get()!!.format(java.util.Date())
        logs.value = (logs.value + "$time  $line").takeLast(200)
    }

    fun clearLogs() {
        logs.value = emptyList()
    }

    /**
     * Seeds the log view from the daemon's durable ring (`diag.recent_logs`)
     * so the view is never empty just because the stderr relay started late.
     * A non-empty live capture always wins (single time source).
     */
    fun seedLogs(lines: List<String>) {
        if (lines.isEmpty() || logs.value.isNotEmpty()) return
        logs.value = lines.takeLast(200)
    }

    /** Latest `applied` event (each event is a new instance). */
    val appliedEvents = MutableStateFlow<AppliedEvent?>(null)
    private val appliedSeq = AtomicLong(0)

    /** Latest `restored` event (each event is a new instance). */
    val restoredEvents = MutableStateFlow<RestoredEvent?>(null)
    private val restoredSeq = AtomicLong(0)

    fun pushApplied(ev: AppliedEvent) {
        appliedEvents.value = ev.copy(seq = appliedSeq.incrementAndGet())
    }

    fun pushRestored(ev: RestoredEvent) {
        restoredEvents.value = ev.copy(seq = restoredSeq.incrementAndGet())
    }

    fun currentAppliedSeq(): Long = appliedSeq.get()

    fun currentRestoredSeq(): Long = restoredSeq.get()
}
