#!/usr/bin/env python3
"""Generate the bundled sample background videos: short seamless loops, drawn from scratch (original art,
released with this repo under MIT; no third-party footage).

    python3 tools/make_videos.py [name ...]  -> public/videos/<id>.mp4 + <id>.jpg (poster)

1280x720, 24 fps, 8 s, H.264 (baseline-friendly: yuv420p, no B-frame tricks needed), no audio, a few MB each.
Every moving thing is periodic over the loop (integer wraps, whole sine cycles), so the last frame flows into
the first. Deterministic: seeded random only.
"""
import math
import random
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

W, H, FPS, SECONDS = 1280, 720, 24, 8
N = FPS * SECONDS
TAU = math.tau
OUT = Path(__file__).resolve().parent.parent / "public" / "videos"


def vgrad(stops):
    """Vertical gradient from [(pos 0..1, (r,g,b)), ...]."""
    ys = np.linspace(0, 1, H)[:, None]
    pos = [p for p, _ in stops]
    cols = np.array([c for _, c in stops], dtype=np.float32)
    out = np.stack([np.interp(ys[:, 0], pos, cols[:, i]) for i in range(3)], axis=1)
    return Image.fromarray(np.repeat(out[:, None, :], W, axis=1).astype(np.uint8))


def blur(img, r):
    return img.filter(ImageFilter.GaussianBlur(r))


def bokeh(seed, palette, count=46, rmin=18, rmax=70, bg=(8, 10, 22)):
    """A static field of soft coloured lights, as seen through a rainy or foggy window at night."""
    rnd = random.Random(seed)
    img = Image.new("RGB", (W, H), bg)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for _ in range(count):
        x, y, r = rnd.randint(0, W), rnd.randint(int(H * 0.25), H), rnd.randint(rmin, rmax)
        c = rnd.choice(palette)
        d.ellipse([x - r, y - r, x + r, y + r], fill=c + (rnd.randint(60, 150),))
    layer = blur(layer, 4)
    return Image.alpha_composite(img.convert("RGBA"), layer).convert("RGB")


CITY = [(255, 190, 110), (255, 120, 90), (120, 190, 255), (255, 235, 170), (190, 140, 255), (110, 230, 200)]


# ---------------------------------------------------------------- rain
def make_rain():
    base = vgrad([(0, (16, 24, 44)), (0.6, (26, 38, 62)), (1, (8, 12, 22))])
    base = Image.alpha_composite(base.convert("RGBA"), blur(bokeh(1, CITY, 30, 30, 90).convert("RGBA"), 30).point(lambda v: int(v * 0.5))).convert("RGB")
    base = blur(base, 6)
    rnd = random.Random(7)
    layers = []
    for n, k, length, width, alpha in [(260, 11, 38, 1, 70), (170, 14, 62, 1, 105), (80, 18, 110, 2, 150)]:
        layers.append([(rnd.randint(0, W), rnd.randint(0, H), k + rnd.choice([0, 0, 1]), length * rnd.uniform(0.8, 1.2), width, alpha) for _ in range(n)])
    slant = 0.12

    def frame(t):
        ov = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(ov)
        for layer in layers:
            for x0, y0, k, length, width, alpha in layer:
                y = (y0 + k * H * t) % H
                for off in (0, -H):
                    d.line([(x0 + slant * (y + off - length), y + off - length), (x0 + slant * (y + off), y + off)], fill=(190, 215, 255, alpha), width=width)
        mist = Image.new("RGBA", (W, H), (150, 175, 210, 0))
        return Image.alpha_composite(base.convert("RGBA"), ov).convert("RGB")
    return frame


# ---------------------------------------------------------------- rainy window
def drop_patch(src, cx, cy, rx, ry):
    """A water drop acts as a tiny lens: it shows the scene upside down, slightly brighter, with a rim and a highlight."""
    box = (int(cx - rx * 2), int(cy - ry * 2), int(cx + rx * 2), int(cy + ry * 2))
    crop = src.crop(box).transpose(Image.FLIP_TOP_BOTTOM).resize((int(rx * 2), int(ry * 2)))
    m = Image.new("L", crop.size, 0)
    ImageDraw.Draw(m).ellipse([0, 0, crop.size[0] - 1, crop.size[1] - 1], fill=255)
    return crop, m


