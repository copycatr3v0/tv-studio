/// <reference types="vite/client" />
import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { LayoutEditor } from "./theme/LayoutEditor";
import { FamilyBar } from "./theme/FamilyBar";
import { defaultLayout, fit, newItem, overlaps, validateLayout, withAppleTvLook, withScrollParts, VIDEO_SAMPLES, type Layout } from "./theme/layout";
import { SAMPLE_APPS } from "./theme/apps";
import { reducer, draftFromCatalog } from "./theme/useThemeEditor";

describe("layout model", () => {
  it("the default layout is valid and starts switched off", () => {
    const l = defaultLayout();
    expect(validateLayout(l)).toEqual([]);
    expect(l.enabled).toBe(false);
    expect(l.items.filter((i) => ["cpu", "memory", "network", "recent", "watchnext"].includes(i.kind)).every((i) => !i.enabled)).toBe(true);
  });
  it("every item kind can be created and is valid", () => {
    const l = defaultLayout();
    for (const k of ["apps", "recent", "watchnext", "clock", "date", "greeting", "text", "cpu", "memory", "network"] as const) {
      expect(validateLayout({ ...l, items: [newItem(k, 1)] })).toEqual([]);
    }
  });
  it("rejects items off the screen, bad fonts and unsafe video names", () => {
    const l = defaultLayout();
    const bad = { ...l, items: [{ ...l.items[0]!, x: 95, w: 20 }] };
    expect(validateLayout(bad).join()).toMatch(/off the screen/);
    expect(validateLayout({ ...l, items: [{ ...l.items[0]!, style: { ...l.items[0]!.style, font: "Papyrus" } }] }).length).toBeGreaterThan(0);
    expect(validateLayout({ ...l, background: { ...l.background, video: "../x" } }).length).toBeGreaterThan(0);
  });
  it("fit() keeps an item on screen", () => {
    const f = fit({ ...newItem("clock", 1), x: 99, y: -5, w: 30 });
    expect(f.x + f.w).toBeLessThanOrEqual(100);
    expect(f.y).toBe(0);
  });
  it("old saved themes without a layout still load, and layout edits are undoable", () => {
    const d = draftFromCatalog("premium");
    const s0 = { draft: { ...d, layout: undefined } as unknown as typeof d, past: [], future: [], savedAs: null };
    const s1 = reducer(s0, { type: "layout", layout: defaultLayout() });
    expect(s1.draft.layout).toBeDefined();
    expect(reducer(s1, { type: "undo" }).draft.layout).toBeUndefined();
  });
  it("every bundled sample video and poster exists", () => {
    const files = Object.keys(import.meta.glob("/public/videos/*.{mp4,jpg}", { query: "?url", eager: false }));
    for (const v of VIDEO_SAMPLES) {
      expect(files, v.id).toContain(`/public/videos/${v.id}.mp4`);
      expect(files, v.id).toContain(`/public/videos/${v.id}.jpg`);
    }
  });
});

function Harness({ onChange, initial }: { onChange?: (l: Layout) => void; initial?: Layout }) {
  const [l, setL] = useState<Layout>(initial ?? { ...defaultLayout(), enabled: true });
  const [v, setV] = useState<{ name: string; url: string }[]>([]);
  return <LayoutEditor layout={l} onChange={(x) => { setL(x); onChange?.(x); }} apps={SAMPLE_APPS} wallpaperUrl={null} videos={v} onVideos={setV} />;
}

