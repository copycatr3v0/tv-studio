package com.gathin.tvhome;

import android.animation.Animator;
import android.animation.ObjectAnimator;
import android.animation.ValueAnimator;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Color;
import android.graphics.Matrix;
import android.graphics.Outline;
import android.graphics.SurfaceTexture;
import android.graphics.Typeface;
import android.os.Build;
import android.graphics.drawable.Drawable;
import android.graphics.drawable.GradientDrawable;
import android.media.MediaPlayer;
import android.os.Handler;
import android.os.Looper;
import android.text.format.DateFormat;
import android.view.Gravity;
import android.view.Surface;
import android.view.TextureView;
import android.view.View;
import android.view.ViewGroup;
import android.view.ViewOutlineProvider;
import android.widget.FrameLayout;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import androidx.recyclerview.widget.GridLayoutManager;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import java.io.File;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Calendar;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.Locale;

/**
 * The free-canvas Home: every item sits at the percentage position, size and style the Studio app saved in
 * layout.json, over a still, colour, gradient or looping-video background. It replaces the standard Home only
 * while layout.json is present, valid and enabled; otherwise {@link #load} returns null and nothing changes.
 */
final class CanvasHome extends FrameLayout {
    private final LayoutSpec spec;
    private final File dir;
    private final boolean reduceMotion;
    /** Moving backgrounds and animations allowed on this TV (see LowRam). */
    private final boolean heavy;
    private AmbientScene ambient;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final StatsSampler stats;
    private final List<Runnable> ticks = new ArrayList<>();
    private final List<Animator> animators = new ArrayList<>();
    private final List<Widgets.Animated> animated = new ArrayList<>();
    private VideoBackground video;
    private boolean started;
    private final float unit;   // pixels per 1080p pixel

    /** The enabled layout, or null (missing, invalid, or switched off), in which case Home stays standard.
     *  With profiles, the chosen profile's layout (profiles/<id>.json) when it is there and valid. */
    static LayoutSpec load(Context ctx) {
        File dir = Config.dir(ctx);
        if (dir == null) return null;
        // Studio asked the TV to forget its PIN.
        File reset = new File(dir, ThemeFiles.PIN_RESET);
        if (reset.isFile()) { Pin.forget(ctx); if (!reset.delete()) reset.deleteOnExit(); }
        LayoutSpec main = read(new File(dir, LayoutSpec.FILE_NAME));
        if (main == null || !main.enabled) return null;
        String id = Profiles.current(ctx);
        if (!id.isEmpty()) {
            for (LayoutSpec.Profile p : main.profiles) {
                if (!p.id.equals(id)) continue;
                LayoutSpec other = read(Profiles.file(dir, id));
                if (other != null && other.enabled) {
                    other.profiles.clear();
                    other.profiles.addAll(main.profiles);
                    other.profileName = main.profileName;
                    other.profileColor = main.profileColor;
                    other.currentProfile = id;
                    TvChoices.loadAndApply(ctx, other);
                    return other;
                }
            }
        }
        TvChoices.loadAndApply(ctx, main);
        return main;
    }

    private static LayoutSpec read(File f) {
        if (!f.isFile() || f.length() > 256 * 1024) return null;
        try (java.io.FileInputStream in = new java.io.FileInputStream(f)) {
            byte[] b = new byte[(int) f.length()];
            int n = 0;
            while (n < b.length) { int r = in.read(b, n, b.length - n); if (r < 0) break; n += r; }
            return LayoutSpec.parse(new String(b, 0, n, "UTF-8"));
        } catch (java.io.IOException e) {
            return null;
        }
    }

    /** A clock, date or greeting on the canvas: in classic mode the standard clock block steps aside for it. */
    static boolean hasClockWidget(LayoutSpec spec) {
        for (LayoutSpec.Item it : spec.items) {
            if (it.enabled && ("clock".equals(it.kind) || "date".equals(it.kind) || "greeting".equals(it.kind))) return true;
        }
        return false;
    }

    CanvasHome(Context ctx, LayoutSpec spec, boolean reduceMotion) { this(ctx, spec, reduceMotion, false); }

