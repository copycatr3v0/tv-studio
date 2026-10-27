import { z } from "zod";

/** Free-canvas home screen layout. Coordinates are percentages of the screen, so one layout fits 720p, 1080p and 4K. */
export const KINDS = ["apps", "recent", "watchnext", "clock", "date", "greeting", "text", "cpu", "memory", "network", "calendar", "visualizer", "shape", "bluetooth", "nowplaying", "weather"] as const;
export type ItemKind = (typeof KINDS)[number];

export const KIND_LABEL: Record<ItemKind, string> = {
  apps: "App tiles", recent: "Recently opened", watchnext: "Continue watching", clock: "Clock", date: "Date",
  greeting: "Greeting", text: "Custom text", cpu: "CPU use", memory: "Memory use", network: "Network speed",
  calendar: "Month calendar", visualizer: "Animated bars", shape: "Shape / panel / line",
  bluetooth: "Bluetooth status", nowplaying: "Now playing", weather: "Weather",
};

/** The looks each kind can take (`variant`). The first is the default when none is set. */
export const VARIANTS: Partial<Record<ItemKind, readonly string[]>> = {
  clock: ["digital", "stacked", "analog"],
  date: ["weekday_date", "weekday", "full", "short", "daymonth"],
  cpu: ["text", "bar", "ring", "graph"], memory: ["text", "bar", "ring", "graph"], network: ["text", "bar", "ring", "graph"],
  visualizer: ["bars", "mirror", "dots"],
  weather: ["compact", "full", "forecast"],
  shape: ["rect", "circle", "line", "ring"],
};
export const VARIANT_LABEL: Record<string, string> = {
  digital: "Digital", stacked: "Stacked (hours over minutes)", analog: "Analog", weekday_date: "Sunday, 28 January", weekday: "Sunday", full: "28 January, 2024",
  short: "Sun", daymonth: "28 Jan", text: "Text", bar: "Bar", ring: "Ring", graph: "Graph", bars: "Bars", mirror: "Bars with reflection", dots: "Dots",
  rect: "Rectangle / panel", circle: "Circle", line: "Line",
};

/** Custom font files the person adds: "file:<name>.ttf" or ".otf" in a layout's font setting. */
export const FONT_FILE = /^file:[A-Za-z0-9][A-Za-z0-9_.-]{0,58}\.(ttf|otf)$/;
export const isFontFile = (f: string) => FONT_FILE.test(f);

/** Android's built-in font families: nothing to ship, nothing to download. */
export const FONTS = ["sans-serif", "sans-serif-light", "sans-serif-thin", "sans-serif-condensed", "sans-serif-medium", "sans-serif-black", "serif", "monospace", "cursive", "casual"] as const;
export const FONT_CSS: Record<(typeof FONTS)[number], string> = {
  "sans-serif": "system-ui, sans-serif", "sans-serif-light": "system-ui, sans-serif", "sans-serif-thin": "system-ui, sans-serif",
  "sans-serif-condensed": "'Arial Narrow', system-ui, sans-serif", "sans-serif-medium": "system-ui, sans-serif", "sans-serif-black": "system-ui, sans-serif",
  serif: "Georgia, serif", monospace: "ui-monospace, Consolas, monospace", cursive: "'Segoe Script', cursive", casual: "'Comic Sans MS', 'Segoe Print', cursive",
};
export const FONT_WEIGHT: Record<(typeof FONTS)[number], number> = {
  "sans-serif": 400, "sans-serif-light": 300, "sans-serif-thin": 200, "sans-serif-condensed": 400, "sans-serif-medium": 500, "sans-serif-black": 900,
  serif: 400, monospace: 400, cursive: 400, casual: 400,
};

export const MOTIONS = ["none", "float", "pulse", "slide", "fade", "drift", "parallax"] as const;
export const SHAPES = ["rounded", "square", "circle"] as const;
export const ALIGN = ["start", "center", "end"] as const;

const color = z.string().regex(/^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/);
const pkgName = z.string().regex(/^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$/).max(200);
const pct = z.number().min(0).max(100);