describe("LayoutEditor", () => {
  it("scroll-mode tile look: style, shape, focus and launch are saved", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("radio", { name: /Scroll: recent apps row/ }));
    fireEvent.change(screen.getByLabelText("Tile style"), { target: { value: "icon" } });
    fireEvent.change(screen.getByLabelText("Icon shape"), { target: { value: "squircle" } });
    fireEvent.change(screen.getByLabelText("Focus style"), { target: { value: "bounce" } });
    fireEvent.change(screen.getByLabelText("App opens with"), { target: { value: "reveal" } });
    expect(last).toMatchObject({ tiles: "icon", iconShape: "squircle", focus: "bounce", launch: "reveal" });
    expect(validateLayout(last!)).toEqual([]);
  });
  it("picks an ambient scene and shows the low-memory switch for heavy effects", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("radio", { name: "ambient scene" }));
    expect(last!.background.kind).toBe("scene");
    fireEvent.click(screen.getByRole("radio", { name: /Northern lights/ }));
    expect(last!.background.scene).toBe("aurora");
    expect(validateLayout(last!)).toEqual([]);
    expect(screen.getByTestId("low-ram-panel")).toHaveTextContent(/ambient scene/);
    fireEvent.click(screen.getByLabelText(/Keep them on even on low-memory TVs/));
    expect(last!.heavyOnLowRam).toBe(true);
  });
  it("scroll mode adds the recent row and app grid, previews the all-apps page, and the screensaver switch is saved", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("radio", { name: /Scroll: recent apps row/ }));
    expect(last!.home).toBe("scroll");
    expect(last!.items.find((i) => i.kind === "recent")!.enabled).toBe(true);
    expect(last!.items.some((i) => i.kind === "apps")).toBe(true);
    expect(screen.getByTestId("all-apps-page")).toBeInTheDocument();
    expect(validateLayout(last!)).toEqual([]);
    fireEvent.click(screen.getByLabelText(/Also use this canvas as the TV's screensaver/));
    expect(last!.screensaver).toBe(true);
  });
  it("Apple TV style sets the look in one click, previews the Top Shelf and dock, and stays valid", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("radio", { name: /Apple TV style/ }));
    expect(last).toMatchObject({ home: "tvos", tiles: "tvos", focus: "parallax", launch: "zoom" });
    expect(last!.items.find((i) => i.kind === "recent")).toMatchObject({ enabled: true, y: 63, h: 27 });
    expect(last!.items.find((i) => i.kind === "apps")!.icon!.columns).toBe(5);
    expect(validateLayout(last!)).toEqual([]);
    expect(screen.getByTestId("top-shelf")).toBeInTheDocument();
    expect(screen.getByTestId("dock-preview")).toBeInTheDocument();
    expect(screen.getByTestId("all-apps-page")).toBeInTheDocument();
    expect(screen.getByLabelText("Focus style")).toHaveValue("parallax");
  });
  it("no built-in look overlaps on the TV, as it is or switched to Scroll or Apple TV style", () => {
    const found: string[] = [];
    for (const look of LOOKS) for (const [name, l] of [["", look.layout], ["+scroll", withScrollParts(look.layout)], ["+tvos", withAppleTvLook(look.layout)]] as const) {
      for (const [a, b] of overlaps(l)) found.push(`${look.id}${name}: ${a.id} / ${b.id}`);
      expect(validateLayout(l)).toEqual([]);
    }
    for (const [a, b] of overlaps(defaultLayout())) found.push(`default: ${a.id} / ${b.id}`);
    expect(found).toEqual([]);
  });
  it("finds what overlaps on the TV, including the launcher's own buttons, and the Apple TV look has none", () => {
    const base = withAppleTvLook(defaultLayout());
    expect(overlaps(base)).toEqual([]);
    // The ⚙ button steps to a free corner; the profile button has a fixed place, so a clock there is reported.
    const moved = { ...base, items: base.items.map((i) => (i.kind === "clock" ? { ...i, x: 80, y: 2 } : i)) };
    expect(overlaps(moved)).toEqual([]);
    const clash = { ...moved, profiles: [{ id: "kids", name: "Kids" }] };
    expect(overlaps(clash).some(([a, b]) => a.id.startsWith("clock") && b.id === "tv-profile")).toBe(true);
    render(<Harness initial={clash} />);
    expect(screen.getByTestId("overlaps")).toHaveTextContent(/profile button/);
    expect(screen.getByTestId("tv-gear")).toBeInTheDocument();
  });
  it("defaults to the classic home: app strips hidden, launcher home shown, canvas mode brings them back", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    expect(screen.getByTestId("classic-home")).toBeInTheDocument();
    const canvas = screen.getByTestId("canvas");
    expect(canvas.querySelector('[aria-label^="App tiles (apps-1)"]')).toBeNull();
    fireEvent.click(screen.getByRole("radio", { name: /Free canvas tiles/ }));
    expect(last!.home).toBe("canvas");
    expect(screen.queryByTestId("classic-home")).toBeNull();
    expect(canvas.querySelector('[aria-label^="App tiles (apps-1)"]')).not.toBeNull();
  });
  it("says plainly that it is only simulator-tested and what is preview-only", () => {
    render(<Harness />);
    expect(screen.getByText(/only been tested on a simulated TV/i)).toBeInTheDocument();
  });
  it("turns an item off and on", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("button", { name: "Turn on cpu-1" }));
    expect(last!.items.find((i) => i.id === "cpu-1")!.enabled).toBe(true);
    expect(screen.getByText(/CPU \d+%/)).toBeInTheDocument();
  });
  it("adds a custom text item and edits its text", () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: /Custom text/ }));
    fireEvent.change(screen.getByLabelText("Text"), { target: { value: "Movie night" } });
    expect(screen.getAllByText("Movie night").length).toBeGreaterThan(0);
  });
  it("nudges the selected item with the arrow keys and keeps it on screen", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    const clock = screen.getByRole("button", { name: /^Clock \(clock-1\)/ });
    clock.focus();
    const x0 = defaultLayout().items.find((i) => i.id === "clock-1")!.x;
    for (let i = 0; i < 60; i++) fireEvent.keyDown(clock, { key: "ArrowRight", shiftKey: true });
    const c = last!.items.find((i) => i.id === "clock-1")!;
    expect(c.x).toBeGreaterThan(x0);
    expect(c.x + c.w).toBeLessThanOrEqual(100.0001);
  });
  it("changes font, motion and offers the sample videos", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("button", { name: /^Clock \(clock-1\)/ }));
    fireEvent.change(screen.getByLabelText("Font"), { target: { value: "monospace" } });
    fireEvent.change(screen.getByLabelText("Motion"), { target: { value: "float" } });
    const c = last!.items.find((i) => i.id === "clock-1")!;
    expect(c.style.font).toBe("monospace");
    expect(c.motion.type).toBe("float");
    fireEvent.click(screen.getByRole("radio", { name: "video" }));
    fireEvent.click(screen.getByRole("button", { name: /Rainy window/ }));
    expect(last!.background).toMatchObject({ kind: "video", video: "rainy-window" });
  });
});
void vi;

