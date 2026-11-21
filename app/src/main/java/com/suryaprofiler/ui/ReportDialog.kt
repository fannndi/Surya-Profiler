package com.suryaprofiler.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.suryaprofiler.core.ApplyReport


@Composable
internal fun ReportDialog(report: ApplyReport, onDismiss: () -> Unit) {
    val title = when (report.mode) {
        "restore" -> "Service off ${if (report.ok) "complete" else "with errors"}"
        else -> "Apply ${report.profile ?: ""} ${if (report.ok) "complete" else "with errors"}"
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column(
                Modifier
                    .fillMaxWidth()
                    .height(320.dp)
                    .verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(
                    "wrote ${report.wrote} · verified ${report.verified} · " +
                        "unchanged ${report.unchanged} · failed ${report.failed} · " +
                        "locked ${report.locked.size}",
                    style = MaterialTheme.typography.labelMedium,
                )
                if (report.snapshotCreated) {
                    Text(
                        "Stock snapshot created before first write.",
                        style = MaterialTheme.typography.labelSmall,
                    )
                }
                report.results.forEach { r -> ResultLine(r) }
                report.locked.forEach { l ->
                    Text(
                        "LOCKED ${l.key}\n    ${l.reason}",
                        style = MaterialTheme.typography.labelSmall,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.secondary,
                    )
                }
                if (report.mode == "restore") {
                    Text(
                        if (report.ok) {
                            "Stock values were written back and verified. " +
                                "The device is back to the pre-tuning state."
                        } else {
                            "Restore was not fully verified — the service stays on. " +
                                "Retry, or run `surya-prof restore`."
                        },
                        style = MaterialTheme.typography.bodySmall,
                    )
                } else if (report.wrote == 0 &&
                    report.failed == 0 &&
                    report.locked.isEmpty() &&
                    report.results.isEmpty()
                ) {
                    Text(
                        "Everything already in sync — nothing to write.",
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text("Close") }
        },
    )
}


@Composable
private fun ResultLine(r: com.suryaprofiler.core.WriteResult) {
    val color = when {
        r.error != null -> MaterialTheme.colorScheme.error
        r.verified -> MaterialTheme.colorScheme.primary
        else -> MaterialTheme.colorScheme.onSurfaceVariant
    }
    val mark = when {
        r.error != null -> "✗"
        r.verified -> "✓"
        else -> "·"
    }
    Text(
        "$mark ${r.key} → ${r.resolved}" + (r.error?.let { "\n    $it" } ?: ""),
        style = MaterialTheme.typography.labelSmall,
        fontFamily = FontFamily.Monospace,
        color = color,
    )
}

