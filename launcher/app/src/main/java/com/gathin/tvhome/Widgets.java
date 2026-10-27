package com.gathin.tvhome;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Path;
import android.graphics.Paint;
import android.graphics.RectF;
import android.graphics.Typeface;
import android.os.Handler;
import android.os.Looper;
import android.view.View;

import java.util.Calendar;
import java.util.Locale;

/** Custom-drawn widgets for the canvas Home. Plain Canvas code, no assets; each redraws only when asked to. */
final class Widgets {
    private Widgets() {}

    interface Animated {
        void startAnim();
        void stopAnim();
    }

    /** Configures and returns a paint the view already owns: nothing is allocated while drawing. */
    private static Paint configure(Paint p, int color, Paint.Style style, float stroke) {
        p.setColor(color);
        p.setStyle(style);
        p.setStrokeWidth(stroke);
        p.setStrokeCap(Paint.Cap.ROUND);
        return p;
    }

    private static int withAlpha(int color, float alpha) {
        return ((int) (alpha * 255) << 24) | (color & 0x00FFFFFF);
    }

    /** A bar, ring or graph for CPU, memory or network use. A negative value means "unavailable on this TV". */
    static final class StatView extends View {
        private static final int HISTORY = 60;
        private final LayoutSpec.Item it;
        private final String label;
        private final Typeface face;
        private final float unit;
        private final double[] hist = new double[HISTORY];
        private int count;
        private double value = -1, max;
        private final boolean network;
        private final Paint pText = new Paint(Paint.ANTI_ALIAS_FLAG), pTrack = new Paint(Paint.ANTI_ALIAS_FLAG), pFill = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final RectF box = new RectF();
        private final Path path = new Path();

        StatView(Context ctx, LayoutSpec.Item it, String label, Typeface face, float unit, boolean network) {
            super(ctx);
            this.it = it;
            this.label = label;
            this.face = face;
            this.unit = unit;
            this.network = network;
            this.max = network ? 10 : 100;
            setFocusable(false);
        }

        void set(double v) {
            value = v;
            if (v >= 0) {
                System.arraycopy(hist, 1, hist, 0, HISTORY - 1);
                hist[HISTORY - 1] = v;
                if (count < HISTORY) count++;
                if (network && v > max) max = v;
            }
            invalidate();
        }

        private String shown() {
            if (value < 0) return "n/a";
            return network ? String.format(Locale.US, "%.1f MB/s", value) : Math.round(value) + "%";
        }

        @Override protected void onDraw(Canvas c) {
            float w = getWidth(), h = getHeight();
            float size = (float) it.size * unit;
            Paint text = configure(pText, it.color, Paint.Style.FILL, 0);
            text.setTypeface(face);
            text.setTextSize(size);
            text.setLetterSpacing((float) it.spacing);
            float frac = value < 0 ? 0 : (float) Math.max(0, Math.min(1, value / max));
            Paint track = configure(pTrack, withAlpha(it.color, 0.25f), Paint.Style.STROKE, 0);
            if ("ring".equals(it.variant)) {
                float d = Math.min(w, h), stroke = Math.max(2f, d * 0.09f), r = d / 2 - stroke;
                float cx = w / 2, cy = h / 2;
                track.setStrokeWidth(stroke);
                c.drawCircle(cx, cy, r, track);
                Paint arc = configure(pFill, it.color, Paint.Style.STROKE, stroke);
                box.set(cx - r, cy - r, cx + r, cy + r);
                c.drawArc(box, -90, 360 * frac, false, arc);
                text.setTextAlign(Paint.Align.CENTER);
                text.setTextSize(Math.min(size, r * 0.6f));
                c.drawText(shown(), cx, cy + text.getTextSize() * 0.35f, text);
                return;
            }
            // Bar and graph: the name on the left and the number on the right of the top line.
            text.setTextAlign(Paint.Align.LEFT);
            c.drawText(label, 0, size, text);
            text.setTextAlign(Paint.Align.RIGHT);
            c.drawText(shown(), w, size, text);
            float top = size * 1.5f;
            if ("bar".equals(it.variant)) {
                float bh = Math.max(3f, size * 0.22f);
                track.setStyle(Paint.Style.FILL);
                box.set(0, top, w, top + bh);
                c.drawRoundRect(box, bh / 2, bh / 2, track);
                Paint f = configure(pFill, it.color, Paint.Style.FILL, 0);
                box.set(0, top, Math.max(bh, w * frac), top + bh);
                c.drawRoundRect(box, bh / 2, bh / 2, f);
            } else {
                Paint line = configure(pFill, it.color, Paint.Style.STROKE, Math.max(2f, size * 0.1f));
                path.rewind();
                float gh = Math.max(4f, h - top - 2);
                for (int i = 0; i < count; i++) {
                    float x = count <= 1 ? 0 : w * i / (count - 1);
                    float y = top + gh * (1 - (float) Math.max(0, Math.min(1, hist[HISTORY - count + i] / max)));
                    if (i == 0) path.moveTo(x, y); else path.lineTo(x, y);
                }
                c.drawPath(path, line);
            }
        }
    }

