package com.suryaprofiler.ui

import android.app.Application
import android.content.Intent
import android.content.pm.ApplicationInfo
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.suryaprofiler.dynamic.DynamicProfileConfig
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** One launchable app for the per-app mapping list. */
data class AppEntry(
    val pkg: String,
    val label: String,
    val isGame: Boolean,
)

data class DynamicProfileUiState(
    val apps: List<AppEntry> = emptyList(),
    val loading: Boolean = true,
    val query: String = "",
)

/**
 * Controller for the Apps Profile screen: installed-app list, per-app
 * mapping and root status. Kept separate from [HomeViewModel] so the
 * (heavy) icon loading happens only when the user opens the apps screen.
 */
class DynamicProfileViewModel(app: Application) : AndroidViewModel(app) {

    private val config = DynamicProfileConfig.get(app)

    val appMap: StateFlow<Map<String, String>> = config.appMapFlow
    val appDownscale: StateFlow<Map<String, String>> = config.appDownscaleFlow

    private val _state = MutableStateFlow(DynamicProfileUiState())
    val state: StateFlow<DynamicProfileUiState> = _state.asStateFlow()

    init {
        loadApps()
    }

    fun setQuery(q: String) = _state.update { it.copy(query = q) }

    /** null profileId removes the mapping. */
    fun setAppProfile(pkg: String, profileId: String?) {
        viewModelScope.launch(Dispatchers.IO) {
            config.setAppProfile(pkg, profileId)
        }
    }

    /** null/blank clears the per-app render scale. */
    fun setAppDownscale(pkg: String, ratio: String?) {
        viewModelScope.launch(Dispatchers.IO) {
            config.setAppDownscale(pkg, ratio)
        }
    }


    fun filteredApps(): List<AppEntry> {
        val s = _state.value
        return s.apps
            .filter { s.query.isBlank() || it.label.contains(s.query, true) || it.pkg.contains(s.query, true) }
    }

    private fun loadApps() {
        viewModelScope.launch {
            _state.update { it.copy(loading = true) }
            val apps = withContext(Dispatchers.IO) {
                val app = getApplication<Application>()
                val pm = app.packageManager
                val seen = HashSet<String>()
                val entries = ArrayList<AppEntry>()
                fun add(ai: ApplicationInfo) {
                    val pkg = ai.packageName
                    if (pkg == app.packageName || !seen.add(pkg)) return
                    entries += AppEntry(
                        pkg = pkg,
                        label = runCatching { pm.getApplicationLabel(ai).toString() }
                            .getOrDefault(pkg),
                        isGame = ai.category == ApplicationInfo.CATEGORY_GAME,
                    )
                }
                // Home packages first: the launcher is a system app whose
                // activity carries CATEGORY_HOME (not LAUNCHER), so it must be
                // merged explicitly to be configurable.
                val home = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME)
                pm.queryIntentActivities(home, 0).forEach { ri ->
                    ri.activityInfo?.applicationInfo?.let(::add)
                }
                // Regular launcher entries; system apps stay hidden (their
                // settings are MIUI's), except ones already merged above.
                val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
                pm.queryIntentActivities(intent, 0).forEach { ri ->
                    val ai = ri.activityInfo?.applicationInfo ?: return@forEach
                    val isSystem = (ai.flags and ApplicationInfo.FLAG_SYSTEM) != 0
                    if (!isSystem) add(ai)
                }
                entries.sortedBy { it.label.lowercase() }
            }
            _state.update { it.copy(apps = apps, loading = false) }
        }
    }
}