export const ItemSchema = z.object({
  id: z.string().regex(/^[a-z0-9-]{1,32}$/),
  kind: z.enum(KINDS),
  enabled: z.boolean(),
  x: pct, y: pct, w: z.number().min(3).max(100), h: z.number().min(3).max(100),
  text: z.string().max(80).optional(),
  variant: z.string().max(16).optional(),
  style: z.object({
    font: z.union([z.enum(FONTS), z.string().regex(FONT_FILE)]), size: z.number().min(8).max(400), color, opacity: z.number().min(0.05).max(1),
    radius: z.number().min(0).max(80), fill: color.nullable(), align: z.enum(ALIGN),
    /** Extra space between letters, in em (0.3 gives the spaced-out capitals of many desktop widgets). */
    spacing: z.number().min(0).max(1).optional(), upper: z.boolean().optional(), shadow: z.boolean().optional(), italic: z.boolean().optional(),
  }),
  icon: z.object({ size: z.number().min(24).max(256), shape: z.enum(SHAPES), labels: z.boolean(), columns: z.number().int().min(1).max(12) }).optional(),
  motion: z.object({ type: z.enum(MOTIONS), speed: z.number().min(0.25).max(4), amount: z.number().min(0).max(100) }),
  options: z.object({
    max: z.number().int().min(1).max(20), twentyFour: z.boolean(), seconds: z.boolean(), compact: z.boolean(),
    /** App tiles: only these apps, in this order. */
    packages: z.array(z.string().regex(/^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$/)).max(24),
    bars: z.number().int().min(8).max(96), stroke: z.number().min(0).max(40), label: z.boolean(), weekStartsMonday: z.boolean(),
  }).partial().optional(),
});
export type LayoutItem = z.infer<typeof ItemSchema>;

export const SCENES = ["rain", "snow", "aurora", "fireflies", "space", "sakura", "embers", "underwater"] as const;
export type SceneId = (typeof SCENES)[number];
export const SCENE_LABEL: Record<SceneId, string> = {
  rain: "Rain", snow: "Snowfall", aurora: "Northern lights", fireflies: "Fireflies", space: "Deep space", sakura: "Cherry blossom", embers: "Embers", underwater: "Underwater",
};
export const SCENE_TIMES = ["auto", "sunrise", "day", "sunset", "night"] as const;

export const BackgroundSchema = z.object({
  kind: z.enum(["image", "video", "color", "gradient", "scene"]),
  /** Ambient scene drawn live on the TV, and "auto" (follows the clock) or a fixed part of the day. */
  scene: z.enum(SCENES).optional(),
  sceneTime: z.enum(SCENE_TIMES).optional(),
  /** With weather on: the scene turns to rain or snow when it rains or snows there. */
  followWeather: z.boolean().optional(),
  color: color, color2: color,
  /** Sample id ("rain") or the name of a video the person added. */
  video: z.string().regex(/^([a-z0-9-]{1,32}|[A-Za-z0-9][A-Za-z0-9_.-]{0,58}\.(mp4|m4v|webm|mkv|mov|3gp|ts))$/).nullable(),
  dim: z.number().min(0).max(0.9), blur: z.number().min(0).max(24),
});
export type Background = z.infer<typeof BackgroundSchema>;

