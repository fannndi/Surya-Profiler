package com.suryaprofiler.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.util.Locale
import kotlin.math.roundToLong

@Composable
internal fun PowerAppRow(stat: AppPowerStat) {
    val context = LocalContext.current
    val icon by produceState(
        initialValue = AppIconCache.get(stat.pkg),
        stat.pkg,
    ) {
        if (value == null) {
            value = withContext(Dispatchers.IO) {
                AppIconCache.load(context, stat.pkg)
            }
        }
    }
    val label = AppIconCache.label(context, stat.pkg)
    Row(
        Modifier
            .fillMaxWidth()
            .padding(vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        val image = icon
        if (image != null) {
            Image(
                bitmap = image,
                contentDescription = null,
                modifier = Modifier
                    .size(34.dp)
                    .clip(RoundedCornerShape(8.dp)),
            )
        } else {
            Box(Modifier.size(34.dp))
        }
        Spacer(Modifier.width(10.dp))
        Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.bodyMedium, maxLines = 1)
            Text(
                listOfNotNull(
                    "≈ ${formatPowerWithDirection(stat.avgMw)}",
                    formatCurrent(stat.avgUa.roundToLong()),
                    stat.avgTempC?.let {
                        String.format(Locale.US, "%.1f °C avg", it)
                    },
                    stat.maxTempC?.let {
                        String.format(Locale.US, "%.1f °C max", it)
                    },
                ).joinToString(" · "),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 2,
            )
        }
        Spacer(Modifier.width(8.dp))
        Text(
            fmtDuration(stat.durMs / 1000),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