def make_rainy_window():
    sharp = bokeh(3, CITY, 52, 14, 60)
    frosted = blur(sharp, 22)
    rnd = random.Random(12)
    statics = [(rnd.randint(0, W), rnd.randint(0, H), rnd.uniform(3, 9)) for _ in range(380)]
    sliders = [(rnd.randint(60, W - 60), rnd.randint(0, H), rnd.choice([1, 1, 2]), rnd.uniform(9, 16)) for _ in range(16)]

    def frame(t):
        img = frosted.copy()
        # Slow breathing of the lights (one whole cycle per loop).
        img = Image.blend(img, blur(sharp, 14), 0.35 + 0.15 * math.sin(TAU * t))
        d = ImageDraw.Draw(img, "RGBA")
        for x, y, r in statics:
            c, m = drop_patch(sharp, x, y, r, r * 1.15)
            img.paste(c, (int(x - r), int(y - r * 1.15)), m)
            d.ellipse([x - r, y - r * 1.15, x + r, y + r * 1.15], outline=(255, 255, 255, 55), width=1)
            d.ellipse([x - r * 0.45, y - r * 0.7, x - r * 0.1, y - r * 0.3], fill=(255, 255, 255, 110))
        for x0, y0, k, r in sliders:
            y = (y0 + k * H * t) % H
            wob = 5 * math.sin(TAU * (t * k * 2) + x0)
            x = x0 + wob
            trail = 250
            for off in (0, -H):
                yy = y + off
                d.line([(x, yy - trail), (x - wob * 0.2, yy)], fill=(255, 255, 255, 22), width=int(r * 0.45))
                c, m = drop_patch(sharp, x, yy, r, r * 1.5)
                img.paste(c, (int(x - r), int(yy - r * 1.5)), m)
                d.ellipse([x - r, yy - r * 1.5, x + r, yy + r * 1.5], outline=(255, 255, 255, 90), width=2)
                d.ellipse([x - r * 0.5, yy - r * 1.0, x - r * 0.1, yy - r * 0.4], fill=(255, 255, 255, 150))
        return img
    return frame


# ---------------------------------------------------------------- foggy window
def periodic_field(rnd, terms=7):
    """Smooth noise that repeats exactly after one loop."""
    comps = []
    for _ in range(terms):
        kx, ky = rnd.uniform(0.8, 4.5), rnd.uniform(0.8, 3.0)
        comps.append((kx, ky, rnd.choice([1, 1, 2, 3]) * rnd.choice([-1, 1]), rnd.uniform(0, TAU), rnd.uniform(0.5, 1.0)))
    gx, gy = np.meshgrid(np.linspace(0, TAU, 96), np.linspace(0, TAU, 54))

    def f(t):
        v = np.zeros_like(gx)
        for kx, ky, n, ph, a in comps:
            v += a * np.sin(kx * gx + ky * gy + TAU * n * t + ph)
        v /= sum(c[4] for c in comps)
        return v
    return f