export const LayoutSchema = z.object({
  version: z.literal(1),
  enabled: z.boolean(),
  /** "classic" (default when missing): the launcher's own Home keeps the apps; "canvas": free app strips. */
  home: z.enum(["classic", "canvas", "scroll", "tvos"]).optional(),
  /** Also use this canvas (background and widgets, no apps) as the TV's screensaver. */
  screensaver: z.boolean().optional(),
  /** Keep moving backgrounds and animations on even on a TV with little memory (Studio warns first). */
  heavyOnLowRam: z.boolean().optional(),
  /** Scroll-mode tiles: focus style, app-open animation, tile style, icon shape, accent colour (focus ring, names). */
  focus: z.enum(["pop", "lift", "glow", "border", "bounce", "tilt", "zoom", "parallax"]).optional(),
  launch: z.enum(["default", "zoom", "reveal", "fade", "slide", "grow", "none"]).optional(),
  tiles: z.enum(["banner", "card", "icon", "poster", "tvos"]).optional(),
  iconShape: z.enum(["rounded", "circle", "squircle", "square", "leaf"]).optional(),
  accent: color.nullable().optional(),
  /** The app Home opens once after the TV starts (an app picked on the TV itself wins). */
  openOnStart: pkgName.nullable().optional(),
  /** Weather from Open-Meteo for one city. Nothing is fetched until consent is true (the TV then contacts Open-Meteo). */
  weather: z.object({ city: z.string().min(1).max(60), units: z.enum(["c", "f"]).optional(), consent: z.boolean().optional() }).optional(),
  /** Scroll mode: up to 8 rows, folders at the start of All apps, hidden apps, and per-app name/colour/picture. */
  rows: z.array(z.object({
    title: z.string().min(1).max(40),
    source: z.enum(["favorites", "recent", "games", "video", "music", "news", "social", "tools"]),
    packages: z.array(pkgName).max(24).optional(),
    max: z.number().int().min(1).max(24).optional(),
  })).max(8).optional(),
  folders: z.array(z.object({ name: z.string().min(1).max(30), packages: z.array(pkgName).max(24) })).max(8).optional(),
  hidden: z.array(pkgName).max(100).optional(),
  /** Apps that ask for the TV's PIN before opening from Home (the PIN is chosen on the TV). */
  locked: z.array(pkgName).max(50).optional(),
  /** Family: this layout is the main profile; up to 3 more, each its own layout (profiles/<id>.json on the TV). */
  profileName: z.string().min(1).max(20).optional(),
  profileColor: color.optional(),
  profiles: z.array(z.object({ id: z.string().regex(/^[a-z0-9-]{1,16}$/), name: z.string().min(1).max(20), color: color.optional(), pin: z.boolean().optional() })).max(3).optional(),
  apps: z.record(pkgName, z.object({
    name: z.string().min(1).max(40).nullable().optional(),
    color: color.nullable().optional(),
    image: z.string().regex(/^[A-Za-z0-9_-]{1,60}\.(jpg|png|webp)$/).nullable().optional(),
  })).optional(),
  background: BackgroundSchema,
  items: z.array(ItemSchema).max(40),
});
export type Layout = z.infer<typeof LayoutSchema>;

export const VIDEO_SAMPLES = [
  { id: "rain", name: "Rain", blurb: "Heavy rain over a dark city." },
  { id: "rainy-window", name: "Rainy window", blurb: "Drops sliding down glass, city lights behind." },
  { id: "foggy-window", name: "Foggy window", blurb: "Mist on glass, streaks wiped clear by drops." },
  { id: "anime-night", name: "Anime night", blurb: "Moon, drifting clouds, a city with lit windows." },
  { id: "anime-sakura", name: "Anime sakura", blurb: "Falling cherry blossom petals at sunset." },
] as const;

const base = { enabled: true, motion: { type: "none" as const, speed: 1, amount: 20 } };
const text = (font: LayoutItem["style"]["font"] = "sans-serif", size = 32) =>
  ({ font, size, color: "#ffffff", opacity: 1, radius: 0, fill: null, align: "start" as const });

/** What in this layout costs a TV real CPU and memory: switched off on low-RAM TVs unless heavyOnLowRam is set. */
export function heavyFeatures(l: Layout, animatedWallpaper = false): string[] {
  const out: string[] = [];
  const bg = l.background;
  if (bg.kind === "scene") out.push("ambient scene");
  if (bg.kind === "video") out.push("video background");
  if (bg.kind === "image" && animatedWallpaper) out.push("animated picture");
  const on = l.items.filter((i) => i.enabled);
  if (on.some((i) => i.kind === "visualizer")) out.push("visualiser");
  if (on.some((i) => i.motion && i.motion.type !== "none" && i.motion.amount > 0)) out.push("moving widgets");
  return out;
}
/** A TV with less memory than this (about 1.75 GB) counts as low-RAM, as the launcher decides (LowRam.java). */
export const LOW_RAM_KB = 1_709_000;

/** Kinds that take D-pad focus: in classic mode the launcher's own Home shows the apps instead. */
export const APP_KINDS: readonly ItemKind[] = ["apps", "recent", "watchnext"];
export const isClassic = (l: Layout) => (l.home ?? "classic") === "classic";
/** Scroll and Apple TV style both put the widgets first and every app on Down. */
export const isScroll = (l: Layout) => l.home === "scroll" || l.home === "tvos";
export const isTvos = (l: Layout) => l.home === "tvos";

/** One click to the set-top-box look: Top Shelf, a glass dock of five wide rounded icons, a five-across grid,
 *  parallax focus with a shine, the clock top-right. Keeps the person's background, widgets and app choices. */
