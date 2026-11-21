package com.suryaprofiler.core

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ModelsTest {
    @Test fun statusToleratesMissingOptionalArrays() {
        val st = parseStatus(JSONObject("""{"active":"balance","root":true}"""))
        assertEquals("balance", st.active)
        assertTrue(st.profiles.isEmpty())
        assertEquals("?", st.device.device)
    }

    @Test fun planParsesStringAndObjectStatus() {
        val plan = parsePlan(
            JSONObject(
                """{"profile_id":"balance","label":"B","ok":true,"ops":[
                   {"key":"k1","path":"/p","tier":"free","wanted":"1","resolved":"1",
                    "status":"unchanged"},
                   {"key":"k2","path":"/q","tier":"free","wanted":"2","resolved":"2",
                    "status":{"locked":"node missing"}}]}""",
            ),
        )
        assertEquals(1, plan.inSync)
        assertEquals(1, plan.locked.size)
        assertEquals("node missing", plan.locked[0].status.reason)
    }

    @Test fun reportToleratesMissingArrays() {
        val report = parseReport(JSONObject("""{"mode":"restore","ok":true,"wrote":3}"""))
        assertEquals(3, report.wrote)
        assertTrue(report.locked.isEmpty())
        assertTrue(report.results.isEmpty())
    }

    @Test fun profilesReadShortSubtitle() {
        val file = parseProfiles(
            JSONObject(
                """{"profiles":[{"id":"balance","label":"Balance","short":"Everyday",
                   "params":{"vm.dirty_ratio":"20"}}]}""",
            ),
        )
        assertEquals("Everyday", file.profiles.single().short)
        assertEquals("20", file.profiles.single().params["vm.dirty_ratio"])
    }

    @Test fun planToleratesMissingStatusField() {
        val plan = parsePlan(
            JSONObject(
                """{"profile_id":"balance","label":"B","ok":false,"ops":[
                   {"key":"k","path":"/p","tier":"free","wanted":"1","resolved":"1"}]}""",
            ),
        )
        assertEquals("unknown", plan.ops.single().status.kind)
    }
}
