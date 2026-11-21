package com.suryaprofiler.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.suryaprofiler.core.Op


@Composable
internal fun ProfileDetailDialog(card: ProfileCard, onDismiss: () -> Unit) {
    val plan = card.plan
    AlertDialog(
        onDismissRequest = onDismiss,
        title = {
            Column {
                Text("${card.profile.label} — detail")
                Text(
                    card.profile.desc,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        text = {
            Column(
                Modifier
                    .fillMaxWidth()
                    .height(400.dp)
                    .verticalScroll(rememberScrollState()),
            ) {
                if (plan == null) {
                    Text(
                        card.planError ?: "Plan unavailable",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                } else if (!plan.ok) {
                    plan.errors.forEach {
                        Text(it, style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.error)
                    }
                } else {
                    Text(
                        "${plan.ops.size} params · ${plan.pending} to write · " +
                            "${plan.inSync} in sync · ${plan.locked.size} locked",
                        style = MaterialTheme.typography.labelMedium,
                        modifier = Modifier.padding(bottom = 8.dp),
                    )
                    plan.ops.forEach { op ->
                        OpDetailRow(op)
                        HorizontalDivider(Modifier.padding(vertical = 2.dp))
                    }
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text("Close") }
        },
    )
}

@Composable
private fun OpDetailRow(op: Op) {
    Column(Modifier.padding(vertical = 6.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                op.key,
                style = MaterialTheme.typography.labelMedium,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.weight(1f),
            )
            TierBadge(op.tier)
        }
        when (op.status.kind) {
            "locked" -> Text(
                "⚠ ${op.status.reason ?: "locked"}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.secondary,
            )
            "unchanged" -> Text(
                "✓ in sync: ${op.resolved}",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.primary,
            )
            else -> Text(
                "→ ${op.resolved}" + (op.current?.let { "   (now: $it)" } ?: ""),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun TierBadge(tier: String) {
    val container = when (tier) {
        "free" -> MaterialTheme.colorScheme.tertiaryContainer
        else -> MaterialTheme.colorScheme.secondaryContainer
    }
    val onContainer = when (tier) {
        "free" -> MaterialTheme.colorScheme.onTertiaryContainer
        else -> MaterialTheme.colorScheme.onSecondaryContainer
    }
    Surface(color = container, shape = MaterialTheme.shapes.small) {
        Text(
            tier,
            Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
            style = MaterialTheme.typography.labelSmall,
            color = onContainer,
        )
    }
}

