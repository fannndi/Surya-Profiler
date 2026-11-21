package com.suryaprofiler.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.suryaprofiler.protocol.HealsInfo
import com.suryaprofiler.protocol.StatEntry

@Composable
internal fun HealCard(heals: HealsInfo) {
    SectionCard("Auto-revive (watchdog)") {
        if (heals.total == 0L) {
            Text("No drift needed healing yet.", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        KV(
            "Fix-ups",
            "${heals.total} · last ${fmtTime(heals.lastT)} (${heals.lastKeys} keys)",
        )
    }
}

@Composable
internal fun TransitionsCard(stats: List<StatEntry>) {
    SectionCard("Transitions (${stats.size})") {
        if (stats.isEmpty()) {
            Text("No switches recorded yet.", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        // newest first, bounded to keep the screen light
        stats.asReversed().take(20).forEachIndexed { i, e ->
            if (i > 0) HorizontalDivider(Modifier.padding(vertical = 6.dp))
            val from = e.from ?: "stock"
            Text(
                "${fmtTime(e.t)}   $from → ${e.to}  (${e.reason})",
                style = MaterialTheme.typography.bodySmall,
                fontWeight = FontWeight.Medium,
            )
            val meta = listOfNotNull(
                e.batteryPct?.let { "$it%" },
                e.tempC?.let { fmtTemp(it) },
            ).joinToString(" · ")
            if (meta.isNotEmpty()) {
                Text(
                    meta,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        if (stats.size > 20) {
            Text(
                "… ${stats.size - 20} older entries",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
internal fun LogCard(logs: List<String>, context: android.content.Context) {
    SectionCard("Daemon log (${logs.size})") {
        if (logs.isEmpty()) {
            Text("No log lines yet.", style = MaterialTheme.typography.bodySmall)
            return@SectionCard
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            TextButton(onClick = {
                val cm = context.getSystemService(android.content.Context.CLIPBOARD_SERVICE)
                    as android.content.ClipboardManager
                cm.setPrimaryClip(
                    android.content.ClipData.newPlainText("SuryaProfiler log", logs.joinToString("\n")),
                )
            }) { Text("Copy") }
        }
        Column(
            Modifier
                .fillMaxWidth()
                .heightIn(max = 320.dp),
        ) {
            logs.takeLast(60).forEach { line ->
                Text(
                    line,
                    style = MaterialTheme.typography.labelSmall,
                    fontFamily = FontFamily.Monospace,
                )
            }
        }
    }
}

// --- bits --------------------------------------------------------------------

@Composable
internal fun KV(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
        Text(
            label,
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.width(110.dp),
        )
        Text(value, style = MaterialTheme.typography.bodySmall)
    }
}

