package com.suryaprofiler.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.suryaprofiler.dynamic.DaemonLink
import com.suryaprofiler.dynamic.DynamicProfileState
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.protocol.PowerPeaks

/**
 * Power usage: live charging/discharging, history chart, sessions, peaks and
 * per-profile/per-app averages. Every value comes from the Rust daemon; the
 * app only formats and draws it (thin-client rule).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PowerScreen(onBack: () -> Unit) {
    val running by DynamicProfileState.running.collectAsStateWithLifecycle()
    val daemonConnected by DynamicProfileState.daemonConnected.collectAsStateWithLifecycle()
    val snapshot by DynamicProfileState.powerSnapshot.collectAsStateWithLifecycle()
    val live by DynamicProfileState.powerLive.collectAsStateWithLifecycle()
    val liveBuckets by DynamicProfileState.powerLiveBuckets.collectAsStateWithLifecycle()
    var windowS by rememberSaveable { mutableStateOf(86_400L) }
    var confirmClear by remember { mutableStateOf(false) }

    LaunchedEffect(running, daemonConnected, windowS) {
        if (running && daemonConnected) {
            DaemonLink.send(DaemonCommands.power(watch = true, windowS = windowS))
        }
    }
    LaunchedEffect(running, daemonConnected) {
        if (!running || !daemonConnected) DynamicProfileState.resetPower()
    }
    LifecycleEventEffect(Lifecycle.Event.ON_START) {
        if (DynamicProfileState.running.value && DynamicProfileState.daemonConnected.value) {
            DaemonLink.send(DaemonCommands.power(watch = true, windowS = windowS))
        }
    }
    LifecycleEventEffect(Lifecycle.Event.ON_STOP) {
        if (DynamicProfileState.running.value) {
            DaemonLink.send(DaemonCommands.power(watch = false))
            DynamicProfileState.clearPowerLive()
        }
    }
    DisposableEffect(Unit) {
        onDispose {
            if (DynamicProfileState.running.value) {
                DaemonLink.send(DaemonCommands.power(watch = false))
                DynamicProfileState.clearPowerLive()
            }
        }
    }

    val history = snapshot?.history ?: emptyList()
    val merged = remember(history, liveBuckets) { mergeBuckets(history, liveBuckets) }
    val points = remember(merged, windowS) {
        powerPoints(history, liveBuckets, windowS, System.currentTimeMillis())
    }
    val session = remember(merged, live) {
        val current = live
        latestChargeSession(merged)?.let { s ->
            val liveStopped = current != null &&
                (current.inputSuspend == true ||
                    current.chargeEnabled == false ||
                    powerDirection(current) == "usage")
            if (s.ongoing && liveStopped) s.copy(ongoing = false) else s
        }
    }
    val profiles = remember(merged) { profileStats(merged) }
    val apps = remember(merged) { appStats(merged) }
    val peaks = snapshot?.peaks ?: PowerPeaks()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Power usage") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(
                        enabled = running && daemonConnected,
                        onClick = {
                            DaemonLink.send(DaemonCommands.power(true, windowS))
                        },
                    ) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(
                        enabled = running && daemonConnected,
                        onClick = { confirmClear = true },
                    ) {
                        Icon(Icons.Default.Delete, contentDescription = "Clear power history")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 32.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            if (!running) {
                item(key = "off") {
                    SectionCard("Service is off") {
                        Text(
                            "Turn on the Service on the Home screen to record charging " +
                                "and power usage. Sampling pauses while the Service is off; " +
                                "recorded history and peaks remain in daemon storage for next time.",
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            } else if (!daemonConnected) {
                item(key = "disconnected") {
                    SectionCard("Daemon is disconnected") {
                        Text(
                            "Waiting for the daemon to reconnect. Power sampling and " +
                                "the previous live snapshot are paused.",
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            } else {
                item(key = "live") { PowerLiveCard(live) }
                if (session != null) {
                    item(key = "session") { PowerSessionCard(session) }
                }
                item(key = "history") {
                    PowerHistoryCard(
                        points = points,
                        windowS = windowS,
                        onWindow = { windowS = it },
                    )
                }
                item(key = "peaks") { PowerPeaksCard(peaks) }
                item(key = "profiles") { PowerProfileCard(profiles) }
                item(key = "apps") { PowerAppsCard(apps) }
                if (snapshot == null) {
                    item(key = "waiting") {
                        SectionCard("Waiting") {
                            Text(
                                "The daemon has not answered yet.",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                } else {
                    item(key = "note") {
                        SectionCard("How to read") {
                            Text(
                                "Live uses a 3 s cadence while the screen is on and " +
                                    "30 s while it is off; history is one-minute buckets. " +
                                    "Records are read-only telemetry and never write power nodes.",
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                }
            }
        }
    }

    if (confirmClear) {
        AlertDialog(
            onDismissRequest = { confirmClear = false },
            title = { Text("Clear power history?") },
            text = {
                Text(
                    "This removes the recorded history and all-time peaks from the " +
                        "daemon state. Live sampling continues immediately. It does not " +
                        "change any tuning or charging setting.",
                )
            },
            confirmButton = {
                Button(onClick = {
                    confirmClear = false
                    DynamicProfileState.resetPower()
                    DaemonLink.send(DaemonCommands.powerClear())
                }) { Text("Clear") }
            },
            dismissButton = {
                TextButton(onClick = { confirmClear = false }) { Text("Cancel") }
            },
        )
    }
}