import { themePayload } from "./theme/payload";
describe("themePayload", () => {
  const b64 = async (u: string) => `b64:${u}`;
  const draftWith = (over: Partial<Layout["background"]>, enabled = true) => {
    const d = draftFromCatalog("premium");
    return { ...d, layout: { ...defaultLayout(), enabled, background: { ...defaultLayout().background, ...over } } };
  };
  it("sends no layout when the canvas is off", async () => {
    const p = await themePayload(draftWith({ kind: "video", video: "rain" }, false), true, b64);
    expect(p.layout).toBeNull();
    expect(p.videos).toEqual([]);
  });
  it("keeps the still wallpaper unless the video is explicitly included", async () => {
    const p = await themePayload(draftWith({ kind: "video", video: "rain" }), false, b64);
    expect((p.layout as Layout).background.kind).toBe("image");
    expect(p.videos).toEqual([]);
  });
  it("sends a bundled sample as <id>.mp4 when included", async () => {
    const p = await themePayload(draftWith({ kind: "video", video: "rain" }), true, b64);
    expect(p.videos).toEqual([{ name: "rain.mp4", data_base64: "b64:/videos/rain.mp4" }]);
    expect((p.layout as Layout).background.kind).toBe("video");
  });
  it("sends the person's own video, and refuses a video that is gone", async () => {
    const d = { ...draftWith({ kind: "video", video: "mine.mp4" }), videos: [{ name: "mine.mp4", url: "blob:x" }] };
    expect((await themePayload(d, true, b64)).videos).toEqual([{ name: "mine.mp4", data_base64: "b64:blob:x" }]);
    await expect(themePayload(draftWith({ kind: "video", video: "gone.mp4" }), true, b64)).rejects.toThrow(/no longer/);
  });
});

