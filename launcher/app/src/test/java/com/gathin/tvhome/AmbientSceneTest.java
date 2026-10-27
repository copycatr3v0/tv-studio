package com.gathin.tvhome;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

public class AmbientSceneTest {
    @Test public void partOfDayFollowsTheHour() {
        assertEquals("night", AmbientScene.phaseFor(2));
        assertEquals("sunrise", AmbientScene.phaseFor(6));
        assertEquals("day", AmbientScene.phaseFor(12));
        assertEquals("sunset", AmbientScene.phaseFor(18));
        assertEquals("night", AmbientScene.phaseFor(21));
    }

    @Test public void everySceneHasAnOpaqueSkyForEveryPart() {
        for (String s : AmbientScene.SCENES) {
            assertTrue(AmbientScene.particles(s) > 0);
            for (String t : new String[]{"sunrise", "day", "sunset", "night"}) {
                int[] c = AmbientScene.skyColors(s, t);
                assertEquals(2, c.length);
                assertEquals(0xFF, c[0] >>> 24);
                assertEquals(0xFF, c[1] >>> 24);
            }
        }
    }

    @Test public void mixAndNames() {
        assertEquals(0xFF7F7F7F, AmbientScene.mix(0xFF000000, 0xFFFEFEFE, 0.5f));
        assertTrue(AmbientScene.valid("aurora", AmbientScene.SCENES));
        assertFalse(AmbientScene.valid("lava", AmbientScene.SCENES));
        assertFalse(AmbientScene.valid(null, AmbientScene.TIMES));
    }

    @Test public void lowRamLimit() {
        assertTrue(LowRam.isLow(1_000_000_000L));
        assertFalse(LowRam.isLow(2_000_000_000L));
        assertFalse(LowRam.isLow(0));
    }
}
