package com.suryaprofiler.dynamic

import com.suryaprofiler.core.ApplyReport
import com.suryaprofiler.core.Tuner
import com.suryaprofiler.protocol.DaemonCommands
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.withTimeoutOrNull

/**
 * The single Service OFF restore path.
 *
 * Uses the running daemon when available, otherwise the CLI (which also
 * releases every persisted bridge hold). A failed/unconfirmed restore returns
 * a report with `ok=false`; callers must keep the service ON.
 */
object StockRestore {
    private const val TIMEOUT_MS = 15_000L

    suspend fun restore(): ApplyReport? {
        if (!DaemonLink.connected()) {
            return runCatching { Tuner.restore() }.getOrNull()
        }
        var event = restoreViaDaemon()
        if (event?.ok != true) {
            // Transient race with MIUI writes / a dropped reply: one retry.
            kotlinx.coroutines.delay(1_500)
            event = restoreViaDaemon() ?: event
        }
        return event?.toReport()
    }

    private suspend fun restoreViaDaemon(): RestoredEvent? {
        val seq = DynamicProfileState.currentRestoredSeq()
        if (!DaemonLink.send(DaemonCommands.restore())) return null
        return withTimeoutOrNull(TIMEOUT_MS) {
            DynamicProfileState.restoredEvents.filterNotNull().first { it.seq > seq }
        }
    }

    private fun RestoredEvent.toReport(): ApplyReport = ApplyReport(
        mode = "restore",
        profile = null,
        wrote = wrote,
        unchanged = 0,
        verified = verified,
        failed = failed,
        locked = emptyList(),
        results = emptyList(),
        ok = ok,
        snapshotCreated = false,
        active = null,
    )
}