    /** {@code dream}: the screensaver; background and widgets only, nothing takes focus. */
    CanvasHome(Context ctx, LayoutSpec spec, boolean reduceMotion, boolean dream) {
        super(ctx);
        this.spec = spec;
        this.dir = Config.dir(ctx);
        // Low-RAM TVs: still pictures and no motion unless the person kept heavy effects on (Studio warned them).
        this.heavy = !reduceMotion && !spec.still && (spec.heavyOnLowRam || !LowRam.isLow(ctx));
        this.reduceMotion = reduceMotion;   // focus and scrolling stay smooth on every TV; they cost almost nothing
        this.stats = new StatsSampler(ctx);
        this.unit = Math.max(0.5f, getResources().getDisplayMetrics().heightPixels / 1080f);
        setBackgroundColor(spec.color | 0xFF000000);
        setFocusable(false);
        boolean scroll = spec.scroll() && !dream;
        if (spec.classic() || dream) setDescendantFocusability(FOCUS_BLOCK_DESCENDANTS);
        buildBackground(ctx);
        if (spec.tvos() && !dream) {
            topShelf = new TopShelf(ctx);
            addView(topShelf);
        }
        ViewGroup host = this;
        LayoutSpec.Item gridItem = null;
        if (scroll) {
            stage = new Stage(ctx);
            host = stage;
        }
        for (LayoutSpec.Item it : spec.items) {
            if (!it.enabled) continue;
            if ((spec.classic() || dream) && LayoutSpec.appKind(it.kind)) continue;
            View v;
            if (scroll && "apps".equals(it.kind)) { gridItem = it; continue; }
            else if (spec.tvos() && !dream && "recent".equals(it.kind)) v = dock(ctx, it);
            else if (scroll && "recent".equals(it.kind)) v = shelf(ctx, it);
            else v = build(ctx, it);
            if (v == null) continue;
            v.setTag(it);
            host.addView(v, new LayoutParams(0, 0));
            motion(v, it);
            if (dream && heavy) drift(v);
        }
        if (!spec.classic() && !dream) {
            // The gear: Home settings on the TV (also the remote's Menu key).
            View gear = gearChip(ctx);
            gearView = gear;
            LayoutSpec.Item at = new LayoutSpec.Item();
            double[] spot = gearSpot(widgetBoxes(spec, scroll), !spec.profiles.isEmpty());
            at.x = spot[0]; at.y = spot[1]; at.w = GEAR_W; at.h = GEAR_H;
            gear.setTag(at);
            host.addView(gear, new LayoutParams(0, 0));
        }
        if (!spec.profiles.isEmpty() && !spec.classic() && !dream) {
            View chip = profileChip(ctx);
            chipView = chip;
            LayoutSpec.Item where = new LayoutSpec.Item();
            where.x = 84; where.y = 3; where.w = 13; where.h = 6;
            chip.setTag(where);
            host.addView(chip, new LayoutParams(0, 0));
        }
        if (scroll) {
            pager = new Pager(ctx);
            LinearLayout col = new LinearLayout(ctx);
            col.setOrientation(LinearLayout.VERTICAL);
            col.setClipChildren(false);
            col.addView(stage, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, getResources().getDisplayMetrics().heightPixels));
            // Page two: the person's rows (empty ones are left out), then folders and every other app.
            LinearLayout page = new LinearLayout(ctx);
            page.setOrientation(LinearLayout.VERTICAL);
            page.setClipChildren(false);
            page.setClipToPadding(false);
            for (LayoutSpec.Row r : spec.rows) {
                View row = appRow(ctx, r, gridItem);
                if (row != null) page.addView(row, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            }
            page.addView(allApps(ctx, gridItem), new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            gridSection = page;
            col.addView(gridSection, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            pager.addView(col, new ScrollView.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
            pager.setOnScrollChangeListener((v, x, y, ox, oy) -> {
                // The widgets drift up slower than the apps and fade, so the grid seems to rise over them.
                float h = Math.max(1, getHeight());
                stage.setTranslationY(y * 0.45f);
                // Gone by the time the grid has risen 40 %: the row never ghosts through behind "All apps".
                stage.setAlpha(Math.max(0f, 1f - y / (h * 0.4f)));
                if (topShelf != null) topShelf.setAlpha(Math.max(0f, 1f - y / (h * 0.4f)));
            });
            addView(pager);
        }
    }

    // ---------------------------------------------------------------- scroll mode

    private Stage stage;
    private Pager pager;
    private View chipView;
    private View gridSection;
    private final java.util.concurrent.ExecutorService loader = java.util.concurrent.Executors.newSingleThreadExecutor();

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        super.onSizeChanged(w, h, ow, oh);
        if (stage != null && h > 0 && stage.getLayoutParams() != null && stage.getLayoutParams().height != h) {
            stage.getLayoutParams().height = h;
            post(stage::requestLayout);
        }
    }

    @Override
    public boolean dispatchKeyEvent(android.view.KeyEvent e) {
        if (e.getKeyCode() == android.view.KeyEvent.KEYCODE_MENU && gearView != null) {
            if (e.getAction() == android.view.KeyEvent.ACTION_UP) openSettings();
            return true;
        }
        if (e.getAction() == android.view.KeyEvent.ACTION_DOWN) {
            switch (e.getKeyCode()) {
                case android.view.KeyEvent.KEYCODE_DPAD_LEFT: lastDx = -1; lastDy = 0; break;
                case android.view.KeyEvent.KEYCODE_DPAD_RIGHT: lastDx = 1; lastDy = 0; break;
                case android.view.KeyEvent.KEYCODE_DPAD_UP: lastDx = 0; lastDy = -1; break;
                case android.view.KeyEvent.KEYCODE_DPAD_DOWN: lastDx = 0; lastDy = 1; break;
                default: break;
            }
        }
        // Back while the grid is up: return to the first screen instead of leaving Home.
        if (pager != null && e.getKeyCode() == android.view.KeyEvent.KEYCODE_BACK && pager.getScrollY() > 0) {
            if (e.getAction() == android.view.KeyEvent.ACTION_UP) {
                ArrayList<View> out = new ArrayList<>();
                stage.addFocusables(out, View.FOCUS_DOWN);
                if (!out.isEmpty()) out.get(0).requestFocus(); else pager.glideTo(0);
            }
            return true;
        }
        return super.dispatchKeyEvent(e);
    }

    /** Lays its children out by their items' percentages, like the canvas itself. */
    private static final class Stage extends FrameLayout {
        Stage(Context c) { super(c); setClipChildren(false); setClipToPadding(false); }
        @Override protected void onLayout(boolean changed, int l, int t, int r, int b) { layoutPercent(this, r - l, b - t); }
    }

    /** The vertical scroller: D-pad focus moves, the page glides so the focused row sits high on screen. */
    private final class Pager extends ScrollView {
        private ValueAnimator glide;

        Pager(Context c) {
            super(c);
            setVerticalScrollBarEnabled(false);
            setOverScrollMode(OVER_SCROLL_NEVER);
            setFillViewport(true);
            setFocusable(false);
            setClipChildren(false);
        }

        // Focus moves by the normal D-pad search; this view never scrolls on its own for a key or a focus change.
        @Override public boolean executeKeyEvent(android.view.KeyEvent event) { return false; }
        @Override protected int computeScrollDeltaToGetChildRectOnScreen(android.graphics.Rect rect) { return 0; }
        @Override public boolean requestChildRectangleOnScreen(View child, android.graphics.Rect r, boolean immediate) { return false; }

        @Override public void requestChildFocus(View child, View focused) {
            super.requestChildFocus(child, focused);
            glideTo(target(focused));
        }

        private int target(View focused) {
            if (gridSection == null || !isInside(focused, gridSection)) return 0;
            android.graphics.Rect r = new android.graphics.Rect();
            focused.getDrawingRect(r);
            offsetDescendantRectToMyCoords(focused, r);
            int max = Math.max(0, getChildAt(0).getHeight() - getHeight());
            return Math.max(0, Math.min(max, r.top - (int) (getHeight() * 0.22f)));
        }

        void glideTo(int y) {
            if (glide != null) glide.cancel();
            if (reduceMotion) { scrollTo(0, y); return; }
            glide = ValueAnimator.ofInt(getScrollY(), y);
            glide.setDuration(380);
            glide.setInterpolator(new android.view.animation.DecelerateInterpolator(2f));
            glide.addUpdateListener(a -> scrollTo(0, (int) a.getAnimatedValue()));
            glide.start();
        }
    }

    /** The sideways row: the focused tile glides to the middle. */
    private final class Row extends android.widget.HorizontalScrollView {
        private ValueAnimator glide;

        Row(Context c) {
            super(c);
            setHorizontalScrollBarEnabled(false);
            setOverScrollMode(OVER_SCROLL_NEVER);
            setFocusable(false);
            setClipChildren(false);
            setClipToPadding(false);
        }

        @Override public boolean executeKeyEvent(android.view.KeyEvent event) { return false; }
        @Override protected int computeScrollDeltaToGetChildRectOnScreen(android.graphics.Rect rect) { return 0; }
        @Override public boolean requestChildRectangleOnScreen(View child, android.graphics.Rect r, boolean immediate) { return false; }

        @Override public void requestChildFocus(View child, View focused) {
            super.requestChildFocus(child, focused);
            android.graphics.Rect r = new android.graphics.Rect();
            focused.getDrawingRect(r);
            offsetDescendantRectToMyCoords(focused, r);
            int max = Math.max(0, getChildAt(0).getWidth() - getWidth());
            int x = Math.max(0, Math.min(max, r.centerX() - getWidth() / 2));
            if (glide != null) glide.cancel();
            if (reduceMotion) { scrollTo(x, 0); return; }
            glide = ValueAnimator.ofInt(getScrollX(), x);
            glide.setDuration(300);
            glide.setInterpolator(new android.view.animation.DecelerateInterpolator(2f));
            glide.addUpdateListener(a -> scrollTo((int) a.getAnimatedValue(), 0));
            glide.start();
        }
    }

    private static boolean isInside(View v, View ancestor) {
        for (android.view.ViewParent p = v.getParent(); p != null; p = p.getParent()) if (p == ancestor) return true;
        return v == ancestor;
    }

    @SuppressWarnings("unchecked")
    private static List<App> castApps(Object o) { return (List<App>) o; }

    /** The listed packages, in the listed order, among the apps that are installed and not hidden. */
    private static List<App> pick(List<App> all, List<String> pkgs) {
        List<App> out = new ArrayList<>();
        for (String p : pkgs) for (App a : all) if (a.pkg.equals(p)) { out.add(a); break; }
        return out;
    }

    /** The apps a row shows: hand-picked, recent, or by the category the app declares on Google Play. */
    private List<App> rowApps(Context ctx, LayoutSpec.Row r) {
        List<App> all = apps(ctx);
        List<App> out = new ArrayList<>();
        switch (r.source) {
            case "favorites": out = pick(all, r.packages); break;
            case "recent": {
                for (App a : all) if (LastOpened.get(ctx, a.pkg) > 0) out.add(a);
                Collections.sort(out, (x, y) -> Long.compare(LastOpened.get(ctx, y.pkg), LastOpened.get(ctx, x.pkg)));
                break;
            }
            default:
                for (App a : all) if (inCategory(a, r.source)) out.add(a);
                break;
        }
        return out.size() > r.max ? new ArrayList<>(out.subList(0, r.max)) : out;
    }

    /** Most TV apps declare no Play category, so well-known streaming and music apps are recognised by name too. */
    static final java.util.Set<String> KNOWN_VIDEO = new java.util.HashSet<>(Arrays.asList(
            "com.netflix.ninja", "com.google.android.youtube.tv", "com.amazon.amazonvideo.livingroom", "in.startv.hotstar",
            "com.jio.media.stb.ondemand", "com.sonyliv", "com.graymatrix.did", "ahaflix.tv", "com.viewlift.hoichoi", "com.suntv.sunnxt",
            "com.eros.now", "com.viki.android", "com.crunchyroll.crunchyroid", "com.mxtech.videoplayer.television", "com.disney.disneyplus",
            "com.plexapp.android", "org.xbmc.kodi", "org.videolan.vlc", "com.apple.atve.androidtv.appletv", "com.wbd.hbomax",
            "com.zee5.androidtv", "com.google.android.videos", "com.sonypicturescore", "com.vzy.one.atv", "com.fancode.tv"));
    static final java.util.Set<String> KNOWN_MUSIC = new java.util.HashSet<>(Arrays.asList(
            "com.spotify.tv.android", "com.google.android.youtube.tvmusic", "com.gaana", "com.jio.media.jiobeats", "com.bsbportal.music",
            "com.amazon.music.tv", "com.apple.android.music", "com.aspiro.tidal", "deezer.android.tv"));

    private static boolean inCategory(App a, String source) {
        switch (source) {
            case "games": return a.game;
            case "video": return a.category == android.content.pm.ApplicationInfo.CATEGORY_VIDEO || KNOWN_VIDEO.contains(a.pkg);
            case "music": return a.category == android.content.pm.ApplicationInfo.CATEGORY_AUDIO || KNOWN_MUSIC.contains(a.pkg);
            case "news": return a.category == android.content.pm.ApplicationInfo.CATEGORY_NEWS;
            case "social": return a.category == android.content.pm.ApplicationInfo.CATEGORY_SOCIAL;
            case "tools": return a.category == android.content.pm.ApplicationInfo.CATEGORY_PRODUCTIVITY
                    || a.category == android.content.pm.ApplicationInfo.CATEGORY_MAPS || a.category == android.content.pm.ApplicationInfo.CATEGORY_IMAGE;
            default: return false;
        }
    }

    /** One of the person's rows on the apps page: a title and a sideways row of tiles; null when it has no apps. */
    private View appRow(Context ctx, LayoutSpec.Row r, LayoutSpec.Item style) {
        List<App> list = rowApps(ctx, r);
        if (list.isEmpty()) return null;
        int sw = getResources().getDisplayMetrics().widthPixels, sh = getResources().getDisplayMetrics().heightPixels;
        int margin = (int) (sw * 0.04f);
        int tileW = (int) (sw * 0.16f);
        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setClipChildren(false);
        col.setClipToPadding(false);
        col.setPadding(margin, 0, 0, (int) (sh * 0.03f));
        TextView head = new TextView(ctx);
        if (style != null) { style(head, style); head.setBackground(null); }
        else { head.setTextColor(Color.WHITE); head.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 28 * unit); }
        head.setText(r.title);
        head.setAlpha(0.85f);
        head.setPadding(0, 0, 0, (int) (sh * 0.015f));
        col.addView(head);
        Row row = new Row(ctx);
        LinearLayout inner = new LinearLayout(ctx);
        inner.setOrientation(LinearLayout.HORIZONTAL);
        inner.setClipChildren(false);
        inner.setClipToPadding(false);
        inner.setPadding((int) (tileW * 0.07f), (int) (tileW * 0.04f), margin, 0);
        for (App a : list) {
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            lp.rightMargin = (int) (tileW * 0.1f);
            inner.addView(tile(ctx, a, tileW, style), lp);
        }
        row.addView(inner);
        col.addView(row, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        return col;
    }

    /** A folder tile: the first four apps' icons in a 2x2 grid and the folder's name; OK opens its apps. */
    private View folderTile(Context ctx, LayoutSpec.Folder f, List<App> in, int w, LayoutSpec.Item style) {
        App stand = new App();
        stand.pkg = "folder." + Math.abs(f.name.hashCode());
        stand.label = f.name;
        View cell = tile(ctx, stand, w, style);
        FrameLayout card = (FrameLayout) ((LinearLayout) cell).getChildAt(0);
        card.setBackgroundColor(spec.hasAccent ? (spec.accent & 0x00FFFFFF) | 0x55000000 : 0x55FFFFFF);
        android.widget.GridLayout grid = new android.widget.GridLayout(ctx);
        grid.setColumnCount(2);
        int cellSize = (int) (w * 9 / 16 * 0.36f);
        PackageManager pm = ctx.getPackageManager();
        for (int i = 0; i < Math.min(4, in.size()); i++) {
            ImageView iv = new ImageView(ctx);
            try { iv.setImageDrawable(pm.getApplicationIcon(in.get(i).pkg)); } catch (PackageManager.NameNotFoundException ignored) { }
            android.widget.GridLayout.LayoutParams lp = new android.widget.GridLayout.LayoutParams();
            lp.width = cellSize; lp.height = cellSize;
            lp.setMargins(cellSize / 8, cellSize / 8, cellSize / 8, cellSize / 8);
            grid.addView(iv, lp);
        }
        card.addView(grid, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT, Gravity.CENTER));
        TextView name = (TextView) ((LinearLayout) cell).getChildAt(1);
        name.setAlpha(1f);
        name.setText(f.name + " (" + in.size() + ")");
        cell.setOnClickListener(v -> openFolder(ctx, f, in, style));
        return cell;
    }

    /** A folder's apps in a pop-up row over a dimmed Home; Back closes it. */
    private void openFolder(Context ctx, LayoutSpec.Folder f, List<App> in, LayoutSpec.Item style) {
        android.app.Dialog d = new android.app.Dialog(ctx, android.R.style.Theme_Translucent_NoTitleBar_Fullscreen);
        FrameLayout dim = new FrameLayout(ctx);
        dim.setBackgroundColor(0xCC000000);
        LayoutSpec.Row r = new LayoutSpec.Row();
        r.title = f.name;
        r.source = "favorites";
        for (App a : in) r.packages.add(a.pkg);
        r.max = 24;
        View row = appRow(ctx, r, style);
        if (row == null) return;
        dim.addView(row, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT, Gravity.CENTER_VERTICAL));
        d.setContentView(dim);
        d.setOnShowListener(x -> {
            ArrayList<View> out = new ArrayList<>();
            row.addFocusables(out, View.FOCUS_DOWN);
            if (!out.isEmpty()) out.get(0).requestFocus();
        });
        openFolders.add(d);
        d.setOnDismissListener(x -> openFolders.remove(d));
        d.show();
    }

    private final List<android.app.Dialog> openFolders = new ArrayList<>();

    /** Top-right button showing who is using the TV; OK opens the profile picker. */
    private View gearView;
    static final double GEAR_W = 4.5, GEAR_H = 6;

    /** The widgets drawn on the first screen, as {x, y, w, h} in percent (what Studio's overlap check uses too). */
    static List<double[]> widgetBoxes(LayoutSpec spec, boolean scroll) {
        List<double[]> out = new ArrayList<>();
        for (LayoutSpec.Item it : spec.items) {
            if (!it.enabled || "shape".equals(it.kind) || (scroll && "apps".equals(it.kind))) continue;
            out.add(new double[]{it.x, it.y, it.w, it.h});
        }
        if (!spec.profiles.isEmpty()) out.add(new double[]{84, 3, 13, 6});
        return out;
    }