    /** A round clock face with hour, minute and optional second hands. */
    static final class AnalogClockView extends View {
        private final LayoutSpec.Item it;
        private final Paint pen = new Paint(Paint.ANTI_ALIAS_FLAG);

        AnalogClockView(Context ctx, LayoutSpec.Item it) { super(ctx); this.it = it; setFocusable(false); }

        @Override protected void onDraw(Canvas c) {
            float d = Math.min(getWidth(), getHeight()), cx = getWidth() / 2f, cy = getHeight() / 2f, r = d / 2f * 0.92f;
            float stroke = Math.max(1.5f, (float) Math.max(2, it.stroke) * d / 100f);
            if (it.hasFill) c.drawCircle(cx, cy, r, configure(pen, it.fill, Paint.Style.FILL, 0));
            c.drawCircle(cx, cy, r, configure(pen, it.color, Paint.Style.STROKE, stroke));
            for (int i = 0; i < 12; i++) {
                double a = Math.toRadians(i * 30);
                float in = r * (i % 3 == 0 ? 0.82f : 0.88f);
                c.drawLine(cx + (float) Math.sin(a) * in, cy - (float) Math.cos(a) * in, cx + (float) Math.sin(a) * r * 0.95f, cy - (float) Math.cos(a) * r * 0.95f, configure(pen, it.color, Paint.Style.STROKE, stroke * 0.6f));
            }
            Calendar now = Calendar.getInstance();
            double s = now.get(Calendar.SECOND), m = now.get(Calendar.MINUTE) + s / 60, h = (now.get(Calendar.HOUR) % 12) + m / 60;
            hand(c, cx, cy, h * 30, r * 0.5f, stroke * 1.4f);
            hand(c, cx, cy, m * 6, r * 0.78f, stroke);
            if (it.seconds) hand(c, cx, cy, s * 6, r * 0.86f, stroke * 0.5f);
            c.drawCircle(cx, cy, stroke * 1.2f, configure(pen, it.color, Paint.Style.FILL, 0));
        }

        private void hand(Canvas c, float cx, float cy, double deg, float len, float w) {
            double a = Math.toRadians(deg);
            c.drawLine(cx, cy, cx + (float) Math.sin(a) * len, cy - (float) Math.cos(a) * len, configure(pen, it.color, Paint.Style.STROKE, w));
        }
    }

    /** The current month as a grid, today circled. */
    static final class CalendarView extends View {
        private final LayoutSpec.Item it;
        private final Typeface face;
        private final float unit;
        private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG), ring = new Paint(Paint.ANTI_ALIAS_FLAG);

        CalendarView(Context ctx, LayoutSpec.Item it, Typeface face, float unit) {
            super(ctx);
            this.it = it;
            this.face = face;
            this.unit = unit;
            setFocusable(false);
        }

