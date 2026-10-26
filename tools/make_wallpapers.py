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


def dunes():
    img = vgrad((255, 150, 90), (60, 28, 60))
    img = glow(img, int(W * 0.7), int(H * 0.38), 260, (255, 226, 160), 235, 90)
    d = ImageDraw.Draw(img)
    for layer, col in enumerate([(176, 80, 70), (130, 56, 66), (84, 38, 62), (40, 22, 46)]):
        base = 560 + layer * 120
        pts = [(0, H)]
        for x in range(0, W + 20, 20):
            y = base + 70 * math.sin(x / (380 - layer * 50) + layer * 1.7) + 26 * math.sin(x / 130 + layer)
            pts.append((x, y))
        pts.append((W, H))
        d.polygon(pts, fill=col)
    return vignette(img, 0.35)


def ocean():
    img = vgrad((10, 70, 120), (2, 12, 34))
    img = glow(img, int(W * 0.3), int(H * 0.1), 650, (70, 190, 230), 130, 280)
    shafts = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shafts)
    for i in range(7):
        x = 200 + i * 230
        sd.polygon([(x, 0), (x + 90, 0), (x + 330, H), (x + 120, H)], fill=(150, 230, 255, 26))
    img = Image.alpha_composite(img.convert("RGBA"), shafts.filter(ImageFilter.GaussianBlur(30))).convert("RGB")
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for i in range(16):
        y0 = 300 + i * 48
        pts = [(x, y0 + 26 * math.sin(x / (170 + i * 11) + i * 0.9)) for x in range(0, W + 12, 12)]
        d.line(pts, fill=(140, 220, 250, 50 + i * 6), width=4)
    layer = layer.filter(ImageFilter.GaussianBlur(1.5))
    img = Image.alpha_composite(img.convert("RGBA"), layer).convert("RGB")
    return vignette(bottom_shade(img, 0.45), 0.4)


def city():
    img = vgrad((14, 12, 40), (60, 24, 70))
    img = glow(img, int(W * 0.5), int(H * 0.72), 700, (255, 110, 140), 70, 260)
    d = ImageDraw.Draw(img)
    rnd = random.Random(11)
    for layer, (col, lit) in enumerate([((30, 22, 56), 0.10), ((16, 12, 34), 0.18), ((6, 5, 16), 0.28)]):
        x = -20
        while x < W:
            bw = rnd.randint(70, 170)
            bh = rnd.randint(160, 420) + layer * 70
            top = H - bh
            d.rectangle([x, top, x + bw, H], fill=col)
            for wy in range(top + 18, H - 20, 26):
                for wx in range(x + 12, x + bw - 14, 22):
                    if rnd.random() < lit:
                        d.rectangle([wx, wy, wx + 8, wy + 12], fill=(255, 214, 140))
            x += bw + rnd.randint(0, 14)
    return vignette(img, 0.4)


def forest():
    img = vgrad((200, 222, 208), (30, 62, 52))
    img = glow(img, int(W * 0.72), int(H * 0.18), 320, (255, 248, 210), 170, 140)
    rnd = random.Random(5)
    for layer, col in enumerate([(130, 170, 148), (90, 136, 112), (52, 100, 80), (26, 62, 50), (12, 38, 31)]):
        d = ImageDraw.Draw(img)
        base = 420 + layer * 120
        pts = [(0, H)]
        for x in range(0, W + 20, 20):
            pts.append((x, base - 90 * math.sin(x / (330 - layer * 30) + layer * 2.1) - 30 * math.sin(x / 95 + layer)))
        pts.append((W, H))
        d.polygon(pts, fill=col)
        for _ in range(7 + layer * 2):
            x = rnd.randint(0, W)
            y = base - 90 * math.sin(x / (330 - layer * 30) + layer * 2.1) - 30 * math.sin(x / 95 + layer)
            h, w = rnd.randint(90, 150) + layer * 20, 36 + layer * 8
            d.polygon([(x - w, y + 6), (x, y - h), (x + w, y + 6)], fill=col)
        fog = Image.new("RGBA", (W, H), (0, 0, 0, 0))
        ImageDraw.Draw(fog).rectangle([0, base - 60, W, base + 60], fill=(225, 242, 232, 85))
        img = Image.alpha_composite(img.convert("RGBA"), fog.filter(ImageFilter.GaussianBlur(45))).convert("RGB")
    return vignette(bottom_shade(img, 0.4), 0.3)


def sakura():
    img = vgrad((255, 226, 232), (236, 170, 196))
    img = glow(img, int(W * 0.2), int(H * 0.2), 420, (255, 255, 255), 150, 200)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    rnd = random.Random(8)
    for _ in range(170):
        x, y, r = rnd.randint(0, W), rnd.randint(0, H), rnd.randint(8, 26)
        a = rnd.randint(90, 200)
        ang = rnd.random() * 3.14
        for k in range(5):
            t = ang + k * 6.283 / 5
            cx, cy = x + math.cos(t) * r * 0.55, y + math.sin(t) * r * 0.55
            d.ellipse([cx - r * 0.5, cy - r * 0.5, cx + r * 0.5, cy + r * 0.5], fill=(255, 190 + rnd.randint(0, 40), 210, a))
    layer = layer.filter(ImageFilter.GaussianBlur(1.5))
    return Image.alpha_composite(img.convert("RGBA"), layer).convert("RGB")


