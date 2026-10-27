package com.gathin.tvhome;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/**
 * The free-canvas home layout the Studio app writes as layout.json. Strict on purpose: any problem returns
 * null and Home falls back to the standard screen, so a bad file can never leave the TV without a usable Home.
 * The rules mirror crates/tvs-launcher/src/layout.rs; coordinates are percentages of the screen.
 */
final class LayoutSpec {
    static final String FILE_NAME = "layout.json";
    static final int MAX_ITEMS = 40;

    static final List<String> KINDS = Arrays.asList("apps", "recent", "watchnext", "clock", "date", "greeting", "text", "cpu", "memory", "network", "calendar", "visualizer", "shape", "bluetooth", "nowplaying", "weather");
    static final List<String> FONTS = Arrays.asList("sans-serif", "sans-serif-light", "sans-serif-thin", "sans-serif-condensed",
            "sans-serif-medium", "sans-serif-black", "serif", "monospace", "cursive", "casual");
    static final List<String> MOTIONS = Arrays.asList("none", "float", "pulse", "slide", "fade", "drift", "parallax");
    static final List<String> SHAPES = Arrays.asList("rounded", "square", "circle");
    static final List<String> ALIGN = Arrays.asList("start", "center", "end");
    static final List<String> BACKGROUNDS = Arrays.asList("image", "video", "color", "gradient", "scene");

    /** The looks (variant) each kind accepts; the first is the default. */
    static List<String> variantsFor(String kind) {
        switch (kind) {
            case "clock": return Arrays.asList("digital", "stacked", "analog");
            case "date": return Arrays.asList("weekday_date", "weekday", "full", "short", "daymonth");
            case "cpu": case "memory": case "network": return Arrays.asList("text", "bar", "ring", "graph");
            case "visualizer": return Arrays.asList("bars", "mirror", "dots");
            case "shape": return Arrays.asList("rect", "circle", "line", "ring");
            case "weather": return Arrays.asList("compact", "full", "forecast");
            default: return new ArrayList<>();
        }
    }

    static final class Item {
        String id, kind, text, font, align, shape, motion, variant, fontFile;
        boolean enabled, labels, twentyFour, seconds, compact, hasIcon, upper, shadow, italic, mondayFirst;
        double x, y, w, h, size, opacity, radius, iconSize, speed, amount, spacing, stroke;
        int bars = 48;
        List<String> packages = new ArrayList<>();
        int color, fill, columns, max;
        boolean hasFill;
    }

    boolean enabled;
    /** "classic": the standard Home keeps the apps (dock, sliding grid, glass focus) and the canvas draws the
     *  background and widgets behind it. "canvas": the canvas replaces Home, app strips included. */
    String home = "classic";
    String bgKind, video;
    /** background.scene / sceneTime: which ambient scene, and "auto" or a fixed part of the day. */
    String scene = "rain", sceneTime = "auto";
    /** Keep moving backgrounds and animations on even on a low-RAM TV (the person was warned in Studio). */
    boolean heavyOnLowRam;
    /** Chosen on the TV (TvChoices): still pictures and no motion, whatever the TV's memory. */
    boolean still;
    /** The home mode Studio sent, before any choice made on the TV. */
    String homeBefore = "classic";
    /** Look of the scroll-mode tiles: focus style, launch animation, tile style, icon shape, and the accent colour
     *  for focus rings and names (picked from the wallpaper in Studio, or none for white). */
    String focus = "pop", launch = "default", tiles = "banner", iconShape = "rounded";
    boolean hasAccent;
    int accent = 0xFFFFFFFF;

    // ---- scroll-mode app organisation
    static final List<String> ROW_SOURCES = Arrays.asList("favorites", "recent", "games", "video", "music", "news", "social", "tools");
    static final int MAX_ROWS = 8, MAX_FOLDERS = 8, MAX_HIDDEN = 100, MAX_LOOKS = 60;