    /** The first corner clear of the widgets for the ⚙ button, in the order Studio's gearSpot uses. */
    static double[] gearSpot(List<double[]> boxes, boolean profiles) {
        double[][] spots = {{profiles ? 78.5 : 92.5, 3}, {94.5, 91}, {1, 91}, {1, 3}};
        for (double[] s : spots) {
            boolean free = true;
            for (double[] b : boxes) {
                double ox = Math.min(s[0] + GEAR_W, b[0] + b[2]) - Math.max(s[0], b[0]);
                double oy = Math.min(s[1] + GEAR_H, b[1] + b[3]) - Math.max(s[1], b[1]);
                if (ox > 0.5 && oy > 0.5) { free = false; break; }
            }
            if (free) return s;
        }
        return spots[0];
    }

    private View gearChip(Context ctx) {
        TextView g = new TextView(ctx);
        g.setText("⚙");
        g.setGravity(Gravity.CENTER);
        g.setIncludeFontPadding(false);
        g.setTextColor(Color.WHITE);
        g.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 30 * unit);
        g.setFocusable(true);
        g.setClickable(true);
        g.setContentDescription("Home settings");
        GradientDrawable bg = new GradientDrawable();
        bg.setShape(GradientDrawable.OVAL);
        bg.setColor(0x44000000);
        bg.setStroke(Math.max(1, (int) (2 * unit)), 0x55FFFFFF);
        g.setBackground(bg);
        final int ring = spec.hasAccent ? spec.accent : Color.WHITE;
        g.setOnFocusChangeListener((v, f) -> bg.setStroke(Math.max(1, (int) ((f ? 4 : 2) * unit)), f ? ring : 0x55FFFFFF));
        g.setOnClickListener(v -> openSettings());
        return g;
    }

    void openSettings() {
        getContext().startActivity(new Intent(getContext(), CanvasSettingsActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
    }

    private View profileChip(Context ctx) {
        String name = spec.profileName;
        int color = spec.profileColor;
        for (LayoutSpec.Profile p : spec.profiles) if (p.id.equals(spec.currentProfile)) { name = p.name; color = p.color; }
        LinearLayout chip = new LinearLayout(ctx);
        chip.setOrientation(LinearLayout.HORIZONTAL);
        chip.setGravity(Gravity.CENTER_VERTICAL);
        chip.setFocusable(true);
        chip.setClickable(true);
        int pad = (int) (10 * unit);
        chip.setPadding(pad, pad / 2, pad * 2, pad / 2);
        GradientDrawable bg = new GradientDrawable();
        bg.setCornerRadius(999);
        bg.setColor(0x44000000);
        bg.setStroke(Math.max(1, (int) (2 * unit)), 0x55FFFFFF);
        chip.setBackground(bg);
        TextView avatar = new TextView(ctx);
        avatar.setText(name.substring(0, 1).toUpperCase(Locale.ROOT));
        avatar.setGravity(Gravity.CENTER);
        avatar.setTextColor(Color.WHITE);
        avatar.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 22 * unit);
        GradientDrawable dot = new GradientDrawable();
        dot.setShape(GradientDrawable.OVAL);
        dot.setColor(color | 0xFF000000);
        avatar.setBackground(dot);
        int a = (int) (40 * unit);
        chip.addView(avatar, new LinearLayout.LayoutParams(a, a));
        TextView label = new TextView(ctx);
        label.setText(name);
        label.setTextColor(Color.WHITE);
        label.setSingleLine(true);
        label.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 22 * unit);
        label.setPadding(pad, 0, 0, 0);
        chip.addView(label);
        final int ring = spec.hasAccent ? spec.accent : Color.WHITE;
        chip.setOnFocusChangeListener((v, f) -> bg.setStroke(Math.max(1, (int) ((f ? 4 : 2) * unit)), f ? ring : 0x55FFFFFF));
        chip.setOnClickListener(v -> pickProfile(ctx));
        return chip;
    }

    /** Choose who is using the TV: the main layout or one of the profiles (a protected one asks for the PIN). */
    private void pickProfile(Context ctx) {
        android.app.Dialog d = new android.app.Dialog(ctx, android.R.style.Theme_Translucent_NoTitleBar_Fullscreen);
        LinearLayout box = new LinearLayout(ctx);
        box.setOrientation(LinearLayout.HORIZONTAL);
        box.setGravity(Gravity.CENTER);
        box.setBackgroundColor(0xE6000000);
        List<String[]> all = new ArrayList<>();
        all.add(new String[]{"", spec.profileName, Integer.toString(spec.profileColor), "0"});
        for (LayoutSpec.Profile p : spec.profiles) all.add(new String[]{p.id, p.name, Integer.toString(p.color), p.pin ? "1" : "0"});
        int size = (int) (150 * unit);
        View first = null;
        for (String[] p : all) {
            LinearLayout cell = new LinearLayout(ctx);
            cell.setOrientation(LinearLayout.VERTICAL);
            cell.setGravity(Gravity.CENTER_HORIZONTAL);
            cell.setFocusable(true);
            cell.setClickable(true);
            cell.setPadding((int) (24 * unit), (int) (16 * unit), (int) (24 * unit), (int) (16 * unit));
            TextView av = new TextView(ctx);
            av.setText(p[1].substring(0, 1).toUpperCase(Locale.ROOT));
            av.setGravity(Gravity.CENTER);
            av.setTextColor(Color.WHITE);
            av.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 64 * unit);
            GradientDrawable dot = new GradientDrawable();
            dot.setShape(GradientDrawable.OVAL);
            dot.setColor(Integer.parseInt(p[2]) | 0xFF000000);
            av.setBackground(dot);
            cell.addView(av, new LinearLayout.LayoutParams(size, size));
            TextView n = new TextView(ctx);
            n.setText(p[1] + ("1".equals(p[3]) ? "  🔒" : ""));
            n.setTextColor(Color.WHITE);
            n.setGravity(Gravity.CENTER);
            n.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 28 * unit);
            n.setPadding(0, (int) (12 * unit), 0, 0);
            cell.addView(n);
            cell.setOnFocusChangeListener((v, f) -> v.animate().scaleX(f ? 1.12f : 1f).scaleY(f ? 1.12f : 1f).setDuration(reduceMotion ? 0 : 180).start());
            final String id = p[0];
            final boolean pin = "1".equals(p[3]);
            cell.setOnClickListener(v -> {
                Runnable go = () -> {
                    d.dismiss();
                    Profiles.choose(ctx, id);
                    if (ctx instanceof android.app.Activity) ((android.app.Activity) ctx).recreate();
                };
                if (pin && !id.equals(spec.currentProfile)) PinDialog.ask(ctx, "Enter the PIN for " + p[1], spec.hasAccent ? spec.accent : 0xFF5B8DEF, go);
                else go.run();
            });
            box.addView(cell);
            if (id.equals(spec.currentProfile)) first = cell;
        }
        d.setContentView(box);
        final View focus = first != null ? first : box.getChildAt(0);
        d.setOnShowListener(x -> focus.requestFocus());
        openFolders.add(d);
        d.setOnDismissListener(x -> openFolders.remove(d));
        d.show();
    }

    /** "Recently used": medium banner tiles in a sideways row, topped up with other apps while history is short. */
    private View shelf(Context ctx, LayoutSpec.Item it) {
        List<App> used = recent(ctx, it);
        boolean history = !used.isEmpty();
        List<App> list = new ArrayList<>(used);
        int want = Math.max(6, Math.min(12, it.max));
        for (App a : apps(ctx)) {
            if (list.size() >= want) break;
            boolean dup = false;
            for (App b : list) if (b.pkg.equals(a.pkg)) { dup = true; break; }
            if (!dup) list.add(a);
        }
        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setClipChildren(false);
        col.setClipToPadding(false);
        TextView head = new TextView(ctx);
        style(head, it);
        head.setBackground(null);
        head.setAlpha(0.8f);
        head.setText(history ? "Recently used" : "Your apps");
        head.setPadding(0, 0, 0, (int) (10 * unit));   // room for the focused tile to grow without touching it
        col.addView(head);
        // Tile size follows the item's height in the editor: header and the focused name take the rest.
        int itemH = (int) (getResources().getDisplayMetrics().heightPixels * it.h / 100);
        float text = (float) it.size * unit;
        int tileH = Math.max((int) (60 * unit), itemH - (int) (text * 3.4f));
        // The card fills the row's height whatever the tile style (icon cards are square, posters tall).
        int tileW;
        switch (spec.tiles) {
            case "icon": tileW = (int) (tileH * 0.82f / 0.62f); break;
            case "poster": tileW = (int) (tileH * 0.95f / (0.62f * 1.5f)); break;
            case "tvos": tileW = tileH * 5 / 3; break;   // 5:3 cards: a 16:9 width made them taller than the row
            default: tileW = tileH * 16 / 9; break;
        }
        Row row = new Row(ctx);
        LinearLayout inner = new LinearLayout(ctx);
        inner.setOrientation(LinearLayout.HORIZONTAL);
        inner.setClipChildren(false);
        inner.setClipToPadding(false);
        int pad = (int) (tileW * 0.07f);
        inner.setPadding(pad, pad / 2, pad, 0);
        for (App a : list) {
            View t = tile(ctx, a, tileW, it);
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            lp.rightMargin = (int) (tileW * 0.08f);
            inner.addView(t, lp);
        }
        row.addView(inner);
        col.addView(row, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        return col;
    }

    /** The second screen: every app, in rows, reached with Down. */
    private View allApps(Context ctx, LayoutSpec.Item it) {
        int sw = getResources().getDisplayMetrics().widthPixels;
        int sh = getResources().getDisplayMetrics().heightPixels;
        int cols = spec.tvos() ? 5 : it != null && it.hasIcon ? Math.max(4, Math.min(8, it.columns)) : 6;
        int margin = (int) (sw * 0.04f), gap = (int) (sw * 0.016f);
        int tileW = (sw - 2 * margin - (cols - 1) * gap) / cols;
        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setClipChildren(false);
        col.setClipToPadding(false);
        col.setPadding(margin, 0, margin, (int) (sh * 0.12f));
        TextView head = new TextView(ctx);
        if (it != null) { style(head, it); head.setBackground(null); }
        else { head.setTextColor(Color.WHITE); head.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 28 * unit); }
        head.setText("All apps");
        head.setAlpha(0.85f);
        head.setPadding(0, 0, 0, (int) (sh * 0.02f));
        col.addView(head);
        List<App> everything = apps(ctx);
        java.util.Set<String> foldered = new java.util.HashSet<>();
        List<Object> cells = new ArrayList<>();
        for (LayoutSpec.Folder f : spec.folders) {
            List<App> in = pick(everything, f.packages);
            if (in.isEmpty()) continue;
            for (App a : in) foldered.add(a.pkg);
            cells.add(new Object[]{f, in});
        }
        for (App a : everything) if (!foldered.contains(a.pkg)) cells.add(a);
        LinearLayout row = null;
        for (int i = 0; i < cells.size(); i++) {
            if (i % cols == 0) {
                row = new LinearLayout(ctx);
                row.setOrientation(LinearLayout.HORIZONTAL);
                row.setClipChildren(false);
                row.setClipToPadding(false);
                LinearLayout.LayoutParams rl = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
                rl.bottomMargin = gap / 2;
                col.addView(row, rl);
            }
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            if (i % cols != cols - 1) lp.rightMargin = gap;
            Object c = cells.get(i);
            View v = c instanceof App ? tile(ctx, (App) c, tileW, it) : folderTile(ctx, (LayoutSpec.Folder) ((Object[]) c)[0], castApps(((Object[]) c)[1]), tileW, it);
            row.addView(v, lp);
        }
        return col;
    }

    /** One app tile in the look Studio chose: banner, card (icon and name on a tinted card), icon, or poster; the
     *  icon shape, the focus style and the accent colour come from the layout. The name shows under it when focused. */
    private View tile(Context ctx, App a, int w, LayoutSpec.Item it) {
        final String style = spec.tiles;
        int cw = w, h = w * 9 / 16;
        if ("tvos".equals(style)) h = w * 3 / 5;
        else if ("icon".equals(style)) { cw = (int) (w * 0.62f); h = cw; }
        else if ("poster".equals(style)) { cw = (int) (w * 0.62f); h = (int) (cw * 1.5f); }
        final int cardW = cw, cardH = h;
        final float radius = "tvos".equals(style) && "rounded".equals(spec.iconShape) ? cardH * 0.13f : cornerRadius(spec.iconShape, cardW, cardH);
        final boolean leaf = "leaf".equals(spec.iconShape);
        final int ringColor = spec.hasAccent ? spec.accent : Color.WHITE;
        LinearLayout cell = new LinearLayout(ctx);
        cell.setOrientation(LinearLayout.VERTICAL);
        cell.setGravity(Gravity.CENTER_HORIZONTAL);
        cell.setFocusable(true);
        cell.setClickable(true);
        cell.setClipChildren(false);
        FrameLayout card = new FrameLayout(ctx);
        card.setClipToOutline(true);
        card.setOutlineProvider(new ViewOutlineProvider() {
            @Override public void getOutline(View v, Outline o) {
                if (leaf) {
                    android.graphics.Path p = new android.graphics.Path();
                    float big = Math.min(v.getWidth(), v.getHeight()) * 0.42f, small = Math.min(v.getWidth(), v.getHeight()) * 0.08f;
                    p.addRoundRect(new android.graphics.RectF(0, 0, v.getWidth(), v.getHeight()), new float[]{big, big, small, small, big, big, small, small}, android.graphics.Path.Direction.CW);
                    o.setConvexPath(p);
                } else o.setRoundRect(0, 0, v.getWidth(), v.getHeight(), radius);
            }
        });
        int tint = a.look != null && a.look.hasColor ? a.look.color | 0xFF000000 : 0xFF000000 | (a.label.hashCode() & 0x3F3F3F) | 0x202020;
        card.setBackgroundColor(tint);
        ImageView iv = new ImageView(ctx);
        card.addView(iv, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        final File ownPicture = a.look != null && a.look.image != null && dir != null ? new File(new File(dir, "tiles"), a.look.image) : null;
        if ("parallax".equals(spec.focus)) {
            // The shine that slides across the icon when it takes focus, as on a set-top box.
            View glare = new View(ctx);
            glare.setBackground(new GradientDrawable(GradientDrawable.Orientation.TL_BR, new int[]{0x00FFFFFF, 0x55FFFFFF, 0x00FFFFFF}));
            glare.setAlpha(0f);
            card.addView(glare, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
            card.setTag(glare);
        }
        TextView inCard = null;
        if ("card".equals(style)) {
            inCard = new TextView(ctx);
            inCard.setTextColor(Color.WHITE);
            inCard.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, cardH * 0.13f);
            inCard.setSingleLine(true);
            inCard.setEllipsize(android.text.TextUtils.TruncateAt.END);
            inCard.setGravity(Gravity.CENTER_HORIZONTAL);
            inCard.setText(a.label);
            FrameLayout.LayoutParams lp = new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT, Gravity.BOTTOM);
            lp.bottomMargin = (int) (cardH * 0.08f);
            card.addView(inCard, lp);
        }
        GradientDrawable ring = new GradientDrawable();
        ring.setCornerRadius(leaf ? Math.min(cardW, cardH) * 0.2f : radius);
        boolean glow = "glow".equals(spec.focus);
        ring.setStroke(Math.max(2, (int) ((glow ? 5 : 3) * unit)), ringColor);
        ring.setColor(glow ? (ringColor & 0x00FFFFFF) | 0x22000000 : Color.TRANSPARENT);
        cell.addView(card, new LinearLayout.LayoutParams(cardW, cardH));
        TextView name = new TextView(ctx);
        if (it != null) {
            name.setTypeface(Typeface.create(it.font, Typeface.NORMAL));
            name.setTextColor(it.color);
            name.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, (float) Math.min(it.size, 30) * unit);
        } else {
            name.setTextColor(Color.WHITE);
            name.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 22 * unit);
        }
        name.setSingleLine(true);
        name.setEllipsize(android.text.TextUtils.TruncateAt.END);
        name.setGravity(Gravity.CENTER_HORIZONTAL);
        name.setText(a.label);
        // Icon tiles always say which app it is; card tiles carry the name inside.
        final boolean nameAlways = "icon".equals(style);
        name.setAlpha(nameAlways ? 1f : 0f);
        if ("card".equals(style)) name.setVisibility(View.GONE);
        name.setPadding(0, (int) (8 * unit), 0, 0);
        cell.addView(name, new LinearLayout.LayoutParams(Math.max(cardW, w), ViewGroup.LayoutParams.WRAP_CONTENT));
        card.setCameraDistance(8000 * unit);
        final boolean ringOn = !"lift".equals(spec.focus) && !"zoom".equals(spec.focus) && !"parallax".equals(spec.focus);
        cell.setOnFocusChangeListener((v, focused) -> focusLook(v, card, name, focused, ringOn ? ring : null, nameAlways));
        cell.setOnClickListener(v -> open(v.getContext(), a.pkg, card));
        final TextView cardName = inCard;
        // Banners decode off the main thread so a long app list never stalls the first frame.
        final PackageManager pm = ctx.getPackageManager();
        if (a.pkg.startsWith("folder.")) return cell;   // folder tiles draw their own picture
        loader.execute(() -> {
            Drawable banner = null, icon = null;
            if (ownPicture != null && ownPicture.isFile()) {
                Bitmap own = decodeStill(ownPicture);
                if (own != null) {
                    final Drawable pic = new android.graphics.drawable.BitmapDrawable(getResources(), own);
                    post(() -> { iv.setScaleType(ImageView.ScaleType.CENTER_CROP); iv.setImageDrawable(pic); });
                    return;
                }
            }
            boolean wantsIcon = "icon".equals(style) || "card".equals(style);
            if (!wantsIcon) { try { banner = pm.getApplicationBanner(a.pkg); } catch (PackageManager.NameNotFoundException ignored) { } }
            if (banner == null && !wantsIcon) {
                Intent li = pm.getLeanbackLaunchIntentForPackage(a.pkg);
                if (li != null) {
                    ResolveInfo ri = pm.resolveActivity(li, 0);
                    if (ri != null) banner = ri.activityInfo.loadBanner(pm);
                }
            }
            if (banner == null) { try { icon = pm.getApplicationIcon(a.pkg); } catch (PackageManager.NameNotFoundException ignored) { } }
            final Drawable b = banner, ic = icon;
            post(() -> {
                if (b != null) { iv.setScaleType(ImageView.ScaleType.CENTER_CROP); iv.setImageDrawable(b); }
                else if (ic != null) {
                    int p = (int) (cardH * ("icon".equals(style) ? 0.16f : 0.2f));
                    int bottom = cardName != null ? (int) (cardH * 0.34f) : p;
                    iv.setPadding(p, p, p, bottom);
                    iv.setScaleType(ImageView.ScaleType.FIT_CENTER);
                    iv.setImageDrawable(ic);
                    if (cardName == null && !"tvos".equals(style)) name.setAlpha(1f);   // an icon alone does not say which app it is
                }
            });
        });
        return cell;
    }

    @Override
    protected void onLayout(boolean changed, int l, int t, int r, int b) { layoutPercent(this, r - l, b - t); }

    /** Children with an item tag at their percentage box; the rest (background, scroller) full size. */
    static void layoutPercent(ViewGroup g, int w, int h) {
        for (int i = 0; i < g.getChildCount(); i++) {
            View v = g.getChildAt(i);
            Object tag = v.getTag();
            if (tag instanceof LayoutSpec.Item) {
                LayoutSpec.Item it = (LayoutSpec.Item) tag;
                int x = (int) (w * it.x / 100), y = (int) (h * it.y / 100), iw = (int) (w * it.w / 100), ih = (int) (h * it.h / 100);
                v.measure(MeasureSpec.makeMeasureSpec(iw, MeasureSpec.EXACTLY), MeasureSpec.makeMeasureSpec(ih, MeasureSpec.EXACTLY));
                v.layout(x, y, x + iw, y + ih);
            } else {
                v.measure(MeasureSpec.makeMeasureSpec(w, MeasureSpec.EXACTLY), MeasureSpec.makeMeasureSpec(h, MeasureSpec.EXACTLY));
                v.layout(0, 0, w, h);
            }
        }
    }

    // ---------------------------------------------------------------- background

    private void buildBackground(Context ctx) {
        if ("scene".equals(spec.bgKind)) {
            ambient = new AmbientScene(ctx, spec.scene, spec.sceneTime, heavy);
            addView(ambient);
        } else if ("gradient".equals(spec.bgKind)) {
            GradientDrawable g = new GradientDrawable(GradientDrawable.Orientation.TL_BR, new int[]{spec.color | 0xFF000000, spec.color2 | 0xFF000000});
            View v = new View(ctx);
            v.setBackground(g);
            addView(v);
        } else if ("image".equals(spec.bgKind) || "video".equals(spec.bgKind)) {
            File picture = firstPicture();
            Drawable moving = "image".equals(spec.bgKind) && heavy ? animatedPicture(picture) : null;
            Bitmap still = moving == null ? decodeStill(picture) : null;
            if (moving != null || still != null) {
                ImageView iv = new ImageView(ctx);
                iv.setScaleType(ImageView.ScaleType.CENTER_CROP);
                if (moving != null) { iv.setImageDrawable(moving); movingPicture = moving; }
                else iv.setImageBitmap(still);
                addView(iv);
            }
        }
        if ("video".equals(spec.bgKind) && spec.video != null && videoAllowed(ctx)) {
            File f = new File(new File(dir, "video"), LayoutSpec.videoFile(spec.video));
            if (f.isFile()) {
                video = new VideoBackground(ctx, f);
                addView(video);
            }
        }
        if (spec.dim > 0) {
            View dim = new View(ctx);
            dim.setBackgroundColor(Color.argb((int) (spec.dim * 255), 0, 0, 0));
            addView(dim);
        }
    }

    /** Video is skipped on low-RAM TVs and when the TV's animations are turned off: the still wallpaper stays. */
    private boolean videoAllowed(Context ctx) {
        return heavy;
    }

    /** An animated GIF or WebP, playing on Android 9+; null for a still picture or an older TV. */
    private Drawable movingPicture;

    /** The theme's first wallpaper (the order the rotation uses), or the single wallpaper.jpg. */
    private File firstPicture() {
        if (dir == null) return null;
        List<File> pics = WallpaperRotation.images(new File(dir, "wallpapers"));
        if (!pics.isEmpty()) return pics.get(0);
        File f = new File(dir, "wallpaper.jpg");
        return f.isFile() ? f : null;
    }

    private static Drawable animatedPicture(File f) {
        if (f == null || android.os.Build.VERSION.SDK_INT < 28) return null;
        String n = f.getName().toLowerCase(Locale.ROOT);
        if (!n.endsWith(".gif") && !n.endsWith(".webp")) return null;
        try {
            Drawable d = android.graphics.ImageDecoder.decodeDrawable(android.graphics.ImageDecoder.createSource(f),
                    (decoder, info, src) -> {
                        // Keep big animations within 1080p so a 4K GIF never holds a 4K frame buffer.
                        int w = info.getSize().getWidth(), h = info.getSize().getHeight();
                        float s = Math.min(1f, Math.min(1920f / Math.max(1, w), 1080f / Math.max(1, h)));
                        if (s < 1f) decoder.setTargetSize(Math.max(1, (int) (w * s)), Math.max(1, (int) (h * s)));
                    });
            if (d instanceof android.graphics.drawable.AnimatedImageDrawable) {
                ((android.graphics.drawable.AnimatedImageDrawable) d).setRepeatCount(android.graphics.drawable.AnimatedImageDrawable.REPEAT_INFINITE);
                return d;
            }
        } catch (java.io.IOException | RuntimeException e) {
            // not decodable as an animation: the still decode below takes over
        }
        return null;
    }

    private static Bitmap decodeStill(File f) {
        if (f == null || !f.isFile()) return null;
        BitmapFactory.Options o = new BitmapFactory.Options();
        o.inJustDecodeBounds = true;
        BitmapFactory.decodeFile(f.getPath(), o);
        int sample = 1;
        while (o.outWidth / (sample * 2) >= 1920) sample *= 2;
        o = new BitmapFactory.Options();
        o.inSampleSize = sample;
        return BitmapFactory.decodeFile(f.getPath(), o);
    }

    // ---------------------------------------------------------------- items

    private View build(Context ctx, LayoutSpec.Item it) {
        switch (it.kind) {
            case "clock":
                if ("analog".equals(it.variant)) {
                    Widgets.AnalogClockView v = new Widgets.AnalogClockView(ctx, it);
                    v.setAlpha((float) it.opacity);
                    ticks.add(() -> v.invalidate());
                    return v;
                }
                if ("stacked".equals(it.variant)) {
                    TextView t = label(ctx, it, () -> stackedClock(it));
                    t.setLineSpacing(0, 0.88f);
                    return t;
                }
                return label(ctx, it, () -> clockText(it));
            case "date": return label(ctx, it, () -> dateText(it));
            case "greeting": return label(ctx, it, CanvasHome::greeting);
            case "text": return label(ctx, it, () -> it.text == null ? "" : it.text);
            case "cpu": return stat(ctx, it, "CPU", "CPU use", () -> stats.cpuPercent(), false);
            case "memory": return stat(ctx, it, "MEM", "Memory in use", () -> stats.memoryPercent(), false);
            case "network": return stat(ctx, it, "\u2193", "Network \u2193", () -> stats.downMBps(), true);
            case "calendar": {
                Widgets.CalendarView v = new Widgets.CalendarView(ctx, it, typeface(it), unit);
                v.setAlpha((float) it.opacity);
                return v;
            }
            case "visualizer": {
                Widgets.VisualizerView v = new Widgets.VisualizerView(ctx, it, heavy);
                animated.add(v);
                return v;
            }
            case "shape": return new Widgets.ShapeView(ctx, it, unit);
            case "apps": return strip(ctx, it, apps(ctx, it.packages), false);
            case "recent": return strip(ctx, it, recent(ctx, it), true);
            case "watchnext": return watchNext(ctx, it);
            case "bluetooth": return label(ctx, it, () -> bluetoothText(ctx));
            case "nowplaying": return nowPlaying(ctx, it);
            case "weather": return weatherView(ctx, it);
            default: return null;
        }
    }

    private interface Text { String get(); }
    private interface Reading { double get(); }

    private View stat(Context ctx, LayoutSpec.Item it, String shortName, String longName, Reading reading, boolean network) {
        if ("text".equals(it.variant)) {
            return label(ctx, it, () -> {
                double v = reading.get();
                String num = v < 0 ? "n/a" : network ? String.format(Locale.US, "%.1f MB/s", v) : Math.round(v) + "%";
                return (it.compact ? shortName + " " : longName + " ") + num;
            });
        }
        Widgets.StatView view = new Widgets.StatView(ctx, it, it.compact ? shortName : longName, typeface(it), unit, network);
        view.setAlpha((float) it.opacity);
        ticks.add(() -> view.set(reading.get()));
        return view;
    }

    /** The item's font: one of Android's built-in families, or the person's own file under fonts/ (any problem: sans-serif). */
    private Typeface typeface(LayoutSpec.Item it) {
        Typeface base = null;
        if (it.fontFile != null && dir != null) {
            try {
                File f = new File(new File(dir, "fonts"), it.fontFile);
                if (f.isFile() && f.length() < 4 * 1024 * 1024) base = Typeface.createFromFile(f);
            } catch (RuntimeException e) {
                base = null;
            }
        }
        if (base == null) base = Typeface.create(it.fontFile != null ? "sans-serif" : it.font, Typeface.NORMAL);
        return it.italic ? Typeface.create(base, Typeface.ITALIC) : base;
    }

    // ---------------------------------------------------------------- weather

    private final List<Runnable> weatherViews = new ArrayList<>();

    /** Current weather (compact: icon and temperature; full: also the condition and today's high/low; forecast:
     *  three days). Shows what to do when weather is off or the city has not been found yet. */
    private View weatherView(Context ctx, LayoutSpec.Item it) {
        TextView t = new TextView(ctx);
        style(t, it);
        t.setGravity(gravity(it) | Gravity.CENTER_VERTICAL);
        t.setFocusable(false);
        Runnable show = () -> {
            String text;
            if (!spec.weatherOn || spec.weatherCity == null) text = "Weather is off";
            else {
                Weather.Now n = Weather.cached(ctx, spec.weatherCity);
                text = n == null ? "Weather for " + spec.weatherCity + "…" : weatherText(n, it.variant);
            }
            if (!text.contentEquals(t.getText())) t.setText(text);
        };
        show.run();
        weatherViews.add(show);
        return t;
    }

    private String weatherText(Weather.Now n, String variant) {
        boolean f = spec.fahrenheit;
        String now = Weather.icon(n.code, n.day) + " " + Weather.temp(n.temp, f);
        if ("full".equals(variant)) return now + "  " + Weather.word(n.code) + "\n" + n.city + "  \u2191" + Weather.temp(n.hi[0], f) + " \u2193" + Weather.temp(n.lo[0], f);
        if ("forecast".equals(variant)) {
            StringBuilder sb = new StringBuilder(now + "  " + n.city);
            String[] names = {"Today", "Tomorrow", dayName(2)};
            for (int i = 0; i < 3; i++) sb.append("\n").append(names[i]).append("  ").append(Weather.icon(n.dayCode[i], true)).append(" ")
                    .append(Weather.temp(n.hi[i], f)).append(" / ").append(Weather.temp(n.lo[i], f));
            return sb.toString();
        }
        return now;
    }

    private static String dayName(int ahead) {
        Calendar c = Calendar.getInstance();
        c.add(Calendar.DAY_OF_YEAR, ahead);
        return c.getDisplayName(Calendar.DAY_OF_WEEK, Calendar.LONG, Locale.getDefault());
    }

    /** Fresh weather when it is on and anything shows it (a widget or a scene that follows it). */
    private void refreshWeather() {
        if (!spec.weatherOn || spec.weatherCity == null) return;
        if (weatherViews.isEmpty() && !(spec.followWeather && ambient != null)) return;
        Weather.refresh(getContext(), spec.weatherCity, () -> post(this::applyWeather));
        applyWeather();
    }

    private void applyWeather() {
        for (Runnable r : weatherViews) r.run();
        if (spec.followWeather && ambient != null && spec.weatherCity != null) {
            Weather.Now n = Weather.cached(getContext(), spec.weatherCity);
            String want = n == null ? spec.scene : Weather.sceneFor(n.code, spec.scene);
            if (!want.equals(ambient.sceneId())) {
                int at = indexOfChild(ambient);
                ambient.stop();
                removeView(ambient);
                ambient = new AmbientScene(getContext(), want, spec.sceneTime, heavy);
                addView(ambient, Math.max(0, at));
                if (started) ambient.start();
            }
        }
    }

    private final Runnable weatherTick = new Runnable() {
        @Override public void run() {
            if (!started) return;
            refreshWeather();
            handler.postDelayed(this, Weather.REFRESH_MS);
        }
    };

    // ---------------------------------------------------------------- bluetooth and now playing

    /** The Bluetooth headphones or speaker the sound goes to, else whether Bluetooth is on. No permission needed:
     *  the audio outputs say which device plays, and Android 12+ lets any app ask if Bluetooth is on. */
    static String bluetoothText(Context ctx) {
        android.media.AudioManager am = (android.media.AudioManager) ctx.getSystemService(Context.AUDIO_SERVICE);
        if (am != null) {
            for (android.media.AudioDeviceInfo d : am.getDevices(android.media.AudioManager.GET_DEVICES_OUTPUTS)) {
                int t = d.getType();
                if (t == android.media.AudioDeviceInfo.TYPE_BLUETOOTH_A2DP || t == android.media.AudioDeviceInfo.TYPE_HEARING_AID
                        || t == 26 /* BLE headset, API 31 */ || t == 27 /* BLE speaker, API 31 */) {
                    CharSequence n = d.getProductName();
                    return "\uD83C\uDFA7 " + (n == null || n.length() == 0 ? "Bluetooth audio" : n);
                }
            }
        }
        try {
            android.bluetooth.BluetoothManager bm = (android.bluetooth.BluetoothManager) ctx.getSystemService(Context.BLUETOOTH_SERVICE);
            android.bluetooth.BluetoothAdapter a = bm == null ? null : bm.getAdapter();
            if (a == null) return "No Bluetooth";
            return a.isEnabled() ? "Bluetooth on" : "Bluetooth off";
        } catch (SecurityException e) {
            return "Bluetooth";
        }
    }

    /** What is playing (title, artist, app) and a play/pause button. Needs the listener Studio switches on. */
    private View nowPlaying(Context ctx, LayoutSpec.Item it) {
        LinearLayout row = new LinearLayout(ctx);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setGravity(Gravity.CENTER_VERTICAL);
        TextView button = new TextView(ctx);
        style(button, it);
        button.setBackground(rounded(0x33FFFFFF, 999));
        button.setGravity(Gravity.CENTER);
        button.setText("\u23EF");
        button.setFocusable(true);
        button.setClickable(true);
        int size = (int) ((float) it.size * unit * 1.8f);
        TextView info = new TextView(ctx);
        style(info, it);
        info.setSingleLine(true);
        info.setEllipsize(android.text.TextUtils.TruncateAt.END);
        info.setPadding((int) (12 * unit), 0, 0, 0);
        info.setFocusable(false);
        row.addView(button, new LinearLayout.LayoutParams(size, size));
        row.addView(info, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f));
        final android.media.session.MediaController[] current = new android.media.session.MediaController[1];
        final int ring = spec.hasAccent ? spec.accent : Color.WHITE;
        button.setOnFocusChangeListener((v, f) -> v.setBackground(f ? rounded(ring, 999) : rounded(0x33FFFFFF, 999)));
        button.setOnClickListener(v -> {
            android.media.session.MediaController c = current[0];
            if (c == null) return;
            android.media.session.PlaybackState st = c.getPlaybackState();
            if (st != null && st.getState() == android.media.session.PlaybackState.STATE_PLAYING) c.getTransportControls().pause();
            else c.getTransportControls().play();
        });
        Runnable update = () -> {
            String text;
            current[0] = null;
            try {
                android.media.session.MediaSessionManager msm = (android.media.session.MediaSessionManager) ctx.getSystemService(Context.MEDIA_SESSION_SERVICE);
                List<android.media.session.MediaController> list = msm == null ? java.util.Collections.emptyList()
                        : msm.getActiveSessions(new android.content.ComponentName(ctx, MediaListener.class));
                android.media.session.MediaController pick = null;
                for (android.media.session.MediaController c : list) {
                    android.media.session.PlaybackState st = c.getPlaybackState();
                    if (st != null && st.getState() == android.media.session.PlaybackState.STATE_PLAYING) { pick = c; break; }
                    if (pick == null) pick = c;
                }
                current[0] = pick;
                text = pick == null ? "Nothing playing" : describe(ctx, pick);
            } catch (SecurityException e) {
                text = "Now Playing is switched off";
            }
            if (!text.contentEquals(info.getText())) info.setText(text);
            button.setVisibility(current[0] == null ? View.INVISIBLE : View.VISIBLE);
        };
        update.run();
        ticks.add(update);
        return row;
    }

    private static String describe(Context ctx, android.media.session.MediaController c) {
        android.media.MediaMetadata m = c.getMetadata();
        String title = m == null ? null : m.getString(android.media.MediaMetadata.METADATA_KEY_DISPLAY_TITLE);
        if (title == null && m != null) title = m.getString(android.media.MediaMetadata.METADATA_KEY_TITLE);
        String artist = m == null ? null : m.getString(android.media.MediaMetadata.METADATA_KEY_ARTIST);
        String app;
        try { app = String.valueOf(ctx.getPackageManager().getApplicationLabel(ctx.getPackageManager().getApplicationInfo(c.getPackageName(), 0))); }
        catch (PackageManager.NameNotFoundException e) { app = c.getPackageName(); }
        android.media.session.PlaybackState st = c.getPlaybackState();
        boolean playing = st != null && st.getState() == android.media.session.PlaybackState.STATE_PLAYING;
        StringBuilder sb = new StringBuilder(playing ? "\u25B6 " : "\u23F8 ");
        sb.append(title == null || title.isEmpty() ? app : title);
        if (artist != null && !artist.isEmpty()) sb.append(" \u2014 ").append(artist);
        if (title != null && !title.isEmpty()) sb.append("  \u00B7  ").append(app);
        return sb.toString();
    }

    private TextView label(Context ctx, LayoutSpec.Item it, Text source) {
        // TextView.setAllCaps needs API 28 and this launcher supports 26, so capitals are applied to the text itself.
        final Text text = it.upper ? () -> source.get().toUpperCase(Locale.getDefault()) : source;
        TextView t = new TextView(ctx);
        style(t, it);
        t.setGravity(gravity(it) | Gravity.CENTER_VERTICAL);
        t.setText(text.get());
        t.setFocusable(false);
        ticks.add(() -> { String s = text.get(); if (!s.contentEquals(t.getText())) t.setText(s); });
        return t;
    }

    private void style(TextView t, LayoutSpec.Item it) {
        t.setTypeface(typeface(it));
        t.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, (float) it.size * unit);
        t.setTextColor(it.color);
        t.setAlpha((float) it.opacity);
        t.setIncludeFontPadding(false);
        if (it.spacing > 0) t.setLetterSpacing((float) it.spacing);
        if (it.shadow) t.setShadowLayer(8 * unit, 0, 3 * unit, 0x99000000);
        if (it.hasFill) t.setBackground(rounded(it.fill, it.radius));
        t.setPadding((int) (8 * unit), 0, (int) (8 * unit), 0);
    }

    private static int gravity(LayoutSpec.Item it) {
        return "end".equals(it.align) ? Gravity.END : "center".equals(it.align) ? Gravity.CENTER_HORIZONTAL : Gravity.START;
    }

    private GradientDrawable rounded(int color, double radius) {
        GradientDrawable g = new GradientDrawable();
        g.setColor(color);
        g.setCornerRadius((float) radius * unit);
        return g;
    }

    private String clockText(LayoutSpec.Item it) {
        String pattern = it.twentyFour ? (it.seconds ? "HH:mm:ss" : "HH:mm") : (it.seconds ? "h:mm:ss a" : "h:mm a");
        return DateFormat.format(pattern, Calendar.getInstance()).toString();
    }

    /** Hours over minutes on two lines (the tall, thin clock look). */
    private String stackedClock(LayoutSpec.Item it) {
        Calendar c = Calendar.getInstance();
        int h = it.twentyFour ? c.get(Calendar.HOUR_OF_DAY) : (c.get(Calendar.HOUR) == 0 ? 12 : c.get(Calendar.HOUR));
        return String.format(Locale.US, "%02d\n%02d", h, c.get(Calendar.MINUTE));
    }

    private static String dateText(LayoutSpec.Item it) {
        String pattern;
        switch (it.variant == null ? "weekday_date" : it.variant) {
            case "weekday": pattern = "EEEE"; break;
            case "full": pattern = "d MMMM, yyyy"; break;
            case "short": pattern = "EEE"; break;
            case "daymonth": pattern = "d MMM"; break;
            default: pattern = "EEEE, d MMMM"; break;
        }
        return DateFormat.format(pattern, Calendar.getInstance()).toString();
    }

    private static String greeting() {
        int h = Calendar.getInstance().get(Calendar.HOUR_OF_DAY);
        return h < 12 ? "Good morning" : h < 18 ? "Good afternoon" : "Good evening";
    }

    // ---------------------------------------------------------------- app tiles

    private static final class App {
        String pkg, label;
        Drawable icon;
        int category = -1;
        boolean game;
        LayoutSpec.AppLook look;
    }

    /** The leanback apps, A to Z; with a non-empty {@code only} list, just those, in that order. */
    private List<App> apps(Context ctx, List<String> only) {
        List<App> all = apps(ctx);
        if (only == null || only.isEmpty()) return all;
        List<App> out = new ArrayList<>();
        for (String pkg : only) for (App a : all) if (a.pkg.equals(pkg)) { out.add(a); break; }
        return out;
    }

    private List<App> apps(Context ctx) {
        PackageManager pm = ctx.getPackageManager();
        List<ResolveInfo> found = pm.queryIntentActivities(new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER), 0);
        List<App> out = new ArrayList<>();
        java.util.Set<String> seen = new java.util.HashSet<>();
        for (ResolveInfo ri : found) {
            String pkg = ri.activityInfo.packageName;
            if (pkg.equals(ctx.getPackageName()) || spec.hidden.contains(pkg) || !seen.add(pkg)) continue;
            App a = new App();
            a.pkg = pkg;
            a.look = spec.looks.get(pkg);
            a.label = a.look != null && a.look.name != null ? a.look.name : String.valueOf(ri.loadLabel(pm));
            android.content.pm.ApplicationInfo ai = ri.activityInfo.applicationInfo;
            if (ai != null) {
                a.category = ai.category;
                a.game = ai.category == android.content.pm.ApplicationInfo.CATEGORY_GAME || (ai.flags & android.content.pm.ApplicationInfo.FLAG_IS_GAME) != 0;
            }
            out.add(a);
        }
        Collections.sort(out, new Comparator<App>() {
            @Override public int compare(App x, App y) { return x.label.compareToIgnoreCase(y.label); }
        });
        return out;
    }

    private List<App> recent(Context ctx, LayoutSpec.Item it) {
        int max = it.max;
        List<App> all = apps(ctx, it.packages);
        List<App> used = new ArrayList<>();
        for (App a : all) if (LastOpened.get(ctx, a.pkg) > 0) used.add(a);
        Collections.sort(used, new Comparator<App>() {
            @Override public int compare(App x, App y) { return Long.compare(LastOpened.get(getContext(), y.pkg), LastOpened.get(getContext(), x.pkg)); }
        });
        return used.size() > max ? new ArrayList<>(used.subList(0, max)) : used;
    }

    private View strip(Context ctx, LayoutSpec.Item it, List<App> list, boolean horizontal) {
        if (list.isEmpty()) {
            TextView t = new TextView(ctx);
            style(t, it);
            t.setText("recent".equals(it.kind) ? "Nothing opened from here yet" : "No apps found");
            t.setFocusable(false);
            return t;
        }
        RecyclerView rv = new RecyclerView(ctx);
        rv.setLayoutManager(horizontal ? new LinearLayoutManager(ctx, LinearLayoutManager.HORIZONTAL, false)
                : new GridLayoutManager(ctx, Math.max(1, it.hasIcon ? it.columns : 6)));
        rv.setClipToPadding(false);
        rv.setPadding((int) (6 * unit), (int) (6 * unit), (int) (6 * unit), (int) (6 * unit));
        rv.setItemAnimator(null);
        rv.setAdapter(new TileAdapter(it, list));
        rv.setAlpha(1f);
        return rv;
    }

    private final class TileAdapter extends RecyclerView.Adapter<TileAdapter.Holder> {
        private final LayoutSpec.Item it;
        private final List<App> list;

        TileAdapter(LayoutSpec.Item it, List<App> list) { this.it = it; this.list = list; }

        final class Holder extends RecyclerView.ViewHolder {
            final ImageView icon;
            final TextView name;
            Holder(LinearLayout cell, ImageView icon, TextView name) { super(cell); this.icon = icon; this.name = name; }
        }

        @Override public Holder onCreateViewHolder(ViewGroup parent, int type) {
            Context c = parent.getContext();
            int size = (int) ((it.hasIcon ? it.iconSize : 96) * unit);
            LinearLayout cell = new LinearLayout(c);
            cell.setOrientation(LinearLayout.VERTICAL);
            cell.setGravity(Gravity.CENTER_HORIZONTAL);
            cell.setFocusable(true);
            cell.setClickable(true);
            int pad = (int) (6 * unit);
            cell.setPadding(pad, pad, pad, pad);
            ImageView iv = new ImageView(c);
            iv.setScaleType(ImageView.ScaleType.CENTER_CROP);
            boolean circle = "circle".equals(it.shape);
            final float radius = "square".equals(it.shape) ? 0 : circle ? size / 2f : (float) it.radius * unit;
            iv.setClipToOutline(true);
            iv.setOutlineProvider(new ViewOutlineProvider() {
                @Override public void getOutline(View v, Outline o) { o.setRoundRect(0, 0, v.getWidth(), v.getHeight(), radius); }
            });
            cell.addView(iv, new LinearLayout.LayoutParams(size, circle ? size : (int) (size * 9f / 16f)));
            TextView tv = new TextView(c);
            tv.setTypeface(Typeface.create(it.font, Typeface.NORMAL));
            tv.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, (float) it.size * unit);
            tv.setTextColor(it.color);
            tv.setSingleLine(true);
            tv.setEllipsize(android.text.TextUtils.TruncateAt.END);
            tv.setGravity(Gravity.CENTER_HORIZONTAL);
            tv.setMaxWidth(size);
            tv.setVisibility(it.hasIcon && !it.labels ? View.GONE : View.VISIBLE);
            cell.addView(tv, new LinearLayout.LayoutParams(size, ViewGroup.LayoutParams.WRAP_CONTENT));
            cell.setOnFocusChangeListener((v, focused) -> {
                float s = focused ? 1.12f : 1f;
                v.animate().scaleX(s).scaleY(s).setDuration(reduceMotion ? 0 : 120).start();
                v.setBackground(focused ? rounded(0x33FFFFFF, 12) : null);
            });
            return new Holder(cell, iv, tv);
        }

        @Override public void onBindViewHolder(Holder h, int pos) {
            final App a = list.get(pos);
            PackageManager pm = h.itemView.getContext().getPackageManager();
            if (a.icon == null) {
                try {
                    Drawable d = pm.getApplicationBanner(a.pkg);
                    a.icon = d != null ? d : pm.getApplicationIcon(a.pkg);
                } catch (PackageManager.NameNotFoundException e) {
                    a.icon = null;
                }
            }
            h.icon.setImageDrawable(a.icon);
            h.name.setText(a.label);
            h.itemView.setOnClickListener(v -> open(v.getContext(), a.pkg));
        }

        @Override public int getItemCount() { return list.size(); }
    }

    private void open(Context ctx, String pkg) { open(ctx, pkg, null); }

    /** Start an app with the layout's launch animation, from the tile that was pressed when there is one. */
    private void open(Context ctx, String pkg, View from) {
        if (spec.locked.contains(pkg)) {
            PinDialog.ask(ctx, "Enter the PIN to open this app", spec.hasAccent ? spec.accent : 0xFF5B8DEF, () -> launch(ctx, pkg, from));
            return;
        }
        launch(ctx, pkg, from);
    }

    private void launch(Context ctx, String pkg, View from) {
        Intent i = ctx.getPackageManager().getLeanbackLaunchIntentForPackage(pkg);
        if (i == null) i = ctx.getPackageManager().getLaunchIntentForPackage(pkg);
        if (i == null) return;
        LastOpened.record(ctx, pkg);
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        android.os.Bundle opts = launchOptions(ctx, from);
        try { ctx.startActivity(i, opts); } catch (RuntimeException e) { /* the app vanished or refuses to start: stay on Home */ }
    }

    private android.os.Bundle launchOptions(Context ctx, View from) {
        if (reduceMotion) return android.app.ActivityOptions.makeCustomAnimation(ctx, 0, 0).toBundle();
        switch (spec.launch) {
            case "zoom": return from == null ? null : android.app.ActivityOptions.makeScaleUpAnimation(from, 0, 0, from.getWidth(), from.getHeight()).toBundle();
            case "reveal": return from == null ? null : android.app.ActivityOptions.makeClipRevealAnimation(from, 0, 0, from.getWidth(), from.getHeight()).toBundle();
            case "fade": return android.app.ActivityOptions.makeCustomAnimation(ctx, R.anim.launch_fade_in, R.anim.launch_fade_out).toBundle();
            case "slide": return android.app.ActivityOptions.makeCustomAnimation(ctx, R.anim.launch_slide_in, R.anim.launch_hold).toBundle();
            case "grow": return android.app.ActivityOptions.makeCustomAnimation(ctx, R.anim.launch_zoom_in, R.anim.launch_zoom_out).toBundle();
            case "none": return android.app.ActivityOptions.makeCustomAnimation(ctx, 0, 0).toBundle();
            default: return null;   // the TV's own app-open animation
        }
    }

    /** Corner radius for a tile shape (the leaf shape is drawn as its own outline). */
    static float cornerRadius(String shape, int w, int h) {
        float m = Math.min(w, h);
        switch (shape) {
            case "circle": return m / 2f;
            case "squircle": return m * 0.28f;
            case "square": return 0f;
            default: return Math.max(8f, Math.max(w, h) * 0.06f);
        }
    }

    /** The layout's focus style: pop, lift, glow, border, bounce, tilt or zoom. */
    private void focusLook(View cell, View card, TextView name, boolean focused, Drawable ring, boolean nameAlways) {
        long d = reduceMotion ? 0 : 240;
        float scale;
        android.view.animation.Interpolator in;
        switch (spec.focus) {
            case "lift": scale = 1.06f; in = new android.view.animation.DecelerateInterpolator(); break;
            case "glow": scale = 1.05f; in = new android.view.animation.DecelerateInterpolator(); break;
            case "border": scale = 1f; in = new android.view.animation.DecelerateInterpolator(); break;
            case "bounce": scale = 1.12f; in = new android.view.animation.OvershootInterpolator(3f); d = reduceMotion ? 0 : 320; break;
            case "tilt": scale = 1.08f; in = new android.view.animation.DecelerateInterpolator(); break;
            case "zoom": scale = 1.18f; in = new android.view.animation.DecelerateInterpolator(1.6f); break;
            case "parallax": scale = 1.13f; in = new android.view.animation.DecelerateInterpolator(1.8f); d = reduceMotion ? 0 : 280; break;
            default: scale = 1.1f; in = new android.view.animation.OvershootInterpolator(1.6f); break;
        }
        float s = focused ? scale : 1f;
        cell.animate().scaleX(s).scaleY(s).setInterpolator(focused ? in : new android.view.animation.DecelerateInterpolator()).setDuration(d).start();
        if ("parallax".equals(spec.focus)) { parallax(card, focused, d); }
        else {
            float z = focused ? ("lift".equals(spec.focus) ? 28 : 16) * unit : 0;
            card.animate().translationZ(z).rotationX(focused && "tilt".equals(spec.focus) ? 10f : 0f).setDuration(d).start();
        }
        if (card instanceof FrameLayout) ((FrameLayout) card).setForeground(focused ? ring : null);
        if (!nameAlways) name.animate().alpha(focused ? 1f : 0f).setDuration(reduceMotion ? 0 : 160).start();
    }

    // ---------------------------------------------------------------- tvOS look

    /** The last D-pad direction: the icon taking focus leans the way the move came from, then settles. */
    private int lastDx, lastDy;
    private TopShelf topShelf;

    /** Parallax focus: lift with a deep shadow, lean towards the move, settle, and sweep a shine across. */
    private void parallax(View card, boolean focused, long d) {
        card.animate().cancel();
        View glare = card.getTag() instanceof View ? (View) card.getTag() : null;
        if (!focused) {
            card.animate().translationZ(0).rotationX(0).rotationY(0).setDuration(d).start();
            if (glare != null) glare.animate().alpha(0f).setDuration(d / 2).start();
            return;
        }
        float z = 30 * unit;
        if (d == 0) { card.setTranslationZ(z); return; }
        card.setRotationY(lastDx * 9f);
        card.setRotationX(-lastDy * 9f);
        card.animate().translationZ(z).rotationX(0).rotationY(0).setDuration(560)
                .setInterpolator(new android.view.animation.OvershootInterpolator(2.2f)).start();
        if (glare != null) {
            float w = Math.max(1, card.getWidth());
            glare.setTranslationX(lastDx >= 0 ? -w : w);
            glare.setAlpha(1f);
            glare.animate().translationX(lastDx >= 0 ? w : -w).setDuration(700)
                    .setInterpolator(new android.view.animation.DecelerateInterpolator())
                    .withEndAction(() -> glare.setAlpha(0f)).start();
        }
    }

    /** The dock: up to ten apps, five on screen, on a strip of frosted glass. Its focused app fills the Top Shelf.
     *  Apps: the item's own list, else the first Favourites row, else the most recently used, then the rest. */
    private View dock(Context ctx, LayoutSpec.Item it) {
        List<App> list = new ArrayList<>();
        List<App> all = apps(ctx);
        if (!it.packages.isEmpty()) list.addAll(pick(all, it.packages));
        if (list.isEmpty()) for (LayoutSpec.Row r : spec.rows) {
            if ("favorites".equals(r.source) && !r.packages.isEmpty()) { list.addAll(pick(all, r.packages)); break; }
        }
        if (list.isEmpty()) list.addAll(recent(ctx, it));
        for (App a : all) {
            if (list.size() >= 10) break;
            boolean dup = false;
            for (App b : list) if (b.pkg.equals(a.pkg)) { dup = true; break; }
            if (!dup) list.add(a);
        }
        if (list.size() > 10) list = new ArrayList<>(list.subList(0, 10));
        int sw = getResources().getDisplayMetrics().widthPixels;
        int boxW = (int) (sw * it.w / 100);
        int pad = (int) (boxW * 0.025f), gap = (int) (boxW * 0.022f);
        int tileW = (boxW - 2 * pad - 4 * gap) / 5;
        FrameLayout glass = new FrameLayout(ctx);
        glass.setClipChildren(false);
        glass.setClipToPadding(false);
        GradientDrawable g = new GradientDrawable();
        g.setCornerRadius(34 * unit);
        g.setColor(it.hasFill ? it.fill : 0x1AFFFFFF);
        g.setStroke(Math.max(1, (int) unit), 0x2EFFFFFF);
        glass.setBackground(g);
        glass.setClipToOutline(true);   // apps past the fifth scroll in under the glass edge
        Row row = new Row(ctx);
        LinearLayout inner = new LinearLayout(ctx);
        inner.setOrientation(LinearLayout.HORIZONTAL);
        inner.setClipChildren(false);
        inner.setClipToPadding(false);
        inner.setPadding(pad, (int) (tileW * 0.09f), pad, 0);
        for (int i = 0; i < list.size(); i++) {
            App a = list.get(i);
            View t = tile(ctx, a, tileW, it);
            OnFocusChangeListener look = t.getOnFocusChangeListener();
            t.setOnFocusChangeListener((v, f) -> {
                if (look != null) look.onFocusChange(v, f);
                if (f && topShelf != null) topShelf.show(a);
            });
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            if (i < list.size() - 1) lp.rightMargin = gap;
            inner.addView(t, lp);
        }
        row.addView(inner);
        glass.addView(row, new FrameLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
        if (topShelf != null && !list.isEmpty()) topShelf.show(list.get(0));
        return glass;
    }

    /** The Top Shelf: the focused dock app across the top of the screen. Studio's own picture for that app when it
     *  has one; otherwise the app's banner, blurred into a wash of its colours (a flat colour on low-RAM TVs and
     *  before Android 12), with its name. Two layers cross-fade so changing app never flashes. */
    private final class TopShelf extends FrameLayout {
        private final ImageView[] layers = new ImageView[2];
        private int front;
        private final TextView title;
        private String showing = "";

        TopShelf(Context c) {
            super(c);
            for (int i = 0; i < 2; i++) {
                ImageView iv = new ImageView(c);
                iv.setScaleType(ImageView.ScaleType.CENTER_CROP);
                iv.setAlpha(0f);
                layers[i] = iv;
                addView(iv, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
            }
            // Fades the picture into the background so the dock and grid read over it.
            View scrim = new View(c);
            scrim.setBackground(new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM,
                    new int[]{0x55000000, 0x33000000, 0xCC000000 | (spec.color & 0xFFFFFF), 0xFF000000 | spec.color}));
            addView(scrim, new LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT));
            title = new TextView(c);
            title.setTextColor(Color.WHITE);
            title.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, 64 * unit);
            title.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
            title.setShadowLayer(18 * unit, 0, 2 * unit, 0x99000000);
            title.setSingleLine(true);
            LayoutParams tl = new LayoutParams(ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT, Gravity.TOP | Gravity.START);
            tl.leftMargin = (int) (getResources().getDisplayMetrics().widthPixels * 0.045f);
            tl.topMargin = (int) (getResources().getDisplayMetrics().heightPixels * 0.42f);
            addView(title, tl);
        }

        void show(App a) {
            if (a.pkg.equals(showing)) return;
            showing = a.pkg;
            final String pkg = a.pkg;
            final File own = a.look != null && a.look.image != null && dir != null ? new File(new File(dir, "tiles"), a.look.image) : null;
            final PackageManager pm = getContext().getPackageManager();
            loader.execute(() -> {
                Drawable pic = null;
                boolean crisp = false;
                if (own != null && own.isFile()) {
                    Bitmap b = decodeStill(own);
                    if (b != null) { pic = new android.graphics.drawable.BitmapDrawable(getResources(), b); crisp = true; }
                }
                if (pic == null) {
                    Drawable banner = null;
                    try { banner = pm.getApplicationBanner(pkg); } catch (PackageManager.NameNotFoundException ignored) { }
                    if (banner == null) { try { banner = pm.getApplicationIcon(pkg); } catch (PackageManager.NameNotFoundException ignored) { } }
                    pic = banner;
                }
                final Drawable d = pic;
                final boolean sharp = crisp;
                final int wash = d == null ? spec.color : averageColor(d);
                post(() -> {
                    if (!pkg.equals(showing)) return;   // the person already moved on
                    ImageView next = layers[1 - front], prev = layers[front];
                    boolean blur = !sharp && heavy && Build.VERSION.SDK_INT >= 31;
                    if (sharp || blur) {
                        next.setImageDrawable(d);
                        next.setBackgroundColor(0);
                        // Banners on white would turn the whole shelf milky: the wash is kept dark.
                        if (blur) next.setColorFilter(0xFF7A7A7A, android.graphics.PorterDuff.Mode.MULTIPLY); else next.clearColorFilter();
                        if (Build.VERSION.SDK_INT >= 31) next.setRenderEffect(blur ? android.graphics.RenderEffect.createBlurEffect(60 * unit, 60 * unit, android.graphics.Shader.TileMode.CLAMP) : null);
                    } else {
                        next.setImageDrawable(null);
                        next.setBackground(new GradientDrawable(GradientDrawable.Orientation.TL_BR, new int[]{wash | 0xFF000000, 0xFF000000 | spec.color}));
                    }
                    title.setText(sharp ? "" : a.label);
                    long t = reduceMotion ? 0 : 450;
                    next.animate().alpha(sharp ? 1f : 0.75f).setDuration(t).start();
                    prev.animate().alpha(0f).setDuration(t).start();
                    front = 1 - front;
                });
            });
        }
    }

    /** One colour for a picture: what it averages to, a little brighter so the wash is not muddy. */
    static int averageColor(Drawable d) {
        try {
            Bitmap b = Bitmap.createBitmap(8, 8, Bitmap.Config.ARGB_8888);
            android.graphics.Canvas c = new android.graphics.Canvas(b);
            d.setBounds(0, 0, 8, 8);
            d.draw(c);
            long r = 0, g = 0, bl = 0;
            for (int x = 0; x < 8; x++) for (int y = 0; y < 8; y++) { int p = b.getPixel(x, y); r += Color.red(p); g += Color.green(p); bl += Color.blue(p); }
            b.recycle();
            return Color.rgb((int) Math.min(255, r / 64 * 1.1), (int) Math.min(255, g / 64 * 1.1), (int) Math.min(255, bl / 64 * 1.1));
        } catch (RuntimeException e) {
            return 0x202020;
        }
    }

    // ---------------------------------------------------------------- watch next

    private View watchNext(Context ctx, LayoutSpec.Item it) {
        LinearLayout col = new LinearLayout(ctx);
        col.setOrientation(LinearLayout.VERTICAL);
        TextView head = new TextView(ctx);
        style(head, it);
        head.setAlpha(0.7f);
        head.setText("Continue watching");
        col.addView(head);
        WatchNext w = WatchNext.load(ctx, it.max);
        if (w.denied || w.entries.isEmpty()) {
            TextView note = new TextView(ctx);
            style(note, it);
            note.setTextSize(android.util.TypedValue.COMPLEX_UNIT_PX, (float) it.size * unit * 0.8f);
            note.setText(w.denied ? "This TV does not let the launcher read Watch Next." : "Nothing to continue right now.");
            col.addView(note);
            return col;
        }
        LinearLayout row = new LinearLayout(ctx);
        row.setOrientation(LinearLayout.HORIZONTAL);
        for (WatchNext.Entry e : w.entries) {
            TextView card = new TextView(ctx);
            style(card, it);
            card.setBackground(rounded(it.hasFill ? it.fill : 0x33FFFFFF, it.radius));
            card.setPadding((int) (16 * unit), (int) (12 * unit), (int) (16 * unit), (int) (12 * unit));
            card.setSingleLine(true);
            card.setEllipsize(android.text.TextUtils.TruncateAt.END);
            card.setText(e.title);
            card.setFocusable(true);
            card.setClickable(true);
            card.setOnFocusChangeListener((v, focused) -> v.setAlpha(focused ? 1f : (float) it.opacity));
            card.setOnClickListener(v -> resume(v.getContext(), e));
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams((int) (320 * unit), ViewGroup.LayoutParams.WRAP_CONTENT);
            lp.rightMargin = (int) (12 * unit);
            row.addView(card, lp);
        }
        col.addView(row);
        return col;
    }

    private void resume(Context ctx, WatchNext.Entry e) {
        try {
            Intent i = e.intentUri == null ? null : Intent.parseUri(e.intentUri, Intent.URI_INTENT_SCHEME);
            if (i == null && e.pkg != null) i = ctx.getPackageManager().getLeanbackLaunchIntentForPackage(e.pkg);
            if (i == null) return;
            // Only open what the other app asked for: never let a Watch Next entry reach a component we did not resolve.
            i.setComponent(null);
            i.setSelector(null);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
        } catch (java.net.URISyntaxException | RuntimeException ex) {
            /* stale entry: nothing to do */
        }
    }

    // ---------------------------------------------------------------- motion

    private void motion(View v, LayoutSpec.Item it) {
        if (!heavy || "none".equals(it.motion) || it.amount <= 0) return;
        long dur = (long) (4000 / it.speed);
        float amt = (float) (it.amount / 4.0) * unit;
        ObjectAnimator a;
        switch (it.motion) {
            case "pulse": {
                a = ObjectAnimator.ofFloat(v, View.SCALE_X, 1f, 1f + (float) it.amount / 400f);
                ObjectAnimator b = ObjectAnimator.ofFloat(v, View.SCALE_Y, 1f, 1f + (float) it.amount / 400f);
                repeat(b, dur);
                animators.add(b);
                break;
            }
            case "slide": a = ObjectAnimator.ofFloat(v, View.TRANSLATION_X, 0f, amt); break;
            case "fade": a = ObjectAnimator.ofFloat(v, View.ALPHA, 1f, 0.35f); break;
            case "drift": a = ObjectAnimator.ofFloat(v, View.TRANSLATION_X, -amt, amt); break;
            default: a = ObjectAnimator.ofFloat(v, View.TRANSLATION_Y, 0f, -amt); break;   // float, parallax
        }
        repeat(a, dur);
        animators.add(a);
    }

    /** Screensaver: every widget wanders a few pixels over a minute or so, so nothing sits still on the panel. */
    private void drift(View v) {
        float d = 14 * unit;
        ObjectAnimator a = ObjectAnimator.ofPropertyValuesHolder(v,
                android.animation.PropertyValuesHolder.ofFloat(View.TRANSLATION_X, -d, d),
                android.animation.PropertyValuesHolder.ofFloat(View.TRANSLATION_Y, d * 0.6f, -d * 0.6f));
        a.setDuration(47_000 + (long) (Math.abs(v.hashCode()) % 20_000));
        a.setRepeatCount(ValueAnimator.INFINITE);
        a.setRepeatMode(ValueAnimator.REVERSE);
        a.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
        animators.add(a);
    }

    private static void repeat(ObjectAnimator a, long dur) {
        a.setDuration(dur);
        a.setRepeatCount(ValueAnimator.INFINITE);
        a.setRepeatMode(ValueAnimator.REVERSE);
        a.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
    }

    // ---------------------------------------------------------------- lifecycle

    private final Runnable tick = new Runnable() {
        @Override public void run() {
            if (!started) return;
            for (Runnable r : ticks) r.run();
            handler.postDelayed(this, 1000);
        }
    };

    /** Home is on screen: start the clock and numbers, the motion and the video. */
    void start() {
        if (started) return;
        started = true;
        for (Animator a : animators) a.start();
        if (heavy) for (Widgets.Animated a : animated) a.startAnim();
        if (ambient != null) ambient.start();
        if (video != null) video.resume();
        if (movingPicture instanceof android.graphics.drawable.Animatable) ((android.graphics.drawable.Animatable) movingPicture).start();
        handler.post(tick);
        handler.post(weatherTick);
        requestFirstFocus();
    }

    /** Another app is in front: stop everything that draws or polls. */
    void stop() {
        started = false;
        handler.removeCallbacks(tick);
        handler.removeCallbacks(weatherTick);
        for (Animator a : animators) a.cancel();
        for (Widgets.Animated a : animated) a.stopAnim();
        if (ambient != null) ambient.stop();
        if (video != null) video.pause();
        if (movingPicture instanceof android.graphics.drawable.Animatable) ((android.graphics.drawable.Animatable) movingPicture).stop();
    }

    /** Home opens on the first app, not on the profile button Android would pick for being top-most. */
    private void requestFirstFocus() {
        post(() -> {
            View now = findFocus();
            if (now != null && now != chipView && now != gearView) return;
            ArrayList<View> out = new ArrayList<>();
            addFocusables(out, View.FOCUS_DOWN);
            for (View v : out) if (v != chipView && v != gearView) { v.requestFocus(); return; }
        });
    }

    @Override
    protected void onDetachedFromWindow() {
        super.onDetachedFromWindow();
        stop();
        if (video != null) video.release();
        loader.shutdownNow();
        for (android.app.Dialog d : new ArrayList<>(openFolders)) d.dismiss();
    }

    // ---------------------------------------------------------------- video

    /** A muted, looping MediaPlayer on a TextureView, centre-cropped to fill; any error leaves the still picture. */
    private static final class VideoBackground extends TextureView implements TextureView.SurfaceTextureListener {
        private final File file;
        private MediaPlayer player;
        private boolean ready, wantPlay, failed;
        private int vw, vh;

        VideoBackground(Context ctx, File file) {
            super(ctx);
            this.file = file;
            setSurfaceTextureListener(this);
            setFocusable(false);
            setOpaque(true);
        }

        void resume() { wantPlay = true; if (ready && player != null) { try { player.start(); } catch (IllegalStateException e) { fail(); } } }
        void pause() { wantPlay = false; if (player != null) { try { if (player.isPlaying()) player.pause(); } catch (IllegalStateException e) { fail(); } } }

        void release() {
            if (player != null) { try { player.release(); } catch (RuntimeException e) { /* already gone */ } player = null; }
            ready = false;
        }

        private void fail() { failed = true; release(); setVisibility(View.GONE); }

        @Override public void onSurfaceTextureAvailable(SurfaceTexture st, int w, int h) {
            if (failed) return;
            try {
                player = new MediaPlayer();
                player.setDataSource(file.getPath());
                player.setSurface(new Surface(st));
                player.setLooping(true);
                player.setVolume(0f, 0f);
                player.setOnVideoSizeChangedListener((mp, ww, hh) -> { vw = ww; vh = hh; crop(); });
                player.setOnPreparedListener(mp -> { ready = true; if (wantPlay) mp.start(); });
                player.setOnErrorListener((mp, what, extra) -> { fail(); return true; });
                player.prepareAsync();
            } catch (java.io.IOException | RuntimeException e) {
                fail();
            }
        }

        @Override public void onSurfaceTextureSizeChanged(SurfaceTexture st, int w, int h) { crop(); }
        @Override public boolean onSurfaceTextureDestroyed(SurfaceTexture st) { release(); return true; }
        @Override public void onSurfaceTextureUpdated(SurfaceTexture st) { }

        private void crop() {
            int w = getWidth(), h = getHeight();
            if (vw <= 0 || vh <= 0 || w <= 0 || h <= 0) return;
            float s = Math.max((float) w / vw, (float) h / vh);
            Matrix m = new Matrix();
            // TextureView stretches the video to its own size; undo that, then scale to cover and centre.
            m.setScale(vw * s / w, vh * s / h, w / 2f, h / 2f);
            setTransform(m);
        }
    }
}
