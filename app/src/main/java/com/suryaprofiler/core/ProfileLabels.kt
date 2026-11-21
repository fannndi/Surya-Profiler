package com.suryaprofiler.core

import com.suryaprofiler.protocol.ProfileId

/** Shared display labels for profile ids (rows, pickers, notifications). */
object ProfileLabels {
    private val LABELS = mapOf(
        "sleep" to "Sleep",
        "powersave" to "Power Save",
        "balance" to "Balance",
        "game" to "Game",
    )

    fun of(id: String?): String {
        val profile = ProfileId.from(id) ?: return id ?: "Default"
        return LABELS[profile.wire] ?: profile.wire
    }
}