    /** A row on the apps page: hand-picked (favorites), recently used, or filled from the apps' Play categories. */
    static final class Row { String title, source; final List<String> packages = new ArrayList<>(); int max = 12; }
    /** A folder tile at the start of All apps; its apps leave the grid and open in a pop-up row. */
    static final class Folder { String name; final List<String> packages = new ArrayList<>(); }
    /** Per-app look: another name, a card colour, or a picture (tiles/<image>) instead of the app's banner. */
    static final class AppLook { String name, image; boolean hasColor; int color; }

    final List<Row> rows = new ArrayList<>();
    final List<Folder> folders = new ArrayList<>();
    final Set<String> hidden = new HashSet<>();
    final java.util.Map<String, AppLook> looks = new java.util.HashMap<>();

    /** Apps that ask for the TV's PIN before they open from this Home. */
    final Set<String> locked = new HashSet<>();
    /** Other people's layouts (profiles/<id>.json); this layout is the main one, called profileName. */
    static final class Profile { String id, name; int color; boolean pin; }
    final List<Profile> profiles = new ArrayList<>();
    String profileName = "Main";
    /** Which profile this layout belongs to: "" for the main one. */
    String currentProfile = "";
    /** The app Home opens once after the TV starts (layout "openOnStart"); null for none. */
    String openOnStart;
    /** Weather (layout "weather"): a city, °C or °F, and the person's agreement that the TV may ask Open-Meteo.
     *  Without that agreement nothing is fetched. followWeather: the scene turns to rain or snow when it rains or snows. */
    String weatherCity;
    boolean fahrenheit, weatherOn, followWeather;
    int profileColor = 0xFF5B8DEF;
    int color, color2;
    double dim;
    final List<Item> items = new ArrayList<>();

    static final List<String> HOMES = Arrays.asList("classic", "canvas", "scroll", "tvos");
    static final List<String> FOCUS = Arrays.asList("pop", "lift", "glow", "border", "bounce", "tilt", "zoom", "parallax");
    static final List<String> LAUNCH = Arrays.asList("default", "zoom", "reveal", "fade", "slide", "grow", "none");
    static final List<String> TILES = Arrays.asList("banner", "card", "icon", "poster", "tvos");
    static final List<String> ICON_SHAPES = Arrays.asList("rounded", "circle", "squircle", "square", "leaf");
    /** Video containers the TV's MediaPlayer opens (H.264/H.265/VP8/VP9 inside). */
    static final String VIDEO_EXT = "(mp4|m4v|webm|mkv|mov|3gp|ts)";

    /** Items that take D-pad focus; in classic mode the standard Home shows the apps instead. */
    static boolean appKind(String kind) {
        return "apps".equals(kind) || "recent".equals(kind) || "watchnext".equals(kind);
    }

    boolean classic() { return "classic".equals(home); }

    /** "scroll": widgets and a sideways "Recently used" row first; Down glides to every app. */
    boolean scroll() { return "scroll".equals(home) || tvos(); }

    /** "tvos": scroll mode laid out like a set-top box: a Top Shelf picture for the focused app, a dock of five, a grid. */
    boolean tvos() { return "tvos".equals(home); }

    private LayoutSpec() {}

