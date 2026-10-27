package com.gathin.tvhome;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.LinearGradient;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.Rect;
import android.graphics.Shader;
import android.os.SystemClock;
import android.view.View;

import java.util.Calendar;
import java.util.Random;

/**
 * A live scene drawn on the TV: rain, snow, aurora, fireflies, space, cherry blossom, embers or underwater, over a
 * sky that follows the time of day (sunrise, day, sunset, night). Our own renderer: plain Canvas drawing, one small
 * gradient bitmap stretched over the screen, at most 30 frames a second and only while Home is on screen.
 * With {@code animate} false (low-RAM TVs, reduced motion) it draws a single still frame.
 */
final class AmbientScene extends View {

    static final String[] SCENES = {"rain", "snow", "aurora", "fireflies", "space", "sakura", "embers", "underwater"};
    static final String[] TIMES = {"auto", "sunrise", "day", "sunset", "night"};

    private final String scene;
    private final String time;
    private final boolean animate;
    private final float unit;
    private final Random rnd = new Random(7);
    private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint band = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path path = new Path();
    private final Rect src = new Rect(0, 0, 1, 64);
    private final Rect dst = new Rect();
    private Bitmap sky;
    private String skyPhase = "";

    // Particles: x, y, speed, size, phase, extra (all in screen pixels / per second).
    private float[] px, py, ps, pz, pp, pe;
    private int count;
    private long last;
    private boolean running;
    private float shootX = -1, shootY, shootT;
    /** Gradients built once per size and part of day, never per frame. */
    private Shader emberGlow, rays;
    private final Shader[] aurora = new Shader[3];
    private static final int[][] AURORA = {{0x0034F5A0, 0x6634F5A0}, {0x0022C8E8, 0x5522C8E8}, {0x00A060F0, 0x44A060F0}};

