import { newItem, type ItemKind, type Layout, type LayoutItem } from "./layout";

/** Ready-made canvas looks to start from. All original; they use only the TV's built-in fonts, so none needs a file. */
export interface Look { id: string; name: string; blurb: string; layout: Layout }

type Over = Partial<Omit<LayoutItem, "style" | "options" | "icon" | "motion">> & {
  style?: Partial<LayoutItem["style"]>; options?: LayoutItem["options"]; icon?: Partial<NonNullable<LayoutItem["icon"]>>; motion?: Partial<LayoutItem["motion"]>;
};

function it(kind: ItemKind, id: string, o: Over): LayoutItem {
  const b = newItem(kind, 1);
  const { style, options, icon, motion, ...rest } = o;
  return {
    ...b, ...rest, id,
    style: { ...b.style, ...style },
    options: options ? { ...b.options, ...options } : b.options,
    icon: b.icon || icon ? { ...(b.icon ?? { size: 72, shape: "rounded", labels: false, columns: 8 }), ...icon } : undefined,
    motion: { ...b.motion, ...motion },
  };
}

const bg = (kind: Layout["background"]["kind"], color: string, color2: string, video: string | null = null, dim = 0.25): Layout["background"] => ({ kind, color, color2, video, dim, blur: 0 });
const scene = (id: NonNullable<Layout["background"]["scene"]>, dim = 0): Layout["background"] => ({ kind: "scene", scene: id, sceneTime: "auto", color: "#05070f", color2: "#141a33", video: null, dim, blur: 0 });
const layout = (background: Layout["background"], items: LayoutItem[], home: NonNullable<Layout["home"]> = "canvas"): Layout => ({ version: 1, enabled: true, home, background, items });
const white = "#ffffff";

