import { useEffect, useMemo, useRef, useState } from "react";
import { Copy, Eye, EyeOff, Film, Plus, Trash2 } from "lucide-react";
import {
  APP_KINDS, isClassic, isScroll, isTvos, withAppleTvLook, withScrollParts, SCENES, SCENE_LABEL, SCENE_TIMES, heavyFeatures,
  ALIGN, FONTS, VARIANTS, VARIANT_LABEL, KINDS, KIND_LABEL, MOTIONS, SHAPES, VIDEO_SAMPLES, clamp, fit, newItem, nextId, validateLayout, overlaps, screenBoxes,
  type Layout, type LayoutItem, type ItemKind,
} from "./layout";
import type { PreviewApp } from "./apps";
import type { Wallpaper } from "./useThemeEditor";
import { Button } from "../components/ui";
import { Item, fontCss, type Stats } from "./CanvasItem";
import { LOOKS } from "./looks";
import { SceneCanvas } from "./SceneCanvas";
import { accentForBackground, colourWidgets } from "./palette";
import { AppsPanel } from "./AppsPanel";
import { MAX_VIDEO_MB, UNPLAYABLE_VIDEO, VIDEO_ACCEPT, isVideoName } from "./media";

interface Props {
  layout: Layout; onChange: (l: Layout) => void; apps: PreviewApp[]; wallpaperUrl: string | null;
  videos: Wallpaper[]; onVideos: (v: Wallpaper[]) => void; sampleBase?: string;
  fonts?: Wallpaper[]; onFonts?: (f: Wallpaper[]) => void;
  /** The theme's first wallpaper is an animated GIF/WebP (counts as a heavy effect). */
  animatedWallpaper?: boolean;
  /** Pictures the person gave single apps (scroll mode). */
  tiles?: Wallpaper[]; onTiles?: (t: Wallpaper[]) => void;
}

type Drag = { id: string; mode: "move" | "size"; sx: number; sy: number; ox: number; oy: number; ow: number; oh: number } | null;

/** Sample numbers for the preview. The real ones come from the TV (Stage 13), so these are labelled as samples. */
const sample = (t: number) => ({ cpu: Math.round(22 + 14 * Math.sin(t / 2)), mem: Math.round(58 + 6 * Math.sin(t / 5)), down: +(4.2 + 2.1 * Math.sin(t / 3)).toFixed(1) });
function useSampleStats(): Stats {
  const [t, setT] = useState(60);
  useEffect(() => { const id = setInterval(() => setT((x) => x + 1), 1000); return () => clearInterval(id); }, []);
  const hist = { cpu: [] as number[], mem: [] as number[], down: [] as number[] };
  for (let i = t - 59; i <= t; i++) { const v = sample(i); hist.cpu.push(v.cpu); hist.mem.push(v.mem); hist.down.push(v.down); }
  return { ...sample(t), hist };
}

function useNow(seconds: boolean) {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => { const id = setInterval(() => setNow(new Date()), seconds ? 1000 : 15000); return () => clearInterval(id); }, [seconds]);
  return now;
}

const motionCss = (it: LayoutItem) => (it.motion.type !== "none" && it.enabled ? `${MOTION_CSS[it.motion.type]} ${4 / it.motion.speed}s ease-in-out infinite` : undefined);
const MOTION_CSS: Record<string, string> = { float: "tvs-float", pulse: "tvs-pulse", slide: "tvs-slide", fade: "tvs-fade", drift: "tvs-drift", parallax: "tvs-float" };

