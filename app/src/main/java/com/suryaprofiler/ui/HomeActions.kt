package com.suryaprofiler.ui

import androidx.lifecycle.viewModelScope
import com.suryaprofiler.core.ApplyReport
import com.suryaprofiler.core.Profile
import com.suryaprofiler.core.Tuner
import com.suryaprofiler.dynamic.DaemonLink
import com.suryaprofiler.dynamic.DynamicProfileService
import com.suryaprofiler.dynamic.ServiceOffCoordinator
import com.suryaprofiler.dynamic.StockRestore
import com.suryaprofiler.dynamic.DynamicProfileState
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.protocol.ProfileId
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/** Manual tap: apply now and remember it as the universal base. */
internal fun HomeViewModel.apply(profileId: String) {
    if (_state.value.busy != null) return
    _state.update { it.copy(busy = profileId, error = null) }
    viewModelScope.launch(Dispatchers.IO) {
        if (DaemonLink.connected()) applyViaDaemon(profileId) else applyViaCli(profileId)
    }
}

/**
 * Service on: the daemon is the single writer — write the base, ask it
 * to apply now, wait for the applied event (it carries real counters).
 */
internal suspend fun HomeViewModel.applyViaDaemon(profileId: String) {
    // capture the seq BEFORE writing config: the config_changed hint can
    // make the daemon re-evaluate and emit an applied event that bumps
    // the seq before set_base is sent.
    val seq = DynamicProfileState.currentAppliedSeq()
    config.baseProfile = profileId
    if (config.baseProfile != profileId) {
        _state.update { it.copy(busy = null, error = "Could not persist the base profile") }
        return
    }
    val sent = DaemonLink.send(DaemonCommands.setBase(ProfileId.require(profileId)))
    val ev = if (sent) {
        withTimeoutOrNull(15_000) {
            DynamicProfileState.appliedEvents
                .filterNotNull()
                .first { it.seq > seq && it.profile == profileId && it.reason == "base" }
        }
    } else {
        null
    }
    val status = runCatching { Tuner.status() }.getOrNull()
    val cards = refreshPlans()
    val report = ev?.let {
        ApplyReport(
            mode = "apply",
            profile = it.profile,
            wrote = it.wrote,
            unchanged = 0,
            verified = it.verified,
            failed = it.failed,
            locked = emptyList(),
            results = emptyList(),
            ok = it.ok,
            snapshotCreated = false,
            active = if (it.ok) it.profile else null,
        )
    }
    _state.update {
        it.copy(
            busy = null,
            report = report,
            status = status,
            cards = cards,
            error = if (ev == null) "Daemon did not confirm the apply" else null,
        )
    }
}

/** Service off: direct CLI apply (no daemon available). */
internal suspend fun HomeViewModel.applyViaCli(profileId: String) {
    runCatching {
        val report = Tuner.apply(profileId)
        report to Tuner.status()
    }.onSuccess { (report, status) ->
        if (report.ok) config.baseProfile = profileId
        // mirror into the shared dynamic profile state so the Service row
        // shows the manual decision with a fresh reason (the service
        // will overwrite both on its next own decision)
        if (report.ok) {
            DynamicProfileState.appliedProfile.value = profileId
            DynamicProfileState.reason.value = null
        }
        val cards = refreshPlans()
        _state.update {
            it.copy(busy = null, report = report, status = status, cards = cards)
        }
    }.onFailure { e ->
        _state.update {
            it.copy(busy = null, error = "Apply failed: ${e.message ?: e}")
        }
    }
}

/** Switch toggle: ON starts the service; OFF restores stock + stops. */
internal fun HomeViewModel.onServiceToggle(v: Boolean) {
    if (v) {
        setServiceEnabled(true)
    } else if (_state.value.status?.snapshot != null) {
        _state.update { it.copy(confirmServiceOff = true) }
    } else {
        setServiceEnabled(false)
    }
}

/**
 * Dynamic Profile toggle: pure config write. The running service
 * observes the flow and re-evaluates instantly; while the service is
 * off nothing happens (the value is picked up at next start).
 */
