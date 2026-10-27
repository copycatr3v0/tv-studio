package com.gathin.tvhome;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.provider.Settings;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import java.io.File;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/**
 * The launcher's one settings screen, in every home style: a panel that slides in from the right, made for the
 * remote. OK or Left/Right change a row. Studio look choices (TvChoices) last until Studio applies a theme again;
 * the Classic Home choices (columns, dock, wallpaper, animations, startup app) are the launcher's own, as the old
 * Launcher settings kept them. "All settings" opens Android's settings. Opened from the ⚙ on Home, the Classic
 * header's settings button, or the Menu key.
 */
public class CanvasSettingsActivity extends Activity {
    @Override
    protected void attachBaseContext(android.content.Context base) {
        super.attachBaseContext(LauncherLocale.wrap(base));
    }

    private float unit;
    private LinearLayout list;
    private int accent = Color.WHITE;

    /** One changeable row: its key, the values (first = Studio's own) and how each is shown. */
    private static final class Choice {
        final String key, label;
        final List<String> values;
        final List<String> names;
        Choice(String key, String label, List<String> values, List<String> names) { this.key = key; this.label = label; this.values = values; this.names = names; }
    }

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        unit = Math.max(0.5f, getResources().getDisplayMetrics().heightPixels / 1080f);
        LayoutSpec spec = CanvasHome.load(this);
        if (spec != null && spec.hasAccent) accent = spec.accent;
        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(0xCC000000);   // Home stays recognisable behind, but nothing on it competes with the panel
        root.setOnClickListener(v -> finish());
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        GradientDrawable glass = new GradientDrawable();
        glass.setColor(0xFF17191E);   // opaque: Home's clock and buttons never show through the text
        glass.setCornerRadius(28 * unit);
        panel.setBackground(glass);
        int pad = (int) (36 * unit);
        panel.setPadding(pad, pad, pad, pad);
        panel.setClickable(true);
        TextView title = text("Settings", 44, Color.WHITE);
        title.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        panel.addView(title);
        TextView sub = text(spec != null ? "Look changes last until the next Apply from Studio on your PC." : "Home and TV settings.", 20, 0x99FFFFFF);
        sub.setPadding(0, (int) (6 * unit), 0, (int) (18 * unit));
        panel.addView(sub);
        ScrollView scroll = new ScrollView(this);
        scroll.setVerticalScrollBarEnabled(false);
        scroll.setClipToPadding(false);
        list = new LinearLayout(this);
        list.setOrientation(LinearLayout.VERTICAL);
        list.setPadding(0, 0, 0, (int) (24 * unit));   // the last row scrolls fully into view
        scroll.addView(list);
        panel.addView(scroll, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, 1f));
        FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams((int) (getResources().getDisplayMetrics().widthPixels * 0.42f),
                ViewGroup.LayoutParams.MATCH_PARENT, Gravity.END);
        int m = (int) (28 * unit);
        lp.setMargins(m, m, m, m);
        root.addView(panel, lp);
        setContentView(root);
        build(spec);
        panel.setTranslationX(lp.width);
        panel.animate().translationX(0).setDuration(260).setInterpolator(new android.view.animation.DecelerateInterpolator(2f)).start();
    }

    private void build(LayoutSpec spec) {
        int keep = list.indexOfChild(list.getFocusedChild());
        list.removeAllViews();
        refocus = keep;
        Map<String, String> saved = TvChoices.read(this);
        if (spec != null) {
            header("Look");
            boolean scroll = TvChoices.canScroll(spec);
            List<String> homes = scroll ? Arrays.asList("", "classic", "scroll", "tvos", "canvas") : Arrays.asList("", "classic", "canvas");
            List<String> homeNames = scroll ? Arrays.asList("From Studio", "Classic", "Scroll", "Apple TV style", "Free canvas") : Arrays.asList("From Studio", "Classic", "Free canvas");
            choice(new Choice(TvChoices.HOME, "Home style", homes, homeNames), saved);
            choice(new Choice(TvChoices.FOCUS, "Focus effect", withStudio(LayoutSpec.FOCUS), names(withStudio(LayoutSpec.FOCUS))), saved);
            choice(new Choice(TvChoices.TILES, "App tiles", withStudio(LayoutSpec.TILES), names(withStudio(LayoutSpec.TILES))), saved);
            List<String> scenes = withStudio(Arrays.asList(AmbientScene.SCENES));
            choice(new Choice(TvChoices.SCENE, "Background", scenes, names(scenes)), saved);
            choice(new Choice(TvChoices.MOTION, LowRam.isLow(this) ? "Motion (low-memory TV)" : "Motion and effects",
                    Arrays.asList("", "on", "off"), Arrays.asList("Automatic", "Always on", "Off (still)")), saved);
            header("Widgets");
            choice(new Choice(TvChoices.UNITS, "Temperature", Arrays.asList("", "c", "f"), Arrays.asList("From Studio", "°C", "°F")), saved);
            choice(new Choice(TvChoices.CLOCK, "Clock", Arrays.asList("", "12", "24"), Arrays.asList("From Studio", "12-hour", "24-hour")), saved);
        }
        if (spec == null || spec.classic()) classicRows();
        Config config = Config.load(this);
        if (config.openOnStartChoice) {
            header("Startup");
            action("Open at power-on: " + openOnStartLabel(), this::showOpenOnStartPicker);
        }
        header("TV");
        action("Screensaver", () -> open(Settings.ACTION_DREAM_SETTINGS));
        action("Network and internet", () -> open(Settings.ACTION_WIFI_SETTINGS));
        action("Default Home app", () -> { Toast.makeText(this, R.string.home_settings_help, Toast.LENGTH_LONG).show(); open(Settings.ACTION_HOME_SETTINGS); });
        action("All settings", () -> open(Settings.ACTION_SETTINGS));
        header("Reset");
        if (!saved.isEmpty() || hasClassicChoices()) action("Undo the changes made here", () -> {
            TvChoices.clear(this);
            classic().edit().remove("columns").remove(Config.ROWS_BELOW_DOCK_KEY).remove("animations").remove("advanced_effects")
                    .remove(Config.WALLPAPER_CHOICE_KEY).remove(Config.DOCK_OPACITY_KEY).commit();
            classicChanged();
            build(CanvasHome.load(this));
        });
        action("Reset everything on this TV…", this::confirmReset);
        focusFirst();
    }

    // ------------------------------------------------------------ Classic Home (the old Launcher settings)

    private SharedPreferences classic() { return getSharedPreferences(Config.SETTINGS_NAME, MODE_PRIVATE); }

    /** Saved: Home rebuilds when it next resumes (the change count moved), left with Back or the Home button. */
    private void classicChanged() {
        SharedPreferences c = classic();
        c.edit().putLong(Config.SETTINGS_REVISION_KEY, c.getLong(Config.SETTINGS_REVISION_KEY, 0) + 1).commit();
        setResult(RESULT_OK);
    }

    private boolean hasClassicChoices() {
        SharedPreferences c = classic();
        return c.contains("columns") || c.contains(Config.ROWS_BELOW_DOCK_KEY) || c.contains("animations") || c.contains("advanced_effects")
                || c.contains(Config.WALLPAPER_CHOICE_KEY) || c.contains(Config.DOCK_OPACITY_KEY);
    }

    private void classicRows() {
        Config config = Config.load(this);
        header("Classic Home");
        intRow("Columns", "columns", Arrays.asList(4, 5, 6, 7), config.columns, null);
        intRow("Rows below the dock", Config.ROWS_BELOW_DOCK_KEY, Arrays.asList(0, 1, 2, 3), config.rowsBelowDock, null);
        intRow("Dock opacity", Config.DOCK_OPACITY_KEY, Arrays.asList(100, 70, 50, 30), config.dockOpacity, "%");
        String wp = classic().getString(Config.WALLPAPER_CHOICE_KEY, null);
        action("Wallpaper: " + ("none".equals(wp) ? getString(R.string.wallpaper_none) : wp != null ? wp : getString(R.string.wallpaper_theme_default)), this::showWallpaperPicker);
        boolean anim = config.animations, adv = config.animations && config.advancedEffects;
        toggle("Animations", anim, on -> {
            classic().edit().putBoolean("animations", on).putBoolean("advanced_effects", on && classic().getBoolean("advanced_effects", false)).commit();
            classicChanged();
            build(CanvasHome.load(this));
        });
        if (anim) toggle("Advanced effects", adv, on -> {
            Runnable set = () -> { classic().edit().putBoolean("advanced_effects", on).commit(); classicChanged(); build(CanvasHome.load(this)); };
            if (!on) { set.run(); return; }
            new AlertDialog.Builder(this).setTitle(R.string.advanced_effects).setMessage(R.string.advanced_effects_warning)
                    .setPositiveButton(R.string.enable, (d, w) -> set.run()).setNegativeButton(android.R.string.cancel, (d, w) -> build(CanvasHome.load(this))).show();
        });
        action("Reorder apps", () -> {
            setResult(RESULT_OK, new Intent().putExtra(LauncherSettingsActivity.EXTRA_REORDER, true));
            finish();
        });
    }

    private interface OnToggle { void set(boolean on); }

    private void toggle(String label, boolean now, OnToggle on) {
        final boolean[] state = {now};
        LinearLayout row = row();
        row.addView(text(label, 24, Color.WHITE), new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView value = text(now ? "On" : "Off", 22, 0xCCFFFFFF);
        row.addView(value);
        row.setOnClickListener(v -> { state[0] = !state[0]; value.setText(state[0] ? "On" : "Off"); on.set(state[0]); });
        list.addView(row);
    }

    private void intRow(String label, String key, List<Integer> values, int now, String unitText) {
        List<String> keys = new ArrayList<>(), names = new ArrayList<>();
        for (int v : values) { keys.add(String.valueOf(v)); names.add(v + (unitText == null ? "" : unitText)); }
        final int[] at = {Math.max(0, values.indexOf(now))};
        LinearLayout row = row();
        row.addView(text(label, 24, Color.WHITE), new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        TextView value = text("‹  " + names.get(at[0]) + "  ›", 22, 0xCCFFFFFF);
        row.addView(value);
        Runnable save = () -> { classic().edit().putInt(key, values.get(at[0])).commit(); classicChanged(); value.setText("‹  " + names.get(at[0]) + "  ›"); };
        row.setOnClickListener(v -> { at[0] = (at[0] + 1) % values.size(); save.run(); });
        row.setOnKeyListener((v, code, e) -> {
            if (e.getAction() != KeyEvent.ACTION_DOWN) return false;
            if (code == KeyEvent.KEYCODE_DPAD_RIGHT) { at[0] = (at[0] + 1) % values.size(); save.run(); return true; }
            if (code == KeyEvent.KEYCODE_DPAD_LEFT) { at[0] = (at[0] - 1 + values.size()) % values.size(); save.run(); return true; }
            return false;
        });
        list.addView(row);
    }

    private void showWallpaperPicker() {
        File dir = Config.dir(this);
        List<File> images = new ArrayList<>(WallpaperRotation.images(dir == null ? null : new File(dir, "wallpapers")));
        File single = dir == null ? null : new File(dir, Config.WALLPAPER_NAME);
        if (single != null && single.isFile()) images.add(0, single);
        String[] labels = new String[images.size() + 1];
        for (int i = 0; i < images.size(); i++) labels[i] = images.get(i).getName();
        labels[images.size()] = getString(R.string.wallpaper_none);
        String current = classic().getString(Config.WALLPAPER_CHOICE_KEY, null);
        int checked = "none".equals(current) ? images.size() : -1;
        for (int i = 0; i < images.size(); i++) if (images.get(i).getName().equals(current)) checked = i;
        new AlertDialog.Builder(this).setTitle(R.string.wallpaper_label)
                .setSingleChoiceItems(labels, checked, (d, which) -> {
                    d.dismiss();
                    classic().edit().putString(Config.WALLPAPER_CHOICE_KEY, which == images.size() ? "none" : images.get(which).getName()).commit();
                    classicChanged();
                    build(CanvasHome.load(this));
                })
                .setNeutralButton(R.string.wallpaper_theme_default, (d, w) -> { classic().edit().remove(Config.WALLPAPER_CHOICE_KEY).commit(); classicChanged(); build(CanvasHome.load(this)); })
                .setNegativeButton(android.R.string.cancel, null).show();
    }

    /** Off, then every app with a TV entry point by name; read on the next boot. */
    private void showOpenOnStartPicker() {
        PackageManager pm = getPackageManager();
        List<String[]> apps = new ArrayList<>();
        for (ResolveInfo r : pm.queryIntentActivities(new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER), 0)) {
            String pkg = r.activityInfo.packageName;
            if (pkg.equals(getPackageName())) continue;
            boolean seen = false;
            for (String[] a : apps) seen |= a[1].equals(pkg);
            if (!seen) apps.add(new String[]{r.loadLabel(pm).toString(), pkg});
        }
        Collections.sort(apps, (a, b) -> a[0].compareToIgnoreCase(b[0]));
        String[] labels = new String[apps.size() + 1];
        labels[0] = getString(R.string.off);
        String current = classic().getString(OpenOnStart.KEY, "");
        int checked = 0;
        for (int i = 0; i < apps.size(); i++) { labels[i + 1] = apps.get(i)[0]; if (apps.get(i)[1].equals(current)) checked = i + 1; }
        new AlertDialog.Builder(this).setTitle(R.string.open_on_start_label)
                .setSingleChoiceItems(labels, checked, (d, which) -> {
                    d.dismiss();
                    if (which == 0) classic().edit().remove(OpenOnStart.KEY).commit();
                    else { OpenOnStart.skipThisBoot(this); classic().edit().putString(OpenOnStart.KEY, apps.get(which - 1)[1]).commit(); }
                    build(CanvasHome.load(this));
                })
                .setNegativeButton(android.R.string.cancel, null).show();
    }

    private String openOnStartLabel() {
        String pkg = classic().getString(OpenOnStart.KEY, "");
        if (pkg.isEmpty()) return getString(R.string.off);
        try { PackageManager pm = getPackageManager(); return pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString(); }
        catch (PackageManager.NameNotFoundException e) { return pkg; }
    }

    private static List<String> withStudio(List<String> values) {
        List<String> out = new ArrayList<>();
        out.add("");
        out.addAll(values);
        return out;
    }

    private static List<String> names(List<String> values) {
        List<String> out = new ArrayList<>();
        for (String v : values) out.add(v.isEmpty() ? "From Studio" : "tvos".equals(v) ? "Apple TV style" : Character.toUpperCase(v.charAt(0)) + v.substring(1));
        return out;
    }

    private void header(String s) {
        TextView t = text(s.toUpperCase(java.util.Locale.ROOT), 18, 0x80FFFFFF);
        t.setLetterSpacing(0.12f);
        t.setPadding((int) (14 * unit), (int) (22 * unit), 0, (int) (6 * unit));
        list.addView(t);
    }

    private void choice(Choice c, Map<String, String> saved) {
        String now = saved.containsKey(c.key) ? saved.get(c.key) : "";
        final int[] at = {Math.max(0, c.values.indexOf(now))};
        LinearLayout row = row();
        TextView label = text(c.label, 24, Color.WHITE);
        label.setSingleLine(true);
        label.setEllipsize(android.text.TextUtils.TruncateAt.END);
        TextView value = text("‹  " + c.names.get(at[0]) + "  ›", 22, 0xCCFFFFFF);
        value.setSingleLine(true);
        value.setPadding((int) (12 * unit), 0, 0, 0);
        value.setGravity(Gravity.END);
        row.addView(label, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        row.addView(value);
        Runnable step1 = () -> step(c, at, value, 1);
        row.setOnClickListener(v -> step1.run());
        row.setOnKeyListener((v, code, e) -> {
            if (e.getAction() != KeyEvent.ACTION_DOWN) return false;
            if (code == KeyEvent.KEYCODE_DPAD_RIGHT) { step(c, at, value, 1); return true; }
            if (code == KeyEvent.KEYCODE_DPAD_LEFT) { step(c, at, value, -1); return true; }
            return false;
        });
        list.addView(row);
    }

    private void step(Choice c, int[] at, TextView value, int by) {
        at[0] = (at[0] + by + c.values.size()) % c.values.size();
        String v = c.values.get(at[0]);
        TvChoices.set(this, c.key, v.isEmpty() ? null : v);
        value.setText("‹  " + c.names.get(at[0]) + "  ›");
    }

    private void action(String label, Runnable run) {
        LinearLayout row = row();
        TextView t = text(label, 24, Color.WHITE);
        t.setSingleLine(true);
        t.setEllipsize(android.text.TextUtils.TruncateAt.END);
        row.addView(t, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        row.addView(text("›", 24, 0x99FFFFFF));
        row.setOnClickListener(v -> run.run());
        list.addView(row);
    }

    private LinearLayout row() {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        row.setFocusable(true);
        row.setClickable(true);
        int p = (int) (14 * unit);
        row.setPadding(p, (int) (11 * unit), p, (int) (11 * unit));
        GradientDrawable bg = new GradientDrawable();
        bg.setCornerRadius(16 * unit);
        bg.setColor(Color.TRANSPARENT);
        row.setBackground(bg);
        row.setOnFocusChangeListener((v, f) -> {
            bg.setColor(f ? 0x26FFFFFF : Color.TRANSPARENT);
            bg.setStroke(f ? Math.max(2, (int) (3 * unit)) : 0, accent);
        });
        return row;
    }

    private TextView text(String s, float px, int color) {
        TextView t = new TextView(this);
        t.setText(s);
        t.setTextColor(color);
        t.setTextSize(TypedValue.COMPLEX_UNIT_PX, px * unit);
        return t;
    }

    private int refocus = -1;

    private void focusFirst() {
        list.post(() -> {
            if (refocus >= 0 && refocus < list.getChildCount() && list.getChildAt(refocus).isFocusable()) { list.getChildAt(refocus).requestFocus(); refocus = -1; return; }
            for (int i = 0; i < list.getChildCount(); i++) if (list.getChildAt(i).isFocusable()) { list.getChildAt(i).requestFocus(); return; }
        });
    }

    private void open(String action) {
        try {
            startActivity(SystemSettings.intent(this, action).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        } catch (ActivityNotFoundException | SecurityException unavailable) {
            Toast.makeText(this, R.string.not_available, Toast.LENGTH_SHORT).show();
        }
    }

    private void confirmReset() {
        AlertDialog d = new AlertDialog.Builder(this)
                .setTitle("Reset everything?")
                .setMessage("Removes the Studio theme, layout, wallpapers and videos from this TV and the launcher's own choices. Home comes back plain. Studio can apply your theme again.")
                .setNegativeButton(android.R.string.cancel, null)
                .setPositiveButton("Reset", (x, w) -> { FullReset.perform(this); setResult(RESULT_OK); finish(); })
                .create();
        d.show();
        d.getButton(AlertDialog.BUTTON_NEGATIVE).requestFocus();   // a stray OK keeps everything
    }
}
