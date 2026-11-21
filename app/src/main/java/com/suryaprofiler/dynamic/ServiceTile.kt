package com.suryaprofiler.dynamic

import android.content.Intent
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import com.suryaprofiler.core.ProfileLabels

/**
 * Quick Settings tile: shows the service state and toggles it with the same
 * semantics as the UI switch — turning off restores stock first.
 *
 * Responsibility: a status/toggle surface. Non-goals: decisions.
 */
class ServiceTile : TileService() {

    companion object {
        /**
         * Ask the system to re-bind the tile so its subtitle updates
         * immediately after a profile switch (no-op when the tile is absent).
         */
        fun refresh(context: android.content.Context) {
            runCatching {
                requestListeningState(
                    context,
                    android.content.ComponentName(context, ServiceTile::class.java),
                )
            }
        }
    }

    override fun onStartListening() = refreshTile()

    override fun onClick() {
        if (DynamicProfileState.running.value) {
            // same path as the notification action: restore stock, then stop
            startService(
                Intent(this, DynamicProfileService::class.java)
                    .setAction(DynamicProfileService.ACTION_STOP),
            )
        } else {
            val config = DynamicProfileConfig.get(this)
            config.enabled = true
            if (!config.enabled) {
                android.util.Log.w("SuryaProfiler", "tile start: enabled=true not persisted")
                return
            }
            if (!DynamicProfileService.start(this)) {
                config.enabled = false
                android.util.Log.w("SuryaProfiler", "tile start rejected")
            }
        }
        refreshTile()
    }

    private fun refreshTile() {
        val t = qsTile ?: return
        val running = DynamicProfileState.running.value
        t.state = if (running) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        t.label = "Surya Profiler"
        if (Build.VERSION.SDK_INT >= 29) {
            val active = DynamicProfileState.appliedProfile.value
            t.subtitle = when {
                !running -> "Off"
                active != null -> ProfileLabels.of(active)
                else -> "On"
            }
        }
        t.updateTile()
    }
}
