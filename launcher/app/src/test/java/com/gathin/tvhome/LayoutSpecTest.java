package com.gathin.tvhome;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

public class LayoutSpecTest {
    private static String item(String id, String extra) {
        return "{\"id\":\"" + id + "\",\"kind\":\"clock\",\"enabled\":true,\"x\":72,\"y\":5,\"w\":24,\"h\":12," + extra
                + "\"style\":{\"font\":\"sans-serif-light\",\"size\":72,\"color\":\"#ffffff\",\"opacity\":1,\"radius\":0,\"fill\":null,\"align\":\"end\"},"
                + "\"motion\":{\"type\":\"float\",\"speed\":1,\"amount\":20},\"options\":{\"seconds\":true,\"max\":4}}";
    }

    private static String layout(String bgVideo, String items) {
        return "{\"version\":1,\"enabled\":true,\"background\":{\"kind\":\"video\",\"color\":\"#0b1020\",\"color2\":\"#27325c\",\"video\":" + bgVideo
                + ",\"dim\":0.3,\"blur\":0},\"items\":[" + items + "]}";
    }

    @Test public void aGoodLayoutParses() {
        LayoutSpec s = LayoutSpec.parse(layout("\"rain\"", item("clock-1", "")));
        assertNotNull(s);
        assertTrue(s.enabled);
        assertEquals("rain", s.video);
        assertEquals(1, s.items.size());
        LayoutSpec.Item it = s.items.get(0);
        assertEquals("sans-serif-light", it.font);
        assertTrue(it.seconds);
        assertEquals(4, it.max);
        assertFalse(it.hasFill);
        assertEquals(0xFFFFFFFF, it.color);
    }

    @Test public void anythingWrongFallsBackToNull() {
        assertNull(LayoutSpec.parse(null));
        assertNull(LayoutSpec.parse("not json"));
        assertNull(LayoutSpec.parse("{}"));
        assertNull(LayoutSpec.parse(layout("null", item("clock-1", "").replace("\"x\":72", "\"x\":90"))));          // off screen
        assertNull(LayoutSpec.parse(layout("null", item("clock-1", "").replace("sans-serif-light", "Papyrus"))));   // font
        assertNull(LayoutSpec.parse(layout("null", item("Clock 1", ""))));                                           // id
        assertNull(LayoutSpec.parse(layout("null", item("a", "") + "," + item("a", ""))));                           // duplicate id
        assertNull(LayoutSpec.parse(layout("\"../../x\"", item("a", ""))));                                          // video path
        assertNull(LayoutSpec.parse(layout("null", item("a", "").replace("\"speed\":1", "\"speed\":99"))));
        assertNull(LayoutSpec.parse(layout("null", item("a", "\"text\":\"bad\\u0007\","))));                          // control character
    }

    @Test public void videoNamesArePlainFiles() {
        assertTrue(LayoutSpec.validVideo("rain"));
        assertTrue(LayoutSpec.validVideo("my_loop.mp4"));
        assertFalse(LayoutSpec.validVideo("a/b.mp4"));
        assertFalse(LayoutSpec.validVideo(".x.mp4"));
        assertFalse(LayoutSpec.validVideo("x.avi"));
        assertEquals("rain.mp4", LayoutSpec.videoFile("rain"));
        assertEquals("my.webm", LayoutSpec.videoFile("my.webm"));
    }

    @Test public void coloursUseTheLayoutsByteOrder() {
        assertEquals(0xFF112233, LayoutSpec.hex("#112233"));
        assertEquals(0x80112233, LayoutSpec.hex("#11223380"));
    }

    @Test public void newKindsLooksAndStyleOptionsParse() {
        String it = "{\"id\":\"x-1\",\"kind\":\"KIND\",\"variant\":\"VAR\",\"enabled\":true,\"x\":10,\"y\":10,\"w\":30,\"h\":20,"
                + "\"style\":{\"font\":\"FONT\",\"size\":300,\"color\":\"#ffffff\",\"opacity\":1,\"radius\":0,\"fill\":\"#00000080\",\"align\":\"center\",\"spacing\":0.3,\"upper\":true,\"shadow\":true,\"italic\":false},"
                + "\"motion\":{\"type\":\"none\",\"speed\":1,\"amount\":0},\"options\":{\"bars\":48,\"stroke\":2,\"weekStartsMonday\":true,\"packages\":[\"com.netflix.ninja\"]}}";
        LayoutSpec s = LayoutSpec.parse(layout("null", it.replace("KIND", "clock").replace("VAR", "stacked").replace("FONT", "file:Mont.ttf")));
        assertNotNull(s);
        LayoutSpec.Item i = s.items.get(0);
        assertEquals("stacked", i.variant);
        assertEquals("Mont.ttf", i.fontFile);
        assertEquals(0.3, i.spacing, 1e-9);
        assertTrue(i.upper && i.shadow && !i.italic && i.mondayFirst);
        assertEquals(48, i.bars);
        assertEquals(1, i.packages.size());
        assertNotNull(LayoutSpec.parse(layout("null", it.replace("KIND", "visualizer").replace("VAR", "dots").replace("FONT", "serif"))));
        assertNotNull(LayoutSpec.parse(layout("null", it.replace("KIND", "shape").replace("VAR", "ring").replace("FONT", "serif"))));
        assertNotNull(LayoutSpec.parse(layout("null", it.replace("KIND", "cpu").replace("VAR", "graph").replace("FONT", "serif"))));
    }