export function LayoutEditor({ layout, onChange, apps, wallpaperUrl, videos, onVideos, sampleBase = "/videos", fonts = [], onFonts, animatedWallpaper = false, tiles = [], onTiles = () => {} }: Props) {
  const [sel, setSel] = useState<string | null>(layout.items[0]?.id ?? null);
  const [drag, setDrag] = useState<Drag>(null);
  const [videoErr, setVideoErr] = useState<string | null>(null);
  const [previewFailed, setPreviewFailed] = useState<string | null>(null);
  const canvas = useRef<HTMLDivElement>(null);
  const videoInput = useRef<HTMLInputElement>(null);
  const fontInput = useRef<HTMLInputElement>(null);

  // Make the person's font files usable in the preview under the name the layout uses.
  useEffect(() => {
    if (typeof FontFace === "undefined") return;
    for (const f of fonts) {
      try { const ff = new FontFace(`tvs-font-${f.name}`, `url(${f.url})`); void ff.load().then(() => document.fonts.add(ff)).catch(() => undefined); } catch { /* preview falls back to a default font */ }
    }
  }, [fonts]);

  function addFont(file: File | undefined) {
    if (!file || !onFonts) return;
    if (!/\.(ttf|otf)$/i.test(file.name) || file.size > 4 * 1024 * 1024) return;
    const safe = file.name.replace(/[^A-Za-z0-9_.-]+/g, "-").replace(/^[.-]+/, "");
    if (!safe) return;
    onFonts([...fonts.filter((f) => f.name !== safe), { name: safe, url: URL.createObjectURL(file) }].slice(-6));
  }
  const reorder = (id: string, dir: -1 | 1) => {
    const i = layout.items.findIndex((x) => x.id === id), j = i + dir;
    if (i < 0 || j < 0 || j >= layout.items.length) return;
    const items = [...layout.items];
    [items[i], items[j]] = [items[j]!, items[i]!];
    onChange({ ...layout, items });
  };
  const stats = useSampleStats();
  const now = useNow(true);
  const problems = useMemo(() => validateLayout(layout), [layout]);
  const clashes = useMemo(() => overlaps(layout), [layout]);
  const clashing = new Set(clashes.flatMap(([a, b]) => [a.id, b.id]));
  const selected = layout.items.find((i) => i.id === sel) ?? null;
  const bg = layout.background;
  const classic = isClassic(layout);
  const scroll = isScroll(layout);

  const patch = (id: string, f: (i: LayoutItem) => LayoutItem) => onChange({ ...layout, items: layout.items.map((i) => (i.id === id ? fit(f(i)) : i)) });
  const setBg = (b: Partial<Layout["background"]>) => onChange({ ...layout, background: { ...bg, ...b } });
  const add = (kind: ItemKind) => {
    const it = { ...newItem(kind, 1), id: nextId(layout.items, kind) };
    onChange({ ...layout, items: [...layout.items, it] });
    setSel(it.id);
  };
  const remove = (id: string) => { onChange({ ...layout, items: layout.items.filter((i) => i.id !== id) }); if (sel === id) setSel(null); };
  const duplicate = (it: LayoutItem) => {
    const c = fit({ ...it, id: nextId(layout.items, it.kind), x: it.x + 3, y: it.y + 3 });
    onChange({ ...layout, items: [...layout.items, c] });
    setSel(c.id);
  };

  function down(e: React.PointerEvent, it: LayoutItem, mode: "move" | "size") {
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    setSel(it.id);
    setDrag({ id: it.id, mode, sx: e.clientX, sy: e.clientY, ox: it.x, oy: it.y, ow: it.w, oh: it.h });
  }
  function move(e: React.PointerEvent) {
    if (!drag || !canvas.current) return;
    const r = canvas.current.getBoundingClientRect();
    const dx = ((e.clientX - drag.sx) / r.width) * 100, dy = ((e.clientY - drag.sy) / r.height) * 100;
    patch(drag.id, (i) => (drag.mode === "move"
      ? { ...i, x: drag.ox + dx, y: drag.oy + dy }
      : { ...i, w: clamp(drag.ow + dx, 3, 100 - drag.ox), h: clamp(drag.oh + dy, 3, 100 - drag.oy) }));
  }
  function key(e: React.KeyboardEvent, it: LayoutItem) {
    const step = e.shiftKey ? 5 : 1;
    const d: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    const v = d[e.key];
    if (v) { e.preventDefault(); patch(it.id, (i) => ({ ...i, x: i.x + v[0], y: i.y + v[1] })); }
    if (e.key === "Delete") remove(it.id);
  }

  function addVideo(f: File | undefined) {
    if (!f) return;
    setVideoErr(null);
    if (UNPLAYABLE_VIDEO.test(f.name)) { setVideoErr(`${f.name}: the TV's player can't open this format. Convert it to MP4 (H.264) first.`); return; }
    if (!isVideoName(f.name)) { setVideoErr(`${f.name}: use MP4, M4V, WebM, MKV, MOV, 3GP or TS.`); return; }
    if (f.size > MAX_VIDEO_MB * 1024 * 1024) { setVideoErr(`${f.name}: ${MAX_VIDEO_MB} MB at most; a short loop is plenty.`); return; }
    const safe = f.name.toLowerCase().replace(/[^a-z0-9_.-]+/g, "-").replace(/^\.+/, "");
    onVideos([...videos.filter((v) => v.name !== safe), { name: safe, url: URL.createObjectURL(f) }].slice(-4));
    setBg({ kind: "video", video: safe });
  }

  const videoUrl = bg.video ? (videos.find((v) => v.name === bg.video)?.url ?? `${sampleBase}/${bg.video}.mp4`) : null;
  const poster = bg.video && VIDEO_SAMPLES.some((s) => s.id === bg.video) ? `${sampleBase}/${bg.video}.jpg` : undefined;

  return (
    <div className="space-y-3">
      <style>{`
        @keyframes tvs-float{0%,100%{transform:translateY(0)}50%{transform:translateY(calc(var(--amt)*-1px))}}
        @keyframes tvs-pulse{0%,100%{transform:scale(1)}50%{transform:scale(calc(1 + var(--amt)/400))}}
        @keyframes tvs-slide{0%,100%{transform:translateX(0)}50%{transform:translateX(calc(var(--amt)*1px))}}
        @keyframes tvs-fade{0%,100%{opacity:1}50%{opacity:.35}}
        @keyframes tvs-drift{0%{transform:translate(0,0)}25%{transform:translate(calc(var(--amt)*1px),calc(var(--amt)*-.5px))}50%{transform:translate(0,calc(var(--amt)*-1px))}75%{transform:translate(calc(var(--amt)*-1px),calc(var(--amt)*-.5px))}100%{transform:translate(0,0)}}
        @media (prefers-reduced-motion: reduce){.tvs-anim{animation:none!important}}
      `}</style>

      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={layout.enabled} onChange={(e) => onChange({ ...layout, enabled: e.target.checked })} />
        Use this canvas layout instead of the standard home screen
      </label>
      <div className="flex flex-wrap items-center gap-2 text-sm" role="radiogroup" aria-label="Apps on Home">
        <span className="text-ink-300">Apps:</span>
        {([["classic", "Classic home (smooth dock, sliding grid, glass focus)"], ["scroll", "Scroll: recent apps row, all apps on Down"], ["tvos", "Apple TV style: Top Shelf, dock of five, grid"], ["canvas", "Free canvas tiles"]] as const).map(([v, label]) => (
          <button key={v} role="radio" aria-checked={(layout.home ?? "classic") === v} onClick={() => onChange(v === "scroll" ? withScrollParts(layout) : v === "tvos" ? withAppleTvLook(layout) : { ...layout, home: v })}
            className={`rounded border px-2 py-1 text-xs ${(layout.home ?? "classic") === v ? "border-accent-400 bg-ink-800" : "border-ink-600"}`}>{label}</button>
        ))}
      </div>
      {!classic && <p className="text-xs text-ink-400">On the TV: the ⚙ button on Home (or the remote's Menu key) opens Home settings, where the style, focus effect, tiles, background, motion, units and clock can be changed with the remote. Those changes last until your next Apply from here.</p>}
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={!!layout.screensaver} onChange={(e) => onChange({ ...layout, screensaver: e.target.checked })} />
        Also use this canvas as the TV's screensaver (background and widgets, no apps)
      </label>
      <label className="flex items-center gap-2 text-sm">
        Open at power-on
        <select aria-label="Open at power-on" value={layout.openOnStart ?? ""} onChange={(e) => onChange({ ...layout, openOnStart: e.target.value || null })}
          className="rounded border border-ink-600 bg-ink-850 px-1 py-0.5 text-xs">
          <option value="">Nothing (show Home)</option>
          {apps.map((a) => <option key={a.pkg} value={a.pkg}>{a.label}</option>)}
        </select>
        <span className="text-xs text-ink-400">once after each start; an app picked on the TV wins</span>
      </label>
      {(layout.items.some((i) => i.kind === "weather") || bg.followWeather || layout.weather) && <WeatherPanel layout={layout} onChange={onChange} />}
      {layout.items.some((i) => i.enabled && i.kind === "nowplaying") && <p className="text-xs text-ink-400">Now playing: Studio switches on the launcher's media listener on the TV while this widget is used (it reads which media is playing, not your notifications) and switches it off again on Revert. In Classic mode the widget shows what plays; its play/pause button works in Canvas and Scroll mode.</p>}
      <LowRamPanel layout={layout} onChange={onChange} animatedWallpaper={animatedWallpaper} />
      <LookPanel layout={layout} onChange={onChange} wallpaperUrl={wallpaperUrl} sampleBase={sampleBase} />
      {isTvos(layout) && <p className="text-xs text-ink-400">Apple TV style: the dock (the “Recently used” item) holds your Favourites row, else your most used apps; the app in focus fills the Top Shelf with its colours, or with the picture you give it under App looks. Icons lean towards the move and catch a shine. Down opens the five-across grid. Choosing this sets the tile style, focus and app-open animation; change any of them below.</p>}
      {scroll && !isTvos(layout) && <p className="text-xs text-ink-400">Scroll mode: your widgets and a sideways “Recently used” row of medium tiles come first (move and resize the row to change the tile size). Press Down on the TV and the page glides to all your apps; Back returns. The App tiles item sets the grid's columns and text style.</p>}
      {classic && <p className="text-xs text-ink-400">Classic keeps the launcher's own animated apps on top; your background and widgets sit behind them. App strips you placed on the canvas are hidden on the TV. Keep widgets in the top-left (clear of the buttons and the shaded area); when the grid slides up it covers the band below the buttons.</p>}
      <p className="rounded-lg border border-ink-700 bg-ink-900 px-3 py-2 text-xs text-ink-300">
        “Apply to TV” sends this canvas to the Studio launcher when the box above is ticked. It has only been tested on a simulated TV so far.
        The numbers and watch-history rows here are samples; the TV shows its real ones (CPU may read “n/a” where the TV hides it from apps, and Continue watching
        needs the TV to allow it). Video is sent only if you tick it in the Apply window, and blur is preview-only.
      </p>

      <details className="rounded-lg border border-ink-700 bg-ink-900 p-3">
        <summary className="cursor-pointer text-sm font-medium">Start from a look ({LOOKS.length})</summary>
        <p className="my-2 text-xs text-ink-400">Replaces the canvas below (Undo brings it back). Everything stays editable. Video looks need “Also send the video” in the Apply window.</p>
        <div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
          {LOOKS.map((l) => (
            <button key={l.id} onClick={() => onChange({ ...l.layout, enabled: true })} aria-label={`Use the look ${l.name}`} title={l.blurb} className="overflow-hidden rounded border border-ink-600 text-left text-xs hover:border-accent-400">
              <Thumb layout={l.layout} apps={apps} wallpaperUrl={wallpaperUrl} stats={stats} now={now} sampleBase={sampleBase} />
              <span className="block px-1 py-0.5">{l.name}</span>
            </button>
          ))}
        </div>
      </details>

      <div ref={canvas} onPointerMove={move} onPointerUp={() => setDrag(null)} onPointerDown={() => setSel(null)} data-testid="canvas"
        className="relative w-full select-none overflow-hidden rounded-lg border border-ink-700" style={{ aspectRatio: "16 / 9", background: bg.kind === "gradient" ? `linear-gradient(135deg, ${bg.color}, ${bg.color2})` : bg.color, touchAction: "none", containerType: "inline-size" }}>
        {bg.kind === "image" && wallpaperUrl && <img src={wallpaperUrl} alt="" className="absolute inset-0 h-full w-full object-cover" style={{ filter: `blur(${bg.blur}px)` }} />}
        {bg.kind === "scene" && <SceneCanvas scene={bg.scene ?? "rain"} time={bg.sceneTime ?? "auto"} />}
        {bg.kind === "video" && videoUrl && <video key={videoUrl} src={videoUrl} poster={poster} autoPlay loop muted playsInline onError={() => setPreviewFailed(videoUrl)} className="absolute inset-0 h-full w-full object-cover" style={{ filter: `blur(${bg.blur}px)` }} />}
        {bg.kind === "video" && videoUrl && previewFailed === videoUrl && <span className="absolute left-[2%] top-[2%] rounded bg-black/60 px-1 text-[11px] text-white/80">This computer can't preview {bg.video}; the TV plays it if it is H.264, H.265, VP8 or VP9.</span>}
        <div className="absolute inset-0" style={{ background: `rgba(0,0,0,${bg.dim})` }} />
        {isTvos(layout) && <TopShelfPreview layout={layout} apps={apps} />}
        {layout.items.filter((it) => !(classic && APP_KINDS.includes(it.kind)) && !(scroll && it.kind === "apps")).map((it) => (
          <div key={it.id} role="button" tabIndex={0} aria-label={`${KIND_LABEL[it.kind]} (${it.id})${it.enabled ? "" : ", off"}`} aria-pressed={sel === it.id}
            onPointerDown={(e) => down(e, it, "move")} onKeyDown={(e) => key(e, it)} onFocus={() => setSel(it.id)}
            className={`absolute cursor-move overflow-hidden outline-none ${sel === it.id ? "ring-2 ring-accent-400" : clashing.has(it.id) && it.enabled ? "ring-2 ring-red-500/80" : "hover:ring-1 hover:ring-white/40"}`}
            style={{ left: `${it.x}%`, top: `${it.y}%`, width: `${it.w}%`, height: `${it.h}%`, opacity: it.enabled ? 1 : 0.25 }}>
            <div style={{ width: "100%", height: "100%", animation: motionCss(it) }} className="tvs-anim">{isTvos(layout) && it.kind === "recent" ? <DockPreview layout={layout} apps={apps} /> : <Item it={it} apps={apps} stats={stats} now={now} />}</div>
            {sel === it.id && <span onPointerDown={(e) => down(e, it, "size")} aria-label="Resize" className="absolute bottom-0 right-0 h-3 w-3 cursor-nwse-resize bg-accent-400" />}
          </div>
        ))}
        {screenBoxes(layout).filter((b) => b.fixed && b.id !== "tv-shelf-title").map((b) => (
          <div key={b.id} data-testid={b.id} title={`On the TV: ${b.label}`} className={`pointer-events-none absolute flex items-center justify-center rounded-full border text-white/80 ${clashing.has(b.id) ? "border-red-500" : "border-white/40"} bg-black/30`}
            style={{ left: `${b.x}%`, top: `${b.y}%`, width: `${b.w}%`, height: `${b.h}%`, fontSize: "1.2cqw" }}>{b.id === "tv-gear" ? "⚙" : "Profile"}</div>
        ))}
        {classic && <ClassicHome apps={apps} />}
      </div>
      {scroll && <AllAppsPage layout={layout} apps={apps} />}
      {scroll && <AppsPanel layout={layout} onChange={onChange} apps={apps} tiles={tiles} onTiles={onTiles} />}
      <p className="text-xs text-ink-400">Drag to move, drag the corner to resize, arrow keys nudge (Shift for bigger steps), Delete removes. Text sizes in the preview scale with the preview width.</p>
      {problems.length > 0 && <p className="text-xs text-warn-400">Layout problems: {problems.slice(0, 3).join("; ")}</p>}
      {clashes.length > 0 && (
        <div className="rounded-lg border border-warn-400/40 bg-ink-900 px-3 py-2 text-xs" data-testid="overlaps">
          <p className="text-warn-400">These overlap on the TV (outlined in red above); move or resize one of each pair:</p>
          <ul className="mt-1 list-disc pl-5 text-ink-300">{clashes.slice(0, 8).map(([a, b]) => <li key={a.id + b.id}>{a.label} ({a.id}) and {b.fixed ? b.label : `${b.label} (${b.id})`}</li>)}</ul>
        </div>
      )}

      <div className="grid gap-3 lg:grid-cols-2">
        <section className="rounded-lg border border-ink-700 bg-ink-900 p-3">
          <h3 className="mb-2 text-sm font-medium">Items</h3>
          <ul className="mb-2 space-y-1">
            {layout.items.map((it) => (
              <li key={it.id} className={`flex items-center gap-1 rounded px-2 py-1 text-sm ${sel === it.id ? "bg-ink-800" : ""}`}>
                <button className="flex-1 text-left" onClick={() => setSel(it.id)}>{KIND_LABEL[it.kind]} <span className="text-xs text-ink-400">{it.id}</span></button>
                <button aria-label={`${it.enabled ? "Turn off" : "Turn on"} ${it.id}`} onClick={() => patch(it.id, (i) => ({ ...i, enabled: !i.enabled }))}>{it.enabled ? <Eye size={14} /> : <EyeOff size={14} />}</button>
                <button aria-label={`Move ${it.id} forward`} onClick={() => reorder(it.id, 1)}>↑</button>
                <button aria-label={`Move ${it.id} back`} onClick={() => reorder(it.id, -1)}>↓</button>
                <button aria-label={`Duplicate ${it.id}`} onClick={() => duplicate(it)}><Copy size={14} /></button>
                <button aria-label={`Delete ${it.id}`} onClick={() => remove(it.id)}><Trash2 size={14} /></button>
              </li>
            ))}
          </ul>
          <div className="flex flex-wrap gap-1">
            {KINDS.map((k) => <Button key={k} onClick={() => add(k)}><Plus size={12} />{KIND_LABEL[k]}</Button>)}
          </div>
        </section>

        <section className="rounded-lg border border-ink-700 bg-ink-900 p-3">
          <h3 className="mb-2 text-sm font-medium">Background</h3>
          <div className="mb-2 flex flex-wrap gap-1" role="radiogroup" aria-label="Background type">
            {(["image", "video", "scene", "color", "gradient"] as const).map((k) => (
              <button key={k} role="radio" aria-checked={bg.kind === k} onClick={() => setBg(k === "scene" ? { kind: k, scene: bg.scene ?? "rain", sceneTime: bg.sceneTime ?? "auto" } : { kind: k })}
                className={`rounded border px-2 py-1 text-xs ${bg.kind === k ? "border-accent-400 text-accent-400" : "border-ink-600"}`}>{k === "scene" ? "ambient scene" : k}</button>
            ))}
          </div>
          {bg.kind === "scene" && (
            <div className="mb-2 space-y-2">
              <div className="grid grid-cols-4 gap-2" role="radiogroup" aria-label="Ambient scene">
                {SCENES.map((id) => (
                  <button key={id} role="radio" aria-checked={(bg.scene ?? "rain") === id} onClick={() => setBg({ scene: id })}
                    className={`overflow-hidden rounded border text-left text-xs ${(bg.scene ?? "rain") === id ? "border-accent-400" : "border-ink-600"}`}>
                    <span className="relative block h-12 w-full"><SceneCanvas scene={id} time={bg.sceneTime ?? "auto"} animate={false} /></span>
                    <span className="block px-1 py-0.5">{SCENE_LABEL[id]}</span>
                  </button>
                ))}
              </div>
              <Select label="Time of day" value={bg.sceneTime ?? "auto"} options={SCENE_TIMES} onChange={(v) => setBg({ sceneTime: v })} />
              <label className="flex items-center gap-2 text-xs"><input type="checkbox" checked={!!bg.followWeather} onChange={(e) => setBg({ followWeather: e.target.checked })} /> Turn to rain or snow when it rains or snows (needs weather)</label>
              <p className="text-[11px] text-ink-400">Drawn live on the TV (no video file). “auto” follows the clock: sunrise, day, sunset and night colours.</p>
            </div>
          )}
          {bg.kind === "video" && (
            <div className="mb-2 space-y-2">
              <div className="grid grid-cols-2 gap-2">
                {VIDEO_SAMPLES.map((s) => (
                  <button key={s.id} onClick={() => setBg({ video: s.id })} aria-pressed={bg.video === s.id} title={s.blurb}
                    className={`overflow-hidden rounded border text-left text-xs ${bg.video === s.id ? "border-accent-400" : "border-ink-600"}`}>
                    <img src={`${sampleBase}/${s.id}.jpg`} alt="" className="h-14 w-full object-cover" /><span className="block px-1 py-0.5">{s.name}</span>
                  </button>
                ))}
              </div>
              {videos.map((v) => <button key={v.name} onClick={() => setBg({ video: v.name })} className={`block text-xs ${bg.video === v.name ? "text-accent-400" : ""}`}><Film size={12} className="mr-1 inline" />{v.name}</button>)}
              <Button onClick={() => videoInput.current?.click()}><Film size={14} />Add your own video</Button>
              {videoErr && <p className="w-full text-xs text-warn-400">{videoErr}</p>}
              <input ref={videoInput} type="file" accept={VIDEO_ACCEPT} hidden onChange={(e) => { addVideo(e.target.files?.[0]); e.target.value = ""; }} />
              <p className="text-[11px] text-ink-400">Short loops work best (under 15 s, 1080p or less, H.264 MP4). Video costs the TV extra CPU and memory; weak TVs may stutter, so check with a benchmark before keeping it.</p>
            </div>
          )}
          {(bg.kind === "color" || bg.kind === "gradient") && (
            <div className="mb-2 flex gap-2 text-xs">
              <label>Colour <input type="color" value={bg.color.slice(0, 7)} onChange={(e) => setBg({ color: e.target.value })} /></label>
              {bg.kind === "gradient" && <label>Second <input type="color" value={bg.color2.slice(0, 7)} onChange={(e) => setBg({ color2: e.target.value })} /></label>}
            </div>
          )}
          <Slider label="Dim" value={bg.dim} min={0} max={0.9} step={0.05} onChange={(v) => setBg({ dim: v })} />
          <Slider label="Blur (preview only)" value={bg.blur} min={0} max={24} step={1} onChange={(v) => setBg({ blur: v })} />
        </section>
      </div>

      {selected && (
        <section className="rounded-lg border border-ink-700 bg-ink-900 p-3" aria-label={`Settings for ${selected.id}`}>
          <h3 className="mb-2 text-sm font-medium">{KIND_LABEL[selected.kind]} <span className="text-xs text-ink-400">{selected.id}</span></h3>
          <div className="grid gap-x-4 gap-y-1 sm:grid-cols-2">
            <Slider label="Left %" value={selected.x} min={0} max={100} step={0.5} onChange={(v) => patch(selected.id, (i) => ({ ...i, x: v }))} />
            <Slider label="Top %" value={selected.y} min={0} max={100} step={0.5} onChange={(v) => patch(selected.id, (i) => ({ ...i, y: v }))} />
            <Slider label="Width %" value={selected.w} min={3} max={100} step={0.5} onChange={(v) => patch(selected.id, (i) => ({ ...i, w: v }))} />
            <Slider label="Height %" value={selected.h} min={3} max={100} step={0.5} onChange={(v) => patch(selected.id, (i) => ({ ...i, h: v }))} />
            {selected.kind === "text" && (
              <label className="text-xs sm:col-span-2">Text <input aria-label="Text" maxLength={80} value={selected.text ?? ""} onChange={(e) => patch(selected.id, (i) => ({ ...i, text: e.target.value }))} className="ml-2 w-64 rounded border border-ink-600 bg-ink-850 px-2 py-1" /></label>
            )}
            {VARIANTS[selected.kind] && (
              <label className="flex items-center gap-2 text-xs">
                <span className="w-24 shrink-0">Look</span>
                <select aria-label="Look" value={selected.variant ?? VARIANTS[selected.kind]![0]} onChange={(e) => patch(selected.id, (i) => ({ ...i, variant: e.target.value }))} className="flex-1 rounded border border-ink-600 bg-ink-850 px-2 py-1">
                  {VARIANTS[selected.kind]!.map((v) => <option key={v} value={v}>{VARIANT_LABEL[v] ?? v}</option>)}
                </select>
              </label>
            )}
            <label className="flex items-center gap-2 text-xs">
              <span className="w-24 shrink-0">Font</span>
              <select aria-label="Font" value={selected.style.font} onChange={(e) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, font: e.target.value } }))} className="flex-1 rounded border border-ink-600 bg-ink-850 px-2 py-1">
                {FONTS.map((o) => <option key={o} value={o}>{o}</option>)}
                {fonts.map((f) => <option key={f.name} value={`file:${f.name}`}>{f.name} (your file)</option>)}
              </select>
            </label>
            {onFonts && (
              <div className="text-xs sm:col-span-2">
                <Button onClick={() => fontInput.current?.click()}>Add a font file (.ttf / .otf)</Button>
                <input ref={fontInput} type="file" accept=".ttf,.otf,font/ttf,font/otf" hidden onChange={(e) => { addFont(e.target.files?.[0]); e.target.value = ""; }} />
                <span className="ml-2 text-ink-400">Use fonts you have the right to use; the file is copied to the TV.</span>
              </div>
            )}
            <Slider label="Letter spacing" value={selected.style.spacing ?? 0} min={0} max={1} step={0.05} onChange={(v) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, spacing: v } }))} />
            <div className="flex gap-3 text-xs">
              <label><input type="checkbox" checked={!!selected.style.upper} onChange={(e) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, upper: e.target.checked } }))} /> CAPITALS</label>
              <label><input type="checkbox" checked={!!selected.style.shadow} onChange={(e) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, shadow: e.target.checked } }))} /> Shadow</label>
              <label><input type="checkbox" checked={!!selected.style.italic} onChange={(e) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, italic: e.target.checked } }))} /> Italic</label>
            </div>
            {(selected.kind === "shape" || selected.kind === "clock") && (
              <Slider label="Line / outline" value={selected.options?.stroke ?? 0} min={0} max={40} step={1} onChange={(v) => patch(selected.id, (i) => ({ ...i, options: { ...i.options, stroke: v } }))} />
            )}
            {selected.kind === "visualizer" && (
              <Slider label="Bars" value={selected.options?.bars ?? 48} min={8} max={96} step={1} onChange={(v) => patch(selected.id, (i) => ({ ...i, options: { ...i.options, bars: v } }))} />
            )}
            {selected.kind === "calendar" && (
              <label className="text-xs"><input type="checkbox" checked={!!selected.options?.weekStartsMonday} onChange={(e) => patch(selected.id, (i) => ({ ...i, options: { ...i.options, weekStartsMonday: e.target.checked } }))} /> Week starts on Monday</label>
            )}
            {(selected.kind === "apps" || selected.kind === "recent") && (
              <details className="text-xs sm:col-span-2">
                <summary className="cursor-pointer">Choose which apps ({selected.options?.packages?.length ? `${selected.options.packages.length} picked` : "all"})</summary>
                <div className="mt-1 max-h-40 overflow-y-auto">
                  {apps.map((a) => {
                    const picked = selected.options?.packages ?? [];
                    return (
                      <label key={a.pkg} className="block"><input type="checkbox" checked={picked.includes(a.pkg)} onChange={(e) => patch(selected.id, (i) => ({ ...i, options: { ...i.options, packages: e.target.checked ? [...picked, a.pkg].slice(0, 24) : picked.filter((p) => p !== a.pkg) } }))} /> {a.label}</label>
                    );
                  })}
                </div>
                <Button onClick={() => patch(selected.id, (i) => ({ ...i, options: { ...i.options, packages: [] } }))}>Show all apps</Button>
              </details>
            )}
            <Slider label="Font size" value={selected.style.size} min={8} max={400} step={1} onChange={(v) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, size: v } }))} />
            <label className="text-xs">Colour <input type="color" aria-label="Text colour" value={selected.style.color.slice(0, 7)} onChange={(e) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, color: e.target.value } }))} /></label>
            <Slider label="Opacity" value={selected.style.opacity} min={0.05} max={1} step={0.05} onChange={(v) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, opacity: v } }))} />
            <Slider label="Corner radius" value={selected.style.radius} min={0} max={80} step={1} onChange={(v) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, radius: v } }))} />
            <Select label="Align" value={selected.style.align} options={ALIGN} onChange={(v) => patch(selected.id, (i) => ({ ...i, style: { ...i.style, align: v } }))} />
            {selected.icon && <>
              <Slider label="Icon size" value={selected.icon.size} min={24} max={256} step={2} onChange={(v) => patch(selected.id, (i) => ({ ...i, icon: { ...i.icon!, size: v } }))} />
              <Select label="Icon shape" value={selected.icon.shape} options={SHAPES} onChange={(v) => patch(selected.id, (i) => ({ ...i, icon: { ...i.icon!, shape: v } }))} />
              <Slider label="Columns" value={selected.icon.columns} min={1} max={12} step={1} onChange={(v) => patch(selected.id, (i) => ({ ...i, icon: { ...i.icon!, columns: v } }))} />
              <label className="text-xs"><input type="checkbox" checked={selected.icon.labels} onChange={(e) => patch(selected.id, (i) => ({ ...i, icon: { ...i.icon!, labels: e.target.checked } }))} /> Show app names</label>
            </>}
            {selected.kind === "clock" && <>
              <label className="text-xs"><input type="checkbox" checked={!!selected.options?.twentyFour} onChange={(e) => patch(selected.id, (i) => ({ ...i, options: { ...i.options, twentyFour: e.target.checked } }))} /> 24-hour</label>
              <label className="text-xs"><input type="checkbox" checked={!!selected.options?.seconds} onChange={(e) => patch(selected.id, (i) => ({ ...i, options: { ...i.options, seconds: e.target.checked } }))} /> Seconds</label>
            </>}
            <Select label="Motion" value={selected.motion.type} options={MOTIONS} onChange={(v) => patch(selected.id, (i) => ({ ...i, motion: { ...i.motion, type: v } }))} />
            <Slider label="Motion speed" value={selected.motion.speed} min={0.25} max={4} step={0.25} onChange={(v) => patch(selected.id, (i) => ({ ...i, motion: { ...i.motion, speed: v } }))} />
            <Slider label="Motion amount" value={selected.motion.amount} min={0} max={100} step={1} onChange={(v) => patch(selected.id, (i) => ({ ...i, motion: { ...i.motion, amount: v } }))} />
          </div>
        </section>
      )}
    </div>
  );
}