internal fun HomeViewModel.onDynamicToggle(v: Boolean) {
    viewModelScope.launch(Dispatchers.IO) { config.dynamicEnabled = v }
}

/** Thermal takeover is a pure config write; the daemon applies it. */
internal fun HomeViewModel.onCustomThermal(v: Boolean) {
    viewModelScope.launch(Dispatchers.IO) { config.customThermal = v }
}

/** R&D switch: daemon-owned, auto-released on Service OFF. */
internal fun HomeViewModel.onRndInputSuspend(v: Boolean) {
    if (!_state.value.serviceRunning) return
    viewModelScope.launch(Dispatchers.IO) { config.rndInputSuspend = v }
}

internal fun HomeViewModel.confirmServiceOff() {
    _state.update { it.copy(confirmServiceOff = false) }
    setServiceEnabled(false)
}

internal fun HomeViewModel.cancelServiceOff() {
    _state.update { it.copy(confirmServiceOff = false) }
}

internal fun HomeViewModel.setServiceEnabled(v: Boolean) {
    if (v) {
        // ON during an in-flight OFF cancels it via the generation check.
        serviceGeneration++
        config.enabled = true
        if (!config.enabled) {
            _state.update { it.copy(error = "Could not persist enabled=true; retry") }
            return
        }
        if (!DynamicProfileService.start(getApplication())) {
            config.enabled = false
            _state.update { it.copy(error = "Failed to start service: start rejected") }
        }
        return
    }
    if (_state.value.busy != null) return
    // OFF = true stock restore: release holds + write stock.json back,
    // verify, then stop. `restore_pending` is persisted first so a process
    // death mid-restore is retried on the next launch. A failed restore
    // keeps the service ON.
    viewModelScope.launch(Dispatchers.IO) {
        val generation = ++serviceGeneration
        _state.update { it.copy(busy = "service-off", error = null) }
        val coordinator = ServiceOffCoordinator(
            begin = { config.beginServiceOff() },
            abort = { config.abortServiceOff() },
            confirm = { config.confirmServiceOff() },
            clearSessionFlag = { if (config.rndInputSuspend) config.rndInputSuspend = false },
            restore = { StockRestore.restore() },
        )
        when (val outcome = coordinator.run { generation == serviceGeneration }) {
            is ServiceOffCoordinator.Outcome.Success -> {
                runCatching { DynamicProfileService.stop(getApplication()) }
                val status = runCatching { Tuner.status() }.getOrNull()
                val cards = refreshPlans()
                _state.update {
                    it.copy(
                        busy = null,
                        report = outcome.report,
                        status = status,
                        cards = cards,
                        serviceReason = null,
                        error = null,
                    )
                }
            }
            ServiceOffCoordinator.Outcome.Cancelled -> {
                runCatching { DynamicProfileService.start(getApplication()) }
                _state.update { it.copy(busy = null, error = null) }
            }
            ServiceOffCoordinator.Outcome.Refused -> {
                _state.update {
                    it.copy(
                        busy = null,
                        error = "Service OFF refused: could not persist restore_pending",
                    )
                }
            }
            is ServiceOffCoordinator.Outcome.RestoreFailed -> {
                runCatching { DynamicProfileService.start(getApplication()) }
                _state.update {
                    it.copy(
                        busy = null,
                        error = "Service off failed: stock restore was not confirmed. " +
                            "The service is still on; retry or run `surya-prof restore`.",
                    )
                }
            }
            is ServiceOffCoordinator.Outcome.ConfirmFailed -> {
                config.abortServiceOff()
                _state.update {
                    it.copy(
                        busy = null,
                        error = "Stock was restored, but the OFF marker could not be " +
                            "cleared; retry before closing the app.",
                    )
                }
            }
        }
    }
}

/** Bring the service back if it is enabled but gone (update, MIUI kill). */
internal fun HomeViewModel.maybeStartService() {
    if (config.enabled && !DynamicProfileState.running.value) {
        if (!DynamicProfileService.start(getApplication())) {
            android.util.Log.w("SuryaProfiler", "service start rejected")
        }
    }
}
