package com.suryaprofiler.ui

import com.suryaprofiler.protocol.PowerBucket
import com.suryaprofiler.protocol.PowerSample
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PowerModelTest {
    private fun bucket(
        t: Long,
        avgUa: Long,
        avgUv: Long = 4_000_000,
        dur: Long = 60_000,
        pct: Int? = null,
        screenOn: Boolean = true,
        pkg: String? = null,
        profile: String? = null,
        minUa: Long = avgUa,
        maxUa: Long = avgUa,
        avgTemp: Float? = null,
        maxTemp: Float? = null,
    ) = PowerBucket(
        tMs = t,
        pct = pct,
        avgUa = avgUa,
        minUa = minUa,
        maxUa = maxUa,
        avgUv = avgUv,
        avgTempC = avgTemp,
        maxTempC = maxTemp,
        screenOn = screenOn,
        durMs = dur,
        samples = 1,
        pkg = pkg,
        profile = profile,
    )

    private fun sample(
        currentUa: Long? = null,
        voltageUv: Long? = 4_000_000,
        pct: Int? = 80,
        status: String? = "discharging",
        counter: Long? = null,
        fullMah: Int? = null,
        inputSuspend: Boolean? = false,
        chargeEnabled: Boolean? = true,
    ) = PowerSample(
        tMs = 1,
        pct = pct,
        status = status,
        currentUa = currentUa,
        voltageUv = voltageUv,
        tempC = 32.0f,
        chargeCounterUah = counter,
        chargeFullMah = fullMah,
        cycleCount = 34,
        inputSuspend = inputSuspend,
        chargeEnabled = chargeEnabled,
        input = null,
        screenOn = true,
        pkg = null,
        profile = null,
    )

    @Test fun powerIsSignedBatterySide() {
        assertEquals(-4_000.0, powerMw(-1_000_000, 4_000_000)!!, 0.01)
        assertEquals(4_000.0, powerMw(1_000_000, 4_000_000)!!, 0.01)
        assertNull(powerMw(null, 4_000_000))
        assertNull(powerMw(1_000_000, 0))
    }

    @Test fun formatsCurrentAndPower() {
        assertEquals("1.24 A", formatCurrent(-1_240_000))
        assertEquals("842 mA", formatCurrent(842_000))
        assertEquals("5.12 W", formatPower(-5_120.0))
        assertEquals("412 mW", formatPower(412.0))
        assertEquals("4.12 V", formatVoltage(4_120_000))
        assertEquals("32.4 °C", formatPowerTemp(32.4f))
    }

    @Test fun directionAndLabelsFollowTheSign() {
        assertEquals("charging", powerDirection(sample(-1_000_000, status = "discharging")))
        assertEquals("usage", powerDirection(sample(800_000)))
        assertEquals("idle", powerDirection(sample(5_000)))
        assertEquals("Not charging", statusLabel("not_charging"))
        assertEquals("—", statusLabel(null))
    }

    @Test fun warningsExplainBypassGuardAndMismatch() {
        assertTrue(powerWarnings(sample(800_000, status = "charging")).isNotEmpty())
        assertTrue(powerWarnings(sample(-500_000, inputSuspend = true)).any { it.contains("bypass") })
        assertTrue(
            powerWarnings(sample(-500_000, chargeEnabled = false)).any { it.contains("guard") },
        )
        assertTrue(
            powerWarnings(sample(null, voltageUv = null)).any { it.contains("unavailable") },
        )
    }

    @Test fun estimatesChargeAndDischargeTime() {
        val charging = sample(
            currentUa = -2_000_000,
            status = "charging",
            pct = 80,
            counter = 4_000_000,
            fullMah = 5_000,
        )
        assertEquals("30m", estimateToFull(charging))
        val empty = sample(currentUa = 1_000_000, pct = 50, counter = null)
        assertEquals("2h 34m", estimateToEmpty(empty))
        assertNull(estimateToFull(sample(1_000_000)))
    }

    @Test fun mergesLiveOverHistoryAndClipsWindow() {
        val t0 = 1_700_000_000_000
        val hist = bucket(t0, 100_000)
        val live = bucket(t0, -100_000).copy(tMs = t0)
        val merged = mergeBuckets(listOf(hist), listOf(live, bucket(t0 + 60_000, 200_000)))
        assertEquals(2, merged.size)
        assertEquals(-100_000, merged[0].avgUa)
        val points = powerPoints(merged, emptyList(), 3600, t0 + 120_000)
        assertEquals(2, points.size)
        assertEquals(-400.0, points[0].mw, 0.01)
        val clipped = powerPoints(merged, emptyList(), 1, t0 + 120_000)
        assertTrue(clipped.isEmpty())
    }

    @Test fun chartRangeIncludesZero() {
        val usage = listOf(PowerPoint(0, 500.0, true), PowerPoint(1, 1500.0, true))
        val r = chartRange(usage)
        assertTrue(r.minMw < 0)
        assertTrue(r.maxMw > 1500.0)
        val charging = listOf(PowerPoint(0, -2000.0, true))
        assertTrue(chartRange(charging).maxMw > 0)
    }

    @Test fun appStatsWeightByDurationAndSkipScreenOff() {
        val t0 = 1_700_000_000_000
        val rows = listOf(
            bucket(t0, -1_000_000, pct = 80, pkg = "a", avgTemp = 30f, maxTemp = 31f),
            bucket(t0 + 60_000, -2_000_000, pct = 79, pkg = "a", avgTemp = 34f, maxTemp = 35f),
            bucket(t0 + 120_000, 500_000, pct = 78, screenOn = false, pkg = "a"),
            bucket(t0 + 180_000, 700_000, pct = 77, screenOn = true, pkg = null),
        )
        val stats = appStats(rows)
        assertEquals(1, stats.size)
        assertEquals("a", stats[0].pkg)
        assertEquals(120_000L, stats[0].durMs)
        assertEquals(-1_500_000.0, stats[0].avgUa, 0.01)
        assertEquals(32.0, stats[0].avgTempC ?: 0.0, 0.01)
        assertEquals(35.0f, stats[0].maxTempC)
    }

    @Test fun profileStatsIncludeSleep() {
        val rows = listOf(
            bucket(1, 200_000, profile = "balance"),
            bucket(2, 50_000, screenOn = false, profile = "sleep"),
        )
        val stats = profileStats(rows)
        assertEquals(2, stats.size)
        assertEquals("balance", stats[0].profile)
        assertTrue(stats.any { it.profile == "sleep" })
    }

    @Test fun chargeSessionIntegratesNegativeCurrent() {
        val t0 = 1_700_000_000_000
        val rows = listOf(
            bucket(t0, -1_000_000, pct = 80, profile = "game", minUa = -1_000_000),
            bucket(t0 + 60_000, -2_000_000, pct = 78, profile = "game", minUa = -2_000_000),
        )
        val s = latestChargeSession(rows)!!
        assertTrue(s.ongoing)
        assertEquals(80, s.startPct)
        assertEquals(78, s.endPct)
        assertEquals(50.0, s.addedMah, 0.01)
        assertEquals(8_000.0, s.peakMw, 0.01)
        assertEquals(120_000L, s.durMs)

        val after = rows + bucket(t0 + 120_000, 800_000, pct = 77)
        val done = latestChargeSession(after)!!
        assertEquals(false, done.ongoing)
        assertNotNull(done.endMs)
    }

    @Test fun batteryHealthUsesDeviceDesignCapacity() {
        val s = sample(chargeEnabled = true, fullMah = 5_008)
        assertEquals("5008 mAh · 97% of 5160 · 34 cycles", batteryHealthLabel(s))
    }
}
