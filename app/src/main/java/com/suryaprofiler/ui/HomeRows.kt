package com.suryaprofiler.ui

import androidx.compose.foundation.clickable
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Balance
import androidx.compose.material.icons.filled.BatterySaver
import androidx.compose.material.icons.filled.Sports
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.suryaprofiler.protocol.ProfileId


// --- profile row -------------------------------------------------------------

private fun iconFor(id: String): ImageVector = when (ProfileId.from(id)) {
    ProfileId.POWERSAVE -> Icons.Default.BatterySaver
    ProfileId.BALANCE -> Icons.Default.Balance
    else -> Icons.Default.Sports
}

@Composable
internal fun ProfileRow(
    card: ProfileCard,
    active: Boolean,
    busy: Boolean,
    enabled: Boolean,
    onApply: () -> Unit,
    onDetail: () -> Unit,
) {
    val plan = card.plan
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .animateContentSize()
            .clickable(enabled = enabled && !busy, onClick = onApply),
        colors = if (active) {
            CardDefaults.cardColors(
                containerColor = MaterialTheme.colorScheme.primaryContainer,
                contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
            )
        } else {
            CardDefaults.cardColors()
        },
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 14.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Surface(
                modifier = Modifier.size(36.dp),
                shape = CircleShape,
                color = if (active) MaterialTheme.colorScheme.primary
                else MaterialTheme.colorScheme.primaryContainer,
            ) {
                Icon(
                    iconFor(card.profile.id),
                    contentDescription = null,
                    modifier = Modifier.padding(8.dp),
                    tint = if (active) MaterialTheme.colorScheme.onPrimary
                    else MaterialTheme.colorScheme.onPrimaryContainer,
                )
            }
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        card.profile.label,
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                    if (active) {
                        Spacer(Modifier.width(8.dp))
                        SuggestionPill(
                            "ACTIVE",
                            MaterialTheme.colorScheme.primary,
                            MaterialTheme.colorScheme.onPrimary,
                        )
                    }
                }
                Text(
                    card.profile.short.ifEmpty { card.profile.desc },
                    style = MaterialTheme.typography.labelSmall,
                    color = if (active) MaterialTheme.colorScheme.onPrimaryContainer
                    else MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                )
            }
            when {
                busy -> CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                plan != null && !plan.ok -> Text(
                    "Unavailable",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.error,
                )
                else -> TextButton(onClick = onDetail) { Text("Detail") }
            }
        }
    }
}

// --- apps entry + service ----------------------------------------------------