/** Where the launcher's own Home sits in classic mode: buttons top right, the focused app's name, the dock row
 *  and the grid peeking below (it slides up on Down). Drawn over the canvas and click-through. */
function ClassicHome({ apps, thumb = false }: { apps: PreviewApp[]; thumb?: boolean }) {
  const dock = apps.slice(0, 4);
  // Positions measured on a 1080p Google TV running the launcher.
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0" data-testid={thumb ? undefined : "classic-home"}>
      <div className="absolute flex gap-[1.2%]" style={{ left: "55%", top: "11%", width: "39%", height: "7%" }}>
        {["Inputs", "Network", "Settings"].map((n) => <span key={n} className="flex flex-1 items-center justify-center rounded-full border border-white/30 bg-black/30 text-white/85" style={{ fontSize: "1.4cqw" }}>{n}</span>)}
      </div>
      <div className="absolute font-light text-white" style={{ left: "7%", top: "50%", fontSize: "4.2cqw", lineHeight: 1 }}>{dock[0]?.label ?? "Netflix"}</div>
      <div className="absolute flex gap-[2%] rounded-[1.4cqw] bg-black/40 p-[1%] ring-1 ring-white/10" style={{ left: "6%", right: "6%", top: "64%", height: "24%" }}>
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className={`flex flex-1 items-center justify-center overflow-hidden rounded-[1cqw] ${i === 0 ? "ring-2 ring-white/90 scale-105" : ""}`} style={{ background: dock[i]?.color ?? "rgba(255,255,255,.15)" }}>
            <span className="font-semibold text-white" style={{ fontSize: "1.8cqw" }}>{dock[i]?.label ?? "App"}</span>
          </div>
        ))}
      </div>
      <div className="absolute flex gap-[2%]" style={{ left: "8%", right: "8%", top: "96%", height: "8%" }}>
        {[0, 1, 2, 3].map((i) => <div key={i} className="flex-1 rounded-t-[1cqw] bg-white/20" />)}
      </div>
      {!thumb && <div className="absolute inset-x-0 bottom-0 top-[46%] bg-accent-400/5 outline-dashed outline-1 outline-accent-400/40" />}
      {!thumb && <span className="absolute bottom-[2%] right-[2%] rounded bg-black/50 px-1 text-white/70" style={{ fontSize: "1cqw" }}>Launcher home: apps slide up on Down</span>}
    </div>
  );
}

