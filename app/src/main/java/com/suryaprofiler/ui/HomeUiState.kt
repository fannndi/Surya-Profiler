package com.suryaprofiler.ui

import com.suryaprofiler.core.ApplyReport
import com.suryaprofiler.core.Plan
import com.suryaprofiler.core.Profile
import com.suryaprofiler.core.Status

/**
 * UI state shapes for the Home screen (kept separate from the ViewModel so
 * the state contract is greppable on its own).
 */

data class ProfileCard(
    val profile: Profile,
    val plan: Plan?,
    val planError: String? = null,
)

data class HomeUiState(
    val loading: Boolean = true,
    val status: Status? = null,
    val cards: List<ProfileCard> = emptyList(),
    /** Profile id (or "service-off") while a root action runs. */
    val busy: String? = null,
    val error: String? = null,
    val report: ApplyReport? = null,
    val confirmServiceOff: Boolean = false,
    // --- service mirror ---
    val serviceEnabled: Boolean = false,
    val serviceRunning: Boolean = false,
    val serviceReason: String? = null,
    /** Dynamic Profile switch (mapped apps auto-override the base). */
    val dynamicEnabled: Boolean = true,
    /** Per-profile MIUI thermal takeover (stock map remains fallback). */
    val customThermal: Boolean = true,
    /** Freq floor audit sentinel (auto re-assert of a drifted CPU floor). */
    val freqFloorAudit: Boolean = true,
    /** R&D: suspend charger input while the service is on. */
    val rndInputSuspend: Boolean = false,
    val mappedCount: Int = 0,
) {
    val canAct: Boolean get() = !loading && busy == null
}
