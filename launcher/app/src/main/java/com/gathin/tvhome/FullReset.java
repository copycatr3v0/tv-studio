package com.gathin.tvhome;

import android.content.Context;
import android.content.SharedPreferences;

import java.io.File;

/**
 * "Reset everything" from the TV itself, for when a theme, layout or video leaves Home unusable and no PC is at hand:
 * removes every file the Studio app pushed (config, canvas layout, wallpapers, videos) and clears the launcher's own
 * saved choices (columns, order, hidden apps, last-opened times), so Home comes back as the built-in default.
 * It touches only this launcher's own storage: no other app, no system setting.
 */
final class FullReset {
    private FullReset() {}

    /** Deletes the pushed theme files under {@code dir}; returns how many files were removed. Safe when dir is null or empty. */
    static int deleteThemeFiles(File dir) {
        if (dir == null || !dir.isDirectory()) return 0;
        int n = 0;
        for (String name : new String[]{Config.FILE_NAME, LayoutSpec.FILE_NAME, "wallpaper.jpg", "theme_revision"}) {
            File f = new File(dir, name);
            if (f.isFile() && f.delete()) n++;
        }
        n += clearFolder(new File(dir, ThemeFiles.WALLPAPERS));
        n += clearFolder(new File(dir, ThemeFiles.VIDEO));
        n += clearFolder(new File(dir, ThemeFiles.FONTS));
        n += clearFolder(new File(dir, ThemeFiles.TILES));
        n += clearFolder(new File(dir, Profiles.DIR));
        return n;
    }

    private static int clearFolder(File folder) {
        File[] files = folder.listFiles();
        int n = 0;
        if (files != null) for (File f : files) if (f.isFile() && f.delete()) n++;
        return n;
    }

    /** Resets everything and moves the change counter on so a running Home rebuilds when it next resumes. */
    static void perform(Context ctx) {
        deleteThemeFiles(Config.dir(ctx));
        SharedPreferences settings = ctx.getSharedPreferences(Config.SETTINGS_NAME, Context.MODE_PRIVATE);
        long revision = settings.getLong(Config.SETTINGS_REVISION_KEY, 0);
        File prefsDir = new File(ctx.getApplicationInfo().dataDir, "shared_prefs");
        File[] files = prefsDir.listFiles((d, name) -> name.endsWith(".xml"));
        if (files != null) {
            for (File f : files) {
                String name = f.getName().substring(0, f.getName().length() - 4);
                ctx.getSharedPreferences(name, Context.MODE_PRIVATE).edit().clear().commit();
            }
        }
        settings.edit().putLong(Config.SETTINGS_REVISION_KEY, revision + 1).commit();
    }
}