export function withAppleTvLook(l: Layout): Layout {
  const base = withScrollParts(l);
  const items = base.items.map((i) => {
    if (i.kind === "recent") return { ...i, enabled: true, x: 2, y: 63, w: 96, h: 27, style: { ...i.style, font: "sans-serif-medium", size: 22, color: "#ffffff" } };
    if (i.kind === "apps") return { ...i, enabled: true, icon: { ...(i.icon ?? { size: 96, shape: "rounded" as const, labels: true }), columns: 5 }, style: { ...i.style, font: "sans-serif-medium", size: 22 } };
    if (i.kind === "clock") return { ...i, enabled: true, x: 54, y: 3, w: 22, h: 7, style: { ...i.style, font: "sans-serif-medium", size: 44, align: "end" as const } };
    if (i.kind === "greeting" || i.kind === "date") return { ...i, enabled: false };
    return i;
  });
  const zones = [{ x: 2, y: 63, w: 96, h: 27 }, { x: 4.5, y: 42, w: 40, h: 8 }, { x: 54, y: 3, w: 22, h: 7 }];
  const placed = clearZone(items, zones, [3, 40], (i) => APP_KINDS.includes(i.kind) || i.kind === "clock");
  items.splice(0, items.length, ...placed);
  const bg = l.background.kind === "color" ? { ...l.background, kind: "gradient" as const, color: "#0a0a0f", color2: "#1d1d26" } : l.background;
  return { ...base, home: "tvos", tiles: "tvos", focus: "parallax", launch: "zoom", iconShape: "rounded", items, background: { ...bg, dim: Math.min(bg.dim, 0.2) } };
}

/** A box on the TV's first screen, in percent of the screen. */
export type ScreenBox = { id: string; label: string; x: number; y: number; w: number; h: number; fixed?: boolean };

/** What the TV draws on its first screen: the enabled widgets, plus the launcher's own buttons (Home settings and
 *  the profile button, in every mode but Classic) and the Top Shelf app name in Apple TV style. Positions match
 *  CanvasHome.java. Background shapes are meant to sit under things, so they are left out. */
export function screenBoxes(l: Layout): ScreenBox[] {
  const classic = isClassic(l), scroll = isScroll(l);
  const boxes: ScreenBox[] = l.items
    .filter((i) => i.enabled && i.kind !== "shape" && !(classic && APP_KINDS.includes(i.kind)) && !(scroll && i.kind === "apps"))
    .map((i) => ({ id: i.id, label: KIND_LABEL[i.kind], x: i.x, y: i.y, w: i.w, h: i.h }));
  if (!classic) {
    const profiles = (l.profiles?.length ?? 0) > 0;
    if (profiles) boxes.push({ id: "tv-profile", label: "the profile button", x: 84, y: 3, w: 13, h: 6, fixed: true });
    const [gx, gy] = gearSpot(boxes, profiles);
    boxes.push({ id: "tv-gear", label: "the ⚙ Home settings button", x: gx, y: gy, w: GEAR.w, h: GEAR.h, fixed: true });
  }
  if (isTvos(l)) boxes.push({ id: "tv-shelf-title", label: "the Top Shelf app name", x: 4.5, y: 42, w: 40, h: 8, fixed: true });
  // Classic: the launcher's own header buttons (Inputs, network, Settings), the focused app's name and the dock.
  if (classic) for (const z of CLASSIC_ZONES) boxes.push({ ...z, fixed: true });
  return boxes;
}

/** Where Classic Home draws its own parts, in percent (measured on a 1080p TV; TvChoices.CLASSIC_ZONES). */
export const CLASSIC_ZONES = [
  { id: "tv-classic-buttons", label: "Classic's Inputs, network and Settings buttons", x: 54, y: 10, w: 41, h: 9 },
  { id: "tv-classic-name", label: "Classic's app name", x: 6, y: 48, w: 30, h: 11 },
  { id: "tv-classic-dock", label: "Classic's app dock", x: 6, y: 63, w: 89, h: 37 },
];
const GEAR = { w: 4.5, h: 6 };
const hits = (a: { x: number; y: number; w: number; h: number }, b: { x: number; y: number; w: number; h: number }, m = 0.5) =>
  Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x) > m && Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y) > m;

/** Where the launcher puts the ⚙ button: the first corner clear of the widgets (CanvasHome.gearSpot, same order). */
export function gearSpot(boxes: { x: number; y: number; w: number; h: number }[], profiles: boolean): [number, number] {
  const spots: [number, number][] = [[profiles ? 78.5 : 92.5, 3], [94.5, 91], [1, 91], [1, 3]];
  for (const [x, y] of spots) if (!boxes.some((b) => hits(b, { x, y, ...GEAR }))) return [x, y];
  return spots[0]!;
}