    /** Parses and checks a layout; null when anything is wrong. */
    static LayoutSpec parse(String json) {
        if (json == null) return null;
        try {
            JSONObject o = new JSONObject(json);
            if (o.optInt("version", 0) != 1 || !(o.opt("enabled") instanceof Boolean)) return null;
            LayoutSpec s = new LayoutSpec();
            s.enabled = o.getBoolean("enabled");
            s.home = oneOf(o.optString("home", "classic"), HOMES);
            s.focus = oneOf(o.optString("focus", "pop"), FOCUS);
            s.launch = oneOf(o.optString("launch", "default"), LAUNCH);
            s.tiles = oneOf(o.optString("tiles", "banner"), TILES);
            s.iconShape = oneOf(o.optString("iconShape", "rounded"), ICON_SHAPES);
            if (o.has("accent") && !o.isNull("accent")) { s.accent = hex(o.getString("accent")); s.hasAccent = true; }
            parseApps(o, s);
            JSONObject b = o.getJSONObject("background");
            s.bgKind = oneOf(b.optString("kind", ""), BACKGROUNDS);
            s.color = hex(b.optString("color", ""));
            s.color2 = hex(b.optString("color2", ""));
            s.dim = range(b, "dim", 0, 0.9);
            range(b, "blur", 0, 24);
            s.video = b.isNull("video") ? null : b.getString("video");
            if (b.has("scene")) s.scene = oneOf(b.getString("scene"), Arrays.asList(AmbientScene.SCENES));
            if (b.has("sceneTime")) s.sceneTime = oneOf(b.getString("sceneTime"), Arrays.asList(AmbientScene.TIMES));
            s.followWeather = b.optBoolean("followWeather", false);
            if (o.has("heavyOnLowRam")) {
                if (!(o.opt("heavyOnLowRam") instanceof Boolean)) return null;
                s.heavyOnLowRam = o.getBoolean("heavyOnLowRam");
            }
            if (s.video != null && !validVideo(s.video)) return null;
            JSONArray arr = o.getJSONArray("items");
            if (arr.length() > MAX_ITEMS) return null;
            Set<String> ids = new HashSet<>();
            for (int i = 0; i < arr.length(); i++) {
                Item it = item(arr.getJSONObject(i));
                if (!ids.add(it.id)) return null;
                s.items.add(it);
            }
            return s;
        } catch (JSONException | IllegalArgumentException e) {
            return null;
        }
    }

    private static Item item(JSONObject o) throws JSONException {
        Item it = new Item();
        it.id = o.getString("id");
        if (!it.id.matches("[a-z0-9-]{1,32}")) throw new IllegalArgumentException("id");
        it.kind = oneOf(o.getString("kind"), KINDS);
        if (!(o.opt("enabled") instanceof Boolean)) throw new IllegalArgumentException("enabled");
        it.enabled = o.getBoolean("enabled");
        it.x = range(o, "x", 0, 100);
        it.y = range(o, "y", 0, 100);
        it.w = range(o, "w", 3, 100);
        it.h = range(o, "h", 3, 100);
        if (it.x + it.w > 100.001 || it.y + it.h > 100.001) throw new IllegalArgumentException("off screen");
        if (o.has("text")) {
            it.text = o.getString("text");
            if (it.text.length() > 80) throw new IllegalArgumentException("text");
            for (int i = 0; i < it.text.length(); i++) if (Character.isISOControl(it.text.charAt(i))) throw new IllegalArgumentException("text");
        }
        JSONObject st = o.getJSONObject("style");
        it.font = st.getString("font");
        if (it.font.startsWith("file:")) {
            String name = it.font.substring(5);
            if (!validFont(name)) throw new IllegalArgumentException("font");
            it.fontFile = name;
        } else {
            oneOf(it.font, FONTS);
        }
        it.spacing = st.has("spacing") ? range(st, "spacing", 0, 1) : 0;
        it.upper = flag(st, "upper");
        it.shadow = flag(st, "shadow");
        it.italic = flag(st, "italic");
        if (o.has("variant")) {
            it.variant = o.getString("variant");
            if (!variantsFor(it.kind).contains(it.variant)) throw new IllegalArgumentException("variant");
        } else {
            List<String> v = variantsFor(it.kind);
            it.variant = v.isEmpty() ? null : v.get(0);
        }
        it.size = range(st, "size", 8, 400);
        it.color = hex(st.getString("color"));
        it.opacity = range(st, "opacity", 0.05, 1);
        it.radius = range(st, "radius", 0, 80);
        it.align = oneOf(st.getString("align"), ALIGN);
        if (!st.isNull("fill") && st.has("fill")) { it.fill = hex(st.getString("fill")); it.hasFill = true; }
        JSONObject ic = o.optJSONObject("icon");
        if (ic != null) {
            it.hasIcon = true;
            it.iconSize = range(ic, "size", 24, 256);
            it.shape = oneOf(ic.getString("shape"), SHAPES);
            it.columns = (int) range(ic, "columns", 1, 12);
            it.labels = ic.getBoolean("labels");
        }
        JSONObject m = o.getJSONObject("motion");
        it.motion = oneOf(m.getString("type"), MOTIONS);
        it.speed = range(m, "speed", 0.25, 4);
        it.amount = range(m, "amount", 0, 100);
        it.max = 6;
        JSONObject op = o.optJSONObject("options");
        if (op != null) {
            if (op.has("max")) it.max = (int) range(op, "max", 1, 20);
            it.twentyFour = op.optBoolean("twentyFour", false);
            it.seconds = op.optBoolean("seconds", false);
            it.compact = op.optBoolean("compact", false);
            it.mondayFirst = op.optBoolean("weekStartsMonday", false);
            if (op.has("bars")) it.bars = (int) range(op, "bars", 8, 96);
            if (op.has("stroke")) it.stroke = range(op, "stroke", 0, 40);
            JSONArray pk = op.optJSONArray("packages");
            if (pk != null) {
                if (pk.length() > 24) throw new IllegalArgumentException("packages");
                for (int i = 0; i < pk.length(); i++) {
                    String name = pk.getString(i);
                    if (!name.matches("[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z0-9_]+)+") || name.length() > 200) throw new IllegalArgumentException("package");
                    it.packages.add(name);
                }
            }
        }
        return it;
    }