/** The apps the TV's dock shows: the first Favourites row, else the first apps (the TV uses the most recent). */
function dockApps(layout: Layout, apps: PreviewApp[]): PreviewApp[] {
  const fav = (layout.rows ?? []).find((r) => r.source === "favorites" && r.packages?.length);
  const hidden = new Set(layout.hidden ?? []);
  const named = (a: PreviewApp) => ({ ...a, label: layout.apps?.[a.pkg]?.name || a.label, color: layout.apps?.[a.pkg]?.color || a.color });
  const pool = apps.filter((a) => !hidden.has(a.pkg)).map(named);
  const picked = fav ? fav.packages!.map((p) => pool.find((a) => a.pkg === p)).filter((a): a is PreviewApp => !!a) : [];
  return (picked.length ? picked : pool).slice(0, 5);
}

/** Apple TV style: the focused (first) dock app washes the top of the screen with its colour and name. */
function TopShelfPreview({ layout, apps }: { layout: Layout; apps: PreviewApp[] }) {
  const first = dockApps(layout, apps)[0];
  if (!first) return null;
  return (
    <div data-testid="top-shelf" className="pointer-events-none absolute inset-x-0 top-0 h-[62%]" style={{ background: `linear-gradient(135deg, ${first.color}, transparent 85%)` }}>
      <div className="absolute inset-0" style={{ background: `linear-gradient(to bottom, transparent 40%, ${layout.background.color})` }} />
      <span className="absolute left-[4.5%] top-[42%] font-medium text-white" style={{ fontSize: "3.3cqw", textShadow: "0 0.2cqw 1cqw rgba(0,0,0,.6)" }}>{first.label}</span>
    </div>
  );
}