def make_foggy_window():
    sharp = bokeh(5, CITY, 56, 12, 56)
    soft = blur(sharp, 34)
    rnd = random.Random(21)
    field = periodic_field(rnd)
    wipes = [(rnd.randint(80, W - 80), rnd.randint(0, H), rnd.choice([1, 1, 2]), rnd.uniform(14, 30)) for _ in range(7)]
    fog_col = np.array([188, 200, 214], dtype=np.float32)
    soft_a, sharp_a = np.asarray(soft, dtype=np.float32), np.asarray(blur(sharp, 5), dtype=np.float32)
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)

    def frame(t):
        f = field(t)
        dens = (0.62 + 0.18 * f).clip(0.3, 0.9)
        dens = np.asarray(Image.fromarray((dens * 255).astype(np.uint8)).resize((W, H), Image.BICUBIC), dtype=np.float32) / 255
        clear = np.zeros((H, W), dtype=np.float32)
        for x0, y0, k, w in wipes:
            y = (y0 + k * H * t) % H
            for off in (0, -H):
                head = y + off
                # A cleared streak that a drop has just run down: bright at the drop, fading upward behind it.
                above = np.clip((head - yy) / 340.0, 0, 1)
                inside = (yy <= head + w) & (yy >= head - 340)
                prof = np.exp(-((xx - x0) / w) ** 2) * (1 - above) * inside
                clear = np.maximum(clear, prof)
        clear = np.asarray(Image.fromarray((clear * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(5)), dtype=np.float32) / 255 * 0.7
        d = (dens * (1 - 0.8 * clear))[..., None]
        scene = soft_a * (1 - clear[..., None]) + sharp_a * clear[..., None]
        out = scene * (1 - d) + fog_col * d
        return Image.fromarray(out.clip(0, 255).astype(np.uint8))
    return frame


# ---------------------------------------------------------------- anime night
def cloud_layer(rnd, count, y_lo, y_hi, scale, col, hi):
    clouds = []
    for _ in range(count):
        clouds.append((rnd.randint(0, W), rnd.randint(y_lo, y_hi), rnd.uniform(0.7, 1.3) * scale, rnd.randint(0, 10000)))
    return clouds, col, hi


def draw_cloud(d, x, y, s, seed, col, hi):
    r = random.Random(seed)
    puffs = [(r.uniform(-1, 1) * 90 * s, r.uniform(-0.5, 0.5) * 18 * s, r.uniform(26, 48) * s) for _ in range(7)]
    for px, py, pr in puffs:
        d.ellipse([x + px - pr, y + py - pr * 0.62, x + px + pr, y + py + pr * 0.62], fill=col)
    for px, py, pr in puffs:  # cel-shaded rim light along the top edge
        d.arc([x + px - pr, y + py - pr * 0.62, x + px + pr, y + py + pr * 0.62], 200, 340, fill=hi, width=3)


def city_skyline(seed, base_y, color, wmin, wmax, hmin, hmax):
    rnd = random.Random(seed)
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    x, windows = -10, []
    while x < W:
        bw, bh = rnd.randint(wmin, wmax), rnd.randint(hmin, hmax)
        d.rectangle([x, base_y - bh, x + bw, H], fill=color)
        if rnd.random() < 0.25:
            d.rectangle([x + bw // 2 - 3, base_y - bh - 26, x + bw // 2 + 3, base_y - bh], fill=color)
        for wy in range(base_y - bh + 12, base_y - 8, 22):
            for wx in range(x + 8, x + bw - 10, 18):
                if rnd.random() < 0.34:
                    windows.append((wx, wy, rnd.randint(1, 3), rnd.uniform(0, 1)))
        x += bw + rnd.randint(0, 8)
    return img, windows


def make_anime_night():
    sky = vgrad([(0, (14, 14, 52)), (0.55, (58, 34, 110)), (0.85, (236, 110, 140)), (1, (255, 170, 150))])
    rnd = random.Random(33)
    stars = [(rnd.randint(0, W), rnd.randint(0, int(H * 0.55)), rnd.uniform(0.8, 2.2), rnd.randint(1, 3), rnd.uniform(0, 1)) for _ in range(150)]
    far, far_w = city_skyline(1, 610, (74, 48, 120, 255), 40, 90, 60, 170)
    mid, mid_w = city_skyline(2, 650, (38, 26, 76, 255), 50, 110, 90, 230)
    near, near_w = city_skyline(3, 700, (14, 10, 34, 255), 70, 150, 120, 300)
    c1 = cloud_layer(random.Random(4), 6, 120, 330, 1.2, (126, 92, 170, 235), (214, 170, 230, 255))
    c2 = cloud_layer(random.Random(5), 5, 260, 480, 1.7, (210, 120, 160, 235), (255, 200, 190, 255))
    flies = [(rnd.randint(0, W), rnd.randint(int(H * 0.45), int(H * 0.9)), rnd.uniform(14, 40), rnd.randint(1, 2), rnd.uniform(0, TAU)) for _ in range(26)]

    def frame(t):
        img = sky.convert("RGBA")
        d = ImageDraw.Draw(img, "RGBA")
        for x, y, r, k, ph in stars:
            a = int(110 + 145 * (0.5 + 0.5 * math.sin(TAU * k * t + ph * TAU)))
            d.ellipse([x - r, y - r, x + r, y + r], fill=(255, 245, 220, a))
        # Moon with a glow.
        glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        ImageDraw.Draw(glow).ellipse([860 - 150, 150 - 150, 860 + 150, 150 + 150], fill=(255, 230, 200, 120))
        img = Image.alpha_composite(img, blur(glow, 45))
        d = ImageDraw.Draw(img, "RGBA")
        d.ellipse([860 - 62, 150 - 62, 860 + 62, 150 + 62], fill=(255, 246, 226, 255))
        d.ellipse([860 - 28, 150 - 30, 860 + 4, 150 + 2], fill=(244, 226, 200, 255))
        # Shooting star once per loop.
        if 0.42 < t < 0.52:
            p = (t - 0.42) / 0.10
            sx, sy = 1000 - 520 * p, 60 + 240 * p
            d.line([(sx + 90, sy - 38), (sx, sy)], fill=(255, 255, 255, int(230 * (1 - p))), width=3)
        # Clouds drift sideways: one whole screen width per loop, seamless because each is drawn at -W, 0, +W.
        for (clouds, col, hi), mult in ((c1, 1), (c2, 2)):
            cl = Image.new("RGBA", (W, H), (0, 0, 0, 0))
            cd = ImageDraw.Draw(cl)
            for cx, cy, s, seed in clouds:
                x = (cx - mult * W * t) % W
                for off in (-W, 0, W):
                    draw_cloud(cd, x + off, cy, s, seed, col, hi)
            img = Image.alpha_composite(img, cl)
        # Parallax city: the layers sway by different amounts; windows blink on their own beat.
        for layer, wins, amp, size in ((far, far_w, 3, 2), (mid, mid_w, 6, 3), (near, near_w, 10, 4)):
            dx = amp * math.sin(TAU * t)
            img.alpha_composite(layer, (int(dx), 0))
            wd = ImageDraw.Draw(img, "RGBA")
            for wx, wy, k, ph in wins:
                on = 0.5 + 0.5 * math.sin(TAU * k * t + ph * TAU)
                if on > 0.2:
                    wd.rectangle([wx + dx, wy, wx + dx + size, wy + size * 2], fill=(255, 214, 130, int(60 + 180 * on)))
        fl = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        fd = ImageDraw.Draw(fl)
        for x, y, r, k, ph in flies:
            px, py = x + r * math.sin(TAU * k * t + ph), y + r * 0.7 * math.cos(TAU * k * t + ph)
            a = int(120 + 120 * math.sin(TAU * 2 * t + ph))
            fd.ellipse([px - 3, py - 3, px + 3, py + 3], fill=(255, 240, 160, a))
        img = Image.alpha_composite(img, blur(fl, 3))
        img = Image.alpha_composite(img, fl)
        return img.convert("RGB")
    return frame


# ---------------------------------------------------------------- anime sakura
def make_anime_sakura():
    sky = vgrad([(0, (255, 196, 190)), (0.45, (255, 218, 190)), (0.8, (255, 170, 160)), (1, (200, 120, 150))])
    rnd = random.Random(44)
    back = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    bd = ImageDraw.Draw(back)
    bd.ellipse([760, 110, 1060, 410], fill=(255, 244, 214, 255))
    for i, col in enumerate([(236, 150, 160, 255), (198, 118, 150, 255), (150, 92, 130, 255)]):
        base = 470 + i * 70
        pts = [(0, H)] + [(x, base - 70 * math.sin(x / (300 - i * 40) + i * 1.9) - 22 * math.sin(x / 80 + i)) for x in range(0, W + 20, 20)] + [(W, H)]
        bd.polygon(pts, fill=col)
    # A blossoming branch in the corner.
    tree = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    td = ImageDraw.Draw(tree)
    td.line([(W + 20, 30), (900, 90), (740, 70), (610, 130)], fill=(70, 42, 56, 255), width=14)
    td.line([(900, 90), (800, 190)], fill=(70, 42, 56, 255), width=9)
    for _ in range(260):
        bx, by = rnd.gauss(800, 150), rnd.gauss(110, 55)
        r = rnd.uniform(9, 22)
        td.ellipse([bx - r, by - r, bx + r, by + r], fill=(255, rnd.randint(176, 208), rnd.randint(196, 220), rnd.randint(190, 255)))
    petals = [(rnd.randint(0, W), rnd.randint(0, H), rnd.choice([1, 2, 2, 3]), rnd.uniform(6, 15), rnd.uniform(0, TAU), rnd.randint(1, 3), rnd.uniform(20, 70)) for _ in range(90)]

    def frame(t):
        img = Image.alpha_composite(sky.convert("RGBA"), back)
        img = Image.alpha_composite(img, tree)
        pl = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        d = ImageDraw.Draw(pl)
        for x0, y0, k, s, ph, spin, sway in petals:
            y = (y0 + k * H * t) % H
            x = x0 + sway * math.sin(TAU * k * t * 1.0 + ph) + 24 * math.sin(TAU * t + ph)
            a = ph + TAU * spin * t
            pts = [(x + s * math.cos(a + q) * (1 if i % 2 == 0 else 0.55), y + s * 0.6 * math.sin(a + q) * (1 if i % 2 == 0 else 0.55)) for i, q in enumerate([0, 1.2, 2.4, 3.6, 4.8])]
            for off in (0, -H, H):
                d.polygon([(px, py + off) for px, py in pts], fill=(255, 205, 220, 230))
        img = Image.alpha_composite(img, pl)
        return img.convert("RGB")
    return frame


SCENES = {
    "rain": make_rain, "rainy-window": make_rainy_window, "foggy-window": make_foggy_window,
    "anime-night": make_anime_night, "anime-sakura": make_anime_sakura,
}


def render(name):
    OUT.mkdir(parents=True, exist_ok=True)
    frame = SCENES[name]()
    mp4 = OUT / f"{name}.mp4"
    ff = subprocess.Popen([
        "ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-",
        "-an", "-c:v", "libx264", "-preset", "slow", "-crf", "27", "-pix_fmt", "yuv420p", "-profile:v", "main", "-level", "4.0",
        "-movflags", "+faststart", str(mp4)], stdin=subprocess.PIPE)
    for i in range(N):
        im = frame(i / N)
        if i == 0:
            im.save(OUT / f"{name}.jpg", quality=84, optimize=True)
        ff.stdin.write(np.asarray(im.convert("RGB"), dtype=np.uint8).tobytes())
    ff.stdin.close()
    ff.wait()
    print(name, mp4.stat().st_size // 1024, "KB")


if __name__ == "__main__":
    for n in (sys.argv[1:] or list(SCENES)):
        render(n)