    static boolean validPackage(String name) {
        return name != null && name.length() <= 200 && name.matches("[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z0-9_]+)+");
    }

    /** A picture the person gave an app, under tiles/: a plain file name, never a path. */
    static boolean validTileImage(String name) {
        return name != null && name.matches("[A-Za-z0-9_-]{1,60}\\.(jpg|png|webp)");
    }

    private static List<String> packages(JSONArray a, int max) throws JSONException {
        List<String> out = new ArrayList<>();
        if (a == null) return out;
        if (a.length() > max) throw new IllegalArgumentException("packages");
        for (int i = 0; i < a.length(); i++) {
            String n = a.getString(i);
            if (!validPackage(n)) throw new IllegalArgumentException("package");
            out.add(n);
        }
        return out;
    }

    private static String text(JSONObject o, String key, int max) throws JSONException {
        String v = o.getString(key);
        if (v.length() > max || v.trim().isEmpty()) throw new IllegalArgumentException(key);
        return v;
    }

    private static void parseApps(JSONObject o, LayoutSpec s) throws JSONException {
        JSONArray rows = o.optJSONArray("rows");
        if (rows != null) {
            if (rows.length() > MAX_ROWS) throw new IllegalArgumentException("rows");
            for (int i = 0; i < rows.length(); i++) {
                JSONObject r = rows.getJSONObject(i);
                Row row = new Row();
                row.title = text(r, "title", 40);
                row.source = oneOf(r.getString("source"), ROW_SOURCES);
                row.packages.addAll(packages(r.optJSONArray("packages"), 24));
                if (r.has("max")) row.max = (int) range(r, "max", 1, 24);
                s.rows.add(row);
            }
        }
        JSONArray folders = o.optJSONArray("folders");
        if (folders != null) {
            if (folders.length() > MAX_FOLDERS) throw new IllegalArgumentException("folders");
            for (int i = 0; i < folders.length(); i++) {
                JSONObject f = folders.getJSONObject(i);
                Folder folder = new Folder();
                folder.name = text(f, "name", 30);
                folder.packages.addAll(packages(f.optJSONArray("packages"), 24));
                s.folders.add(folder);
            }
        }
        s.hidden.addAll(packages(o.optJSONArray("hidden"), MAX_HIDDEN));
        s.locked.addAll(packages(o.optJSONArray("locked"), 50));
        JSONObject w = o.optJSONObject("weather");
        if (w != null) {
            s.weatherCity = text(w, "city", 60);
            s.fahrenheit = "f".equals(oneOf(w.optString("units", "c"), Arrays.asList("c", "f")));
            s.weatherOn = w.optBoolean("consent", false);
        }
        if (o.has("openOnStart") && !o.isNull("openOnStart")) {
            s.openOnStart = o.getString("openOnStart");
            if (!validPackage(s.openOnStart)) throw new IllegalArgumentException("openOnStart");
        }
        if (o.has("profileName")) s.profileName = text(o, "profileName", 20);
        if (o.has("profileColor")) s.profileColor = hex(o.getString("profileColor"));
        JSONArray profiles = o.optJSONArray("profiles");
        if (profiles != null) {
            if (profiles.length() > 3) throw new IllegalArgumentException("profiles");
            for (int i = 0; i < profiles.length(); i++) {
                JSONObject p = profiles.getJSONObject(i);
                Profile pr = new Profile();
                pr.id = p.getString("id");
                if (!Profiles.validId(pr.id)) throw new IllegalArgumentException("profile id");
                pr.name = text(p, "name", 20);
                pr.color = p.has("color") ? hex(p.getString("color")) : 0xFF888888;
                pr.pin = p.optBoolean("pin", false);
                s.profiles.add(pr);
            }
        }
        JSONObject looks = o.optJSONObject("apps");
        if (looks != null) {
            if (looks.length() > MAX_LOOKS) throw new IllegalArgumentException("apps");
            java.util.Iterator<String> keys = looks.keys();
            while (keys.hasNext()) {
                String pkg = keys.next();
                if (!validPackage(pkg)) throw new IllegalArgumentException("package");
                JSONObject l = looks.getJSONObject(pkg);
                AppLook look = new AppLook();
                if (l.has("name") && !l.isNull("name")) look.name = text(l, "name", 40);
                if (l.has("color") && !l.isNull("color")) { look.color = hex(l.getString("color")); look.hasColor = true; }
                if (l.has("image") && !l.isNull("image")) {
                    look.image = l.getString("image");
                    if (!validTileImage(look.image)) throw new IllegalArgumentException("image");
                }
                s.looks.put(pkg, look);
            }
        }
    }