/** Apple TV style: five wide rounded icons on frosted glass, the first one lifted with a shine. */
function DockPreview({ layout, apps }: { layout: Layout; apps: PreviewApp[] }) {
  const list = dockApps(layout, apps);
  return (
    <div data-testid="dock-preview" className="flex h-full w-full items-start gap-[2.2%] rounded-[1.8cqw] border border-white/20 bg-white/10 px-[2.5%] pt-[2%]" style={{ backdropFilter: "blur(8px)" }}>
      {list.map((a, i) => (
        <div key={a.pkg} className="flex min-w-0 flex-1 flex-col items-center gap-[6%]">
          <div className="relative flex w-full items-center justify-center overflow-hidden font-semibold text-white"
            style={{ aspectRatio: "5 / 3", background: a.color, borderRadius: "1.1cqw", fontSize: "1.3cqw", transform: i === 0 ? "scale(1.13)" : undefined, boxShadow: i === 0 ? "0 1.2cqw 2.4cqw rgba(0,0,0,.55)" : undefined }}>
            {a.label}
            {i === 0 && <span className="absolute inset-0" style={{ background: "linear-gradient(135deg, transparent 30%, rgba(255,255,255,.35) 50%, transparent 70%)" }} />}
          </div>
          {i === 0 && <span className="mt-[6%] truncate text-white" style={{ fontSize: "1.1cqw" }}>{a.label}</span>}
        </div>
      ))}
    </div>
  );
}

