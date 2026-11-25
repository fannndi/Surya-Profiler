package com.suryaprofiler.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.BatteryChargingFull
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.suryaprofiler.protocol.PowerPeaks
import com.suryaprofiler.protocol.PowerSample
import java.util.Locale
import kotlin.math.abs

@Composable
internal fun PowerLiveCard(sample: PowerSample?) {
    SectionCard("Live") {
        if (sample == null) {
            Text(
                "Waiting for the first power sample…",
                style = MaterialTheme.typography.bodySmall,
            )
            return@SectionCard
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconCircle(Icons.Default.BatteryChargingFull)
            Spacer(Modifier.width(10.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    statusLabel(sample.status) +
                        (sample.pct?.let { " · $it%" } ?: ""),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    "Battery-side, sign verified (negative = charging)",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Spacer(Modifier.height(8.dp))
        Text(
            formatCurrent(sample.currentUa),
            style = MaterialTheme.typography.displaySmall,
            fontWeight = FontWeight.SemiBold,
        )
        Text(
            powerDirection(sample).replaceFirstChar { it.uppercase() } +
                " · " + formatPower(powerMw(sample.currentUa, sample.voltageUv)),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(6.dp))
        KV(
            "Battery",
            listOfNotNull(
                formatVoltage(sample.voltageUv),
                formatPowerTemp(sample.tempC),
            ).joinToString(" · "),
        )
        KV("Health", batteryHealthLabel(sample))
        sample.input?.let { input ->
            KV(
                "Charger input",
                listOfNotNull(
                    input.kind?.let { "${input.name} ($it)" } ?: input.name,
                    formatVoltage(input.voltageUv),
                    formatCurrent(input.currentUa),
                    powerMw(input.currentUa, input.voltageUv)?.let(::formatPower),
                ).joinToString(" · "),
            )
        }
        estimateToFull(sample)?.let { KV("Est. full", it) }
        estimateToEmpty(sample)?.let { KV("Est. left", it) }
        powerWarnings(sample).forEach { warning ->
            Spacer(Modifier.height(4.dp))
            Text(
                warning,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.tertiary,
            )
        }
        Spacer(Modifier.height(4.dp))
        Text(
            "Power is current × battery voltage; while charging it is net into " +
                "the battery, not wall power.",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun PowerSessionCard(session: ChargeSession?) {
    if (session == null) return
    SectionCard(if (session.ongoing) "Charging session · now" else "Last charge session") {
        KV("Added", String.format(Locale.US, "%.0f mAh", session.addedMah))
        KV("Duration", fmtDuration(session.durMs / 1000))
        KV("Peak", formatPower(session.peakMw))
        KV(
            "Level",
            "${session.startPct?.toString() ?: "—"}% → " +
                (session.endPct?.toString()?.plus("%") ?: "—"),
        )
        if (!session.ongoing) KV("Ended", fmtWhen(session.endMs))
    }
}

@Composable
internal fun PowerPeaksCard(peaks: PowerPeaks) {
    SectionCard("Peak record") {
        if (peaks.charge == null && peaks.discharge == null && peaks.temp == null) {
            Text("No peaks recorded yet.", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            PeakTile(
                Modifier.weight(1f),
                "Usage",
                peaks.discharge?.let { formatPower(peakPowerMw(it)) } ?: "—",
                peaks.discharge?.let { formatCurrent(it.ua) } ?: "—",
                peaks.discharge?.let { "${fmtWhen(it.tMs)} · ${it.pct?.let { p -> "$p%" } ?: "—"}" },
            )
            PeakTile(
                Modifier.weight(1f),
                "Charge",
                peaks.charge?.let { formatPower(peakPowerMw(it)) } ?: "—",
                peaks.charge?.let { formatCurrent(abs(it.ua)) } ?: "—",
                peaks.charge?.let { "${fmtWhen(it.tMs)} · ${it.pct?.let { p -> "$p%" } ?: "—"}" },
            )
            PeakTile(
                Modifier.weight(1f),
                "Temp",
                peaks.temp?.let { formatPowerTemp(it.c) } ?: "—",
                "",
                peaks.temp?.let { "${fmtWhen(it.tMs)} · ${it.pct?.let { p -> "$p%" } ?: "—"}" },
            )
        }
    }
}

@Composable
private fun PeakTile(
    modifier: Modifier,
    label: String,
    value: String,
    sub: String,
    detail: String?,
) {
    Column(modifier.padding(vertical = 2.dp)) {
        Text(
            label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            value,
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
        )
        if (sub.isNotEmpty()) {
            Text(
                sub,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        detail?.let {
            Text(
                it,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
