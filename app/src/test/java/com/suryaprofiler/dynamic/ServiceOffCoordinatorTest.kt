package com.suryaprofiler.dynamic

import com.suryaprofiler.core.ApplyReport
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ServiceOffCoordinatorTest {
    private fun report(ok: Boolean) = ApplyReport(
        mode = "restore",
        profile = null,
        wrote = 1,
        unchanged = 0,
        verified = 1,
        failed = if (ok) 0 else 1,
        locked = emptyList(),
        results = emptyList(),
        ok = ok,
        snapshotCreated = false,
        active = null,
    )

    private class Fakes {
        var begins = 0
        var aborts = 0
        var confirms = 0
        var cleared = 0
        var beginOk = true
        var confirmOk = true
        var restoreReport: ApplyReport? = null
    }

    private fun coordinator(f: Fakes, restore: suspend () -> ApplyReport? = { f.restoreReport }) =
        ServiceOffCoordinator(
            begin = {
                f.begins++
                f.beginOk
            },
            abort = {
                f.aborts++
                true
            },
            confirm = {
                f.confirms++
                f.confirmOk
            },
            clearSessionFlag = { f.cleared++ },
            restore = restore,
        )

    @Test
    fun successRunsInOrderAndClearsTheSessionFlag() = runBlocking {
        val f = Fakes().apply { restoreReport = report(true) }
        val out = coordinator(f).run { true }
        assertTrue(out is ServiceOffCoordinator.Outcome.Success)
        assertEquals(1, f.begins)
        assertEquals(0, f.aborts)
        assertEquals(1, f.confirms)
        assertEquals(1, f.cleared)
    }

    @Test
    fun restoreFailureAbortsAndNeverConfirms() = runBlocking {
        val f = Fakes().apply { restoreReport = report(false) }
        val out = coordinator(f).run { true }
        assertTrue(out is ServiceOffCoordinator.Outcome.RestoreFailed)
        assertEquals(1, f.aborts)
        assertEquals(0, f.confirms)
        assertEquals(0, f.cleared)
    }

    @Test
    fun aUserTurningServiceBackOnCancelsTheOff() = runBlocking {
        val f = Fakes().apply { restoreReport = report(true) }
        val out = coordinator(f).run { false }
        assertTrue(out is ServiceOffCoordinator.Outcome.Cancelled)
        assertEquals(1, f.aborts)
        assertEquals(0, f.confirms)
    }

    @Test
    fun beginAndConfirmFailuresAreSurfaced() = runBlocking {
        val beginFail = Fakes().apply { beginOk = false }
        assertTrue(coordinator(beginFail).run { true } is ServiceOffCoordinator.Outcome.Refused)
        assertEquals(0, beginFail.confirms)

        val confirmFail = Fakes().apply {
            restoreReport = report(true)
            confirmOk = false
        }
        assertTrue(
            coordinator(confirmFail).run { true } is ServiceOffCoordinator.Outcome.ConfirmFailed,
        )
        assertEquals(0, confirmFail.aborts)
    }
}
