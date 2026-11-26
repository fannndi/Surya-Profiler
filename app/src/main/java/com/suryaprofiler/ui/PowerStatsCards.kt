package com.suryaprofiler.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.suryaprofiler.core.ProfileLabels
import kotlin.math.roundToLong

private val windows = listOf(3_600L to "1h", 21_600L to "6h", 86_400L to "24h")

@Composable
internal fun PowerHistoryCard(
    points: List<PowerPoint>,
    windowS: Long,
    onWindow: (Long) -> Unit,
) {
    val scheme = MaterialTheme.colorScheme
    SectionCard("Power history") {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            windows.forEach { (s, label) ->
                FilterChip(
                    selected = windowS == s,
                    onClick = { onWindow(s) },
                    label = { Text(label) },
                )
            }
        }
        Spacer(Modifier.height(6.dp))
        if (points.isEmpty()) {
            Text(
                "Collecting power samples…",
                style = MaterialTheme.typography.bodySmall,
            )
        } else {
            PowerChart(points)
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (points.any { it.mw > 0 }) LegendDot(scheme.primary, "Usage")
                if (points.any { it.mw < 0 }) LegendDot(scheme.tertiary, "Charge")
                LegendDot(scheme.onSurface.copy(alpha = 0.18f), "Screen off")
            }
            Text(
                "Positive = usage · negative = charging (battery-side). " +
                    "One-minute buckets; short switches inside a minute average out.",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun LegendDot(color: Color, label: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Surface(
            Modifier.size(8.dp),
            shape = CircleShape,
            color = color,
            content = {},
        )
        Spacer(Modifier.width(4.dp))
        Text(
            label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun PowerProfileCard(stats: List<ProfilePowerStat>) {
    SectionCard("By profile") {
        if (stats.isEmpty()) {
            Text("No profile samples yet.", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        stats.forEach { s ->
            Row(
                Modifier
                    .fillMaxWidth()
                    .padding(vertical = 3.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    ProfileLabels.of(s.profile),
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Text(
                    formatPowerWithDirection(s.avgMw) + " · " + formatCurrent(s.avgUa.roundToLong()),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.width(10.dp))
                Text(
                    fmtDuration(s.durMs / 1000),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        Text(
            "Includes screen-off sleep buckets; averages are time-weighted.",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
internal fun PowerAppsCard(stats: List<AppPowerStat>) {
    SectionCard("By app") {
        val usable = stats.filter { it.durMs >= 120_000 }
        if (usable.isEmpty()) {
            Text(
                "Apps appear after 2+ minutes in the foreground with the screen on.",
                style = MaterialTheme.typography.bodySmall,
            )
            return@SectionCard
        }
        usable.forEach { PowerAppRow(it) }
        Text(
            "Average battery power while the app was visible; it also includes " +
                "system work, so it is not app-only consumption.",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}