def nebula():
    img = vgrad((8, 6, 22), (2, 2, 10))
    for cx, cy, r, col, a in [(0.3, 0.4, 560, (120, 60, 220), 150), (0.7, 0.55, 520, (40, 120, 230), 130), (0.55, 0.25, 380, (230, 70, 160), 110)]:
        img = glow(img, int(W * cx), int(H * cy), r, col, a, 230)
    d = ImageDraw.Draw(img)
    rnd = random.Random(21)
    for _ in range(420):
        x, y = rnd.randint(0, W), rnd.randint(0, H)
        c = rnd.randint(150, 255)
        d.point((x, y), fill=(c, c, 255))
        if rnd.random() < 0.06:
            d.ellipse([x - 1, y - 1, x + 1, y + 1], fill=(255, 255, 255))
    return vignette(img, 0.45)


def ember():
    img = vgrad((10, 6, 6), (46, 14, 6))
    img = glow(img, int(W * 0.5), int(H * 1.1), 900, (255, 110, 30), 150, 280)
    layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    rnd = random.Random(2)
    for _ in range(140):
        x = rnd.randint(0, W)
        y = int(H - (rnd.random() ** 1.6) * H * 0.95)
        r = rnd.randint(1, 4)
        d.ellipse([x - r, y - r, x + r, y + r], fill=(255, rnd.randint(120, 200), 60, rnd.randint(120, 255)))
    layer = layer.filter(ImageFilter.GaussianBlur(1.2))
    return vignette(Image.alpha_composite(img.convert("RGBA"), layer).convert("RGB"), 0.4)


def arctic():
    img = vgrad((232, 242, 250), (150, 190, 220))
    img = glow(img, int(W * 0.75), int(H * 0.1), 500, (255, 255, 255), 200, 220)
    d = ImageDraw.Draw(img)
    for layer, col in enumerate([(200, 222, 240), (170, 202, 228), (132, 172, 208)]):
        base = 600 + layer * 120
        pts = [(0, H)]
        for x in range(0, W + 30, 30):
            y = base - 150 * abs(math.sin(x / (260 + layer * 40) + layer)) - 40 * math.sin(x / 90)
            pts.append((x, y))
        pts.append((W, H))
        d.polygon(pts, fill=col)
    return bottom_shade(img, 0.25)


def terminal():
    img = Image.new("RGB", (W, H), (2, 10, 4))
    img = glow(img, W // 2, H // 2, 800, (20, 120, 50), 70, 300)
    d = ImageDraw.Draw(img)
    for y in range(0, H, 4):
        d.line([(0, y), (W, y)], fill=(0, 0, 0))
    rnd = random.Random(3)
    for col in range(0, W, 28):
        n = rnd.randint(0, 22)
        top = rnd.randint(0, H)
        for k in range(n):
            y = (top + k * 22) % H
            d.text((col, y), rnd.choice("01ABCDEF"), fill=(20, 90 + k * 6, 40))
    return vignette(img, 0.5)


def kids():
    img = vgrad((120, 200, 255), (200, 235, 255))
    d = ImageDraw.Draw(img)
    for cx, cy, r, col in [(0.12, 0.25, 150, (255, 205, 60)), (0.86, 0.2, 110, (255, 120, 140)), (0.78, 0.8, 170, (110, 220, 150)), (0.2, 0.85, 120, (160, 130, 255)), (0.5, 0.12, 80, (255, 160, 90))]:
        x, y = int(W * cx), int(H * cy)
        d.ellipse([x - r, y - r, x + r, y + r], fill=col)
    for i in range(7):
        x = 160 + i * 270
        d.rectangle([x, H - 150, x + 160, H], fill=[(255, 120, 120), (255, 200, 80), (120, 220, 140), (110, 170, 255), (190, 140, 255), (255, 150, 200), (90, 210, 220)][i])
    return img.filter(ImageFilter.GaussianBlur(2))


WALLPAPERS = {
    "cinematic-minimal": cinematic, "premium": premium, "material": material, "cyberpunk-neon": neon,
    "retro-futuristic": synthwave, "frosted-glass": frosted, "aurora": aurora, "luxury-dark": luxury,
    "monochrome": monochrome, "gaming": gaming, "family": family, "nature": nature, "gradient-modern": gradient_modern,
    "sunset-dunes": dunes, "ocean-deep": ocean, "midnight-city": city, "forest-mist": forest, "sakura": sakura,
    "space-nebula": nebula, "ember": ember, "arctic": arctic, "retro-terminal": terminal, "kids-bright": kids,
}

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in WALLPAPERS.items():
        fn().save(OUT / f"{name}.jpg", quality=84, optimize=True, progressive=True)
        print(name, (OUT / f"{name}.jpg").stat().st_size // 1024, "KB")
