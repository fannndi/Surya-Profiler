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
import androidx.compose.material.icons.filled.Share
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.suryaprofiler.dynamic.DaemonLink
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.dynamic.DynamicProfileState

/**
 * Diagnostics: live daemon health, env telemetry, transition history and the
 * relayed daemon log — everything the user needs to verify behaviour without
 * a cable. All data comes from the Rust daemon (thin-client rule).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DiagnosticsScreen(onBack: () -> Unit) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val running by DynamicProfileState.running.collectAsStateWithLifecycle()
    val diag by DynamicProfileState.diag.collectAsStateWithLifecycle()
    val env by DynamicProfileState.env.collectAsStateWithLifecycle()
    val stats by DynamicProfileState.stats.collectAsStateWithLifecycle()
    val heals by DynamicProfileState.heals.collectAsStateWithLifecycle()
    val logs by DynamicProfileState.logs.collectAsStateWithLifecycle()

    // Fresh data on open and when the daemon becomes ready (a Diagnostics
    // screen opened during service start must not stay empty forever).
    LaunchedEffect(running) {
        if (running) {
            DaemonLink.send(DaemonCommands.diag())
            DaemonLink.send(DaemonCommands.stats())
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Diagnostics") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    IconButton(onClick = {
                        DaemonLink.send(DaemonCommands.diag())
                        DaemonLink.send(DaemonCommands.stats())
                    }) {
                        Icon(Icons.Default.Refresh, contentDescription = "Refresh")
                    }
                    IconButton(onClick = {
                        DynamicProfileState.clearLogs()
                    }) {
                        Icon(Icons.Default.Delete, contentDescription = "Clear log")
                    }
                    IconButton(onClick = {
                        val body = buildString {
                            append("Surya Profiler diagnostics\n")
                            append("daemon: ${diag ?: "unavailable"}\n")
                            append("env: ${env ?: "unavailable"}\n")
                            append("heals: $heals\n")
                            append("transitions: ${stats.size}\n")
                            append("logs:\n")
                            append(logs.joinToString("\n"))
                        }
                        val i = android.content.Intent(android.content.Intent.ACTION_SEND).apply {
                            type = "text/plain"
                            putExtra(android.content.Intent.EXTRA_TEXT, body)
                        }
                        context.startActivity(android.content.Intent.createChooser(i, "Share diagnostics"))
                    }) {
                        Icon(Icons.Default.Share, contentDescription = "Share")
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
                            "Turn on the Service to see live daemon data.",
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            }
            item(key = "daemon") { DaemonCard(diag) }
            item(key = "env") { EnvCard(env) }
            item(key = "thermal") { ThermalCard(env, diag) }
            item(key = "thermal-events") { ThermalEventsCard(diag?.thermalEvents ?: emptyList()) }
            item(key = "heal") { HealCard(heals) }
            item(key = "stats24") { StatsSummaryCard(stats) }
            item(key = "stats") { TransitionsCard(stats) }
            item(key = "log") { LogCard(logs, context) }
        }
    }
}

// --- cards -------------------------------------------------------------------