import { LOOKS } from "./theme/looks";
import { VARIANTS, missingFonts, isFontFile } from "./theme/layout";
describe("looks and new widgets", () => {
  it("every ready-made look is a valid layout that needs no font file", () => {
    expect(LOOKS.length).toBeGreaterThanOrEqual(8);
    const ids = new Set<string>();
    for (const l of LOOKS) {
      expect(validateLayout(l.layout), l.id).toEqual([]);
      expect(ids.has(l.id)).toBe(false); ids.add(l.id);
      expect(missingFonts(l.layout, []), l.id).toEqual([]);
      expect(l.layout.items.every((i) => !isFontFile(i.style.font))).toBe(true);
    }
  });
  it("looks that name a video use a bundled sample", () => {
    for (const l of LOOKS) if (l.layout.background.kind === "video") expect(VIDEO_SAMPLES.map((s) => s.id)).toContain(l.layout.background.video);
  });
  it("every variant of every kind validates, and a wrong one is named", () => {
    for (const [kind, vs] of Object.entries(VARIANTS)) for (const v of vs!) {
      const item = { ...newItem(kind as Parameters<typeof newItem>[0], 1), variant: v };
      expect(validateLayout({ ...defaultLayout(), items: [item] }), `${kind}/${v}`).toEqual([]);
    }
    const bad = { ...newItem("clock", 1), variant: "ring" };
    expect(validateLayout({ ...defaultLayout(), items: [bad] }).join()).toMatch(/not a look/);
  });
  it("font files must be named, and only plain ttf/otf names count", () => {
    const l = defaultLayout();
    const withFont = { ...l, items: [{ ...l.items[0]!, style: { ...l.items[0]!.style, font: "file:Mont.ttf" } }] };
    expect(validateLayout(withFont)).toEqual([]);
    expect(missingFonts(withFont, [])).toEqual(["Mont.ttf"]);
    expect(missingFonts(withFont, ["Mont.ttf"])).toEqual([]);
    expect(validateLayout({ ...l, items: [{ ...l.items[0]!, style: { ...l.items[0]!.style, font: "file:../x.ttf" } }] }).length).toBeGreaterThan(0);
  });
  it("the editor draws new widgets and the chosen look", () => {
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: /Month calendar/ }));
    expect(screen.getAllByRole("table").length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: /Animated bars/ }));
    expect(screen.getAllByLabelText(/Animated bars \(decoration\)/).length).toBeGreaterThan(0);
    fireEvent.change(screen.getByLabelText("Look"), { target: { value: "dots" } });
  });
});

describe("media types", () => {
  it("accepts the TV's video containers and names the ones it can't play", async () => {
    const { isVideoName, UNPLAYABLE_VIDEO } = await import("./theme/media");
    for (const n of ["a.mp4", "b.MKV", "c.mov", "d.webm", "e.ts", "f.3gp", "g.m4v"]) expect(isVideoName(n)).toBe(true);
    expect(isVideoName("x.avi")).toBe(false);
    expect(UNPLAYABLE_VIDEO.test("x.avi")).toBe(true);
    expect(validateLayout({ ...defaultLayout(), background: { ...defaultLayout().background, kind: "video", video: "clip.mkv" } })).toEqual([]);
  });
  it("keeps readable pictures (GIFs included) as they are", async () => {
    const { pictureForTheme } = await import("./theme/media");
    const f = new File([new Uint8Array([71, 73, 70])], "loop.gif", { type: "image/gif" });
    const r = await pictureForTheme(f, async () => "data:image/gif;base64,R0lG");
    expect(r).toEqual({ name: "loop.gif", url: "data:image/gif;base64,R0lG" });
  });
});

