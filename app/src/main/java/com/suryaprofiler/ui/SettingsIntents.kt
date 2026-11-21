package com.suryaprofiler.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.PowerManager
import android.provider.Settings
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue

/**
 * Settings: permissions/background checklist plus the R&D input-suspend
 * switch. Turning the service off (on Home/tile) restores stock; there is no
 * backup/guards/charging/bridge configuration anymore.
 */
@OptIn(ExperimentalMaterial3Api::class)

internal fun openMiuiAutostart(context: Context) {
    val candidates = listOf(
        Intent().setClassName(
            "com.miui.securitycenter",
            "com.miui.permcenter.autostart.AutoStartManagementActivity",
        ),
        Intent("miui.intent.action.OP_AUTO_START").addCategory(Intent.CATEGORY_DEFAULT),
    )
    for (intent in candidates) {
        val started = runCatching {
            context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
            true
        }.getOrDefault(false)
        if (started) return
    }
    openAppSettings(context)
}

internal fun openAppSettings(context: Context) {
    runCatching {
        context.startActivity(
            Intent(
                Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", context.packageName, null),
            ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )
    }
}

internal fun requestBatteryExemption(context: Context) {
    val pm = context.getSystemService(PowerManager::class.java)
    if (pm?.isIgnoringBatteryOptimizations(context.packageName) == true) return
    val intent = Intent(
        Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS,
        Uri.parse("package:${context.packageName}"),
    ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
    val started = runCatching { context.startActivity(intent) }.isSuccess
    if (!started) openAppSettings(context)
}