    private static boolean flag(JSONObject o, String key) throws JSONException {
        if (!o.has(key)) return false;
        if (!(o.get(key) instanceof Boolean)) throw new IllegalArgumentException(key);
        return o.getBoolean(key);
    }

    /** A plain ttf/otf file name, never a path. */
    static boolean validFont(String s) {
        return s != null && s.matches("[A-Za-z0-9][A-Za-z0-9_.-]{0,58}\\.(ttf|otf)");
    }

    private static String oneOf(String v, List<String> allowed) {
        if (!allowed.contains(v)) throw new IllegalArgumentException(v);
        return v;
    }

    private static double range(JSONObject o, String key, double lo, double hi) throws JSONException {
        double v = o.getDouble(key);
        if (Double.isNaN(v) || v < lo || v > hi) throw new IllegalArgumentException(key);
        return v;
    }

    /** #rrggbb or #rrggbbaa (the layout's order) as an Android ARGB int. */
    static int hex(String s) {
        if (s == null || !s.matches("#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?")) throw new IllegalArgumentException("colour");
        int rgb = (int) Long.parseLong(s.substring(1, 7), 16);
        int a = s.length() == 9 ? Integer.parseInt(s.substring(7, 9), 16) : 255;
        return (a << 24) | rgb;
    }

    /** A bundled sample id ("rain") or a plain video file name; never a path. */
    static boolean validVideo(String s) {
        return s != null && (s.matches("[a-z0-9-]{1,32}") || s.matches("[A-Za-z0-9][A-Za-z0-9_.-]{0,58}\\." + VIDEO_EXT));
    }

    /** The file name under video/ for the layout's video setting. */
    static String videoFile(String v) {
        return v.matches(".*\\." + VIDEO_EXT) ? v : v + ".mp4";
    }
}