describe("colours from the background", () => {
  it("picks the most vivid common hue and lifts it for a dark TV", async () => {
    const { pickAccent } = await import("./theme/palette");
    const px: number[] = [];
    for (let i = 0; i < 50; i++) px.push(200, 40, 40, 255);   // mostly red
    for (let i = 0; i < 10; i++) px.push(40, 40, 200, 255);   // some blue
    for (let i = 0; i < 80; i++) px.push(128, 128, 128, 255); // grey is ignored
    const a = pickAccent(px);
    expect(a).toMatch(/^#[0-9a-f]{6}$/);
    expect(parseInt(a.slice(1, 3), 16)).toBeGreaterThan(parseInt(a.slice(5, 7), 16));
    expect(pickAccent([128, 128, 128, 255])).toBe("#ffffff");
  });
  it("scenes have their own accent and the widgets can follow it", async () => {
    const { accentForBackground, colourWidgets } = await import("./theme/palette");
    const l = { ...defaultLayout(), background: { ...defaultLayout().background, kind: "scene" as const, scene: "aurora" as const } };
    const a = await accentForBackground(l, null, "/videos");
    expect(a).toBe("#34f5a0");
    const c = colourWidgets(l, a!);
    expect(c.accent).toBe(a);
    expect(c.items.find((i) => i.kind === "clock")!.style.color).toBe(a);
    expect(c.items.find((i) => i.kind === "greeting")!.style.color).not.toBe(a);
    expect(validateLayout(c)).toEqual([]);
  });
});

describe("apps panel (scroll mode)", () => {
  it("adds rows, a folder, hides an app and renames one; the layout stays valid", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("radio", { name: /Scroll: recent apps row/ }));
    fireEvent.click(screen.getByRole("button", { name: /Add row/ }));
    fireEvent.click(screen.getByRole("button", { name: /Add row/ }));
    expect(last!.rows).toHaveLength(2);
    fireEvent.change(screen.getByLabelText("Row 2 source"), { target: { value: "music" } });
    expect(last!.rows![1]!.source).toBe("music");
    fireEvent.click(screen.getByRole("button", { name: /Add folder/ }));
    fireEvent.change(screen.getByLabelText("Folder 1 name"), { target: { value: "Kids" } });
    expect(last!.folders![0]!.name).toBe("Kids");
    fireEvent.change(screen.getByLabelText("Name shown on the TV"), { target: { value: "Films" } });
    const pkg = Object.keys(last!.apps ?? {})[0]!;
    expect(last!.apps![pkg]!.name).toBe("Films");
    expect(validateLayout(last!)).toEqual([]);
    // An empty folder is left out, as on the TV.
    expect(screen.getByTestId("all-apps-page")).not.toHaveTextContent(/Kids/);
  });
});

describe("app pictures in the payload", () => {
  it("sends only the pictures the layout names, and fails clearly when one is missing", async () => {
    const { themePayload } = await import("./theme/payload");
    const layout = { ...defaultLayout(), enabled: true, home: "scroll" as const, apps: { "com.netflix.ninja": { image: "com-netflix-ninja.jpg" } } };
    const draft = { name: "x", baseId: "b", config: {}, wallpapers: [], layout, tiles: [{ name: "com-netflix-ninja.jpg", url: "u1" }, { name: "unused.jpg", url: "u2" }] };
    const p = await themePayload(draft, false, async (u) => `b64:${u}`);
    expect(p.tiles).toEqual([{ name: "com-netflix-ninja.jpg", data_base64: "b64:u1" }]);
    await expect(themePayload({ ...draft, tiles: [] }, false, async () => "")).rejects.toThrow(/no longer in this theme/);
  });
});