export const LOOKS: Look[] = [
  // Classic looks: the launcher's own animated apps (dock, grid that slides up, glass focus) stay; widgets live in
  // the top-left band (x 4-52 %, y 3-22 %), clear of the top-right buttons, the app name and the dock. Measured on a
  // 1080p Sony Google TV: buttons x 55-94 % / y 11-18 %, app name y 50-58 %, dock y 64-88 %.
  {
    id: "classic-hud", name: "Classic HUD", blurb: "Smooth launcher apps below; monospace clock, memory and network readouts in the top-left corner.",
    layout: layout(bg("gradient", "#0b0f14", "#1b2530", null, 0), [
      it("clock", "clock-1", { x: 4, y: 4, w: 26, h: 10, style: { font: "monospace", size: 64, color: "#ff9a2e", align: "start" }, options: { twentyFour: true, seconds: true } }),
      it("date", "date-1", { variant: "weekday_date", x: 4, y: 14, w: 26, h: 4, style: { font: "monospace", size: 22, upper: true, spacing: 0.1, align: "start" } }),
      it("memory", "mem-1", { variant: "bar", x: 32, y: 5, w: 20, h: 5, style: { font: "monospace", size: 18, color: "#6fd3ff" } }),
      it("network", "net-1", { variant: "text", x: 32, y: 11, w: 20, h: 4, style: { font: "monospace", size: 18, color: "#9cff8a" } }),
      it("visualizer", "bars-1", { variant: "bars", x: 32, y: 16, w: 20, h: 4, style: { color: "#ff9a2e", opacity: 0.8 }, options: { bars: 32 } }),
    ], "classic"),
  },
  {
    id: "classic-rain", name: "Classic rain", blurb: "Rain on the city behind the launcher's own apps, with a thin clock and greeting.",
    layout: layout(bg("video", "#0b1020", "#27325c", "rain", 0.4), [
      it("clock", "clock-1", { x: 4, y: 4, w: 34, h: 13, style: { font: "sans-serif-thin", size: 104, shadow: true, align: "start" } }),
      it("date", "date-1", { variant: "full", x: 4, y: 18, w: 40, h: 4, style: { font: "sans-serif-light", size: 22, upper: true, spacing: 0.25, opacity: 0.9, align: "start" } }),
      it("greeting", "greet-1", { x: 4, y: 24, w: 40, h: 5, style: { font: "sans-serif-light", size: 26, opacity: 0.8 } }),
    ], "classic"),
  },
  {
    id: "classic-dusk", name: "Classic dusk", blurb: "A small analog clock and a big serif date over warm dusk colours; apps stay animated.",
    layout: layout(bg("gradient", "#1a0f12", "#3b1c1a", null, 0), [
      it("clock", "clock-1", { variant: "analog", x: 4, y: 5, w: 10, h: 18, style: { color: white, fill: null }, options: { stroke: 2, seconds: true } }),
      it("date", "date-1", { variant: "daymonth", x: 16, y: 5, w: 24, h: 12, style: { font: "serif", size: 84, color: "#e0703f", align: "start" } }),
      it("date", "day-1", { variant: "weekday", x: 16, y: 17, w: 24, h: 4, style: { size: 22, spacing: 0.4, upper: true, opacity: 0.85, align: "start" } }),
      it("memory", "mem-1", { variant: "ring", x: 42, y: 6, w: 8, h: 14, style: { size: 14, color: "#e0703f" }, options: { compact: true } }),
    ], "classic"),
  },
  {
    id: "scroll-aurora", name: "Scroll: northern lights", blurb: "Live aurora over a starry sky (no video file), a thin clock, recent apps; Down for all apps.",
    layout: layout(scene("aurora"), [
      it("clock", "clock-1", { x: 5, y: 7, w: 36, h: 16, style: { font: "sans-serif-thin", size: 130, align: "start", shadow: true } }),
      it("date", "date-1", { variant: "full", x: 5, y: 24, w: 40, h: 5, style: { font: "sans-serif-light", size: 26, upper: true, spacing: 0.25, align: "start", opacity: 0.85 } }),
      it("recent", "recent-1", { x: 4, y: 64, w: 92, h: 24, style: { font: "sans-serif-medium", size: 26 }, options: { max: 10 } }),
      it("apps", "apps-1", { x: 5, y: 62, w: 90, h: 30, style: { font: "sans-serif-medium", size: 28 }, icon: { size: 96, shape: "rounded", labels: true, columns: 6 } }),
    ], "scroll"),
  },
  {
    id: "classic-snow", name: "Classic snowfall", blurb: "Live snow behind the launcher's own animated apps; sky colours follow the time of day.",
    layout: layout(scene("snow", 0.1), [
      it("clock", "clock-1", { x: 4, y: 4, w: 30, h: 12, style: { font: "sans-serif-thin", size: 96, align: "start", shadow: true } }),
      it("date", "date-1", { variant: "full", x: 4, y: 17, w: 40, h: 4, style: { font: "sans-serif-light", size: 22, upper: true, spacing: 0.25, align: "start" } }),
    ], "classic"),
  },
  // Scroll looks: widgets and a "Recently used" row of medium tiles; Down glides to every app.
  {
    id: "scroll-anime-night", name: "Scroll: anime night", blurb: "Anime city video, a big clock, recently used apps in a row; press Down for all apps.",
    layout: layout(bg("video", "#0b1020", "#27325c", "anime-night", 0.3), [
      it("clock", "clock-1", { x: 5, y: 8, w: 40, h: 18, style: { font: "sans-serif-thin", size: 150, shadow: true, align: "start" } }),
      it("date", "date-1", { variant: "full", x: 5, y: 27, w: 40, h: 5, style: { font: "sans-serif-light", size: 28, upper: true, spacing: 0.25, align: "start" } }),
      it("greeting", "greet-1", { x: 5, y: 33, w: 50, h: 6, style: { font: "sans-serif-light", size: 30, opacity: 0.85 } }),
      it("memory", "mem-1", { variant: "ring", x: 82, y: 8, w: 7, h: 13, style: { size: 14, color: "#9ecbff" }, options: { compact: true } }),
      it("network", "net-1", { variant: "text", x: 72, y: 23, w: 23, h: 4, style: { size: 18, color: "#9ecbff", align: "end" } }),
      it("recent", "recent-1", { x: 4, y: 64, w: 92, h: 24, style: { font: "sans-serif-medium", size: 26 }, options: { max: 10 } }),
      it("apps", "apps-1", { x: 5, y: 62, w: 90, h: 30, style: { font: "sans-serif-medium", size: 28 }, icon: { size: 96, shape: "rounded", labels: true, columns: 6 } }),
    ], "scroll"),
  },
  {
    id: "scroll-calm", name: "Scroll: calm", blurb: "Your own picture (animated GIFs move), a thin clock and a recent-apps row.",
    layout: layout(bg("image", "#101418", "#2a3340", null, 0.35), [
      it("clock", "clock-1", { variant: "stacked", x: 70, y: 6, w: 26, h: 44, style: { font: "sans-serif-thin", size: 220, align: "end", opacity: 0.9 }, options: { twentyFour: true } }),
      it("date", "day-1", { variant: "weekday", x: 5, y: 10, w: 40, h: 8, style: { font: "serif", size: 64 } }),
      it("date", "date-1", { variant: "daymonth", x: 5, y: 19, w: 40, h: 5, style: { size: 26, upper: true, spacing: 0.3, opacity: 0.8 } }),
      it("recent", "recent-1", { x: 4, y: 64, w: 92, h: 24, style: { size: 24 }, options: { max: 10 } }),
      it("apps", "apps-1", { x: 5, y: 62, w: 90, h: 30, style: { size: 26 }, icon: { size: 96, shape: "rounded", labels: true, columns: 5 } }),
    ], "scroll"),
  },
  {
    id: "misty-sunday", name: "Misty Sunday", blurb: "Big script weekday, spaced-out date, animated bars. Works best on the foggy-window video.",
    layout: layout(bg("video", "#1b2140", "#4a5a96", "foggy-window", 0.3), [
      it("date", "day-1", { variant: "weekday", x: 25, y: 30, w: 50, h: 22, style: { font: "cursive", size: 190, align: "center", shadow: true } }),
      it("date", "date-1", { variant: "full", x: 30, y: 54, w: 40, h: 5, style: { font: "sans-serif-light", size: 26, align: "center", spacing: 0.3, upper: true } }),
      it("clock", "clock-1", { x: 35, y: 60, w: 30, h: 5, style: { font: "sans-serif-light", size: 24, align: "center", spacing: 0.3, upper: true, opacity: 0.85 }, options: { twentyFour: true } }),
      it("visualizer", "bars-1", { variant: "mirror", x: 32, y: 70, w: 36, h: 12, style: { color: white, opacity: 0.9 }, options: { bars: 56 } }),
      it("apps", "apps-1", { x: 25, y: 87, w: 50, h: 10, style: { align: "center", size: 18 }, icon: { size: 56, shape: "circle", labels: false, columns: 8 }, options: { max: 8 } }),
    ]),
  },
  {
    id: "stacked-calm", name: "Stacked calm", blurb: "A tall thin clock, hours over minutes, with quiet rings for CPU, memory and network.",
    layout: layout(bg("gradient", "#0f1613", "#27402f", null, 0), [
      it("clock", "clock-1", { variant: "stacked", x: 36, y: 8, w: 28, h: 58, style: { font: "sans-serif-thin", size: 330, align: "center", opacity: 0.92 }, options: { twentyFour: true } }),
      it("date", "date-1", { variant: "weekday_date", x: 30, y: 68, w: 40, h: 5, style: { size: 24, align: "center", spacing: 0.25, upper: true, opacity: 0.8 } }),
      it("cpu", "cpu-1", { variant: "ring", x: 36, y: 78, w: 8, h: 14, style: { size: 14, color: "#8bd8a0" }, options: { compact: true } }),
      it("memory", "mem-1", { variant: "ring", x: 46, y: 78, w: 8, h: 14, style: { size: 14, color: "#8bd8a0" }, options: { compact: true } }),
      it("network", "net-1", { variant: "ring", x: 56, y: 78, w: 8, h: 14, style: { size: 14, color: "#8bd8a0" }, options: { compact: true } }),
      it("shape", "line-1", { variant: "line", x: 36, y: 66.5, w: 28, h: 3, style: { color: white, fill: "#ffffff88" }, options: { stroke: 2 } }),
    ]),
  },
  {
    id: "orange-split", name: "Orange split", blurb: "Bold orange and charcoal: clock and bars on the left, calendar on a dark panel.",
    layout: layout(bg("color", "#e58a1f", "#e58a1f", null, 0), [
      it("shape", "panel-1", { variant: "rect", x: 72, y: 0, w: 28, h: 100, style: { fill: "#222222", radius: 0 } }),
      it("clock", "clock-1", { x: 6, y: 28, w: 40, h: 22, style: { font: "sans-serif-light", size: 190, color: "#141414", align: "start" }, options: { twentyFour: true } }),
      it("greeting", "greet-1", { x: 6, y: 52, w: 50, h: 7, style: { font: "sans-serif", size: 40, color: "#141414" } }),
      it("cpu", "cpu-1", { variant: "bar", x: 6, y: 68, w: 16, h: 6, style: { size: 18, color: "#141414" } }),
      it("memory", "mem-1", { variant: "bar", x: 26, y: 68, w: 16, h: 6, style: { size: 18, color: "#141414" } }),
      it("network", "net-1", { variant: "bar", x: 46, y: 68, w: 16, h: 6, style: { size: 18, color: "#141414" } }),
      it("calendar", "cal-1", { x: 75, y: 18, w: 22, h: 40, style: { size: 26, color: "#e58a1f" }, options: { weekStartsMonday: false } }),
      it("date", "date-1", { variant: "full", x: 75, y: 8, w: 22, h: 5, style: { size: 22, color: white, spacing: 0.2, upper: true } }),
    ]),
  },
  {
    id: "glass-dock", name: "Glass dock", blurb: "Rainy-window video, a clock top right, and your apps on a frosted shelf.",
    layout: layout(bg("video", "#101828", "#1c2a40", "rainy-window", 0.2), [
      it("clock", "clock-1", { x: 62, y: 5, w: 34, h: 16, style: { font: "sans-serif-light", size: 120, align: "end", shadow: true }, options: { twentyFour: false } }),
      it("date", "date-1", { variant: "weekday_date", x: 62, y: 21, w: 34, h: 6, style: { size: 28, align: "end", shadow: true } }),
      it("greeting", "greet-1", { x: 4, y: 6, w: 40, h: 8, style: { font: "sans-serif-medium", size: 40, shadow: true } }),
      it("shape", "shelf-1", { variant: "rect", x: 8, y: 72, w: 84, h: 22, style: { fill: "#ffffff26", radius: 36 }, options: { stroke: 0 } }),
      it("apps", "apps-1", { x: 10, y: 74, w: 80, h: 18, style: { align: "center", size: 20, radius: 16 }, icon: { size: 110, shape: "rounded", labels: true, columns: 8 } }),
    ]),
  },
  {
    id: "hud", name: "HUD", blurb: "Technical readout: outlined boxes, monospace numbers and live graphs.",
    layout: layout(bg("gradient", "#0b0f14", "#1b2530", null, 0), [
      it("shape", "box-1", { variant: "rect", x: 4, y: 6, w: 28, h: 30, style: { fill: "#00000055", radius: 4, color: "#ff9a2e" }, options: { stroke: 1 } }),
      it("clock", "clock-1", { x: 5, y: 8, w: 26, h: 12, style: { font: "monospace", size: 64, color: "#ff9a2e" }, options: { twentyFour: true, seconds: true } }),
      it("date", "date-1", { variant: "weekday_date", x: 5, y: 22, w: 26, h: 5, style: { font: "monospace", size: 22, upper: true, spacing: 0.1 } }),
      it("cpu", "cpu-1", { variant: "graph", x: 5, y: 40, w: 26, h: 12, style: { font: "monospace", size: 20, color: "#ff9a2e" } }),
      it("memory", "mem-1", { variant: "graph", x: 5, y: 56, w: 26, h: 12, style: { font: "monospace", size: 20, color: "#6fd3ff" } }),
      it("network", "net-1", { variant: "graph", x: 5, y: 72, w: 26, h: 12, style: { font: "monospace", size: 20, color: "#9cff8a" } }),
      it("shape", "box-2", { variant: "rect", x: 70, y: 6, w: 26, h: 44, style: { fill: "#00000055", radius: 4, color: "#ff9a2e" }, options: { stroke: 1 } }),
      it("recent", "recent-1", { x: 71, y: 8, w: 24, h: 40, style: { font: "monospace", size: 18 }, icon: { size: 64, shape: "square", labels: true, columns: 3 }, options: { max: 6 } }),
      it("visualizer", "bars-1", { variant: "bars", x: 36, y: 80, w: 28, h: 12, style: { color: "#ff9a2e", opacity: 0.9 }, options: { bars: 40 } }),
    ]),
  },
  {
    id: "analog-dusk", name: "Analog dusk", blurb: "A round clock over a glowing disc, big thin date, and small bar panels.",
    layout: layout(bg("gradient", "#141414", "#1f1a1a", null, 0), [
      it("shape", "moon-1", { variant: "circle", x: 36, y: 8, w: 22, h: 39, style: { fill: "#c2401a99", radius: 0 } }),
      it("clock", "clock-1", { variant: "analog", x: 46, y: 38, w: 12, h: 21, style: { color: white, fill: null }, options: { stroke: 2, seconds: true } }),
      it("date", "date-1", { variant: "daymonth", x: 14, y: 40, w: 30, h: 14, style: { font: "serif", size: 100, color: "#c8502a", align: "end" } }),
      it("date", "day-1", { variant: "weekday", x: 14, y: 54, w: 30, h: 5, style: { size: 24, spacing: 0.4, upper: true, align: "end" } }),
      it("shape", "p1", { variant: "rect", x: 12, y: 76, w: 17, h: 16, style: { fill: "#ffffff12", radius: 18 } }),
      it("shape", "p2", { variant: "rect", x: 31, y: 76, w: 17, h: 16, style: { fill: "#ffffff12", radius: 18 } }),
      it("shape", "p3", { variant: "rect", x: 50, y: 76, w: 17, h: 16, style: { fill: "#ffffff12", radius: 18 } }),
      it("calendar", "cal-1", { x: 12.5, y: 77, w: 16, h: 14, style: { size: 11 } }),
      it("cpu", "cpu-1", { variant: "ring", x: 33, y: 78, w: 6, h: 12, style: { size: 14, color: "#ff3d71" }, options: { compact: true } }),
      it("memory", "mem-1", { variant: "ring", x: 40, y: 78, w: 6, h: 12, style: { size: 14, color: "#c6ff00" }, options: { compact: true } }),
      it("network", "net-1", { variant: "graph", x: 52, y: 78, w: 14, h: 12, style: { size: 16, color: "#00e5ff" }, options: { compact: true } }),
    ]),
  },
  {
    id: "sunrise-minimal", name: "Sunrise minimal", blurb: "Soft gradient, large clock top left, a quiet row of apps.",
    layout: layout(bg("gradient", "#3a1f4a", "#e0527a", null, 0), [
      it("clock", "clock-1", { x: 4, y: 6, w: 38, h: 20, style: { font: "sans-serif-light", size: 150, shadow: true } }),
      it("date", "date-1", { variant: "weekday_date", x: 4, y: 26, w: 38, h: 6, style: { size: 30, spacing: 0.15, upper: true, opacity: 0.9 } }),
      it("greeting", "greet-1", { x: 4, y: 34, w: 38, h: 6, style: { size: 28, opacity: 0.8, italic: true } }),
      it("apps", "apps-1", { x: 4, y: 72, w: 92, h: 22, style: { size: 20, radius: 18 }, icon: { size: 120, shape: "rounded", labels: true, columns: 8 } }),
    ]),
  },
  {
    id: "forest-lines", name: "Forest lines", blurb: "A tall hollow-feel clock between two thin lines, with dotted bars along the bottom.",
    layout: layout(bg("gradient", "#0e2a18", "#2f6b3c", null, 0), [
      it("shape", "line-1", { variant: "line", x: 38, y: 7, w: 24, h: 3, style: { fill: "#ffffff99" }, options: { stroke: 2 } }),
      it("clock", "clock-1", { variant: "stacked", x: 38, y: 10, w: 24, h: 62, style: { font: "sans-serif-thin", size: 300, align: "center", opacity: 0.85 }, options: { twentyFour: true } }),
      it("shape", "line-2", { variant: "line", x: 38, y: 74, w: 24, h: 3, style: { fill: "#ffffff99" }, options: { stroke: 2 } }),
      it("date", "date-1", { variant: "full", x: 38, y: 77, w: 24, h: 5, style: { size: 22, align: "center", opacity: 0.85 } }),
      it("visualizer", "dots-1", { variant: "dots", x: 4, y: 86, w: 92, h: 9, style: { color: "#7dff8a", opacity: 0.8 }, options: { bars: 72 } }),
    ]),
  },
];
