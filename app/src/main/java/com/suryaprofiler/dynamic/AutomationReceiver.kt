package com.suryaprofiler.dynamic

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import com.suryaprofiler.protocol.DaemonCommands
import com.suryaprofiler.protocol.ProfileId

/**
 * Automation API (Tasker / MacroDroid / `adb shell am broadcast`).
 *
 * Deliberately tiny: it only changes the base Device Profile. It never
 * starts/stops the Service and never touches tuning nodes. All decisions
 * stay in the Rust daemon; this receiver only writes app-owned config + hints.
 *
 *   am broadcast -a com.suryaprofiler.action.SET_PROFILE \
 *       -n com.suryaprofiler/.dynamic.AutomationReceiver --es profile game
 */
class AutomationReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "SuryaProfiler"
        const val ACTION_SET_PROFILE = "com.suryaprofiler.action.SET_PROFILE"

        /** The three user-facing Device Profiles. */
        private val PROFILES: Set<String> = ProfileId.USER.map { it.wire }.toSet()
    }

    override fun onReceive(context: Context, intent: Intent) {
        // Only accept targeted broadcasts (explicit component), so an app
        // cannot fire these actions implicitly. Documented callers (adb,
        // Tasker, MacroDroid) all use an explicit component (-n / component).
        if (intent.component?.packageName != context.packageName) {
            Log.w(TAG, "automation: rejected non-targeted broadcast from ${intent.`package` ?: "?"}")
            return
        }
        val pending = goAsync()
        val config = DynamicProfileConfig.get(context)
        Thread {
            try {
                if (intent.action != ACTION_SET_PROFILE) return@Thread
                val profile = intent.getStringExtra("profile")?.trim()
                if (profile == null || profile !in PROFILES) {
                    Log.w(TAG, "automation: bad profile '${intent.getStringExtra("profile")}'")
                    return@Thread
                }
                Log.d(TAG, "automation: set profile $profile")
                config.baseProfile = profile
                if (config.baseProfile != profile) {
                    Log.w(TAG, "automation: could not persist base profile")
                    return@Thread
                }
                // immediate apply when the service (daemon) is running;
                // otherwise the base takes effect at the next service start
                DaemonLink.send(DaemonCommands.setBase(ProfileId.require(profile)))
            } finally {
                pending.finish()
            }
        }.start()
    }
}