/** Scroll mode's second screen, as the TV shows it after Down: every app in rows of banner tiles. */
function AllAppsPage({ layout, apps: allApps }: { layout: Layout; apps: PreviewApp[] }) {
  const hidden = new Set(layout.hidden ?? []);
  const foldered = new Set((layout.folders ?? []).flatMap((f) => f.packages));
  const named = (a: PreviewApp) => ({ ...a, label: layout.apps?.[a.pkg]?.name || a.label, color: layout.apps?.[a.pkg]?.color || a.color });
  const visible = allApps.filter((a) => !hidden.has(a.pkg)).map(named);
  const folderCells: PreviewApp[] = (layout.folders ?? []).filter((f) => f.packages.some((p) => !hidden.has(p))).map((f) => ({ pkg: `folder:${f.name}`, label: `▦ ${f.name}`, color: "rgba(255,255,255,.25)" }));
  const apps = [...folderCells, ...visible.filter((a) => !foldered.has(a.pkg))];
  const rowNames = (layout.rows ?? []).map((r) => r.title);
  const grid = layout.items.find((i) => i.kind === "apps");
  const cols = isTvos(layout) ? 5 : Math.max(4, Math.min(8, grid?.icon?.columns ?? 6));
  const bg = layout.background;
  return (
    <div data-testid="all-apps-page" className="relative w-full overflow-hidden rounded-lg border border-ink-700 p-[3%]"
      style={{ aspectRatio: "16 / 9", containerType: "inline-size", background: bg.kind === "gradient" ? `linear-gradient(135deg, ${bg.color}, ${bg.color2})` : bg.color }}>
      {rowNames.length > 0 && <div className="mb-[1%] text-white/60" style={{ fontSize: "1.3cqw" }}>Rows above: {rowNames.join(" · ")}</div>}
      <div className="mb-[1.5%] text-white/85" style={{ fontSize: "2.2cqw", fontFamily: grid ? fontCss(grid.style.font).family : undefined }}>All apps</div>
      <div className="grid gap-[1.6%]" style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}>
        {apps.slice(0, cols * 4).map((a, i) => {
          const style = layout.tiles ?? "banner", shape = layout.iconShape ?? "rounded";
          const radius = shape === "circle" ? "9999px" : shape === "square" ? "0" : shape === "squircle" ? "28%" : shape === "leaf" ? "40% 8% 40% 8%" : "0.8cqw";
          const aspect = style === "icon" ? "1 / 1" : style === "poster" ? "2 / 3" : style === "tvos" ? "5 / 3" : "16 / 9";
          const ring = i === 0 ? { boxShadow: `0 0 0 0.25cqw ${layout.accent ?? "#ffffff"}`, transform: "scale(1.1)" } : {};
          return (
            <div key={a.pkg} className="flex flex-col items-center gap-[4%]">
              <div className="flex w-full items-center justify-center font-semibold text-white" style={{ background: a.color, aspectRatio: aspect, borderRadius: radius, fontSize: "1.3cqw", width: style === "icon" || style === "poster" ? "62%" : "100%", ...ring }}>
                {style === "icon" ? a.label.slice(0, 1) : a.label}
              </div>
              {style === "icon" && <span className="text-white/80" style={{ fontSize: "1cqw" }}>{a.label}</span>}
            </div>
          );
        })}
      </div>
      <span className="absolute bottom-[2%] right-[2%] rounded bg-black/50 px-1 text-white/70" style={{ fontSize: "1cqw" }}>After Down: the page glides up to every app</span>
    </div>
  );
}

