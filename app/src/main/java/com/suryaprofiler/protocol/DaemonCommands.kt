package com.suryaprofiler.protocol

import org.json.JSONObject

/**
 * One place that builds every app -> daemon command. Keeping the wire strings
 * here (instead of scattered `JSONObject().put("cmd", ...)` calls) is what
 * lets the protocol be grepped, versioned and contract-tested.
 */
object DaemonCommands {
    /** Must match `core/src/daemon/proto.rs::VERSION`. */
    const val PROTOCOL_VERSION = 3

    fun hello() = JSONObject().put("cmd", "hello")

    fun ping() = JSONObject().put("cmd", "ping")

    fun diag() = JSONObject().put("cmd", "diag")

    fun stats() = JSONObject().put("cmd", "stats")

    fun configChanged() = JSONObject().put("cmd", "config_changed")

    fun userPresent() = JSONObject().put("cmd", "user_present")

    fun screen(on: Boolean, locked: Boolean) = JSONObject()
        .put("cmd", "screen")
        .put("on", on)
        .put("locked", locked)

    fun setBase(profile: ProfileId) = JSONObject()
        .put("cmd", "set_base")
        .put("profile", profile.wire)

    fun restore() = JSONObject().put("cmd", "restore")

    fun shutdown() = JSONObject().put("cmd", "shutdown")
}
