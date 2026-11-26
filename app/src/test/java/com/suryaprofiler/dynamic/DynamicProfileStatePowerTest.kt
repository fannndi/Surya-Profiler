package com.suryaprofiler.dynamic

import com.suryaprofiler.protocol.PowerBucket
import com.suryaprofiler.protocol.PowerPeaks
import com.suryaprofiler.protocol.PowerSample
import com.suryaprofiler.protocol.PowerSnapshot
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class DynamicProfileStatePowerTest {
    private fun sample(t: Long = 1_000) = PowerSample(
        tMs = t,
        pct = 80,
        status = "discharging",
        currentUa = 100_000,
        voltageUv = 4_000_000,
        tempC = 30f,
        chargeCounterUah = null,
        chargeFullMah = null,
        cycleCount = null,
        inputSuspend = null,
        chargeEnabled = null,
        input = null,
        screenOn = true,
        pkg = "app",
        profile = "game",
    )

    private fun bucket(segmentId: String?, durMs: Long = 12_000) = PowerBucket(
        tMs = 1_000,
        pct = 80,
        avgUa = 100_000,
        minUa = 100_000,
        maxUa = 100_000,
        avgUv = 4_000_000,
        avgTempC = 30f,
        maxTempC = 31f,
        screenOn = true,
        durMs = durMs,
        samples = 1,
        pkg = "app",
        profile = "game",
        segmentId = segmentId,
    )

    @Before fun reset() = DynamicProfileState.resetPower()

    @After fun resetAfter() {
        DynamicProfileState.setDaemonConnected(false)
        DynamicProfileState.resetPower()
    }

    @Test fun malformedSnapshotDoesNotReplaceLiveState() {
        DynamicProfileState.onPowerSnapshot(
            PowerSnapshot(3600, sample(), null, emptyList(), PowerPeaks()),
        )
        DynamicProfileState.onPowerSnapshot(
            PowerSnapshot(0, null, null, emptyList(), PowerPeaks(), isValid = false),
        )
        assertEquals(3600L, DynamicProfileState.powerSnapshot.value?.windowS)
        assertEquals(1_000L, DynamicProfileState.powerLive.value?.tMs)
    }

    @Test fun distinctSameMinuteLiveSegmentsAreRetained() {
        DynamicProfileState.onPowerLive(sample(), bucket("segment-1"))
        DynamicProfileState.onPowerLive(sample(2_000), bucket("segment-2", 48_000))
        assertEquals(listOf("segment-1", "segment-2"), DynamicProfileState.powerLiveBuckets.value.map { it.segmentId })
    }

    @Test fun cumulativeUpdatesForOneSegmentReplaceInsteadOfDoubleCounting() {
        DynamicProfileState.onPowerLive(sample(), bucket("segment-1", 12_000))
        DynamicProfileState.onPowerLive(sample(2_000), bucket("segment-1", 60_000))
        val rows = DynamicProfileState.powerLiveBuckets.value
        assertEquals(1, rows.size)
        assertEquals(60_000L, rows.single().durMs)
    }

    @Test fun daemonDisconnectResetsSnapshotAndLivePower() {
        DynamicProfileState.setDaemonConnected(true)
        DynamicProfileState.onPowerSnapshot(
            PowerSnapshot(3600, sample(), bucket("segment-1"), emptyList(), PowerPeaks()),
        )
        DynamicProfileState.setDaemonConnected(false)
        assertFalse(DynamicProfileState.daemonConnected.value)
        assertNull(DynamicProfileState.powerSnapshot.value)
        assertNull(DynamicProfileState.powerLive.value)
        assertTrue(DynamicProfileState.powerLiveBuckets.value.isEmpty())
    }
}