    private void buildShaders(int w, int h, boolean night) {
        emberGlow = new LinearGradient(0, h * 0.7f, 0, h, 0x00FF6A1A, 0x55FF6A1A, Shader.TileMode.CLAMP);
        rays = new LinearGradient(0, 0, 0, h * 0.8f, night ? 0x18FFFFFF : 0x30BFF6FF, 0x00BFF6FF, Shader.TileMode.CLAMP);
        for (int k = 0; k < 3; k++) {
            float base = h * (0.22f + k * 0.08f), amp = h * 0.06f, height = h * (0.22f - k * 0.03f);
            aurora[k] = new LinearGradient(0, base + height, 0, base - amp, AURORA[k][0], AURORA[k][1], Shader.TileMode.CLAMP);
        }
    }

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        super.onSizeChanged(w, h, ow, oh);
        if (w > 0 && h > 0) { seed(w, h); buildShaders(w, h, "night".equals(phase())); }
    }

    AmbientScene(Context ctx, String scene, String time, boolean animate) {
        super(ctx);
        this.scene = valid(scene, SCENES) ? scene : "rain";
        this.time = valid(time, TIMES) ? time : "auto";
        this.animate = animate;
        this.unit = Math.max(0.5f, ctx.getResources().getDisplayMetrics().heightPixels / 1080f);
        setFocusable(false);
        setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_NO);
    }

    static boolean valid(String v, String[] set) {
        if (v == null) return false;
        for (String s : set) if (s.equals(v)) return true;
        return false;
    }

    /** The part of the day for an hour (0-23): fixed hours, close enough for a backdrop anywhere in India or Europe. */
    static String phaseFor(int hour) {
        if (hour >= 5 && hour < 8) return "sunrise";
        if (hour >= 8 && hour < 17) return "day";
        if (hour >= 17 && hour < 20) return "sunset";
        return "night";
    }

    String phase() {
        return "auto".equals(time) ? phaseFor(Calendar.getInstance().get(Calendar.HOUR_OF_DAY)) : time;
    }

    /** Top and bottom sky colours for a scene at a part of the day. */
    static int[] skyColors(String scene, String phase) {
        switch (scene) {
            case "space": return new int[]{0xFF02030A, 0xFF0B1030};
            case "embers": return new int[]{0xFF0B0503, "night".equals(phase) ? 0xFF1E0A04 : 0xFF2E1206};
            case "underwater":
                return "night".equals(phase) ? new int[]{0xFF01121A, 0xFF03303D}
                        : "day".equals(phase) ? new int[]{0xFF0A5A6E, 0xFF022B3A} : new int[]{0xFF06404F, 0xFF021E29};
            case "aurora": return "day".equals(phase) ? new int[]{0xFF10233F, 0xFF2A4A6A} : new int[]{0xFF020614, 0xFF0C1A2E};
            default: break;
        }
        int top, bottom;
        switch (phase) {
            case "sunrise": top = 0xFF2B2F5A; bottom = 0xFFF29E7A; break;
            case "day": top = 0xFF4F8CC9; bottom = 0xFFB9DCF2; break;
            case "sunset": top = 0xFF2A1F4D; bottom = 0xFFE8735A; break;
            default: top = 0xFF050816; bottom = 0xFF141A33; break;
        }
        if ("rain".equals(scene)) { top = mix(top, 0xFF2A2F38, 0.55f); bottom = mix(bottom, 0xFF3A4048, 0.55f); }
        if ("snow".equals(scene)) { top = mix(top, 0xFF8A97A8, 0.3f); bottom = mix(bottom, 0xFFD5DEE8, 0.35f); }
        if ("sakura".equals(scene) && !"night".equals(phase)) bottom = mix(bottom, 0xFFF6C6D4, 0.35f);
        return new int[]{top, bottom};
    }

    /** Opaque colour between a and b (plain bit maths, so it runs in JVM unit tests too). */
    static int mix(int a, int b, float t) {
        int r = (int) (((a >> 16) & 0xFF) + ((((b >> 16) & 0xFF) - ((a >> 16) & 0xFF)) * t));
        int g = (int) (((a >> 8) & 0xFF) + ((((b >> 8) & 0xFF) - ((a >> 8) & 0xFF)) * t));
        int bl = (int) ((a & 0xFF) + (((b & 0xFF) - (a & 0xFF)) * t));
        return 0xFF000000 | (r << 16) | (g << 8) | bl;
    }

    /** How many particles a scene draws at 1080p. */
    static int particles(String scene) {
        switch (scene) {
            case "rain": return 220;
            case "snow": return 170;
            case "fireflies": return 46;
            case "space": return 240;
            case "sakura": return 42;
            case "embers": return 80;
            case "underwater": return 50;
            case "aurora": return 160;   // stars behind the ribbons
            default: return 100;
        }
    }

    private void seed(int w, int h) {
        count = particles(scene);
        px = new float[count]; py = new float[count]; ps = new float[count]; pz = new float[count]; pp = new float[count]; pe = new float[count];
        for (int i = 0; i < count; i++) {
            px[i] = rnd.nextFloat() * w;
            py[i] = rnd.nextFloat() * h;
            pp[i] = rnd.nextFloat() * 6.283f;
            pe[i] = rnd.nextFloat();
            switch (scene) {
                case "rain": ps[i] = (900 + rnd.nextFloat() * 600) * unit; pz[i] = (16 + rnd.nextFloat() * 22) * unit; break;
                case "snow": ps[i] = (35 + rnd.nextFloat() * 80) * unit; pz[i] = (1.5f + rnd.nextFloat() * 3.5f) * unit; break;
                case "fireflies": ps[i] = (10 + rnd.nextFloat() * 25) * unit; pz[i] = (2 + rnd.nextFloat() * 2.5f) * unit; py[i] = h * (0.35f + rnd.nextFloat() * 0.65f); break;
                case "space": case "aurora": ps[i] = (2 + rnd.nextFloat() * 6) * unit; pz[i] = (0.6f + rnd.nextFloat() * 1.4f) * unit;
                    if ("aurora".equals(scene)) py[i] = rnd.nextFloat() * h * 0.7f; break;
                case "sakura": ps[i] = (40 + rnd.nextFloat() * 50) * unit; pz[i] = (6 + rnd.nextFloat() * 7) * unit; break;
                case "embers": ps[i] = (40 + rnd.nextFloat() * 90) * unit; pz[i] = (1.5f + rnd.nextFloat() * 2.5f) * unit; break;
                case "underwater": ps[i] = (30 + rnd.nextFloat() * 60) * unit; pz[i] = (3 + rnd.nextFloat() * 9) * unit; break;
                default: ps[i] = 50 * unit; pz[i] = 2 * unit; break;
            }
        }
    }

    void start() {
        running = animate;
        last = SystemClock.uptimeMillis();
        invalidate();
    }

    void stop() { running = false; }

    String sceneId() { return scene; }

    @Override
    protected void onDraw(Canvas c) {
        int w = getWidth(), h = getHeight();
        if (w <= 0 || h <= 0) return;
        if (px == null) { seed(w, h); buildShaders(w, h, "night".equals(phase())); }
        String ph = phase();
        if (sky == null || !ph.equals(skyPhase)) {
            int[] col = skyColors(scene, ph);
            if (sky == null) sky = Bitmap.createBitmap(1, 64, Bitmap.Config.ARGB_8888);
            Canvas sc = new Canvas(sky);
            Paint g = new Paint();
            g.setShader(new LinearGradient(0, 0, 0, 64, col[0], col[1], Shader.TileMode.CLAMP));
            sc.drawRect(0, 0, 1, 64, g);
            skyPhase = ph;
            buildShaders(w, h, "night".equals(ph));
        }
        dst.set(0, 0, w, h);
        c.drawBitmap(sky, src, dst, null);

        long now = SystemClock.uptimeMillis();
        float dt = Math.min(0.1f, (now - last) / 1000f);
        last = now;
        float t = now / 1000f;
        if (!running) dt = 0;
        draw(c, w, h, dt, t, ph);

        if (running) {
            // ~30 fps: the scene is a backdrop, not a game.
            postInvalidateDelayed(33);
        }
    }

    private void draw(Canvas c, int w, int h, float dt, float t, String ph) {
        boolean night = "night".equals(ph);
        switch (scene) {
            case "rain": {
                p.setStrokeWidth(Math.max(1f, 1.4f * unit));
                p.setStrokeCap(Paint.Cap.ROUND);
                for (int i = 0; i < count; i++) {
                    py[i] += ps[i] * dt; px[i] += ps[i] * 0.18f * dt;
                    if (py[i] > h) { py[i] = -pz[i]; px[i] = rnd.nextFloat() * w * 1.2f - w * 0.1f; }
                    p.setColor(Color.argb((int) (60 + pe[i] * 90), 200, 215, 235));
                    c.drawLine(px[i], py[i], px[i] - pz[i] * 0.18f, py[i] - pz[i], p);
                }
                break;
            }
            case "snow": {
                for (int i = 0; i < count; i++) {
                    py[i] += ps[i] * dt; px[i] += (float) Math.sin(t * 0.8 + pp[i]) * 18 * unit * dt;
                    if (py[i] > h + 6) { py[i] = -6; px[i] = rnd.nextFloat() * w; }
                    p.setColor(Color.argb((int) (150 + pe[i] * 100), 255, 255, 255));
                    c.drawCircle(px[i], py[i], pz[i], p);
                }
                break;
            }
            case "fireflies": {
                for (int i = 0; i < count; i++) {
                    px[i] += (float) Math.cos(t * 0.3 + pp[i]) * ps[i] * dt;
                    py[i] += (float) Math.sin(t * 0.23 + pp[i] * 1.7) * ps[i] * dt;
                    if (px[i] < 0) px[i] += w; if (px[i] > w) px[i] -= w;
                    if (py[i] < h * 0.3f) py[i] = h * 0.3f; if (py[i] > h) py[i] = h;
                    float glow = 0.5f + 0.5f * (float) Math.sin(t * (1.2 + pe[i]) + pp[i]);
                    p.setColor(Color.argb((int) (40 * glow), 255, 230, 120));
                    c.drawCircle(px[i], py[i], pz[i] * 5, p);
                    p.setColor(Color.argb((int) (230 * glow), 255, 240, 150));
                    c.drawCircle(px[i], py[i], pz[i], p);
                }
                break;
            }
            case "space":
            case "aurora": {
                for (int i = 0; i < count; i++) {
                    px[i] -= ps[i] * dt;
                    if (px[i] < 0) px[i] += w;
                    float tw = 0.6f + 0.4f * (float) Math.sin(t * (0.7 + pe[i] * 2) + pp[i]);
                    p.setColor(Color.argb((int) ((night || "space".equals(scene) ? 220 : 90) * tw), 255, 255, 255));
                    c.drawCircle(px[i], py[i], pz[i], p);
                }
                if ("aurora".equals(scene)) drawAurora(c, w, h, t);
                else drawShootingStar(c, w, h, dt);
                break;
            }
            case "sakura": {
                for (int i = 0; i < count; i++) {
                    py[i] += ps[i] * dt; px[i] += ((float) Math.sin(t * 0.9 + pp[i]) * 40 + 20) * unit * dt;
                    if (py[i] > h + 20 || px[i] > w + 20) { py[i] = -20; px[i] = rnd.nextFloat() * w - w * 0.2f; }
                    c.save();
                    c.translate(px[i], py[i]);
                    c.rotate((t * 60 * (0.5f + pe[i]) + pp[i] * 57) % 360);
                    p.setColor(Color.argb(220, 248, 183, 205));
                    c.drawOval(-pz[i], -pz[i] * 0.55f, pz[i], pz[i] * 0.55f, p);
                    c.restore();
                }
                break;
            }
            case "embers": {
                for (int i = 0; i < count; i++) {
                    py[i] -= ps[i] * dt; px[i] += (float) Math.sin(t * 1.3 + pp[i]) * 22 * unit * dt;
                    if (py[i] < -6) { py[i] = h + 6; px[i] = rnd.nextFloat() * w; }
                    float life = Math.max(0f, Math.min(1f, py[i] / h));
                    float flick = 0.7f + 0.3f * (float) Math.sin(t * 9 + pp[i] * 3);
                    p.setColor(Color.argb((int) (255 * life * flick), 255, (int) (120 + 80 * pe[i]), 40));
                    c.drawCircle(px[i], py[i], pz[i], p);
                }
                // A warm glow along the bottom, as if from the fire below.
                band.setShader(emberGlow);
                c.drawRect(0, h * 0.7f, w, h, band);
                break;
            }
            case "underwater": {
                // Light rays from the surface, slowly swaying.
                for (int r = 0; r < 5; r++) {
                    float x = w * (0.1f + r * 0.2f) + (float) Math.sin(t * 0.2 + r) * 40 * unit;
                    band.setShader(rays);
                    path.reset();
                    path.moveTo(x - 30 * unit, 0); path.lineTo(x + 30 * unit, 0);
                    path.lineTo(x + 150 * unit, h * 0.8f); path.lineTo(x - 10 * unit, h * 0.8f); path.close();
                    c.drawPath(path, band);
                }
                p.setStyle(Paint.Style.STROKE);
                p.setStrokeWidth(Math.max(1f, 1.5f * unit));
                for (int i = 0; i < count; i++) {
                    py[i] -= ps[i] * dt; px[i] += (float) Math.sin(t * 2 + pp[i]) * 14 * unit * dt;
                    if (py[i] < -pz[i]) { py[i] = h + pz[i]; px[i] = rnd.nextFloat() * w; }
                    p.setColor(Color.argb((int) (90 + pe[i] * 80), 210, 245, 255));
                    c.drawCircle(px[i], py[i], pz[i], p);
                }
                p.setStyle(Paint.Style.FILL);
                break;
            }
            default: break;
        }
    }

    private void drawAurora(Canvas c, int w, int h, float t) {
        for (int k = 0; k < 3; k++) {
            float base = h * (0.22f + k * 0.08f);
            float amp = h * 0.06f, height = h * (0.22f - k * 0.03f);
            path.reset();
            int step = Math.max(8, w / 80);
            for (int x = 0; x <= w; x += step) {
                float y = base + amp * (float) Math.sin(x * 0.004f + t * (0.25f + k * 0.07f) + k)
                        + amp * 0.5f * (float) Math.sin(x * 0.011f - t * 0.4f);
                if (x == 0) path.moveTo(x, y); else path.lineTo(x, y);
            }
            for (int x = w; x >= 0; x -= step) {
                float y = base + height + amp * 0.6f * (float) Math.sin(x * 0.005f + t * 0.3f + k * 2);
                path.lineTo(x, y);
            }
            path.close();
            band.setShader(aurora[k]);
            c.drawPath(path, band);
        }
    }

    private void drawShootingStar(Canvas c, int w, int h, float dt) {
        if (!running) return;
        if (shootX < 0) {
            if (rnd.nextFloat() < dt / 8f) { shootX = rnd.nextFloat() * w; shootY = rnd.nextFloat() * h * 0.4f; shootT = 0; }
            return;
        }
        shootT += dt;
        float len = 160 * unit, sp = 900 * unit;
        float x = shootX + sp * shootT, y = shootY + sp * 0.45f * shootT;
        p.setStrokeWidth(2 * unit);
        p.setColor(Color.argb((int) (220 * Math.max(0, 1 - shootT / 0.9f)), 255, 255, 255));
        c.drawLine(x, y, x - len, y - len * 0.45f, p);
        if (shootT > 0.9f) shootX = -1;
    }
}
