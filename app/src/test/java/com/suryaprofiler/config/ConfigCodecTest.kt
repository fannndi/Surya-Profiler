package com.suryaprofiler.config

import com.suryaprofiler.protocol.ProfileId
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConfigCodecTest {
    @Test fun decodesDefaultsFromEmpty() {
        val cfg = ConfigCodec.decode(JSONObject())
        assertTrue(cfg.enabled)
        assertTrue(cfg.dynamic)
        assertTrue(cfg.customThermal)
        assertTrue(cfg.freqFloorAudit)
        assertFalse(cfg.threadAffinity)
        assertEquals(null, cfg.logLevel)
        assertEquals(ProfileId.BALANCE, cfg.baseProfile)
        assertFalse(cfg.restorePending)
    }

    @Test fun foldsLegacySchemaOneMapping() {
        val cfg = ConfigCodec.decode(
            JSONObject(
                """{"schema":1,"enabled":false,"app_profiles":{
                   "com.p":{"profile":"game"}},"app_map":{"com.p":"powersave"}}""",
            ),
        )
        assertEquals(ProfileId.GAME, cfg.appMap["com.p"])
        assertEquals(false, cfg.enabled)
    }

    @Test fun ignoresUnknownProfilesAndSleepAsBase() {
        val cfg = ConfigCodec.decode(
            JSONObject(
                """{"base_profile":"sleep","app_map":{
                   "a":"boost","b":"sleep","c":"game"}}""",
            ),
        )
        assertEquals(ProfileId.BALANCE, cfg.baseProfile)
        assertEquals(mapOf("c" to ProfileId.GAME), cfg.appMap)
    }

    @Test fun roundTripsCanonicalShape() {
        val cfg = AppConfig(
            enabled = false,
            dynamic = false,
            customThermal = true,
            baseProfile = ProfileId.GAME,
            appMap = mapOf("com.p" to ProfileId.POWERSAVE),
            rndInputSuspend = false,
            restorePending = true,
            freqFloorAudit = false,
            threadAffinity = true,
            logLevel = "debug",
        )
        val json = ConfigCodec.encode(cfg)
        assertEquals(ConfigCodec.SCHEMA, json.getInt("schema"))
        assertEquals(ProfileId.GAME, ConfigCodec.decode(json).baseProfile)
        assertTrue(ConfigCodec.decode(json).restorePending)
        assertFalse(ConfigCodec.decode(json).freqFloorAudit)
        assertTrue(ConfigCodec.decode(json).threadAffinity)
        assertEquals("debug", ConfigCodec.decode(json).logLevel)
        assertEquals("powersave", json.getJSONObject("app_map").getString("com.p"))
    }

    @Test fun omitsNullLogLevelOnWrite() {
        val json = ConfigCodec.encode(AppConfig())
        assertFalse("null log level must not become a string", json.has("log_level"))
        assertTrue(json.getBoolean("freq_floor_audit"))
        assertFalse(json.getBoolean("thread_affinity"))
    }
}
