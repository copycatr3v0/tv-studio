package com.gathin.tvhome;

import android.app.ActivityManager;
import android.content.Context;
import android.net.TrafficStats;
import android.os.SystemClock;

import java.io.BufferedReader;
import java.io.FileReader;
import java.io.IOException;

/**
 * CPU, memory and network numbers for the canvas. Memory comes from ActivityManager and the network from
 * TrafficStats (both allowed to any app). Android 8+ hides /proc/stat from apps, so the total CPU number is
 * often unavailable: {@link #cpuPercent} then returns -1 and the canvas shows "n/a" rather than a made-up figure.
 */
final class StatsSampler {
    private final ActivityManager am;
    private long lastIdle = -1, lastTotal = -1, lastRx = -1, lastAt;

    StatsSampler(Context ctx) {
        am = (ActivityManager) ctx.getSystemService(Context.ACTIVITY_SERVICE);
    }

    /** Percent of RAM in use, or -1. */
    int memoryPercent() {
        if (am == null) return -1;
        ActivityManager.MemoryInfo mi = new ActivityManager.MemoryInfo();
        am.getMemoryInfo(mi);
        return mi.totalMem <= 0 ? -1 : (int) Math.round(100.0 * (mi.totalMem - mi.availMem) / mi.totalMem);
    }

    /** Percent of CPU busy since the previous call, or -1 when this TV does not let apps read it. */
    int cpuPercent() {
        try (BufferedReader r = new BufferedReader(new FileReader("/proc/stat"))) {
            String line = r.readLine();
            if (line == null || !line.startsWith("cpu ")) return -1;
            String[] p = line.trim().split("\\s+");
            long total = 0;
            for (int i = 1; i < p.length; i++) total += Long.parseLong(p[i]);
            long idle = Long.parseLong(p[4]) + (p.length > 5 ? Long.parseLong(p[5]) : 0);
            int out = -1;
            if (lastTotal >= 0 && total > lastTotal) out = (int) Math.round(100.0 * ((total - lastTotal) - (idle - lastIdle)) / (total - lastTotal));
            lastTotal = total;
            lastIdle = idle;
            return Math.max(-1, Math.min(100, out));
        } catch (IOException | RuntimeException e) {
            return -1;
        }
    }

    /** Download speed in MB/s since the previous call, or -1 when unknown. */
    double downMBps() {
        long rx = TrafficStats.getTotalRxBytes(), now = SystemClock.elapsedRealtime();
        double out = -1;
        if (rx != TrafficStats.UNSUPPORTED && lastRx >= 0 && now > lastAt) out = (rx - lastRx) / 1048576.0 / ((now - lastAt) / 1000.0);
        lastRx = rx == TrafficStats.UNSUPPORTED ? -1 : rx;
        lastAt = now;
        return out;
    }
}
