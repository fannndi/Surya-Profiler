package com.suryaprofiler.protocol

/**
 * The four profiles the engine understands. `sleep` is automatic; only the
 * three user profiles can be selected as a base or mapped to an app.
 *
 * This is the single Kotlin source of profile ids. The Rust side validates
 * the same wire values in `daemon::config::{USER_PROFILES, ALL_PROFILES}`.
 */
enum class ProfileId(val wire: String) {
    POWERSAVE("powersave"),
    BALANCE("balance"),
    GAME("game"),
    SLEEP("sleep"),
    ;

    val isUser: Boolean get() = this != SLEEP

    companion object {
        val USER: List<ProfileId> = entries.filter { it.isUser }
        val ALL: List<ProfileId> = entries

        fun from(value: String?): ProfileId? =
            entries.firstOrNull { it.wire == value }

        fun require(value: String?): ProfileId =
            from(value) ?: throw IllegalArgumentException("unknown profile '$value'")
    }
}