/** Moves widgets off {@code zone} (the app row or dock): into the first free place in {@code band} (top to bottom,
 *  left to right), or switches them off when there is none, so nothing is drawn under the apps. */
function clearZone(items: LayoutItem[], zones: { x: number; y: number; w: number; h: number }[], band: [number, number], skip: (i: LayoutItem) => boolean): LayoutItem[] {
  const out = [...items];
  for (let k = 0; k < out.length; k++) {
    const it = out[k]!;
    if (!it.enabled || skip(it) || it.kind === "shape" || !zones.some((z) => hits(it, z))) continue;
    const others = () => [...zones, ...out.filter((o, j) => j !== k && o.enabled && !skip(o) && o.kind !== "shape")];
    let placed: LayoutItem | null = null;
    // Same size first; a widget too big for the free band is shrunk (its text with it) rather than lost.
    for (const f of [1, 0.8, 0.65, 0.5]) {
      const w = it.w * f, h = it.h * f;
      for (let y = band[0]; y + h <= band[1] && !placed; y += 1) for (let x = 2; x + w <= 98 && !placed; x += 1) {
        const cand = { ...it, x, y, w, h, style: { ...it.style, size: Math.max(12, Math.round(it.style.size * f)) } };
        if (!others().some((o) => hits(cand, o))) placed = cand;
      }
      if (placed) break;
    }
    out[k] = placed ?? { ...it, enabled: false };
  }
  return out;
}

/** Pairs of boxes that cover each other on the TV (more than a sliver: 0.5 % of the screen each way). */
export function overlaps(l: Layout): [ScreenBox, ScreenBox][] {
  const b = screenBoxes(l), out: [ScreenBox, ScreenBox][] = [];
  const m = 0.5;
  for (let i = 0; i < b.length; i++) for (let j = i + 1; j < b.length; j++) {
    const p = b[i]!, q = b[j]!;
    if (p.fixed && q.fixed) continue;
    const ox = Math.min(p.x + p.w, q.x + q.w) - Math.max(p.x, q.x), oy = Math.min(p.y + p.h, q.y + q.h) - Math.max(p.y, q.y);
    if (ox > m && oy > m) out.push(p.fixed ? [q, p] : [p, q]);
  }
  return out;
}

/** Scroll mode needs a "Recently used" row and an app grid: add whichever is missing, and switch the row on. */
export function withScrollParts(l: Layout): Layout {
  const out = withScrollPartsRaw(l);
  const row = out.items.find((i) => i.kind === "recent")!;
  const items = clearZone(out.items, [row], [3, Math.max(3, row.y - 1)], (i) => APP_KINDS.includes(i.kind));
  return { ...out, items };
}

function withScrollPartsRaw(l: Layout): Layout {
  let items = l.items;
  if (!items.some((i) => i.kind === "recent")) items = [...items, { ...newItem("recent", 1), id: nextId(items, "recent"), x: 4, y: 64, w: 92, h: 24, options: { max: 10 } }];
  else items = items.map((i) => (i.kind === "recent" ? { ...i, enabled: true } : i));
  if (!items.some((i) => i.kind === "apps")) items = [...items, { ...newItem("apps", 1), id: nextId(items, "apps") }];
  return { ...l, home: "scroll", items };
}

