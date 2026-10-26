#!/usr/bin/env python3
"""Generate the bundled theme wallpapers (original procedural art, released with this repo under MIT).

    python3 tools/make_wallpapers.py      -> public/themes/<id>.jpg (1920x1080)

Deterministic: the same script always draws the same pictures.
"""
import math
import random
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

W, H = 1920, 1080
OUT = Path(__file__).resolve().parent.parent / "public" / "themes"


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def vgrad(top, bottom, stops=None):
    img = Image.new("RGB", (W, H))
    d = ImageDraw.Draw(img)
    for y in range(H):
        d.line([(0, y), (W, y)], fill=lerp(top, bottom, y / (H - 1)))
    return img


def glow(img, cx, cy, r, color, alpha=160, blur=180):
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    ImageDraw.Draw(layer).ellipse([cx - r, cy - r, cx + r, cy + r], fill=color + (alpha,))
    layer = layer.filter(ImageFilter.GaussianBlur(blur))
    return Image.alpha_composite(img.convert("RGBA"), layer).convert("RGB")


def vignette(img, strength=0.55):
    mask = Image.new("L", img.size, 0)
    ImageDraw.Draw(mask).ellipse([-W * 0.25, -H * 0.35, W * 1.25, H * 1.35], fill=255)
    mask = mask.filter(ImageFilter.GaussianBlur(220))
    dark = Image.new("RGB", img.size, (0, 0, 0))
    return Image.composite(img, Image.blend(img, dark, strength), mask)


def bottom_shade(img, amount=0.6):
    shade = Image.new("L", img.size, 0)
    d = ImageDraw.Draw(shade)
    for y in range(H):
        t = max(0, (y - H * 0.45) / (H * 0.55))
        d.line([(0, y), (W, y)], fill=int(255 * amount * t))
    return Image.composite(Image.new("RGB", img.size, (0, 0, 0)), img, shade)


def aurora():
    img = vgrad((6, 10, 26), (3, 5, 12))
    rnd = random.Random(4)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for band, col in enumerate([(40, 220, 170), (90, 120, 255), (170, 80, 230)]):
        base = 300 + band * 90
        for x in range(0, W, 3):
            y = base + 120 * math.sin(x / 260 + band * 1.7) + 40 * math.sin(x / 90 + band)
            h = 160 + 60 * math.sin(x / 150 + band * 2)
            d.line([(x, y), (x, y + h)], fill=col + (80,), width=4)
    layer = layer.filter(ImageFilter.GaussianBlur(28))
    img = Image.alpha_composite(img.convert("RGBA"), layer).convert("RGB")
    d = ImageDraw.Draw(img)
    for _ in range(260):
        x, y = rnd.randrange(W), rnd.randrange(int(H * 0.55))
        v = rnd.randrange(120, 255)
        d.point((x, y), fill=(v, v, v))
    return bottom_shade(img, 0.7)