    @Test public void wrongLooksFontsAndPackagesFallBackToNull() {
        String it = "{\"id\":\"x-1\",\"kind\":\"KIND\",\"variant\":\"VAR\",\"enabled\":true,\"x\":10,\"y\":10,\"w\":30,\"h\":20,"
                + "\"style\":{\"font\":\"FONT\",\"size\":30,\"color\":\"#ffffff\",\"opacity\":1,\"radius\":0,\"fill\":null,\"align\":\"center\"},"
                + "\"motion\":{\"type\":\"none\",\"speed\":1,\"amount\":0},\"options\":{\"packages\":[\"PKG\"]}}";
        String ok = it.replace("KIND", "clock").replace("VAR", "analog").replace("FONT", "serif").replace("PKG", "com.a.b");
        assertNotNull(LayoutSpec.parse(layout("null", ok)));
        assertNull(LayoutSpec.parse(layout("null", ok.replace("analog", "ring"))));            // not a look a clock has
        assertNull(LayoutSpec.parse(layout("null", ok.replace("serif", "file:../x.ttf"))));
        assertNull(LayoutSpec.parse(layout("null", ok.replace("serif", "file:x.exe"))));
        assertNull(LayoutSpec.parse(layout("null", ok.replace("com.a.b", "not a package"))));
        assertNull(LayoutSpec.parse(layout("null", ok.replace("\"size\":30", "\"size\":900"))));
        assertTrue(LayoutSpec.validFont("Mont-Light.ttf"));
        assertFalse(LayoutSpec.validFont("../Mont.ttf"));
    }

