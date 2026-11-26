package com.suryaprofiler.protocol

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PowerParseTest {
    private val bucketJson = """
        {"t_ms":1700000000000,"pct":78,"avg_ua":-1240000,"min_ua":-1600000,
         "max_ua":-900000,"avg_uv":4120000,"avg_temp_c":32.4,"max_temp_c":33.1,
         "screen_on":true,"dur_ms":60000,"samples":12,
         "pkg":"com.foo","profile":"game"}
    """.trimIndent()

    @Test fun parsesSnapshotWithLiveHistoryAndPeaks() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"power","snapshot":{"window_s":3600,
                   "live":{"t_ms":1700000000500,"pct":78,"status":"charging",
                     "current_ua":-1240000,"voltage_uv":4120000,"temp_c":32.4,
                     "charge_counter_uah":3980000,"charge_full_mah":5008,
                     "cycle_count":34,"input_suspend":false,"charge_enabled":true,
                     "input":{"name":"usb","online":true,"voltage_uv":4748096,
                       "current_ua":1317690,"kind":"USB_CDP"},
                     "screen_on":true,"pkg":"com.foo","profile":"game"},
                   "current_bucket":$bucketJson,
                   "history":[$bucketJson],
                   "peaks":{"charge":{"ua":-2000000,"uv":4100000,"t_ms":1,"pct":60},
                     "discharge":{"ua":800000,"uv":4000000,"t_ms":2,"pct":70},
                     "temp":{"c":38.5,"t_ms":3,"pct":60}}}}""",
            ),
        ) as DaemonEvent.Power
        val snap = ev.snapshot
        assertEquals(3600L, snap.windowS)
        assertEquals(78, snap.live?.pct)
        assertEquals("charging", snap.live?.status)
        assertEquals(-1_240_000L, snap.live?.currentUa)
        assertFalse(snap.live?.inputSuspend ?: true)
        assertEquals("usb", snap.live?.input?.name)
        assertEquals(4_748_096L, snap.live?.input?.voltageUv)
        assertEquals("USB_CDP", snap.live?.input?.kind)
        assertEquals(1, snap.history.size)
        assertEquals(12, snap.history[0].samples)
        assertEquals("com.foo", snap.history[0].pkg)
        assertEquals(78, snap.currentBucket?.pct)
        assertNull(snap.history[0].startMs)
        assertNull(snap.history[0].endMs)
        assertNull(snap.history[0].energyMwh)
        assertNull(snap.history[0].segmentId)
        assertEquals(-2_000_000L, snap.peaks.charge?.ua)
        assertEquals(800_000L, snap.peaks.discharge?.ua)
        assertEquals(38.5f, snap.peaks.temp?.c)
    }

    @Test fun parsesLiveWithOptionalBucket() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"power_live","sample":{"t_ms":10,"current_ua":800000,
                   "voltage_uv":4000000,"screen_on":true},"bucket":$bucketJson}""",
            ),
        ) as DaemonEvent.PowerLive
        assertEquals(800_000L, ev.sample.currentUa)
        assertEquals(12, ev.bucket?.samples)
        assertNull(ev.sample.pct)
    }

    @Test fun liveWithoutSampleIsUnknown() {
        val ev = DaemonEvent.parse(JSONObject("""{"event":"power_live","bucket":$bucketJson}"""))
        assertTrue(ev is DaemonEvent.Unknown)
    }

    @Test fun parsesOptionalBucketSegmentFields() {
        val json = JSONObject(bucketJson)
            .put("start_ms", 1_699_999_999_000)
            .put("end_ms", 1_700_000_005_000)
            .put("energy_mwh", -66.5)
            .put("segment_id", "power-1-2")
        val b = PowerParse.bucket(json)!!
        assertEquals(1_699_999_999_000L, b.startMs)
        assertEquals(1_700_000_005_000L, b.endMs)
        assertEquals(-66.5, b.energyMwh!!, 0.000_001)
        assertEquals("power-1-2", b.segmentId)
    }

    @Test fun malformedBucketIsRejectedInsteadOfBecomingZeros() {
        val missing = JSONObject(bucketJson).apply { remove("avg_ua") }
        assertNull(PowerParse.bucket(missing))
        val wrongType = JSONObject(bucketJson).put("dur_ms", "60000")
        assertNull(PowerParse.bucket(wrongType))
        val negative = JSONObject(bucketJson).put("samples", -1)
        assertNull(PowerParse.bucket(negative))
    }

    @Test fun malformedLiveSampleIsUnknownAndMalformedBucketIsOptional() {
        val missingSampleTime = JSONObject("""{"event":"power_live","sample":{"screen_on":true},"bucket":$bucketJson}""")
        assertTrue(DaemonEvent.parse(missingSampleTime) is DaemonEvent.Unknown)
        val missingBucketCurrent = JSONObject(bucketJson).apply { remove("avg_ua") }
        val ev = DaemonEvent.parse(
            JSONObject("""{"event":"power_live","sample":{"t_ms":10,"screen_on":true}}""")
                .put("bucket", missingBucketCurrent),
        ) as DaemonEvent.PowerLive
        assertNull(ev.bucket)
    }

    @Test fun missingSnapshotIsInvalidRatherThanAnEmptyZeroSnapshot() {
        val snap = PowerParse.snapshot(JSONObject("""{"event":"power"}"""))
        assertFalse(snap.isValid)
        assertTrue(snap.history.isEmpty())
    }

    @Test fun clearReplyIsAnEmptySnapshot() {
        val ev = DaemonEvent.parse(
            JSONObject("""{"event":"power","snapshot":{"window_s":86400,"history":[]}}"""),
        ) as DaemonEvent.Power
        assertTrue(ev.snapshot.history.isEmpty())
        assertNull(ev.snapshot.live)
        assertNull(ev.snapshot.peaks.charge)
    }

    @Test fun powerCommandsCarryWatchAndWindow() {
        val cmd = DaemonCommands.power(watch = true, windowS = 3600)
        assertEquals("power", cmd.getString("cmd"))
        assertTrue(cmd.getBoolean("watch"))
        assertEquals(3600L, cmd.getLong("window_s"))
        assertEquals("power_clear", DaemonCommands.powerClear().getString("cmd"))
    }
}
