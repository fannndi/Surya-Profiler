package com.suryaprofiler.protocol

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ProfileIdTest {
    @Test fun userProfilesExcludeSleep() {
        assertEquals(listOf("powersave", "balance", "game"), ProfileId.USER.map { it.wire })
        assertTrue(ProfileId.SLEEP in ProfileId.ALL)
    }

    @Test fun parsesWireValues() {
        assertEquals(ProfileId.GAME, ProfileId.from("game"))
        assertNull(ProfileId.from("boost"))
        assertNull(ProfileId.from(null))
    }

    @Test(expected = IllegalArgumentException::class)
    fun requireRejectsUnknown() {
        ProfileId.require("boost")
    }
}
