import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { Settings2, Tv, Wifi } from "lucide-react";
import { value, type ThemeConfig } from "./schema";
import type { PreviewApp } from "./apps";

/** Desktop rendering of the Studio launcher's home screen from the same keys the launcher reads.
 * It is an approximation of layout and look; app artwork is drawn by the TV. */
export interface PreviewProps {
  config: ThemeConfig;
  apps: PreviewApp[];
  wallpaperUrl: string | null;
  interactive?: boolean;
  /** Pixel width; height follows 16:9. */
  width: number;
}

type Area = "top" | "dock" | "grid";
const V_W = 1920;
const V_H = 1080;

export function Preview({ config, apps, wallpaperUrl, interactive = false, width }: PreviewProps) {
  const c = <T,>(k: string) => value(config, k) as T;
  const columns = Math.max(1, Math.min(8, c<number>("columns") ?? 4));
  const hidden = new Set(c<string[]>("hidden_apps") ?? []);
  const topBarApps = (c<string[]>("top_bar_apps") ?? []).filter((p) => apps.some((a) => a.pkg === p));
  const visible = apps.filter((a) => !hidden.has(a.pkg) && !topBarApps.includes(a.pkg));
  const dockList = c<string[]>("dock_apps") ?? [];
  const dockNamed = dockList.map((p) => visible.find((a) => a.pkg === p)).filter((a): a is PreviewApp => !!a);
  const rest = visible.filter((a) => !dockNamed.includes(a));
  const fill = c<boolean>("fill_dock") !== false;
  const dock = [...dockNamed, ...(fill ? rest.slice(0, Math.max(0, columns - dockNamed.length)) : [])].slice(0, columns);
  const grid = visible.filter((a) => !dock.includes(a));
  const singleRow = c<boolean>("single_row") === true;
  const dockRow = c<boolean>("dock_row") !== false;
  const rowsBelow = Math.max(0, Math.min(3, c<number>("rows_below_dock") ?? 0));

  const [area, setArea] = useState<Area>("dock");
  const [index, setIndex] = useState(0);
  const items: Record<Area, number> = { top: 3, dock: singleRow ? visible.length : dock.length, grid: grid.length };
  useEffect(() => { if (index >= Math.max(1, items[area])) setIndex(0); }, [area, index, items]);

  function onKey(e: KeyboardEvent) {
    if (!interactive) return;
    const k = e.key;
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(k)) return;
    e.preventDefault();
    if (k === "ArrowLeft") setIndex((i) => Math.max(0, i - 1));
    if (k === "ArrowRight") setIndex((i) => Math.min(items[area] - 1, i + 1));
    if (k === "ArrowUp") {
      if (area === "grid" && index >= columns) setIndex(index - columns);
      else if (area === "grid") { setArea("dock"); setIndex(Math.min(index, dock.length - 1)); }
      else if (area === "dock") { setArea("top"); setIndex(0); }
    }
    if (k === "ArrowDown") {
      if (area === "top") { setArea("dock"); setIndex(0); }
      else if (area === "dock" && grid.length && !singleRow) { setArea("grid"); setIndex(Math.min(index, grid.length - 1)); }
      else if (area === "grid" && index + columns < grid.length) setIndex(index + columns);
    }
  }

  const focusedApp = area === "dock" ? (singleRow ? visible[index] : dock[index]) : area === "grid" ? grid[index] : undefined;
  const glass = c<string>("glass_style") ?? "glass";
  const appGlow = c<boolean>("app_glow") === true;
  const motion = c<boolean>("animations") !== false;
  const reduced = c<boolean>("reduced_motion") === true;
  const transition = !motion ? "none" : reduced ? "all 90ms linear" : c<boolean>("focus_overshoot") !== false ? "all 260ms cubic-bezier(.34,1.56,.64,1)" : "all 200ms ease-out";
  const names = c<boolean>("always_show_names") === true;
  const uniform = c<boolean>("uniform_tiles") === true;
  const scale = width / V_W;

  const tileW = (V_W - 240 - (columns - 1) * 36) / columns;
  const tileH = tileW * 9 / 16;
  const now = useMemo(() => new Date(2026, 9, 26, 21, 41), []);
  const greeting = c<string>("greeting_name");
  const wallpaperOn = c<boolean>("wallpaper") !== false && !!wallpaperUrl;
  const dim = c<number>("wallpaper_dim") ?? 0;
  const sky = c<number>("sky_intensity") ?? 0;
  const bandAlpha = (c<number>("dock_opacity") ?? 100) / 100;
  const cinematic = c<boolean>("cinematic_home") === true;
  const ref = useRef<HTMLDivElement>(null);

  const Tile = ({ app, focused, w, h }: { app: PreviewApp; focused: boolean; w: number; h: number }) => {
    const glowColor = appGlow ? app.color : "rgba(255,255,255,0.55)";
    const highlight =
      glass === "rim" ? { boxShadow: `0 0 0 4px ${appGlow ? app.color : "rgba(255,255,255,0.9)"}` }
      : glass === "halo" ? { boxShadow: `0 0 0 4px rgba(255,255,255,0.9), 0 0 48px 8px ${glowColor}` }
      : { boxShadow: `0 0 0 3px rgba(255,255,255,0.85), 0 18px 60px ${glowColor}`, background: "linear-gradient(180deg, rgba(255,255,255,0.35), rgba(255,255,255,0) 40%)" };
    return (
      <div style={{ width: w, transition, transform: focused ? "scale(1.08)" : "scale(1)", zIndex: focused ? 2 : 1, position: "relative" }}>
        <div style={{ width: w, height: h, borderRadius: 18, overflow: "hidden", position: "relative",
          background: uniform ? `color-mix(in srgb, ${app.color} 35%, #1c1f24)` : `linear-gradient(135deg, ${app.color}, color-mix(in srgb, ${app.color} 55%, black))` }}>
          {uniform
            ? <div style={{ position: "absolute", inset: 0, display: "grid", placeItems: "center" }}><div style={{ width: h * 0.45, height: h * 0.45, borderRadius: 22, background: app.color }} /></div>
            : <div style={{ position: "absolute", left: 22, bottom: 16, fontSize: h * 0.17, fontWeight: 700, color: "white", letterSpacing: -0.5 }}>{app.label}</div>}
          {focused && <div style={{ position: "absolute", inset: 0, borderRadius: 18, transition, ...highlight }} />}
        </div>
        {(names || focused) && <div style={{ marginTop: 10, textAlign: "center", fontSize: 26, color: focused ? "white" : "rgba(255,255,255,0.55)" }}>{app.label}</div>}
      </div>
    );
  };

  const pill = (content: React.ReactNode, focused: boolean, round = false) => (
    <div style={{ height: 64, minWidth: 64, padding: round ? 0 : "0 26px", borderRadius: 32, display: "flex", alignItems: "center", justifyContent: "center", gap: 12,
      background: focused ? "rgba(255,255,255,0.9)" : "rgba(255,255,255,0.12)", color: focused ? "#111" : "white", fontSize: 26, backdropFilter: "blur(8px)", transition }}>{content}</div>
  );
  const iconOnly = c<boolean>("icon_only_buttons") === true;
  const topFocused = (i: number) => area === "top" && index === i;

  return (
    <div ref={ref} tabIndex={interactive ? 0 : -1} onKeyDown={onKey} role={interactive ? "application" : "img"}
      aria-label={interactive ? "TV preview. Use the arrow keys like a remote." : "Theme preview"}
      style={{ width, height: width * 9 / 16, overflow: "hidden", position: "relative", borderRadius: 10, outline: "none" }}
      className={interactive ? "ring-1 ring-ink-700 focus-visible:ring-2 focus-visible:ring-accent-400" : ""}>
      <div style={{ width: V_W, height: V_H, transform: `scale(${scale})`, transformOrigin: "0 0", position: "relative",
        background: c<string>("background_color") ?? "#000000", fontFamily: "'Segoe UI Variable','Segoe UI',system-ui,sans-serif", color: "white" }}>
        {wallpaperOn && <img src={wallpaperUrl!} alt="" style={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover" }} />}
        {wallpaperOn && dim > 0 && <div style={{ position: "absolute", inset: 0, background: `rgba(0,0,0,${dim / 100})` }} />}
        {sky > 0 && <div style={{ position: "absolute", inset: 0, opacity: sky / 100, background: "linear-gradient(180deg,#1b2440,#0a0d18 60%,#000)" }} />}
        {cinematic && focusedApp && (
          <div style={{ position: "absolute", left: 0, right: 0, top: 0, height: 620, background: `radial-gradient(ellipse at 70% 30%, ${focusedApp.color}, transparent 70%)`, opacity: ((c<number>("cinematic_art") ?? 50) / 100), filter: "blur(30px)", transition }} />
        )}
        {c<boolean>("ambient_glow") !== false && focusedApp && !cinematic && (
          <div style={{ position: "absolute", left: "50%", bottom: 120, width: 1400, height: 600, transform: "translateX(-50%)", background: `radial-gradient(ellipse, ${focusedApp.color}55, transparent 65%)`, transition }} />
        )}

        {/* Header */}
        <div style={{ position: "absolute", left: 120, top: 80 }}>
          {c<boolean>("show_greeting") !== false && <div style={{ fontSize: 26, letterSpacing: 4, opacity: 0.7 }}>GOOD EVENING{greeting ? `, ${greeting.toUpperCase()}` : ""}</div>}
          <div style={{ display: "flex", alignItems: c<boolean>("date_below_clock") ? "flex-start" : "baseline", flexDirection: c<boolean>("date_below_clock") ? "column" : "row", gap: 24 }}>
            <div style={{ fontSize: 132, fontWeight: 200, lineHeight: 1 }}>{now.getHours()}:{String(now.getMinutes()).padStart(2, "0")}</div>
            <div style={{ fontSize: 32, opacity: c<boolean>("header_white") ? 1 : 0.6 }}>Monday, 26 October</div>
          </div>
          {c<boolean>("show_stats") && <div style={{ fontSize: 24, opacity: 0.6, marginTop: 8 }}>CPU · free memory · free storage</div>}
        </div>
        <div style={{ position: "absolute", right: 120, top: 90, display: "flex", gap: 18 }}>
          {c<boolean>("top_menu") ? pill(<Settings2 size={30} />, topFocused(0), true) : (<>
            {c<boolean>("show_inputs_pill") !== false && pill(<><Tv size={28} />{!c<boolean>("icon_only_inputs") && "Inputs"}</>, topFocused(0), c<boolean>("icon_only_inputs") === true)}
            {topBarApps.map((p) => { const a = apps.find((x) => x.pkg === p)!; return <div key={p} style={{ width: 64, height: 64, borderRadius: 32, background: a.color }} />; })}
            {c<boolean>("show_network_pill") !== false && pill(<><Wifi size={28} />{!iconOnly && "Home Wi-Fi"}</>, topFocused(1), iconOnly)}
            {pill(<><Settings2 size={28} />{!iconOnly && "Settings"}</>, topFocused(2), iconOnly)}
          </>)}
        </div>

        {/* Dock title */}
        {c<boolean>("dock_title") !== false && area === "dock" && focusedApp && (
          <div style={{ position: "absolute", left: 120, top: cinematic ? 400 : 470, fontSize: cinematic ? 110 : 84, fontWeight: 300, textShadow: `0 0 60px ${focusedApp.color}`, transition }}>{focusedApp.label}</div>
        )}

        {/* Dock + grid */}
        <div style={{ position: "absolute", left: 120, right: 120, top: 620 }}>
          <div style={{ position: "relative", padding: dockRow && c<boolean>("dock_band") !== false ? "26px 0" : 0 }}>
            {dockRow && c<boolean>("dock_band") !== false && (
              <div style={{ position: "absolute", inset: "0 -40px", borderRadius: 34,
                background: c<boolean>("dock_glass") ? "rgba(255,255,255,0.14)" : `rgba(28,30,36,${bandAlpha})`,
                backdropFilter: c<boolean>("dock_glass") ? "blur(24px)" : undefined, boxShadow: "inset 0 1px 0 rgba(255,255,255,0.08)",
                right: singleRow && !c<boolean>("dock_band_all") ? undefined : "-40px", width: singleRow && !c<boolean>("dock_band_all") ? dock.length * (tileW + 36) + 44 : undefined }} />
            )}
            <div style={{ position: "relative", display: "flex", gap: 36, overflow: "hidden" }}>
              {(singleRow ? visible : dock).map((a, i) => <Tile key={a.pkg} app={a} focused={area === "dock" && index === i} w={tileW} h={tileH} />)}
            </div>
          </div>
          {!singleRow && (
            <div style={{ display: "grid", gridTemplateColumns: `repeat(${columns}, ${tileW}px)`, gap: 36, marginTop: 44,
              transform: area === "grid" ? `translateY(${-Math.floor(index / columns) * (tileH + 80) - 360}px)` : "none", transition,
              maxHeight: area === "grid" ? undefined : rowsBelow * (tileH + 80), overflow: area === "grid" ? "visible" : "hidden" }}>
              {grid.map((a, i) => <Tile key={a.pkg} app={a} focused={area === "grid" && index === i} w={tileW} h={tileH} />)}
            </div>
          )}
          {c<boolean>("app_library") && c<boolean>("show_library_pill") !== false && area === "grid" && (
            <div style={{ marginTop: 40, display: "flex", justifyContent: "center" }}>{pill("App library", false)}</div>
          )}
        </div>
      </div>
    </div>
  );
}
