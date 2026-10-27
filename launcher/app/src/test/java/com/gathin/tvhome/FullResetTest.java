package com.gathin.tvhome;

import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;

import java.io.File;
import java.io.IOException;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

public class FullResetTest {
    @Rule public TemporaryFolder tmp = new TemporaryFolder();

    @Test public void removesEveryPushedFileAndNothingElse() throws IOException {
        File dir = tmp.newFolder("files");
        for (String n : new String[]{"config.json", "layout.json", "wallpaper.jpg", "theme_revision"}) assertTrue(new File(dir, n).createNewFile());
        File wp = new File(dir, "wallpapers"), vid = new File(dir, "video"), fonts = new File(dir, "fonts");
        assertTrue(wp.mkdir() && vid.mkdir() && fonts.mkdir());
        assertTrue(new File(fonts, "Mont.ttf").createNewFile());
        assertTrue(new File(wp, "a.jpg").createNewFile() && new File(wp, "captions.txt").createNewFile() && new File(vid, "rain.mp4").createNewFile());
        File other = new File(dir, "keep.txt");
        assertTrue(other.createNewFile());

        assertEquals(8, FullReset.deleteThemeFiles(dir));
        assertFalse(new File(dir, "layout.json").exists());
        assertFalse(new File(wp, "a.jpg").exists());
        assertFalse(new File(vid, "rain.mp4").exists());
        assertFalse(new File(fonts, "Mont.ttf").exists());
        assertTrue("files it does not own stay", other.exists());
    }

    @Test public void isSafeWithNothingToDelete() throws IOException {
        assertEquals(0, FullReset.deleteThemeFiles(null));
        assertEquals(0, FullReset.deleteThemeFiles(tmp.newFolder("empty")));
        assertEquals(0, FullReset.deleteThemeFiles(new File(tmp.getRoot(), "missing")));
    }
}
