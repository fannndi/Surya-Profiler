package com.suryaprofiler.ui

import android.os.PowerManager
import android.provider.Settings
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import com.suryaprofiler.core.Status

/**
 * Settings: permissions/background checklist plus the R&D input-suspend
 * switch. Turning the service off (on Home/tile) restores stock; there is no
 * backup/guards/charging/bridge configuration anymore.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    status: Status?,
    serviceRunning: Boolean,
    customThermal: Boolean,
    onCustomThermal: (Boolean) -> Unit,
    rndInputSuspend: Boolean,
    onRndInputSuspend: (Boolean) -> Unit,
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val version = remember {
        runCatching {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName
        }.getOrNull() ?: "?"
    }
    var ignoringBattery by remember { mutableStateOf<Boolean?>(null) }
    fun refreshBattery() {
        ignoringBattery = runCatching {
            context.getSystemService(PowerManager::class.java)
                ?.isIgnoringBatteryOptimizations(context.packageName)
        }.getOrNull()
    }
    LaunchedEffect(Unit) { refreshBattery() }
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { refreshBattery() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            ElevatedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text("Background", style = MaterialTheme.typography.titleMedium)
                    SetupRow(
                        title = "Root",
                        subtitle = "Required — tuning goes through su",
                        ok = status?.root,
                    )
                    SetupRow(
                        title = "MIUI Autostart",
                        subtitle = "Keeps the service alive",
                        ok = null,
                        actionLabel = "Open",
                        onAction = { openMiuiAutostart(context) },
                    )
                    SetupRow(
                        title = "Battery saver: no restrictions",
                        subtitle = if (ignoringBattery == true) {
                            "Exempt from Doze/App Standby"
                        } else {
                            "App info → Battery saver → No restrictions"
                        },
                        ok = ignoringBattery,
                        actionLabel = if (ignoringBattery == false) "Allow" else null,
                        onAction = if (ignoringBattery == false) {
                            { requestBatteryExemption(context) }
                        } else {
                            null
                        },
                    )
                }
            }

            ElevatedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Advanced (R&D)", style = MaterialTheme.typography.titleMedium)
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text("Custom thermal profiles", style = MaterialTheme.typography.bodyMedium)
                            Text(
                                "Follow the active Device Profile (balance, game, power save, " +
                                    "sleep). Off = stock MIUI thermal. Service OFF removes the " +
                                    "custom config and restores stock.",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(
                            checked = customThermal,
                            onCheckedChange = onCustomThermal,
                        )
                    }
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text("Suspend charger input", style = MaterialTheme.typography.bodyMedium)
                            Text(
                                "Writes input_suspend=1 while the service is on. The " +
                                    "device runs on battery and charging resumes on Service OFF. " +
                                    "For experiments only.",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(
                            checked = rndInputSuspend,
                            onCheckedChange = onRndInputSuspend,
                            enabled = serviceRunning,
                        )
                    }
                }
            }

            ElevatedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Diagnostics", style = MaterialTheme.typography.titleMedium)
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        FrameworkDot("thermal", status?.framework?.miThermald)
                        FrameworkDot("perf", status?.framework?.perfHal)
                        FrameworkDot("root", if (status?.root == true) "granted" else null)
                    }
                    status?.let { st ->
                        Text(
                            "${st.catalog.total} nodes · ${st.catalog.free} free · " +
                                "${st.catalog.present} present",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Text(
                        "SuryaProfiler $version · ${status?.device?.device ?: "surya"}",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            Spacer(Modifier.size(16.dp))
        }
    }
}

