package com.gathin.tvhome;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotEquals;
import static org.junit.Assert.assertTrue;

public class PinTest {
    @Test public void onlyFourToEightDigits() {
        assertTrue(Pin.valid("1234"));
        assertTrue(Pin.valid("12345678"));
        assertFalse(Pin.valid("123"));
        assertFalse(Pin.valid("123456789"));
        assertFalse(Pin.valid("12a4"));
        assertFalse(Pin.valid(null));
    }

    @Test public void hashIsSaltedAndStable() {
        String a = Pin.hash("00ff", "1234");
        assertEquals(64, a.length());
        assertEquals(a, Pin.hash("00ff", "1234"));
        assertNotEquals(a, Pin.hash("00fe", "1234"));
        assertNotEquals(a, Pin.hash("00ff", "1235"));
    }

    @Test public void profileIds() {
        assertTrue(Profiles.validId("kids"));
        assertTrue(Profiles.validId("guest-2"));
        assertFalse(Profiles.validId("../x"));
        assertFalse(Profiles.validId("Kids"));
        assertFalse(Profiles.validId(""));
    }
}
