package com.gathin.tvhome;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.util.HashMap;
import java.util.Map;

import org.junit.Test;

public class TvChoicesTest {
    private static final String WITH_ROWS = "{\"version\":1,\"enabled\":true,\"home\":\"classic\",\"items\":["
            + "{\"id\":\"clock-1\",\"kind\":\"clock\",\"enabled\":true,\"x\":5,\"y\":5,\"w\":20,\"h\":8,\"style\":{\"font\":\"sans-serif\",\"size\":40,\"color\":\"#ffffff\",\"opacity\":1,\"radius\":0,\"fill\":null,\"align\":\"start\"},\"motion\":{\"type\":\"none\",\"speed\":1,\"amount\":0},\"options\":{\"twentyFour\":false}},"
            + "{\"id\":\"recent-1\",\"kind\":\"recent\",\"enabled\":false,\"x\":5,\"y\":60,\"w\":90,\"h\":25,\"style\":{\"font\":\"sans-serif\",\"size\":20,\"color\":\"#ffffff\",\"opacity\":1,\"radius\":0,\"fill\":null,\"align\":\"start\"},\"motion\":{\"type\":\"none\",\"speed\":1,\"amount\":0}},"
            + "{\"id\":\"apps-1\",\"kind\":\"apps\",\"enabled\":false,\"x\":5,\"y\":60,\"w\":90,\"h\":30,\"style\":{\"font\":\"sans-serif\",\"size\":20,\"color\":\"#ffffff\",\"opacity\":1,\"radius\":0,\"fill\":null,\"align\":\"start\"},\"motion\":{\"type\":\"none\",\"speed\":1,\"amount\":0}}],"
            + "\"background\":{\"kind\":\"color\",\"color\":\"#101010\",\"color2\":\"#202020\",\"video\":null,\"dim\":0,\"blur\":0}}";

    private static Map<String, String> c(String... kv) {
        Map<String, String> m = new HashMap<>();
        for (int i = 0; i < kv.length; i += 2) m.put(kv[i], kv[i + 1]);
        return m;
    }

    @Test public void appleTvFromTheTvSwitchesTheLookAndTurnsTheDockOn() {
        LayoutSpec s = LayoutSpec.parse(WITH_ROWS);
        TvChoices.apply(s, c(TvChoices.HOME, "tvos"));
        assertTrue(s.tvos());
        assertEquals("tvos", s.tiles);
        assertEquals("parallax", s.focus);
        for (LayoutSpec.Item it : s.items) assertTrue(it.kind + " on", it.enabled);
    }

    @Test public void choicesLayOverStudiosAndBadValuesAreIgnored() {
        LayoutSpec s = LayoutSpec.parse(WITH_ROWS);
        TvChoices.apply(s, c(TvChoices.SCENE, "snow", TvChoices.MOTION, "off", TvChoices.UNITS, "f", TvChoices.CLOCK, "24",
                TvChoices.FOCUS, "spin", TvChoices.TILES, "poster"));
        assertEquals("scene", s.bgKind);
        assertEquals("snow", s.scene);
        assertTrue(s.still);
        assertTrue(s.fahrenheit);
        assertTrue(s.items.get(0).twentyFour);
        assertEquals("pop", s.focus);      // "spin" is not a focus effect
        assertEquals("poster", s.tiles);
    }

    @Test public void scrollNeedsARecentRowAndAGrid() {
        String noRows = WITH_ROWS.replace("\"kind\":\"recent\"", "\"kind\":\"date\"");
        LayoutSpec s = LayoutSpec.parse(noRows);
        assertFalse(TvChoices.canScroll(s));
        TvChoices.apply(s, c(TvChoices.HOME, "scroll"));
        assertTrue(s.classic());
    }

    @Test public void theGearTakesTheFirstFreeCorner() {
        java.util.List<double[]> none = new java.util.ArrayList<>();
        assertEquals(92.5, CanvasHome.gearSpot(none, false)[0], 0.01);
        assertEquals(78.5, CanvasHome.gearSpot(none, true)[0], 0.01);
        java.util.List<double[]> clockTopRight = new java.util.ArrayList<>();
        clockTopRight.add(new double[]{72, 5, 24, 12});
        double[] s = CanvasHome.gearSpot(clockTopRight, false);
        assertEquals(94.5, s[0], 0.01);
        assertEquals(91, s[1], 0.01);
    }

    @Test public void switchingToScrollOnTheTvKeepsWidgetsFromUnderTheAppRow() {
        String withLowClock = WITH_ROWS.replace("\"x\":5,\"y\":5,\"w\":20,\"h\":8", "\"x\":5,\"y\":75,\"w\":20,\"h\":8");
        LayoutSpec s = LayoutSpec.parse(withLowClock);
        TvChoices.apply(s, c(TvChoices.HOME, "scroll"));
        assertTrue(s.scroll());
        assertFalse("the clock would sit under the apps", s.items.get(0).enabled);
        assertEquals(63, s.items.get(1).y, 0.01);
        LayoutSpec top = LayoutSpec.parse(WITH_ROWS);
        TvChoices.apply(top, c(TvChoices.HOME, "scroll"));
        assertTrue("a clock at the top stays", top.items.get(0).enabled);
    }

    @Test public void switchingToClassicOnTheTvHidesWidgetsOnClassicsButtons() {
        String scroll = WITH_ROWS.replace("\"home\":\"classic\"", "\"home\":\"scroll\"")
                .replace("\"x\":5,\"y\":5,\"w\":20,\"h\":8", "\"x\":70,\"y\":13,\"w\":26,\"h\":8");
        LayoutSpec s = LayoutSpec.parse(scroll);
        TvChoices.apply(s, c(TvChoices.HOME, "classic"));
        assertTrue(s.classic());
        assertFalse("on Classic's Settings button", s.items.get(0).enabled);
    }
}
