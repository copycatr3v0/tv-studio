package com.gathin.tvhome;

import org.junit.Test;

import java.util.Arrays;
import java.util.Collections;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

public class ThemeFilesTest {
    @Test public void themeFilesAreAllowed() {
        assertEquals("config.json", ThemeFiles.path(Collections.singletonList("config.json")));
        assertEquals("wallpaper.jpg", ThemeFiles.path(Collections.singletonList("wallpaper.jpg")));
        assertEquals("theme_revision", ThemeFiles.path(Collections.singletonList("theme_revision")));
    }

    @Test public void collectionImagesAreAllowed() {
        assertEquals("wallpapers/dusk.jpg", ThemeFiles.path(Arrays.asList("wallpapers", "dusk.jpg")));
        assertEquals("wallpapers/Night_2.WEBP", ThemeFiles.path(Arrays.asList("wallpapers", "Night_2.WEBP")));
    }

    @Test public void theCollectionsCaptionsAreAllowed() {
        assertEquals("wallpapers/captions.txt", ThemeFiles.path(Arrays.asList("wallpapers", "captions.txt")));
        assertNull(ThemeFiles.path(Collections.singletonList("captions.txt")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", "notes.txt")));
        assertFalse(ThemeFiles.collectionImage("captions.txt"));
    }

    @Test public void anythingElseIsRefused() {
        assertNull(ThemeFiles.path(null));
        assertNull(ThemeFiles.path(Collections.emptyList()));
        assertNull(ThemeFiles.path(Collections.singletonList("cards.json")));
        assertNull(ThemeFiles.path(Collections.singletonList("..")));
        assertNull(ThemeFiles.path(Arrays.asList("..", "config.json")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", "..")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", "run.sh")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", "a b.jpg")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", "../config.jpg")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", ".jpg")));
        assertNull(ThemeFiles.path(Arrays.asList("other", "dusk.jpg")));
        assertNull(ThemeFiles.path(Arrays.asList("wallpapers", "sub", "dusk.jpg")));
    }

    @Test public void onlyTheBareFolderIsTheCollection() {
        assertTrue(ThemeFiles.isCollection(Collections.singletonList("wallpapers")));
        assertFalse(ThemeFiles.isCollection(Arrays.asList("wallpapers", "dusk.jpg")));
        assertFalse(ThemeFiles.isCollection(Collections.singletonList("config.json")));
    }

    @Test public void theCanvasLayoutAndVideosAreAllowed() {
        assertEquals("layout.json", ThemeFiles.path(Collections.singletonList("layout.json")));
        assertEquals("video/rain.mp4", ThemeFiles.path(Arrays.asList("video", "rain.mp4")));
        assertEquals("video/my_loop-2.webm", ThemeFiles.path(Arrays.asList("video", "my_loop-2.webm")));
        assertTrue(ThemeFiles.isVideoCollection(Collections.singletonList("video")));
        assertFalse(ThemeFiles.isVideoCollection(Arrays.asList("video", "rain.mp4")));
    }

    @Test public void unsafeVideoPathsAreRefused() {
        assertNull(ThemeFiles.path(Arrays.asList("video", "..")));
        assertNull(ThemeFiles.path(Arrays.asList("video", "../config.json")));
        assertNull(ThemeFiles.path(Arrays.asList("video", "run.sh")));
        assertNull(ThemeFiles.path(Arrays.asList("video", "a b.mp4")));
        assertNull(ThemeFiles.path(Arrays.asList("video", ".hidden.mp4")));
        assertNull(ThemeFiles.path(Arrays.asList("video", "x.avi")));
        assertNull(ThemeFiles.path(Arrays.asList("video", "sub", "x.mp4")));
        assertNull(ThemeFiles.path(Collections.singletonList("video.mp4")));
    }

    @Test public void fontFilesAreAllowedAsPlainTtfOrOtfNamesOnly() {
        assertEquals("fonts/Mont-Light.ttf", ThemeFiles.path(Arrays.asList("fonts", "Mont-Light.ttf")));
        assertEquals("fonts/a_b.otf", ThemeFiles.path(Arrays.asList("fonts", "a_b.otf")));
        assertTrue(ThemeFiles.isFontCollection(Collections.singletonList("fonts")));
        assertNull(ThemeFiles.path(Arrays.asList("fonts", "..")));
        assertNull(ThemeFiles.path(Arrays.asList("fonts", "x.woff")));
        assertNull(ThemeFiles.path(Arrays.asList("fonts", "a b.ttf")));
        assertNull(ThemeFiles.path(Arrays.asList("fonts", ".h.ttf")));
        assertNull(ThemeFiles.path(Arrays.asList("fonts", "sub", "x.ttf")));
    }

    @Test public void moreVideoAndPictureTypesAndTheScreensaverRecord() {
        for (String v : new String[]{"a.mkv", "b.mov", "c.m4v", "d.3gp", "e.ts", "f.webm"}) assertEquals("video/" + v, ThemeFiles.path(Arrays.asList("video", v)));
        assertEquals("wallpapers/loop.gif", ThemeFiles.path(Arrays.asList("wallpapers", "loop.gif")));
        assertEquals("screensaver_prev", ThemeFiles.path(Collections.singletonList("screensaver_prev")));
        assertEquals("tiles/netflix.jpg", ThemeFiles.path(Arrays.asList("tiles", "netflix.jpg")));
        assertNull(ThemeFiles.path(Arrays.asList("tiles", "../x.jpg")));
        assertNull(ThemeFiles.path(Arrays.asList("tiles", "x.gif")));
        assertTrue(ThemeFiles.isTileCollection(Collections.singletonList("tiles")));
        assertEquals("profiles/kids.json", ThemeFiles.path(Arrays.asList("profiles", "kids.json")));
        assertNull(ThemeFiles.path(Arrays.asList("profiles", "../x.json")));
        assertEquals("pin_reset", ThemeFiles.path(Collections.singletonList("pin_reset")));
        assertTrue(ThemeFiles.isProfileCollection(Collections.singletonList("profiles")));
    }
}
