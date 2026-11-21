package com.suryaprofiler.dynamic

import com.suryaprofiler.core.ApplyReport

/**
 * The single ordering for Service OFF:
 *
 *   persist restore_pending -> restore stock -> confirm OFF (or abort).
 *
 * The side effects are injected as functions so the state machine is
 * JVM-testable and the Home and service paths cannot drift.
 */
class ServiceOffCoordinator(
    private val begin: () -> Boolean,
    private val abort: () -> Boolean,
    private val confirm: () -> Boolean,
    private val clearSessionFlag: () -> Unit,
    private val restore: suspend () -> ApplyReport?,
) {
    sealed interface Outcome {
        data class Success(val report: ApplyReport) : Outcome
        data object Refused : Outcome
        data object Cancelled : Outcome
        data class RestoreFailed(val report: ApplyReport?) : Outcome
        data class ConfirmFailed(val report: ApplyReport) : Outcome
    }

    /**
     * `stillWanted` is checked after the restore: a user who toggled the
     * service back ON cancels the OFF instead of being overwritten.
     */
    suspend fun run(stillWanted: () -> Boolean): Outcome {
        if (!begin()) return Outcome.Refused

        val report = restore()
        if (!stillWanted()) {
            abort()
            return Outcome.Cancelled
        }
        if (report?.ok != true) {
            abort()
            return Outcome.RestoreFailed(report)
        }

        // The R&D switch is per-session: once stock is back it must not
        // silently re-suspend charger input on the next ON.
        clearSessionFlag()
        if (!confirm()) return Outcome.ConfirmFailed(report)
        return Outcome.Success(report)
    }
}
