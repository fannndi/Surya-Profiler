package com.suryaprofiler.ui

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.suryaprofiler.dynamic.DaemonLink
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.protocol.DiagInfo
import com.suryaprofiler.protocol.EnvSnapshot
import com.suryaprofiler.protocol.ThermalEvent
import java.util.Locale

/**
 * Diagnostics: live daemon health, env telemetry, transition history and the
 * relayed daemon log — everything the user needs to verify behaviour without
 * a cable. All data comes from the Rust daemon (thin-client rule).
 */
@Composable
internal fun SectionCard(title: String, content: @Composable () -> Unit) {
    ElevatedCard(Modifier.fillMaxWidth().animateContentSize()) {
        Column(Modifier.padding(14.dp)) {
            Text(
                title,
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
            )
            Spacer(Modifier.height(8.dp))
            content()
        }
    }
}

@Composable
internal fun DaemonCard(diag: DiagInfo?) {
    SectionCard("Daemon") {
        if (diag == null) {
            Text("Waiting for a `diag` reply…", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        KV("PID", "${diag.pid} · up ${fmtUptime(diag.uptimeS)}")
        KV("Active", (diag.active ?: "stock") + (diag.reason?.let { " · $it" } ?: ""))
        KV("Foreground", diag.foreground ?: "—")
        val screen = when {
            !diag.screenOn -> "off"
            diag.locked -> "on · locked"
            else -> "on · unlocked"
        }
        KV("Screen", screen + if (diag.multiWindow) " · multi-window" else "")
        KV("Watchers", "foreground ${alive(diag.watcherFg)}")
        KV(
            "MIUI holds",
            "saver ${if (diag.saverHeld) "held" else "—"}" +
                " · R&D input suspend ${if (diag.inputSuspendHeld) "ON" else "—"}",
        )
        KV(
            "Config",
            "service ${if (diag.configEnabled) "on" else "off"} · " +
                "dynamic ${if (diag.configDynamic) "on" else "off"} · base ${diag.baseProfile}",
        )
        Spacer(Modifier.height(4.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = {
                DaemonLink.send(DaemonCommands.shutdown())
            }) { Text("Restart daemon (restores stock)") }
        }
    }
}

@Composable
internal fun EnvCard(env: EnvSnapshot?) {
    SectionCard("Environment") {
        if (env == null) {
            Text("Waiting for the first sample (30 s cadence)…", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        KV(
            "Battery",
            (env.batteryPct?.let { "$it%" } ?: "—") +
                (env.charging?.let { if (it) " · charging" else " · discharging" } ?: ""),
        )
        KV("Battery temp", fmtTemp(env.batteryTempC))
        KV("CPU / GPU temp", "${fmtTemp(env.cpuTempC)} · ${fmtTemp(env.gpuTempC)}")
        KV("Battery I/V", fmtCurrent(env.batteryCurrentUa, env.batteryVoltageUv))
        KV("Battery health", fmtBatteryHealth(env))
        KV("CPU L/B MHz", fmtPairMhz(env.littleFreqMhz, env.bigFreqMhz))
        KV("GPU MHz", env.gpuFreqMhz?.let { "$it" } ?: "—")
        KV("Storage written", env.storageWrittenKb?.let { String.format(Locale.US, "%.1f GB", it / 1_000_000.0) } ?: "—")
        KV("GPU busy", env.gpuBusyPct?.let { "$it%" } ?: "—")
        KV(
            "Panel FPS",
            env.screenFps?.let { String.format(Locale.US, "%.1f fps", it) } ?: "—",
        )
    }
}

@Composable
internal fun ThermalCard(env: EnvSnapshot?, diag: DiagInfo?) {
    SectionCard("Thermal (MIUI)") {
        if (env == null) {
            Text("Waiting for the first sample (30 s cadence)…", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        val mode = env.thermalMode?.let {
            "$it · sconfig ${env.thermalSconfig ?: "—"}"
        } ?: "—"
        val scenario = env.thermalScenario
            ?.takeUnless { it.startsWith("custom-") }
            ?: "—"
        val perfMode = if (env.thermalPerfConfig == 1 || env.thermalGlobalMode == 1) "ON" else "off"
        val takeover = when {
            diag == null -> "—"
            diag.thermalHeld -> "custom · ${diag.thermalProfile ?: "?"}"
            diag.thermalYield != null -> "stock (MIUI ${diag.thermalYield})"
            else -> "stock"
        }
        KV("Mode", mode)
        KV("Scenario", scenario)
        KV("MIUI perf", perfMode + " · global ${env.thermalGlobalMode ?: "—"}")
        KV("Takeover", takeover)
        KV(
            "State",
            "temp ${env.thermalTempState ?: "—"} · screen ${env.thermalScreenState ?: "—"}",
        )
        KV("Quiet / NVM", "${fmtTemp(env.quietTempC)} · ${fmtTemp(env.nvmTempC)}")
        if (diag?.miuiSpGame == true) {
            Text(
                "MIUI SP-game compensation likely active (−2 °C virtual sensor) — " +
                    "quiet/board temps read low.",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
        KV("CPU6 cooling", fmtCooling(env.cpu6Cooling, env.cpu6CoolingMax))
        KV("CPU7 cooling", fmtCooling(env.cpu7Cooling, env.cpu7CoolingMax))
        KV("Policy max", "L ${fmtMhz(env.littleMaxMhz)} · B ${fmtMhz(env.bigMaxMhz)}")
        Text(
            "mi_thermald owns the scenario; Surya swaps stock/custom per profile.",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun ThermalEventsCard(events: List<ThermalEvent>) {
    SectionCard("MIUI thermal events") {
        if (events.isEmpty()) {
            Text(
                "No mi_thermald events yet. The dump appears after the first thermal step.",
                style = MaterialTheme.typography.bodySmall,
            )
            return@SectionCard
        }
        // Newest first; the daemon returns oldest -> newest.
        events.asReversed().forEach { ev ->
            Text(
                formatThermalEvent(ev),
                style = MaterialTheme.typography.labelSmall,
                fontFamily = FontFamily.Monospace,
            )
        }
        Spacer(Modifier.height(4.dp))
        Text(
            "Source: /data/vendor/thermal/thermal.dump (MIUI, read-only)",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

internal fun formatThermalEvent(ev: ThermalEvent): String {
    val sensor = listOfNotNull(ev.sensor, ev.sensorValue).joinToString(" ")
    val device = listOfNotNull(ev.device, ev.deviceValue).joinToString(" ")
    return "${ev.time} ${ev.algo} · $sensor → $device"
}

