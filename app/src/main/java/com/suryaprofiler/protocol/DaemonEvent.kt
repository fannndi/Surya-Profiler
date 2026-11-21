package com.suryaprofiler.protocol

import org.json.JSONObject

/** One parsed daemon -> app event. Unknown/future events are non-fatal. */
sealed interface DaemonEvent {
    data class Hello(val version: Int, val pid: Int) : DaemonEvent

    data class State(val state: Snapshot) : DaemonEvent

    data class Applied(
        val profile: String,
        val reason: String,
        val srcPkg: String?,
        val ok: Boolean,
        val applied: Boolean,
        val wrote: Int,
        val verified: Int,
        val failed: Int,
        val ms: Long,
        val settleMs: Long,
    ) : DaemonEvent

    data class Restored(
        val ok: Boolean,
        val wrote: Int,
        val verified: Int,
        val failed: Int,
        val saverOk: Boolean,
        val inputSuspendOk: Boolean,
        val thermalOk: Boolean,
    ) : DaemonEvent

    data class Env(val env: EnvSnapshot) : DaemonEvent

    data class Diag(
        val diag: DiagInfo?,
        val env: EnvSnapshot?,
    ) : DaemonEvent

    data class Stats(
        val entries: List<StatEntry>,
        val heals: HealsInfo,
    ) : DaemonEvent

    data class Decision(
        val trigger: String,
        val action: String,
        val profile: String?,
        val reason: String?,
    ) : DaemonEvent

    data class Error(val msg: String) : DaemonEvent

    data object Pong : DaemonEvent
    data object Bye : DaemonEvent

    data class Unknown(val raw: JSONObject) : DaemonEvent

    data class Snapshot(
        val screenOn: Boolean,
        val locked: Boolean,
        val multiWindow: Boolean,
        val secondWindow: String?,
        val foreground: String?,
        val active: String?,
        val reason: String?,
        val srcPkg: String?,
    )

    companion object {
        fun parse(json: JSONObject): DaemonEvent = when (json.optString("event")) {
            "hello" -> Hello(json.optInt("version"), json.optInt("pid"))

            "state" -> {
                val s = json.optJSONObject("state") ?: JSONObject()
                State(
                    Snapshot(
                        screenOn = s.optBoolean("screen_on"),
                        locked = s.optBoolean("locked"),
                        multiWindow = s.optBoolean("multi_window"),
                        secondWindow = s.strOrNull("second_window"),
                        foreground = s.strOrNull("foreground"),
                        active = s.strOrNull("active"),
                        reason = s.strOrNull("reason"),
                        srcPkg = s.strOrNull("src_pkg"),
                    ),
                )
            }

            "applied" -> Applied(
                profile = json.optString("profile"),
                reason = json.optString("reason"),
                srcPkg = json.strOrNull("src_pkg"),
                ok = json.optBoolean("ok"),
                applied = json.optBoolean("applied", true),
                wrote = json.optInt("wrote"),
                verified = json.optInt("verified"),
                failed = json.optInt("failed"),
                ms = json.optLong("ms"),
                settleMs = json.optLong("settle_ms"),
            )

            "restored" -> Restored(
                ok = json.optBoolean("ok"),
                wrote = json.optInt("wrote"),
                verified = json.optInt("verified"),
                failed = json.optInt("failed"),
                saverOk = json.optBoolean("saver_ok", true),
                inputSuspendOk = json.optBoolean("input_suspend_ok", true),
                thermalOk = json.optBoolean("thermal_ok", true),
            )

            "env" -> Env(DiagnosticsParse.env(json))

            "diag" -> Diag(
                diag = DiagnosticsParse.diag(json),
                env = json.optJSONObject("diag")
                    ?.optJSONObject("env")
                    ?.let(DiagnosticsParse::envObject),
            )

            "stats" -> Stats(
                entries = DiagnosticsParse.stats(json),
                heals = DiagnosticsParse.heals(json),
            )

            "decision" -> Decision(
                trigger = json.optString("trigger"),
                action = json.optString("action"),
                profile = json.strOrNull("profile"),
                reason = json.strOrNull("reason"),
            )

            "error" -> Error(json.optString("msg"))
            "pong" -> Pong
            "bye" -> Bye
            else -> Unknown(json)
        }
    }
}