/** Weather: the city, units, and the person's agreement that the TV may contact Open-Meteo. */
function WeatherPanel({ layout, onChange }: { layout: Layout; onChange: (l: Layout) => void }) {
  const w = layout.weather ?? { city: "", units: "c" as const, consent: false };
  const set = (p: Partial<NonNullable<Layout["weather"]>>) => onChange({ ...layout, weather: { ...w, ...p } });
  return (
    <div className="rounded-lg border border-ink-700 bg-ink-900 p-3 text-xs" data-testid="weather-panel">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">Weather</span>
        <input aria-label="Weather city" placeholder="City, e.g. Ongole" value={w.city} maxLength={60} onChange={(e) => set({ city: e.target.value })}
          className="w-44 rounded border border-ink-600 bg-ink-850 px-2 py-0.5" />
        <select aria-label="Temperature units" value={w.units ?? "c"} onChange={(e) => set({ units: e.target.value as "c" | "f" })} className="rounded border border-ink-600 bg-ink-850 px-1 py-0.5">
          <option value="c">°C</option><option value="f">°F</option>
        </select>
      </div>
      <label className="mt-2 flex items-start gap-2">
        <input type="checkbox" checked={!!w.consent} disabled={!w.city.trim()} onChange={(e) => set({ consent: e.target.checked })} />
        <span>Let the TV fetch the weather from Open-Meteo (open-meteo.com): it sends the city name once to find it, then its coordinates every 30 minutes while Home is on screen. No account, no location permission. Off until you tick this; the launcher gets internet access only for this.</span>
      </label>
    </div>
  );
}

