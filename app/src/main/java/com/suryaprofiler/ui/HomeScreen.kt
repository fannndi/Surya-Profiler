package com.suryaprofiler.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Apps
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel

/**
 * Home: compact profile rows (tap = apply + base), the Apps Profile entry and
 * the Service switch. Everything else lives in Settings.
 */

private enum class HomeDest { HOME, APPS, SETTINGS, DIAGNOSTICS }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(vm: HomeViewModel) {
    val state by vm.state.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }
    var dest by rememberSaveable { mutableStateOf(HomeDest.HOME) }
    var detailProfileId by rememberSaveable { mutableStateOf<String?>(null) }

    LaunchedEffect(state.error) {
        state.error?.let {
            snackbarHostState.showSnackbar(it)
            vm.clearError()
        }
    }

    // keep the status fresh when the app comes back to the foreground (the
    // service may have switched profiles while we were away)
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { vm.refreshLight() }

    BackHandler(enabled = dest != HomeDest.HOME) { dest = HomeDest.HOME }

    Box(Modifier.fillMaxSize()) {
        when (dest) {
        HomeDest.APPS -> {
            val avm: DynamicProfileViewModel = viewModel()
            AppsProfileScreen(vm = avm, onBack = { dest = HomeDest.HOME })
        }
        HomeDest.SETTINGS -> {
            SettingsScreen(
                status = state.status,
                serviceRunning = state.serviceRunning,
                customThermal = state.customThermal,
                onCustomThermal = vm::onCustomThermal,
                freqFloorAudit = state.freqFloorAudit,
                onFreqFloorAudit = vm::onFreqFloorAudit,
                rndInputSuspend = state.rndInputSuspend,
                onRndInputSuspend = vm::onRndInputSuspend,
                onBack = { dest = HomeDest.HOME },
            )
        }
        HomeDest.DIAGNOSTICS -> {
            DiagnosticsScreen(onBack = { dest = HomeDest.HOME })
        }
        HomeDest.HOME -> {
            Scaffold(
                topBar = {
                    TopAppBar(
                        title = { Text("Surya Profiler", style = MaterialTheme.typography.titleLarge) },
                        actions = {
                            IconButton(onClick = { dest = HomeDest.SETTINGS }) {
                                Icon(Icons.Default.Settings, contentDescription = "Settings")
                            }
                        },
                        colors = TopAppBarDefaults.topAppBarColors(
                            containerColor = MaterialTheme.colorScheme.surface,
                        ),
                    )
                },
            ) { padding ->
                Column(
                    Modifier
                        .fillMaxSize()
                        .padding(padding),
                ) {
                    if (state.loading || state.busy != null) {
                        LinearProgressIndicator(Modifier.fillMaxWidth())
                    }
                    LazyColumn(
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 12.dp, bottom = 32.dp),
                        verticalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        items(state.cards, key = { it.profile.id }) { card ->
                            ProfileRow(
                                card = card,
                                active = state.status?.active == card.profile.id,
                                busy = state.busy == card.profile.id,
                                enabled = state.canAct,
                                onApply = { vm.apply(card.profile.id) },
                                onDetail = { detailProfileId = card.profile.id },
                            )
                        }
                        item(key = "apps") {
                            AppsRow(
                                mappedCount = state.mappedCount,
                                onClick = { dest = HomeDest.APPS },
                            )
                        }
                        item(key = "service") {
                            ServiceRow(
                                state = state,
                                onToggle = vm::onServiceToggle,
                            )
                        }
                        item(key = "dynamic") {
                            DynamicProfileRow(
                                state = state,
                                onToggle = vm::onDynamicToggle,
                            )
                        }
                        item(key = "diagnostics") {
                            DiagnosticsRow(
                                onClick = { dest = HomeDest.DIAGNOSTICS },
                            )
                        }
                    }
                }
            }
        }
        }
        SnackbarHost(
            snackbarHostState,
            Modifier.align(Alignment.BottomCenter),
        )
    }

    state.report?.let { report ->
        ReportDialog(report = report, onDismiss = vm::dismissReport)
    }
    detailProfileId?.let { id ->
        state.cards.firstOrNull { it.profile.id == id }?.let { card ->
            ProfileDetailDialog(card = card, onDismiss = { detailProfileId = null })
        }
    }
    if (state.confirmServiceOff) {
        AlertDialog(
            onDismissRequest = vm::cancelServiceOff,
            title = { Text("Turn off service?") },
            text = {
                Text(
                    "The service will stop, release every hold and write the " +
                        "captured stock values back. It verifies the restore before " +
                        "stopping; no reboot is needed. If the restore fails, the " +
                        "service stays on so nothing is left half-applied."
                )
            },
            confirmButton = {
                Button(onClick = vm::confirmServiceOff) { Text("Turn off") }
            },
            dismissButton = {
                TextButton(onClick = vm::cancelServiceOff) { Text("Cancel") }
            },
        )
    }
}