        @Override protected void onDraw(Canvas c) {
            float w = getWidth(), h = getHeight();
            Calendar now = Calendar.getInstance();
            Calendar first = (Calendar) now.clone();
            first.set(Calendar.DAY_OF_MONTH, 1);
            int lead = (first.get(Calendar.DAY_OF_WEEK) - 1 + (it.mondayFirst ? 6 : 0)) % 7;
            int days = now.getActualMaximum(Calendar.DAY_OF_MONTH);
            int rows = (lead + days + 6) / 7;
            float size = (float) it.size * unit * 0.8f;
            configure(p, it.color, Paint.Style.FILL, 0);
            p.setTypeface(face);
            p.setLetterSpacing((float) it.spacing);
            p.setTextAlign(Paint.Align.CENTER);
            p.setTextSize(size * 1.5f);
            c.drawText(new java.text.SimpleDateFormat("LLLL", Locale.getDefault()).format(now.getTime()), w / 2, size * 1.5f, p);
            p.setTextSize(size);
            float top = size * 3f, cell = w / 7f, rowH = Math.min(size * 1.9f, (h - top) / (rows + 1));
            String heads = it.mondayFirst ? "MTWTFSS" : "SMTWTFS";
            p.setAlpha(150);
            for (int i = 0; i < 7; i++) c.drawText(String.valueOf(heads.charAt(i)), cell * (i + 0.5f), top, p);
            p.setAlpha(255);
            int today = now.get(Calendar.DAY_OF_MONTH);
            for (int d = 1; d <= days; d++) {
                int pos = lead + d - 1;
                float x = cell * (pos % 7 + 0.5f), y = top + rowH * (pos / 7 + 1);
                c.drawText(String.valueOf(d), x, y, p);
                if (d == today) c.drawCircle(x, y - size * 0.35f, size * 0.95f, configure(ring, it.color, Paint.Style.STROKE, Math.max(1.5f, size * 0.09f)));
            }
        }
    }

    /**
     * Animated bars, mirrored bars or dots. Decoration only: a TV launcher cannot hear other apps' audio, so the
     * movement is generated, not read from sound. Redraws about 12 times a second while Home is on screen.
     */
    static final class VisualizerView extends View implements Animated {
        private final LayoutSpec.Item it;
        private final boolean animate;
        private final Handler handler = new Handler(Looper.getMainLooper());
        private double t;
        private boolean running;
        private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG), ref = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final RectF bar = new RectF();
        private final Runnable step = new Runnable() {
            @Override public void run() {
                if (!running) return;
                t += 0.09;
                invalidate();
                handler.postDelayed(this, 80);
            }
        };

        VisualizerView(Context ctx, LayoutSpec.Item it, boolean animate) {
            super(ctx);
            this.it = it;
            this.animate = animate;
            setFocusable(false);
        }

        @Override public void startAnim() {
            if (!animate || running) return;
            running = true;
            handler.post(step);
        }

        @Override public void stopAnim() {
            running = false;
            handler.removeCallbacks(step);
        }

        static double level(int i, double t, int n) {
            double x = (double) i / n;
            double v = 0.5 + 0.28 * Math.sin(t * 2.1 + i * 0.7) + 0.2 * Math.sin(t * 3.7 + i * 1.9) + 0.12 * Math.sin(t * 1.3 + x * 9);
            double env = 0.45 + 0.55 * Math.sin(Math.PI * Math.min(1, Math.max(0, x * 0.9 + 0.05)));
            return Math.min(1, Math.max(0.04, v * env));
        }

        @Override protected void onDraw(Canvas c) {
            int n = it.bars;
            float w = getWidth(), h = getHeight();
            boolean mirror = "mirror".equals(it.variant), dots = "dots".equals(it.variant);
            float top = mirror ? h * 0.6f : h, slot = w / n, bw = slot * 0.55f;
            configure(p, withAlpha(it.color, (float) it.opacity), Paint.Style.FILL, 0);
            configure(ref, withAlpha(it.color, (float) it.opacity * 0.3f), Paint.Style.FILL, 0);
            for (int i = 0; i < n; i++) {
                float lvl = (float) level(i, t, n), x = slot * (i + 0.5f);
                if (dots) { c.drawCircle(x, top - lvl * top, bw * 0.6f, p); continue; }
                bar.set(x - bw / 2, top - lvl * top, x + bw / 2, top);
                c.drawRoundRect(bar, bw / 3, bw / 3, p);
                if (mirror) {
                    bar.set(x - bw / 2, top + 2, x + bw / 2, top + 2 + lvl * h * 0.38f);
                    c.drawRoundRect(bar, bw / 3, bw / 3, ref);
                }
            }
        }
    }

    /** A rectangle or panel, circle, line or ring: the building blocks for glass shelves, discs and dividers. */
    static final class ShapeView extends View {
        private final LayoutSpec.Item it;
        private final float unit;
        private final Paint pen = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final RectF box = new RectF();

        ShapeView(Context ctx, LayoutSpec.Item it, float unit) { super(ctx); this.it = it; this.unit = unit; setFocusable(false); }

        @Override protected void onDraw(Canvas c) {
            float w = getWidth(), h = getHeight(), stroke = (float) it.stroke * unit;
            int fill = it.hasFill ? it.fill : it.color;
            float a = (float) it.opacity;
            switch (it.variant == null ? "rect" : it.variant) {
                case "line":
                    float t = stroke > 0 ? stroke : 2 * unit;
                    box.set(0, h / 2 - t / 2, w, h / 2 + t / 2);
                    c.drawRoundRect(box, t / 2, t / 2, configure(pen, withAlpha(fill, a), Paint.Style.FILL, 0));
                    break;
                case "circle": {
                    float r = Math.min(w, h) / 2;
                    if (it.hasFill) c.drawCircle(w / 2, h / 2, r - stroke / 2, configure(pen, withAlpha(it.fill, a), Paint.Style.FILL, 0));
                    if (stroke > 0) c.drawCircle(w / 2, h / 2, r - stroke / 2, configure(pen, withAlpha(it.color, a), Paint.Style.STROKE, stroke));
                    break;
                }
                case "ring": {
                    float sw = Math.max(2 * unit, stroke), r = Math.min(w, h) / 2 - sw / 2;
                    c.drawCircle(w / 2, h / 2, r, configure(pen, withAlpha(fill, a), Paint.Style.STROKE, sw));
                    break;
                }
                default: {
                    float rad = (float) it.radius * unit;
                    box.set(stroke / 2, stroke / 2, w - stroke / 2, h - stroke / 2);
                    if (it.hasFill) c.drawRoundRect(box, rad, rad, configure(pen, withAlpha(it.fill, a), Paint.Style.FILL, 0));
                    if (stroke > 0) c.drawRoundRect(box, rad, rad, configure(pen, withAlpha(it.color, a), Paint.Style.STROKE, stroke));
                }
            }
        }
    }
}
