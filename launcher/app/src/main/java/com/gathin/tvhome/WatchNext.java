package com.gathin.tvhome;

import android.content.ContentResolver;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;

import java.util.ArrayList;
import java.util.List;

/**
 * "Continue watching" from Android TV's Watch Next list. Reading it takes a permission that many TVs grant only to
 * their own launcher, so {@link #load} reports {@link #denied} and the canvas says so instead of showing nothing.
 */
final class WatchNext {
    static final Uri URI = Uri.parse("content://android.media.tv/watch_next_program");

    static final class Entry {
        String title, pkg, intentUri;
    }

    boolean denied;
    final List<Entry> entries = new ArrayList<>();

    static WatchNext load(Context ctx, int max) {
        WatchNext out = new WatchNext();
        ContentResolver cr = ctx.getContentResolver();
        String[] cols = {"title", "package_name", "intent_uri"};
        try (Cursor c = cr.query(URI, cols, null, null, null)) {
            if (c == null) { out.denied = true; return out; }
            while (c.moveToNext() && out.entries.size() < max) {
                Entry e = new Entry();
                e.title = c.getString(0);
                e.pkg = c.getString(1);
                e.intentUri = c.getString(2);
                if (e.title != null && !e.title.isEmpty()) out.entries.add(e);
            }
        } catch (SecurityException | IllegalArgumentException e) {
            out.denied = true;
        } catch (RuntimeException e) {
            out.denied = true;
        }
        return out;
    }
}
