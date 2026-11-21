package com.suryaprofiler.protocol

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DaemonEventTest {
    @Test fun parsesState() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"state","state":{"screen_on":true,"locked":false,
                   "active":"balance","reason":"base"}}""",
            ),
        ) as DaemonEvent.State
        assertTrue(ev.state.screenOn)
        assertEquals("balance", ev.state.active)
        assertEquals("base", ev.state.reason)
        assertNull(ev.state.srcPkg)
    }

    @Test fun parsesApplied() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"applied","profile":"game","reason":"app",
                   "src_pkg":"com.stove.epic7.google","ok":true,"applied":true,
                   "wrote":4,"verified":4,"failed":0,"ms":120,"settle_ms":400}""",
            ),
        ) as DaemonEvent.Applied
        assertEquals("game", ev.profile)
        assertEquals(4, ev.wrote)
        assertTrue(ev.ok)
    }

    @Test fun parsesRestoredHoldFailure() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"restored","ok":false,"wrote":0,"verified":0,"failed":0,
                   "saver_ok":false,"input_suspend_ok":true,"thermal_ok":true}""",
            ),
        ) as DaemonEvent.Restored
        assertEquals(false, ev.ok)
        assertEquals(false, ev.saverOk)
    }

    @Test fun parsesEnvAndDiag() {
        val env = DaemonEvent.parse(
            JSONObject("""{"event":"env","env":{"battery_pct":80,"charging":true}}"""),
        ) as DaemonEvent.Env
        assertEquals(80, env.env.batteryPct)
        assertEquals(true, env.env.charging)

        val diag = DaemonEvent.parse(
            JSONObject(
                """{"event":"diag","diag":{"pid":7,"holds":{"saver_held":true},
                   "config":{"enabled":true,"dynamic":true,"base_profile":"balance"},
                   "env":{"battery_pct":79}}}""",
            ),
        ) as DaemonEvent.Diag
        assertEquals(7, diag.diag?.pid)
        assertEquals(true, diag.diag?.saverHeld)
        assertEquals(79, diag.env?.batteryPct)
    }

    @Test fun parsesUnknownWithoutCrashing() {
        val ev = DaemonEvent.parse(JSONObject("""{"event":"future_thing","x":1}"""))
        assertTrue(ev is DaemonEvent.Unknown)
    }

    @Test fun parsesStatsWithHeals() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"stats","entries":[{"t":1,"to":"game","reason":"app"}],
                   "heals_total":2,"heals_last_t":3,"heals_last_keys":4}""",
            ),
        ) as DaemonEvent.Stats
        assertEquals(1, ev.entries.size)
        assertEquals(2L, ev.heals.total)
        assertEquals(4, ev.heals.lastKeys)
    }

    @Test fun restoredCarriesHoldSurfaces() {
        val ev = DaemonEvent.parse(
            JSONObject(
                """{"event":"restored","ok":false,"wrote":1,"verified":1,"failed":0,
                   "saver_ok":true,"input_suspend_ok":false,"thermal_ok":true}""",
            ),
        ) as DaemonEvent.Restored
        assertEquals(false, ev.ok)
        assertEquals(false, ev.inputSuspendOk)
    }
}