def synthwave():
    img = vgrad((20, 4, 40), (70, 10, 70))
    img = glow(img, W // 2, int(H * 0.52), 420, (255, 70, 150), 150, 160)
    d = ImageDraw.Draw(img)
    cx, cy, r = W // 2, int(H * 0.5), 260
    for i in range(r * 2):
        y = cy - r + i
        if y > cy + 20 and (y // 18) % 2 == 0:
            continue
        half = int(math.sqrt(max(0, r * r - (y - cy) ** 2)))
        d.line([(cx - half, y), (cx + half, y)], fill=lerp((255, 220, 90), (255, 60, 140), i / (2 * r)))
    horizon = int(H * 0.62)
    d.rectangle([0, horizon, W, H], fill=(12, 2, 28))
    for i in range(1, 22):
        y = horizon + int((i / 22) ** 2.2 * (H - horizon))
        d.line([(0, y), (W, y)], fill=(255, 60, 200), width=2)
    for i in range(-24, 25):
        d.line([(cx + i * 22, horizon), (cx + i * 260, H)], fill=(255, 60, 200), width=2)
    return bottom_shade(img, 0.45)


def neon():
    img = vgrad((4, 4, 14), (10, 0, 24))
    img = glow(img, int(W * 0.2), int(H * 0.3), 380, (0, 230, 255), 120)
    img = glow(img, int(W * 0.82), int(H * 0.25), 420, (255, 0, 170), 130)
    d = ImageDraw.Draw(img, "RGBA")
    for x in range(0, W, 80):
        d.line([(x, 0), (x, H)], fill=(0, 220, 255, 22))
    for y in range(0, H, 80):
        d.line([(0, y), (W, y)], fill=(255, 0, 200, 18))
    return bottom_shade(img, 0.65)


def frosted():
    img = vgrad((40, 60, 90), (20, 24, 40))
    rnd = random.Random(7)
    for col in [(255, 160, 120), (120, 190, 255), (200, 140, 255), (120, 240, 210)]:
        img = glow(img, rnd.randrange(W), rnd.randrange(H), rnd.randrange(260, 420), col, 150, 200)
    return bottom_shade(img, 0.4)


def nature():
    img = vgrad((255, 196, 140), (110, 150, 190))
    img = glow(img, int(W * 0.7), int(H * 0.38), 160, (255, 240, 200), 220, 60)
    d = ImageDraw.Draw(img)
    rnd = random.Random(11)
    for layer, col in enumerate([(120, 120, 150), (80, 90, 110), (45, 60, 70), (22, 32, 34)]):
        base = int(H * (0.5 + layer * 0.1))
        pts = [(0, H)]
        for x in range(0, W + 40, 40):
            pts.append((x, base - abs(math.sin(x / (300 - layer * 40) + layer)) * (180 - layer * 30) - rnd.randrange(20)))
        pts.append((W, H))
        d.polygon(pts, fill=col)
    return bottom_shade(img, 0.55)


def gradient_modern():
    img = Image.new("RGB", (W, H))
    px = img.load()
    a, b, c = (255, 94, 98), (104, 72, 255), (20, 210, 200)
    for y in range(0, H):
        for x in range(0, W):
            t = (x / W * 0.7 + y / H * 0.3)
            px[x, y] = lerp(a, b, t * 1.4) if t < 0.7 else lerp(b, c, (t - 0.7) / 0.3)
    return bottom_shade(img.filter(ImageFilter.GaussianBlur(4)), 0.55)


def luxury():
    img = vgrad((14, 12, 10), (2, 2, 2))
    img = glow(img, int(W * 0.5), int(-H * 0.1), 700, (190, 150, 80), 90, 260)
    d = ImageDraw.Draw(img, "RGBA")
    for i in range(14):
        y = int(H * 0.15) + i * 9
        d.line([(int(W * 0.35), y), (int(W * 0.65), y)], fill=(212, 175, 95, 10))
    return vignette(img, 0.4)


def monochrome():
    img = vgrad((58, 58, 58), (8, 8, 8))
    img = glow(img, int(W * 0.3), int(H * 0.2), 500, (200, 200, 200), 50, 240)
    return vignette(img, 0.5)


def gaming():
    img = vgrad((8, 6, 10), (2, 2, 4))
    img = glow(img, int(W * 0.15), int(H * 0.9), 500, (255, 30, 60), 130, 220)
    img = glow(img, int(W * 0.9), int(H * 0.1), 420, (40, 255, 120), 90, 220)
    d = ImageDraw.Draw(img, "RGBA")
    for i in range(-10, 30):
        d.line([(i * 110, 0), (i * 110 + 700, H)], fill=(255, 255, 255, 8), width=2)
    return bottom_shade(img, 0.5)


def family():
    img = vgrad((255, 222, 196), (190, 220, 255))
    rnd = random.Random(3)
    for col in [(255, 170, 190), (170, 230, 200), (255, 230, 150), (180, 200, 255)]:
        img = glow(img, rnd.randrange(W), rnd.randrange(H), rnd.randrange(240, 380), col, 170, 160)
    return bottom_shade(img, 0.55)


def cinematic():
    img = vgrad((18, 22, 30), (4, 4, 6))
    img = glow(img, int(W * 0.72), int(H * 0.3), 520, (90, 120, 160), 90, 260)
    return vignette(bottom_shade(img, 0.6), 0.45)


def premium():
    img = vgrad((16, 18, 24), (0, 0, 0))
    img = glow(img, int(W * 0.5), int(H * 1.05), 700, (60, 70, 100), 110, 260)
    return img


def material():
    img = Image.new("RGB", (W, H), (18, 18, 18))
    d = ImageDraw.Draw(img)
    d.ellipse([int(W * 0.62), -300, int(W * 1.25), int(H * 0.75)], fill=(98, 0, 238))
    d.ellipse([-260, int(H * 0.55), int(W * 0.35), int(H * 1.5)], fill=(3, 218, 197))
    img = img.filter(ImageFilter.GaussianBlur(6))
    return bottom_shade(img, 0.55)


WALLPAPERS = {
    "cinematic-minimal": cinematic, "premium": premium, "material": material, "cyberpunk-neon": neon,
    "retro-futuristic": synthwave, "frosted-glass": frosted, "aurora": aurora, "luxury-dark": luxury,
    "monochrome": monochrome, "gaming": gaming, "family": family, "nature": nature, "gradient-modern": gradient_modern,
}

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in WALLPAPERS.items():
        fn().save(OUT / f"{name}.jpg", quality=84, optimize=True, progressive=True)
        print(name, (OUT / f"{name}.jpg").stat().st_size // 1024, "KB")
