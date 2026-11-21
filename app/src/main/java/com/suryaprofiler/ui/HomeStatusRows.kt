package com.suryaprofiler.ui

import com.suryaprofiler.core.ProfileLabels
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Apps
import androidx.compose.material.icons.filled.AutoMode
import androidx.compose.material.icons.filled.MonitorHeart
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle


// --- profile row -------------------------------------------------------------

@Composable
internal fun AppsRow(mappedCount: Int, onClick: () -> Unit) {
    ElevatedCard(Modifier.fillMaxWidth()) {
        Row(
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconCircle(Icons.Default.Apps)
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text("Apps Profile", style = MaterialTheme.typography.titleMedium)
                Text(
                    if (mappedCount > 0) "Per-app rules · $mappedCount apps"
                    else "Per-app rules",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Icon(
                Icons.AutoMirrored.Filled.KeyboardArrowRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
internal fun ServiceRow(state: HomeUiState, onToggle: (Boolean) -> Unit) {
    // The daemon event is fresher than the CLI status probe after an
    // automatic app switch; fall back to the probe when the service is off.
    val live by com.suryaprofiler.dynamic.DynamicProfileState.appliedProfile
        .collectAsStateWithLifecycle()
    val active = if (state.serviceRunning) live ?: state.status?.active else state.status?.active
    val statusText = when {
        state.serviceRunning && active != null -> {
            val reason = state.serviceReason
            val suffix = if (reason.isNullOrEmpty() || reason == "base") null else reason
            "Active · ${ProfileLabels.of(active)}" + (suffix?.let { " · $it" } ?: "")
        }
        state.serviceRunning -> "On · idle"
        state.serviceEnabled -> "Starting…"
        else -> "Off · stock"
    }
    ElevatedCard(Modifier.fillMaxWidth()) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconCircle(Icons.Default.PowerSettingsNew)
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text("Service", style = MaterialTheme.typography.titleMedium)
                Text(
                    statusText,
                    style = MaterialTheme.typography.labelSmall,
                    color = if (state.serviceRunning) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(
                checked = state.serviceEnabled,
                onCheckedChange = onToggle,
                enabled = state.busy == null,
            )
        }
    }
}

/**
 * Dynamic Profile: the auto-switch switch. OFF keeps the universal profile
 * no matter which mapped app is opened; sleep / multi-window / MIUI-saver
 * rules still apply. Requires the Service to be on.
 */
@Composable
internal fun DynamicProfileRow(state: HomeUiState, onToggle: (Boolean) -> Unit) {
    val serviceOn = state.serviceEnabled
    val statusText = when {
        !serviceOn -> "Turn on Service first"
        state.dynamicEnabled -> "App profiles switch automatically"
        else -> "Universal profile only"
    }
    ElevatedCard(Modifier.fillMaxWidth()) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconCircle(Icons.Default.AutoMode)
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text("Dynamic Profile", style = MaterialTheme.typography.titleMedium)
                Text(
                    statusText,
                    style = MaterialTheme.typography.labelSmall,
                    color = if (serviceOn && state.dynamicEnabled) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(
                checked = state.dynamicEnabled,
                onCheckedChange = onToggle,
                enabled = serviceOn,
            )
        }
    }
}

// --- diagnostics entry ---------------------------------------------------------

@Composable
internal fun DiagnosticsRow(onClick: () -> Unit) {
    // Collect inside the row so a 30 s env sample does not recompose the
    // whole Home screen.
    val env by com.suryaprofiler.dynamic.DynamicProfileState.env
        .collectAsStateWithLifecycle()
    val sample = env
    val subtitle = if (sample != null) {
        listOfNotNull(
            sample.batteryPct?.let { "$it%" },
            sample.cpuTempC?.let { String.format(java.util.Locale.US, "%.1f °C", it) },
            sample.thermalScenario
                ?.takeUnless { it.startsWith("custom-") }
                ?.let { "MIUI $it" }
                ?: sample.thermalMode?.let { "thermal $it" },
            sample.gpuBusyPct?.let { "GPU $it%" },
        ).joinToString(" · ").ifEmpty { "Daemon · battery · thermal · log" }
    } else {
        "Daemon · battery · thermal · log"
    }
    ElevatedCard(Modifier.fillMaxWidth()) {
        Row(
            Modifier
                .fillMaxWidth()
                .clickable(onClick = onClick)
                .padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconCircle(Icons.Default.MonitorHeart)
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text("Diagnostics", style = MaterialTheme.typography.titleMedium)
                Text(
                    subtitle,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Icon(
                Icons.AutoMirrored.Filled.KeyboardArrowRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

// --- dialogs -----------------------------------------------------------------

