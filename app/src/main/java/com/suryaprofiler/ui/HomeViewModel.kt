package com.suryaprofiler.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.suryaprofiler.core.Profile
import com.suryaprofiler.core.RootBridge
import com.suryaprofiler.core.Tuner
import com.suryaprofiler.core.parseProfiles
import com.suryaprofiler.dynamic.DaemonLink
import com.suryaprofiler.dynamic.DynamicProfileConfig
import com.suryaprofiler.dynamic.DynamicProfileState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

/**
 * UI state machine: boot-time deploy + probe, profile rows with live plans,
 * apply/service-off with report dialogs, service status mirror.
 *
 * Responsibility: orchestrating [Tuner] calls and exposing [HomeUiState].
 * Non-goals: tuning logic (Rust core), service decisions (Rust daemon).
 */
class HomeViewModel(app: Application) : AndroidViewModel(app) {

    internal val config = DynamicProfileConfig.get(app)
    private var serviceOffCleaned = false
    /** Cancels an in-flight Service OFF when the user turns it back on. */
    internal var serviceGeneration = 0

    internal val _state = MutableStateFlow(HomeUiState())
    val state: StateFlow<HomeUiState> = _state.asStateFlow()

    init {
        observeService()
        refresh()
    }

    private fun observeService() {
        viewModelScope.launch {
            kotlinx.coroutines.flow.combine(
                config.enabledFlow,
                config.dynamicEnabledFlow,
                config.customThermalFlow,
                config.rndInputSuspendFlow,
                config.appMapFlow,
            ) { enabled, dynamic, thermal, rnd, map ->
                HomeUiState(
                    serviceEnabled = enabled,
                    dynamicEnabled = dynamic,
                    customThermal = thermal,
                    rndInputSuspend = rnd,
                    mappedCount = map.size,
                )
            }.collect { patch ->
                _state.update { current ->
                    current.copy(
                        serviceEnabled = patch.serviceEnabled,
                        dynamicEnabled = patch.dynamicEnabled,
                        customThermal = patch.customThermal,
                        rndInputSuspend = patch.rndInputSuspend,
                        mappedCount = patch.mappedCount,
                    )
                }
            }
        }
        // combine() caps at 5 flows: these toggles get their own collectors.
        viewModelScope.launch {
            config.freqFloorAuditFlow.collect { v ->
                _state.update { it.copy(freqFloorAudit = v) }
            }
        }
        viewModelScope.launch {
            config.threadAffinityFlow.collect { v ->
                _state.update { it.copy(threadAffinity = v) }
            }
        }
        viewModelScope.launch {
            config.displayScaleFlow.collect { v ->
                _state.update { it.copy(displayScale = v) }
            }
        }
        viewModelScope.launch {
            DynamicProfileState.running.collect { v -> _state.update { it.copy(serviceRunning = v) } }
        }
        viewModelScope.launch {
            DynamicProfileState.reason.collect { v -> _state.update { it.copy(serviceReason = v) } }
        }
        viewModelScope.launch {
            DynamicProfileState.appliedProfile.collect { id ->
                // the service switched profile in the background while this
                // screen is open -> keep status + plans fresh
                val cur = _state.value.status?.active
                if (id != null && id != cur && _state.value.busy == null) {
                    // Plans are profile-derived, not profile-state-derived;
                    // refreshing them here cost 3 extra `su` calls per app
                    // switch. Only the live status needs updating.
                    runCatching { Tuner.status() }.onSuccess { st ->
                        _state.update { it.copy(status = st) }
                    }
                }
            }
        }
    }

    /** Quiet status refresh (on resume) — no deploy, no plans, no spinner. */
    fun refreshLight() {
        if (_state.value.loading || _state.value.busy != null) return
        maybeStartService()
        viewModelScope.launch(Dispatchers.IO) {
            runCatching { Tuner.status() }
                .onSuccess { st -> _state.update { it.copy(status = st) } }
        }
    }

    fun refresh() {
        viewModelScope.launch {
            _state.update { it.copy(loading = true, error = null) }
            val deployError = Tuner.deploy(getApplication())
            if (deployError != null) {
                _state.update { it.copy(loading = false, error = deployError) }
                android.util.Log.w("SuryaProfiler", "refresh: abort on deploy error")
                return@launch
            }
            // A crashed Service OFF must be finished before anything else:
            // `restore_pending` is written before `enabled=false`, so a
            // process death mid-restore is retried here.
            if (!config.enabled && !DynamicProfileState.running.value) {
                if (config.restorePending) {
                    val report = runCatching { Tuner.restore() }.getOrNull()
                    if (report?.ok == true && config.confirmServiceOff()) {
                        // OFF confirmed; maybeStartService() will stay off.
                    } else {
                        config.abortServiceOff()
                        _state.update {
                            it.copy(
                                error = "Service off was interrupted. Restore was not " +
                                    "confirmed; the service is ON again — retry when ready.",
                            )
                        }
                    }
                } else if (!serviceOffCleaned) {
                    serviceOffCleaned = true
                    runCatching { Tuner.clean() }
                }
            }

            // A profile pack update must reach an ALREADY-RUNNING daemon:
            // the hint re-evaluates and the engine re-reads profiles.json
            // (without it, a new catalog key would wait for the next switch).
            DaemonLink.configChanged()
            // start the service while the app is still foreground
            // (background FGS start would be rejected by the OS)
            maybeStartService()
            loadAll()
        }
    }

    private suspend fun loadAll() = withContext(Dispatchers.IO) {
        runCatching {
            val status = Tuner.status()
            val profiles = loadBundledProfiles().filter { !it.hidden }
            // Sequential: APatch su serializes anyway; three concurrent su
            // children can leave output pipes open and stall the UI.
            val cards = profiles.map { planCard(it) }
            _state.update {
                it.copy(
                    loading = false,
                    status = status,
                    cards = cards,
                    error = null,
                )
            }
        }.onFailure { e ->
            android.util.Log.w("SuryaProfiler", "loadAll failed: $e")
            _state.update {
                it.copy(
                    loading = false,
                    error = "Probe failed: ${e.message ?: e}" +
                        if (!Tuner.bridge.isRoot()) " (root not granted?)" else "",
                )
            }
        }
    }

    private fun loadBundledProfiles(): List<Profile> {
        val json = getApplication<Application>().assets
            .open(RootBridge.ASSET_PROFILES)
            .bufferedReader().use { it.readText() }
        return parseProfiles(JSONObject(json)).profiles
    }

    private suspend fun planCard(p: Profile): ProfileCard {
        val res = runCatching { Tuner.plan(p.id) }
        return ProfileCard(
            profile = p,
            plan = res.getOrNull(),
            planError = res.exceptionOrNull()?.message,
        )
    }

    internal suspend fun refreshPlans(): List<ProfileCard> =
        _state.value.cards.map { planCard(it.profile) }

    fun dismissReport() = _state.update { it.copy(report = null) }

    fun clearError() = _state.update { it.copy(error = null) }
}
