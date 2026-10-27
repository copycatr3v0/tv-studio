package com.gathin.tvhome;

import android.app.ActivityManager;
import android.content.Context;

/** TVs where moving backgrounds and animations should stay off unless the person asked for them anyway. */
final class LowRam {
    private LowRam() {}

    /** Below this much memory (about 1.75 GB, so "1.5 GB" boards and smaller) heavy effects default to off. */
    static final long LIMIT_BYTES = 1_750_000_000L;

    static boolean isLow(Context ctx) {
        ActivityManager am = (ActivityManager) ctx.getSystemService(Context.ACTIVITY_SERVICE);
        if (am == null) return false;
        if (am.isLowRamDevice()) return true;
        ActivityManager.MemoryInfo mi = new ActivityManager.MemoryInfo();
        am.getMemoryInfo(mi);
        return isLow(mi.totalMem);
    }

    static boolean isLow(long totalBytes) { return totalBytes > 0 && totalBytes < LIMIT_BYTES; }
}
