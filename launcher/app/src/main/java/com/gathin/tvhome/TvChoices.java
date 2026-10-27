package com.gathin.tvhome;

import android.content.Context;
import android.content.SharedPreferences;

import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Look choices made on the TV itself (Home settings, CanvasSettingsActivity), laid over the layout Studio sent:
 * home style, focus effect, tile style, background scene, motion, temperature units and clock format. They last
 * until Studio applies a theme again (its theme_revision changes), so the PC and the TV never fight: the newest
 * one wins. Kept in this app's private storage only.
 */
final class TvChoices {
    private TvChoices() {}

    static final String PREFS = "tv_choices";
    static final String HOME = "home", FOCUS = "focus", TILES = "tiles", SCENE = "scene", MOTION = "motion",
            UNITS = "units", CLOCK = "clock", REV = "rev";
    static final String[] KEYS = {HOME, FOCUS, TILES, SCENE, MOTION, UNITS, CLOCK};
    /** "studio": the layout's own background; any scene name: that ambient scene. */
    static final String STUDIO = "studio";
    static final List<String> MOTIONS = Arrays.asList("auto", "on", "off");

    /** Whether the layout can show Scroll and Apple TV style (it has a recent row and an app grid). */
    static boolean canScroll(LayoutSpec s) {
        boolean recent = false, apps = false;
        for (LayoutSpec.Item it : s.items) { if ("recent".equals(it.kind)) recent = true; if ("apps".equals(it.kind)) apps = true; }
        return recent && apps;
    }

    /** Lays {@code c} over {@code s}; unknown or impossible values are ignored. */
    static void apply(LayoutSpec s, Map<String, String> c) {
        String home = c.get(HOME);
        s.homeBefore = s.home;
        if (home != null && LayoutSpec.HOMES.contains(home)) {
            boolean scrolling = "scroll".equals(home) || "tvos".equals(home);
            if (!scrolling || canScroll(s)) {
                s.home = home;
                boolean wasScrolling = "scroll".equals(s.homeBefore) || "tvos".equals(s.homeBefore);
                if ((scrolling && !wasScrolling) || ("tvos".equals(home) && !"tvos".equals(s.homeBefore))) clearRow(s, "tvos".equals(home));
                if (scrolling) for (LayoutSpec.Item it : s.items) if ("recent".equals(it.kind) || "apps".equals(it.kind)) it.enabled = true;
                if ("tvos".equals(home)) { s.tiles = "tvos"; s.focus = "parallax"; }
                if ("classic".equals(home) && !"classic".equals(s.homeBefore)) clearClassic(s);
            }
        }
        String focus = c.get(FOCUS);
        if (focus != null && LayoutSpec.FOCUS.contains(focus)) s.focus = focus;
        String tiles = c.get(TILES);
        if (tiles != null && LayoutSpec.TILES.contains(tiles)) s.tiles = tiles;
        String scene = c.get(SCENE);
        if (scene != null && Arrays.asList(AmbientScene.SCENES).contains(scene)) { s.bgKind = "scene"; s.scene = scene; }
        String motion = c.get(MOTION);
        if ("on".equals(motion)) { s.heavyOnLowRam = true; s.still = false; }
        else if ("off".equals(motion)) s.still = true;
        String units = c.get(UNITS);
        if ("f".equals(units)) s.fahrenheit = true; else if ("c".equals(units)) s.fahrenheit = false;
        String clock = c.get(CLOCK);
        if ("24".equals(clock) || "12".equals(clock)) for (LayoutSpec.Item it : s.items) if ("clock".equals(it.kind)) it.twentyFour = "24".equals(clock);
    }

    static boolean hits(LayoutSpec.Item it, double x, double y, double w, double h) {
        return Math.min(it.x + it.w, x + w) - Math.max(it.x, x) > 0.5 && Math.min(it.y + it.h, y + h) - Math.max(it.y, y) > 0.5;
    }

    /** Where Classic Home draws its header buttons, the focused app's name and its dock ({x, y, w, h}, percent;
     *  CLASSIC_ZONES in Studio's layout.ts). */
    static final double[][] CLASSIC_ZONES = {{54, 10, 41, 9}, {6, 48, 30, 11}, {6, 63, 89, 37}};

    /** Switching to Classic on the TV: widgets that would sit on Classic's own buttons, name or dock are hidden. */
    static void clearClassic(LayoutSpec s) {
        for (LayoutSpec.Item it : s.items) {
            if (!it.enabled || LayoutSpec.appKind(it.kind) || "shape".equals(it.kind)) continue;
            for (double[] z : CLASSIC_ZONES) if (hits(it, z[0], z[1], z[2], z[3])) { it.enabled = false; break; }
        }
    }

    /** Switching a layout made for Classic or Canvas to Scroll or Apple TV style on the TV: the app row goes along
     *  the bottom (where Studio puts it) and widgets it would cover are hidden, so nothing sits under the apps. */
    static void clearRow(LayoutSpec s, boolean shelf) {
        for (LayoutSpec.Item it : s.items) if ("recent".equals(it.kind)) { it.x = 2; it.y = 63; it.w = 96; it.h = 27; }
        for (LayoutSpec.Item it : s.items) {
            if (!it.enabled || LayoutSpec.appKind(it.kind) || "shape".equals(it.kind)) continue;
            if (hits(it, 2, 63, 96, 27) || (shelf && hits(it, 4.5, 42, 40, 8))) it.enabled = false;
        }
    }

    private static SharedPreferences prefs(Context ctx) { return ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE); }

    /** The saved choices, or none once Studio has applied a theme since they were made. */
    static Map<String, String> read(Context ctx) {
        SharedPreferences p = prefs(ctx);
        Map<String, String> out = new HashMap<>();
        if (!p.contains(REV)) return out;
        if (!Config.themeRevision(Config.dir(ctx)).equals(p.getString(REV, ""))) { p.edit().clear().apply(); return out; }
        for (String k : KEYS) { String v = p.getString(k, null); if (v != null) out.put(k, v); }
        return out;
    }

    static void loadAndApply(Context ctx, LayoutSpec spec) { apply(spec, read(ctx)); }

    /** Saves one choice (null removes it) and moves the settings counter on so Home rebuilds when it resumes. */
    static void set(Context ctx, String key, String value) {
        SharedPreferences.Editor e = prefs(ctx).edit().putString(REV, Config.themeRevision(Config.dir(ctx)));
        if (value == null) e.remove(key); else e.putString(key, value);
        e.commit();
        bump(ctx);
    }

    static void clear(Context ctx) {
        prefs(ctx).edit().clear().commit();
        bump(ctx);
    }

    static void bump(Context ctx) {
        SharedPreferences s = ctx.getSharedPreferences(Config.SETTINGS_NAME, Context.MODE_PRIVATE);
        s.edit().putLong(Config.SETTINGS_REVISION_KEY, s.getLong(Config.SETTINGS_REVISION_KEY, 0) + 1).commit();
    }
}