    @Test public void homeModeDefaultsToClassic() {
        String base = layout("null", item("clock-1", ""));
        assertTrue(LayoutSpec.parse(base).classic());
        String canvas = base.replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"home\":\"canvas\",\"background\"");
        assertFalse(LayoutSpec.parse(canvas).classic());
        assertNull(LayoutSpec.parse(base.replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"home\":\"tv\",\"background\"")));
        assertTrue(LayoutSpec.appKind("apps") && LayoutSpec.appKind("recent") && LayoutSpec.appKind("watchnext"));
        assertFalse(LayoutSpec.appKind("clock"));
        LayoutSpec scroll = LayoutSpec.parse(base.replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"home\":\"scroll\",\"background\""));
        assertTrue(scroll.scroll());
        assertFalse(scroll.classic());
    }

    @Test public void appleTvLookParses() {
        String base = layout("null", item("clock-1", ""));
        LayoutSpec s = LayoutSpec.parse(base.replace("\"enabled\":true,\"background\"",
                "\"enabled\":true,\"home\":\"tvos\",\"tiles\":\"tvos\",\"focus\":\"parallax\",\"background\""));
        assertTrue(s.tvos());
        assertTrue(s.scroll());   // the dock and grid ride on scroll mode
        assertFalse(s.classic());
        assertEquals("tvos", s.tiles);
        assertEquals("parallax", s.focus);
        assertFalse(LayoutSpec.parse(base).tvos());
    }

    @Test public void moreVideoContainers() {
        assertTrue(LayoutSpec.validVideo("clip.mkv"));
        assertTrue(LayoutSpec.validVideo("clip.mov"));
        assertFalse(LayoutSpec.validVideo("clip.avi"));
        assertEquals("clip.mkv", LayoutSpec.videoFile("clip.mkv"));
        assertEquals("rain.mp4", LayoutSpec.videoFile("rain"));
    }

    @Test public void scenesParseAndUnknownOnesFallBackToNull() {
        String base = layout("null", item("clock-1", ""));
        String scene = base.replace("\"kind\":\"video\"", "\"kind\":\"scene\",\"scene\":\"aurora\",\"sceneTime\":\"night\"");
        LayoutSpec s = LayoutSpec.parse(scene);
        assertEquals("scene", s.bgKind);
        assertEquals("aurora", s.scene);
        assertEquals("night", s.sceneTime);
        assertFalse(s.heavyOnLowRam);
        assertNull(LayoutSpec.parse(scene.replace("aurora", "lava")));
        assertTrue(LayoutSpec.parse(scene.replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"heavyOnLowRam\":true,\"background\"")).heavyOnLowRam);
    }

    @Test public void tileLooksParseAndBadOnesFallBackToNull() {
        String base = layout("null", item("clock-1", ""));
        String look = base.replace("\"enabled\":true,\"background\"",
                "\"enabled\":true,\"focus\":\"tilt\",\"launch\":\"grow\",\"tiles\":\"poster\",\"iconShape\":\"leaf\",\"accent\":\"#34f5a0\",\"background\"");
        LayoutSpec s = LayoutSpec.parse(look);
        assertEquals("tilt", s.focus);
        assertEquals("grow", s.launch);
        assertEquals("poster", s.tiles);
        assertEquals("leaf", s.iconShape);
        assertTrue(s.hasAccent);
        assertEquals(0xFF34F5A0, s.accent);
        LayoutSpec plain = LayoutSpec.parse(base);
        assertEquals("pop", plain.focus);
        assertFalse(plain.hasAccent);
        assertNull(LayoutSpec.parse(look.replace("tilt", "spin")));
    }

    @Test public void tileCorners() {
        assertEquals(50f, CanvasHome.cornerRadius("circle", 100, 200), 0.01f);
        assertEquals(0f, CanvasHome.cornerRadius("square", 100, 100), 0.01f);
        assertEquals(28f, CanvasHome.cornerRadius("squircle", 100, 100), 0.01f);
        assertEquals(12f, CanvasHome.cornerRadius("rounded", 200, 100), 0.01f);
    }

    @Test public void rowsFoldersHiddenAndAppLooksParse() {
        String base = layout("null", item("clock-1", ""));
        String apps = base.replace("\"enabled\":true,\"background\"", "\"enabled\":true,"
                + "\"rows\":[{\"title\":\"Favourites\",\"source\":\"favorites\",\"packages\":[\"com.netflix.ninja\"]},{\"title\":\"Games\",\"source\":\"games\",\"max\":8}],"
                + "\"folders\":[{\"name\":\"Kids\",\"packages\":[\"com.a.b\",\"com.c.d\"]}],"
                + "\"hidden\":[\"com.sony.dtv.promos\"],"
                + "\"apps\":{\"com.netflix.ninja\":{\"name\":\"Films\",\"color\":\"#e50914\",\"image\":\"netflix.jpg\"}},"
                + "\"background\"");
        LayoutSpec s = LayoutSpec.parse(apps);
        assertEquals(2, s.rows.size());
        assertEquals("favorites", s.rows.get(0).source);
        assertEquals(8, s.rows.get(1).max);
        assertEquals("Kids", s.folders.get(0).name);
        assertTrue(s.hidden.contains("com.sony.dtv.promos"));
        LayoutSpec.AppLook look = s.looks.get("com.netflix.ninja");
        assertEquals("Films", look.name);
        assertEquals(0xFFE50914, look.color);
        assertEquals("netflix.jpg", look.image);
        assertNull(LayoutSpec.parse(apps.replace("netflix.jpg", "../x.jpg")));
        assertNull(LayoutSpec.parse(apps.replace("\"source\":\"games\"", "\"source\":\"ads\"")));
        assertNull(LayoutSpec.parse(apps.replace("com.sony.dtv.promos", "rm -rf")));
    }

    @Test public void lockedAppsAndProfilesParse() {
        String base = layout("null", item("clock-1", ""));
        String fam = base.replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"locked\":[\"com.netflix.ninja\"],"
                + "\"profileName\":\"Dad\",\"profiles\":[{\"id\":\"kids\",\"name\":\"Kids\",\"color\":\"#ffaa00\"},{\"id\":\"mum\",\"name\":\"Mum\",\"pin\":true}],\"background\"");
        LayoutSpec s = LayoutSpec.parse(fam);
        assertTrue(s.locked.contains("com.netflix.ninja"));
        assertEquals("Dad", s.profileName);
        assertEquals(2, s.profiles.size());
        assertEquals(0xFFFFAA00, s.profiles.get(0).color);
        assertTrue(s.profiles.get(1).pin);
        assertNull(LayoutSpec.parse(fam.replace("\"kids\"", "\"../kids\"")));
    }

    @Test public void statusWidgetsAndOpenOnStartParse() {
        String base = layout("null", item("clock-1", ""));
        String w = base.replace("\"kind\":\"clock\"", "\"kind\":\"nowplaying\"").replace("\"options\":{\"seconds\":true,\"max\":4}", "\"options\":{}")
                .replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"openOnStart\":\"com.netflix.ninja\",\"background\"");
        LayoutSpec s = LayoutSpec.parse(w);
        assertEquals("nowplaying", s.items.get(0).kind);
        assertEquals("com.netflix.ninja", s.openOnStart);
        assertNull(LayoutSpec.parse(w.replace("com.netflix.ninja", "not a pkg")));
    }

    @Test public void weatherNeedsConsentAndParses() {
        String base = layout("null", item("clock-1", ""));
        String w = base.replace("\"enabled\":true,\"background\"", "\"enabled\":true,\"weather\":{\"city\":\"Ongole\",\"units\":\"c\",\"consent\":true},\"background\"")
                .replace("\"blur\":0}", "\"blur\":0,\"followWeather\":true}");
        LayoutSpec s = LayoutSpec.parse(w);
        assertEquals("Ongole", s.weatherCity);
        assertTrue(s.weatherOn);
        assertFalse(s.fahrenheit);
        assertTrue(s.followWeather);
        assertFalse(LayoutSpec.parse(w.replace(",\"consent\":true", "")).weatherOn);
        assertNull(LayoutSpec.parse(w.replace("\"units\":\"c\"", "\"units\":\"k\"")));
    }
}