export function newItem(kind: ItemKind, n: number): LayoutItem {
  const id = `${kind}-${n}`;
  switch (kind) {
    case "apps": return { ...base, id, kind, x: 5, y: 62, w: 90, h: 30, style: { ...text(), size: 22, radius: 14 }, icon: { size: 96, shape: "rounded", labels: true, columns: 8 } };
    case "recent": return { ...base, id, kind, x: 5, y: 40, w: 60, h: 18, style: { ...text(), size: 20, radius: 12 }, icon: { size: 72, shape: "rounded", labels: true, columns: 6 }, options: { max: 6 } };
    case "watchnext": return { ...base, id, kind, x: 5, y: 12, w: 70, h: 24, style: { ...text(), size: 20, radius: 12 }, options: { max: 6 } };
    case "clock": return { ...base, id, kind, x: 72, y: 5, w: 24, h: 12, style: { ...text("sans-serif-light", 72), align: "end" }, options: { twentyFour: false, seconds: false } };
    case "date": return { ...base, id, kind, x: 72, y: 17, w: 24, h: 6, style: { ...text("sans-serif", 28), align: "end", opacity: 0.85 } };
    case "greeting": return { ...base, id, kind, x: 5, y: 5, w: 50, h: 8, style: text("sans-serif-medium", 36) };
    case "text": return { ...base, id, kind, x: 5, y: 5, w: 30, h: 8, text: "Hello", style: text("sans-serif", 32) };
    case "calendar": return { ...base, id, kind, x: 5, y: 40, w: 24, h: 36, style: { ...text("sans-serif", 24), align: "center" }, options: { weekStartsMonday: false } };
    case "visualizer": return { ...base, id, kind, variant: "mirror", x: 30, y: 60, w: 40, h: 14, style: { ...text("sans-serif", 24), color: "#ffffff", opacity: 0.9 }, options: { bars: 48 }, motion: { type: "none", speed: 1.5, amount: 50 } };
    case "bluetooth": return { ...base, id, kind, x: 72, y: 24, w: 24, h: 5, style: { ...text("sans-serif", 22), align: "end", opacity: 0.85 } };
    case "nowplaying": return { ...base, id, kind, x: 5, y: 40, w: 40, h: 7, style: { ...text("sans-serif-medium", 24) } };
    case "weather": return { ...base, id, kind, variant: "full", x: 70, y: 28, w: 26, h: 10, style: { ...text("sans-serif-light", 28), align: "end" } };
    case "shape": return { ...base, id, kind, variant: "rect", x: 10, y: 10, w: 30, h: 20, style: { ...text("sans-serif", 24), radius: 24, fill: "#ffffff22", opacity: 1 }, options: { stroke: 0 } };
    case "cpu": case "memory": case "network":
      return { ...base, id, kind, x: 5, y: 92, w: 18, h: 5, style: { ...text("monospace", 22), opacity: 0.9 }, options: { compact: true } };
  }
}

export function defaultLayout(): Layout {
  const items: LayoutItem[] = [
    // Top-left: Classic Home keeps its own buttons top-right and its apps below.
    { ...newItem("clock", 1), x: 5, y: 4, w: 24, h: 12, style: { ...newItem("clock", 1).style, align: "start" as const } },
    { ...newItem("date", 1), x: 5, y: 16, w: 30, h: 6, style: { ...newItem("date", 1).style, align: "start" as const } },
    { ...newItem("greeting", 1), x: 5, y: 23, w: 45, h: 8 },
    { ...newItem("recent", 1), enabled: false }, { ...newItem("watchnext", 1), enabled: false },
    newItem("apps", 1),
    { ...newItem("cpu", 1), enabled: false, x: 5, y: 92 }, { ...newItem("memory", 1), enabled: false, x: 25, y: 92 }, { ...newItem("network", 1), enabled: false, x: 45, y: 92 },
  ];
  return { version: 1, enabled: false, home: "classic", items, background: { kind: "image", color: "#0b1020", color2: "#27325c", video: null, dim: 0.25, blur: 0 } };
}

/** Fonts a layout names as files that the draft does not carry (checked before sending). */
export function missingFonts(l: Layout, have: string[]): string[] {
  const out = new Set<string>();
  for (const it of l.items) if (it.enabled && isFontFile(it.style.font) && !have.includes(it.style.font.slice(5))) out.add(it.style.font.slice(5));
  return [...out];
}

export function validateLayout(l: unknown): string[] {
  const r = LayoutSchema.safeParse(l);
  if (!r.success) return r.error.issues.map((i) => `${i.path.join(".")}: ${i.message}`);
  const out: string[] = [];
  const ids = new Set<string>();
  for (const it of r.data.items) {
    if (ids.has(it.id)) out.push(`${it.id}: duplicate id`);
    ids.add(it.id);
    const allowed = VARIANTS[it.kind];
    if (it.variant !== undefined && !(allowed ?? []).includes(it.variant)) out.push(`${it.id}: ${it.variant} is not a look for ${it.kind}`);
    if (it.x + it.w > 100.001 || it.y + it.h > 100.001) out.push(`${it.id}: runs off the screen`);
  }
  return out;
}

export const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Keep an item fully on screen. */
export function fit(it: LayoutItem): LayoutItem {
  const w = clamp(it.w, 3, 100), h = clamp(it.h, 3, 100);
  return { ...it, w, h, x: clamp(it.x, 0, 100 - w), y: clamp(it.y, 0, 100 - h) };
}

export function nextId(items: LayoutItem[], kind: ItemKind): string {
  let n = 1;
  while (items.some((i) => i.id === `${kind}-${n}`)) n++;
  return `${kind}-${n}`;
}
