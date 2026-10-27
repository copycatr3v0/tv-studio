package com.gathin.tvhome;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class BootHomeTest {
    private static final String OWN = "studio.tvcustomizer.home";
    private static final String GTV = "com.google.android.apps.tv.launcherx";

    @Test public void powerOnSendsAnyStockLauncherScreenHomeUntilAKeyIsPressed() {
        assertTrue(HomeKeyService.bootRedirect(GTV, GTV, 40_000, HomeKeyService.BOOT_WINDOW_MS));
        assertFalse(HomeKeyService.bootRedirect(GTV, "com.netflix.ninja", 40_000, HomeKeyService.BOOT_WINDOW_MS));
        assertFalse(HomeKeyService.bootRedirect(GTV, GTV, 40_000, 0));                               // a key was pressed
        assertFalse(HomeKeyService.bootRedirect(GTV, GTV, HomeKeyService.BOOT_WINDOW_MS + 1, HomeKeyService.BOOT_WINDOW_MS));
        assertFalse(HomeKeyService.bootRedirect("", GTV, 40_000, HomeKeyService.BOOT_WINDOW_MS));
    }
}
