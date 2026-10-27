package com.gathin.tvhome;

import java.util.List;
import java.util.regex.Pattern;

/** The files ThemeProvider may write or delete under Config.dir, by the URI's path segments. */
final class ThemeFiles {
    static final String WALLPAPERS = "wallpapers";
    static final String VIDEO = "video";
    static final String FONTS = "fonts";
    /** Pictures the person gave individual apps (layout.json "apps": {pkg: {image}}). */
    static final String TILES = "tiles";
    /** Studio asks the TV to forget its PIN by writing this file; the launcher removes it with the PIN. */
    static final String PIN_RESET = "pin_reset";
    /** The TV's screensaver before Studio set the canvas one, so Revert can put it back. */
    static final String SCREENSAVER_PREV = "screensaver_prev";
    private static final Pattern FONT_FILE = Pattern.compile("[A-Za-z0-9][A-Za-z0-9_.-]{0,58}\\.(ttf|otf)");
    private static final Pattern VIDEO_FILE = Pattern.compile("[A-Za-z0-9][A-Za-z0-9_.-]{0,58}\\." + LayoutSpec.VIDEO_EXT);
    /** The names tools/theme.sh accepts for a collection image, and the types WallpaperRotation reads. */
    private static final Pattern IMAGE = Pattern.compile("[A-Za-z0-9_.-]+\\.(jpg|jpeg|png|webp|gif)",
            Pattern.CASE_INSENSITIVE);

    private ThemeFiles() {}

    /** The path relative to the theme directory, e.g. "config.json" or "wallpapers/dusk.jpg", or null if refused. */
    static String path(List<String> segments) {
        if (segments == null) return null;
        if (segments.size() == 1) {
            String name = segments.get(0);
            return Config.FILE_NAME.equals(name) || LayoutSpec.FILE_NAME.equals(name) || "wallpaper.jpg".equals(name) || "theme_revision".equals(name)
                    || SCREENSAVER_PREV.equals(name) || PIN_RESET.equals(name)
                    ? name : null;
        }
        if (segments.size() == 2 && WALLPAPERS.equals(segments.get(0))
                && (collectionImage(segments.get(1)) || WallpaperCaptions.FILE_NAME.equals(segments.get(1)))) {
            return WALLPAPERS + "/" + segments.get(1);
        }
        if (segments.size() == 2 && VIDEO.equals(segments.get(0)) && videoFile(segments.get(1))) {
            return VIDEO + "/" + segments.get(1);
        }
        if (segments.size() == 2 && Profiles.DIR.equals(segments.get(0)) && profileFile(segments.get(1))) {
            return Profiles.DIR + "/" + segments.get(1);
        }
        if (segments.size() == 2 && TILES.equals(segments.get(0)) && LayoutSpec.validTileImage(segments.get(1))) {
            return TILES + "/" + segments.get(1);
        }
        if (segments.size() == 2 && FONTS.equals(segments.get(0)) && fontFile(segments.get(1))) {
            return FONTS + "/" + segments.get(1);
        }
        return null;
    }

    static boolean fontFile(String name) {
        return name != null && FONT_FILE.matcher(name).matches();
    }

    /** True for "fonts" alone: a delete of it clears the font folder. */
    static boolean profileFile(String name) {
        return name != null && name.endsWith(".json") && Profiles.validId(name.substring(0, name.length() - 5));
    }

    static boolean isProfileCollection(List<String> segments) {
        return segments != null && segments.size() == 1 && Profiles.DIR.equals(segments.get(0));
    }

    static boolean isTileCollection(List<String> segments) {
        return segments != null && segments.size() == 1 && TILES.equals(segments.get(0));
    }

    static boolean isFontCollection(List<String> segments) {
        return segments != null && segments.size() == 1 && FONTS.equals(segments.get(0));
    }

    static boolean videoFile(String name) {
        return name != null && VIDEO_FILE.matcher(name).matches();
    }

    /** True for "video" alone: a delete of it clears the video folder. */
    static boolean isVideoCollection(List<String> segments) {
        return segments != null && segments.size() == 1 && VIDEO.equals(segments.get(0));
    }

    /** True for "wallpapers" alone: a delete of it clears the collection's images. */
    static boolean isCollection(List<String> segments) {
        return segments != null && segments.size() == 1 && WALLPAPERS.equals(segments.get(0));
    }

    static boolean collectionImage(String name) {
        return name != null && !name.startsWith(".") && IMAGE.matcher(name).matches();
    }
}
