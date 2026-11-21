package com.suryaprofiler.core

import com.suryaprofiler.protocol.ProfileId
import org.json.JSONObject

/**
 * Typed client for the `surya-prof` binary (all commands answer JSON on stdout).
 *
 * Responsibility: command dispatch + JSON parsing.
 * Non-goals: process lifecycle (RootBridge), UI state (ViewModel).
 */
class FtClient(private val bridge: RootBridge) {

    class FtException(message: String) : Exception(message)

    private val knownProfiles = ProfileId.USER.map { it.wire }.toSet()

    /** Runs `surya-prof <args>`; throws [FtException] on non-JSON or crash. */
    fun exec(args: String): JSONObject {
        val r = bridge.sh("${RootBridge.BIN} $args")
        val raw = r.out.trim()
        val start = raw.indexOf('{')
        if (start < 0) {
            throw FtException("surya-prof $args -> exit ${r.code}: ${raw.ifEmpty { "(no output)" }}")
        }
        // stderr noise (if any) precedes the JSON payload; parse from first '{'
        val json = try {
            JSONObject(raw.substring(start))
        } catch (e: Exception) {
            throw FtException("surya-prof $args -> bad JSON: ${e.message}\n$raw")
        }
        return json
    }

    fun status(): Status = parseStatus(exec("status"))

    fun plan(profileId: String): Plan {
        require(profileId in knownProfiles) { "unknown profile '$profileId'" }
        return parsePlan(exec("plan $profileId"))
    }

    fun apply(profileId: String): ApplyReport {
        require(profileId in knownProfiles) { "unknown profile '$profileId'" }
        return parseReport(exec("apply $profileId"))
    }

    fun restore(): ApplyReport = parseReport(exec("restore"))

    /** Releases saver/input-suspend/thermal holds without a daemon. */
    fun clean(): Boolean = exec("clean").optBoolean("ok", false)
}