/** Colours from the background, and (scroll mode) the tile style, icon shape, focus style and app-open animation. */
function LookPanel({ layout, onChange, wallpaperUrl, sampleBase }: { layout: Layout; onChange: (l: Layout) => void; wallpaperUrl: string | null; sampleBase: string }) {
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  async function pick(widgets: boolean) {
    setBusy(true); setNote(null);
    const a = await accentForBackground(layout, wallpaperUrl, sampleBase);
    setBusy(false);
    if (!a) { setNote("Couldn't read colours from this background."); return; }
    onChange(widgets ? colourWidgets(layout, a) : { ...layout, accent: a });
  }
  return (
    <details className="rounded-lg border border-ink-700 bg-ink-900 p-3 text-sm" open>
      <summary className="cursor-pointer font-medium">Look: colours, tiles and animations</summary>
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
        <span>Accent</span>
        <span aria-label="Accent colour" className="inline-block h-5 w-5 rounded border border-ink-600" style={{ background: layout.accent ?? "#ffffff" }} />
        <Button disabled={busy} onClick={() => void pick(false)}>Pick from background</Button>
        <Button disabled={busy} onClick={() => void pick(true)}>…and colour the widgets</Button>
        <input aria-label="Accent colour picker" type="color" value={(layout.accent ?? "#ffffff").slice(0, 7)} onChange={(e) => onChange({ ...layout, accent: e.target.value })} />
        {layout.accent && <button className="underline" onClick={() => onChange({ ...layout, accent: null })}>white</button>}
        {note && <span className="text-warn-400">{note}</span>}
      </div>
      {isScroll(layout) ? (
        <div className="mt-2 grid gap-2 sm:grid-cols-2">
          <Select label="Tile style" value={layout.tiles ?? "banner"} options={["banner", "card", "icon", "poster", "tvos"] as const} onChange={(v) => onChange({ ...layout, tiles: v })} />
          <Select label="Icon shape" value={layout.iconShape ?? "rounded"} options={["rounded", "circle", "squircle", "square", "leaf"] as const} onChange={(v) => onChange({ ...layout, iconShape: v })} />
          <Select label="Focus style" value={layout.focus ?? "pop"} options={["pop", "lift", "glow", "border", "bounce", "tilt", "zoom", "parallax"] as const} onChange={(v) => onChange({ ...layout, focus: v })} />
          <Select label="App opens with" value={layout.launch ?? "default"} options={["default", "zoom", "reveal", "fade", "slide", "grow", "none"] as const} onChange={(v) => onChange({ ...layout, launch: v })} />
        </div>
      ) : <p className="mt-2 text-xs text-ink-400">Tile style, icon shape, focus style and app-open animation apply in Scroll mode. The accent colours focus rings there.</p>}
    </details>
  );
}

/** Heavy effects: what this layout has, and the switch that keeps them on for TVs with little memory. */
function LowRamPanel({ layout, onChange, animatedWallpaper }: { layout: Layout; onChange: (l: Layout) => void; animatedWallpaper: boolean }) {
  const heavy = heavyFeatures(layout, animatedWallpaper);
  if (heavy.length === 0) return null;
  return (
    <div className="rounded-lg border border-warn-400/40 bg-ink-900 px-3 py-2 text-xs" data-testid="low-ram-panel">
      <p className="text-ink-200">Heavy effects in this layout: {heavy.join(", ")}.</p>
      <p className="text-ink-400">On TVs with under 2 GB of memory they are switched off (still picture, no motion) so Home stays quick. Focus and scrolling animations stay on everywhere.</p>
      <label className="mt-1 flex items-center gap-2">
        <input type="checkbox" checked={!!layout.heavyOnLowRam} onChange={(e) => onChange({ ...layout, heavyOnLowRam: e.target.checked })} />
        Keep them on even on low-memory TVs <span className="text-warn-400">(may stutter or make apps reload)</span>
      </label>
    </div>
  );
}

function Slider({ label, value, min, max, step, onChange }: { label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void }) {
  return (
    <label className="flex items-center gap-2 text-xs">
      <span className="w-24 shrink-0">{label}</span>
      <input type="range" aria-label={label} min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} className="flex-1" />
      <span className="w-10 text-right tabular-nums text-ink-400">{Math.round(value * 100) / 100}</span>
    </label>
  );
}

function Select<T extends string>({ label, value, options, onChange }: { label: string; value: T; options: readonly T[]; onChange: (v: T) => void }) {
  return (
    <label className="flex items-center gap-2 text-xs">
      <span className="w-24 shrink-0">{label}</span>
      <select aria-label={label} value={value} onChange={(e) => onChange(e.target.value as T)} className="flex-1 rounded border border-ink-600 bg-ink-850 px-2 py-1">
        {options.map((o) => <option key={o} value={o}>{o}</option>)}
      </select>
    </label>
  );
}

function Thumb({ layout, apps, wallpaperUrl, stats, now, sampleBase }: { layout: Layout; apps: PreviewApp[]; wallpaperUrl: string | null; stats: Stats; now: Date; sampleBase: string }) {
  const bg = layout.background;
  const back = bg.kind === "gradient" ? `linear-gradient(135deg, ${bg.color}, ${bg.color2})` : bg.color;
  const poster = bg.kind === "video" && bg.video ? `${sampleBase}/${bg.video}.jpg` : bg.kind === "image" ? wallpaperUrl : null;
  return (
    <div className="pointer-events-none relative w-full overflow-hidden" style={{ aspectRatio: "16 / 9", background: back, containerType: "inline-size" }}>
      {poster && <img src={poster} alt="" className="absolute inset-0 h-full w-full object-cover" />}
      {bg.kind === "scene" && <SceneCanvas scene={bg.scene ?? "rain"} time={bg.sceneTime ?? "auto"} animate={false} />}
      <div className="absolute inset-0" style={{ background: `rgba(0,0,0,${bg.dim})` }} />
      {layout.items.filter((i) => i.enabled && !(isClassic(layout) && APP_KINDS.includes(i.kind)) && !(isScroll(layout) && i.kind === "apps")).map((i) => (
        <div key={i.id} className="absolute overflow-hidden" style={{ left: `${i.x}%`, top: `${i.y}%`, width: `${i.w}%`, height: `${i.h}%` }}>
          <Item it={i} apps={apps} stats={stats} now={now} animate={false} />
        </div>
      ))}
      {isClassic(layout) && <ClassicHome apps={apps} thumb />}
    </div>
  );
}
