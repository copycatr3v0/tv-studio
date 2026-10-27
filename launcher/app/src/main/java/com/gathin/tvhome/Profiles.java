package com.gathin.tvhome;

import android.content.Context;

import java.io.File;

/**
 * Up to four profiles, each its own layout under profiles/<id>.json; layout.json is the main one and lists them.
 * The chosen profile is remembered on the TV.
 */
final class Profiles {
    private Profiles() {}

    static final String DIR = "profiles";
    private static final String PREFS = "profiles";

    static boolean validId(String id) { return id != null && id.matches("[a-z0-9-]{1,16}"); }

    static String current(Context c) { return c.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString("current", ""); }

    static void choose(Context c, String id) {
        c.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString("current", validId(id) ? id : "").apply();
    }

    static File file(File dir, String id) { return new File(new File(dir, DIR), id + ".json"); }
}
