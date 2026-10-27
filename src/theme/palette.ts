import type { Layout, LayoutItem, SceneId } from "./layout";

/** Our own "colours from your wallpaper": the most vivid common hue in the picture, lifted so it reads on a TV. */

const SCENE_ACCENT: Record<SceneId, string> = {
  rain: "#9ecbff", snow: "#cfe3ff", aurora: "#34f5a0", fireflies: "#ffe680", space: "#9fb4ff", sakura: "#f8b7cd", embers: "#ff9a3d", underwater: "#6fe0ff",
};

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b), l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min, s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h * 60, s, l];
}

function hslToHex(h: number, s: number, l: number): string {
  const k = (n: number) => (n + h / 30) % 12, a = s * Math.min(l, 1 - l);
  const f = (n: number) => Math.round(255 * (l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1))));
  return `#${[f(0), f(8), f(4)].map((x) => x.toString(16).padStart(2, "0")).join("")}`;
}

/** RGBA pixels → an accent colour: 12 hue buckets scored by how many pixels and how vivid; grey pictures give white. */
export function pickAccent(px: Uint8ClampedArray | number[]): string {
  const score = new Array(12).fill(0), hues = new Array(12).fill(0), sats = new Array(12).fill(0);
  for (let i = 0; i + 3 < px.length; i += 4) {
    const [h, s, l] = rgbToHsl(px[i]!, px[i + 1]!, px[i + 2]!);
    if (s < 0.22 || l < 0.12 || l > 0.92) continue;
    const b = Math.floor(h / 30) % 12, w = s * (1 - Math.abs(l - 0.5));
    score[b] += w; hues[b] += h * w; sats[b] += s * w;
  }
  let best = -1;
  for (let b = 0; b < 12; b++) if (score[b] > 0 && (best < 0 || score[b] > score[best])) best = b;
  if (best < 0) return "#ffffff";
  const h = hues[best] / score[best], s = Math.max(0.55, Math.min(0.9, sats[best] / score[best]));
  return hslToHex(h, s, 0.68);   // light enough to read over a dark TV background
}

/** Decode a picture in the browser and pick its accent; null when it can't be read (or no canvas, as in tests). */
export async function accentFromImage(url: string): Promise<string | null> {
  const img = new Image();
  img.crossOrigin = "anonymous";
  img.src = url;
  try { await img.decode(); } catch { return null; }
  const c = document.createElement("canvas");
  c.width = 64; c.height = 36;
  const ctx = c.getContext("2d");
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, 64, 36);
  return pickAccent(ctx.getImageData(0, 0, 64, 36).data);
}

/** The accent for a layout's background: scenes have their own, colours and gradients use their colours. */
export async function accentForBackground(l: Layout, wallpaperUrl: string | null, sampleBase: string): Promise<string | null> {
  const bg = l.background;
  if (bg.kind === "scene") return SCENE_ACCENT[bg.scene ?? "rain"];
  if (bg.kind === "color" || bg.kind === "gradient") {
    const hex = (c: string) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));
    const a = hex(bg.color), b = hex(bg.color2);
    return pickAccent([...a, 255, ...b, 255, ...b, 255]);
  }
  if (bg.kind === "video" && bg.video && !bg.video.includes(".")) return accentFromImage(`${sampleBase}/${bg.video}.jpg`);
  return wallpaperUrl ? accentFromImage(wallpaperUrl) : null;
}

/** Kinds whose main colour follows the accent when the person asks to colour the widgets too. */
const ACCENTED: LayoutItem["kind"][] = ["clock", "cpu", "memory", "network", "visualizer", "calendar"];
export function colourWidgets(l: Layout, accent: string): Layout {
  return { ...l, accent, items: l.items.map((i) => (ACCENTED.includes(i.kind) ? { ...i, style: { ...i.style, color: accent } } : i)) };
}