describe("family: profiles and locked apps", () => {
  function FamilyHarness({ out }: { out: { main?: Layout; layouts?: Record<string, Layout>; editing?: string } }) {
    const [main, setMain] = useState<Layout>({ ...defaultLayout(), enabled: true });
    const [layouts, setLayouts] = useState<Record<string, Layout>>({});
    const [editing, setEditing] = useState("");
    out.main = main; out.layouts = layouts; out.editing = editing;
    return <FamilyBar main={main} onMain={setMain} layouts={layouts} onLayouts={setLayouts} editing={editing} onEditing={setEditing} apps={SAMPLE_APPS} serial={null} />;
  }
  it("adds a profile with its own layout, protects it, and locks an app in it", () => {
    const out: { main?: Layout; layouts?: Record<string, Layout>; editing?: string } = {};
    render(<FamilyHarness out={out} />);
    fireEvent.click(screen.getByRole("button", { name: /Add profile/ }));
    expect(out.main!.profiles).toHaveLength(1);
    const id = out.main!.profiles![0]!.id;
    expect(out.editing).toBe(id);
    expect(out.layouts![id]).toBeTruthy();
    fireEvent.click(screen.getByLabelText(/Ask for the PIN to switch to it/));
    expect(out.main!.profiles![0]!.pin).toBe(true);
    fireEvent.click(screen.getAllByRole("checkbox", { name: new RegExp(SAMPLE_APPS[0]!.label) })[0]!);
    expect(out.layouts![id]!.locked).toEqual([SAMPLE_APPS[0]!.pkg]);
    expect(out.main!.locked ?? []).toEqual([]);
    expect(validateLayout(out.main!)).toEqual([]);
    expect(validateLayout(out.layouts![id]!)).toEqual([]);
  });
  it("sends each listed profile's layout without its own profile list", async () => {
    const { themePayload } = await import("./theme/payload");
    const main = { ...defaultLayout(), enabled: true, profiles: [{ id: "kids", name: "Kids" }] };
    const kids = { ...defaultLayout(), enabled: false, background: { ...defaultLayout().background, kind: "video" as const, video: "rain" } };
    const p = await themePayload({ name: "x", baseId: "b", config: {}, wallpapers: [], layout: main, profileLayouts: { kids, stale: kids } }, false, async () => "");
    expect(p.profiles).toHaveLength(1);
    const sent = p.profiles![0]!.layout as Layout;
    expect(sent.enabled).toBe(true);
    expect(sent.background.kind).toBe("image");
  });
});

describe("status widgets and power-on app", () => {
  it("adds Bluetooth and Now playing widgets and picks an app to open at power-on", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("button", { name: /Bluetooth status/ }));
    fireEvent.click(screen.getByRole("button", { name: /Now playing/ }));
    expect(last!.items.some((i) => i.kind === "bluetooth")).toBe(true);
    expect(last!.items.some((i) => i.kind === "nowplaying")).toBe(true);
    expect(screen.getByText(/media listener/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Open at power-on"), { target: { value: SAMPLE_APPS[0]!.pkg } });
    expect(last!.openOnStart).toBe(SAMPLE_APPS[0]!.pkg);
    expect(validateLayout(last!)).toEqual([]);
  });
});

describe("weather", () => {
  it("adds a weather widget; nothing is fetched until the person agrees, and the layout stays valid", () => {
    let last: Layout | null = null;
    render(<Harness onChange={(l) => (last = l)} />);
    fireEvent.click(screen.getByRole("button", { name: /^\+?\s*Weather$/ }));
    expect(screen.getByTestId("weather-panel")).toBeInTheDocument();
    const consent = screen.getByRole("checkbox", { name: /Let the TV fetch the weather/ });
    expect(consent).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Weather city"), { target: { value: "Ongole" } });
    expect(last!.weather).toMatchObject({ city: "Ongole", consent: false });
    fireEvent.click(screen.getByRole("checkbox", { name: /Let the TV fetch the weather/ }));
    expect(last!.weather!.consent).toBe(true);
    expect(validateLayout(last!)).toEqual([]);
  });
});
