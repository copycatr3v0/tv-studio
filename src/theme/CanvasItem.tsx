import { useEffect, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { FONT_CSS, FONT_WEIGHT, isFontFile, type LayoutItem } from "./layout";
import { toApp, type PreviewApp } from "./apps";

export interface Stats { cpu: number; mem: number; down: number; hist: { cpu: number[]; mem: number[]; down: number[] } }

type Builtin = keyof typeof FONT_CSS;
export function fontCss(font: string): { family: string; weight: number } {
  if (isFontFile(font)) return { family: `'tvs-font-${font.slice(5)}', sans-serif`, weight: 400 };
  return { family: FONT_CSS[font as Builtin] ?? "sans-serif", weight: FONT_WEIGHT[font as Builtin] ?? 400 };
}

/** Pixels in the 1920-wide design space → the preview's container width. */
export const cq = (px: number) => `${(px / 1920) * 100}cqw`;

export function textStyle(it: LayoutItem): CSSProperties {
  const f = fontCss(it.style.font);
  return {
    fontFamily: f.family, fontWeight: f.weight, fontSize: cq(it.style.size), color: it.style.color, opacity: it.style.opacity,
    letterSpacing: it.style.spacing ? `${it.style.spacing}em` : undefined, textTransform: it.style.upper ? "uppercase" : undefined,
    textShadow: it.style.shadow ? "0 0.2em 0.5em rgba(0,0,0,.55)" : undefined, fontStyle: it.style.italic ? "italic" : undefined,
    textAlign: it.style.align === "start" ? "left" : it.style.align === "end" ? "right" : "center",
  };
}

const pad2 = (n: number) => String(n).padStart(2, "0");

export function dateText(variant: string | undefined, d: Date): string {
  const wd = d.toLocaleDateString("en-GB", { weekday: "long" });
  const month = d.toLocaleDateString("en-GB", { month: "long" });
  switch (variant) {
    case "weekday": return wd;
    case "full": return `${d.getDate()} ${month}, ${d.getFullYear()}`;
    case "short": return d.toLocaleDateString("en-GB", { weekday: "short" });
    case "daymonth": return `${d.getDate()} ${d.toLocaleDateString("en-GB", { month: "short" })}`;
    default: return `${wd}, ${d.getDate()} ${month}`;
  }
}

export function clockParts(it: LayoutItem, d: Date): { hh: string; mm: string; ss: string; ampm: string } {
  const h = d.getHours();
  const hh = it.options?.twentyFour ? pad2(h) : String(h % 12 || 12);
  return { hh, mm: pad2(d.getMinutes()), ss: pad2(d.getSeconds()), ampm: it.options?.twentyFour ? "" : h < 12 ? "AM" : "PM" };
}

/** A smooth pseudo-random level for bar i at time t (0..1); the visualiser is decoration, not audio. */
export function barLevel(i: number, t: number, n: number): number {
  const x = i / n;
  const v = 0.5 + 0.28 * Math.sin(t * 2.1 + i * 0.7) + 0.2 * Math.sin(t * 3.7 + i * 1.9) + 0.12 * Math.sin(t * 1.3 + x * 9);
  const envelope = 0.45 + 0.55 * Math.sin(Math.PI * Math.min(1, Math.max(0, x * 0.9 + 0.05)));
  return Math.min(1, Math.max(0.04, v * envelope));
}

function useTick(ms: number, on: boolean) {
  const [t, setT] = useState(0);
  useEffect(() => {
    if (!on) return;
    const id = setInterval(() => setT((x) => x + ms / 1000), ms);
    return () => clearInterval(id);
  }, [ms, on]);
  return t;
}

export function monthGrid(d: Date, mondayFirst: boolean): (number | null)[][] {
  const first = new Date(d.getFullYear(), d.getMonth(), 1);
  const lead = (first.getDay() + (mondayFirst ? 6 : 0)) % 7;
  const days = new Date(d.getFullYear(), d.getMonth() + 1, 0).getDate();
  const cells: (number | null)[] = [...Array<null>(lead).fill(null), ...Array.from({ length: days }, (_, i) => i + 1)];
  while (cells.length % 7) cells.push(null);
  const rows: (number | null)[][] = [];
  for (let i = 0; i < cells.length; i += 7) rows.push(cells.slice(i, i + 7));
  return rows;
}

export function Item({ it, apps, stats, now, animate = true }: { it: LayoutItem; apps: PreviewApp[]; stats: Stats; now: Date; animate?: boolean }) {
  const base: CSSProperties = { ...textStyle(it), width: "100%", height: "100%" };
  const box = "flex flex-col justify-center overflow-hidden px-1";
  const fill = it.style.fill ?? undefined;
  const c = it.style.color;
  switch (it.kind) {
    case "clock": return <Clock it={it} now={now} base={base} box={box} />;
    case "date": return <div style={base} className={box}>{dateText(it.variant, now)}</div>;
    case "greeting": return <div style={base} className={box}>{now.getHours() < 12 ? "Good morning" : now.getHours() < 18 ? "Good afternoon" : "Good evening"}</div>;
    case "text": return <div style={base} className={box}>{it.text}</div>;
    case "cpu": return <Stat it={it} label="CPU" long="CPU use" value={stats.cpu} unit="%" hist={stats.hist.cpu} max={100} base={base} box={box} />;
    case "memory": return <Stat it={it} label="MEM" long="Memory in use" value={stats.mem} unit="%" hist={stats.hist.mem} max={100} base={base} box={box} />;
    case "network": return <Stat it={it} label="↓" long="Network ↓" value={stats.down} unit=" MB/s" hist={stats.hist.down} max={Math.max(10, ...stats.hist.down)} base={base} box={box} />;
    case "calendar": return <Calendar it={it} now={now} base={base} box={box} />;
    case "visualizer": return <Visualizer it={it} animate={animate} />;
    case "bluetooth": return <div style={base} className={box}>🎧 Sample headphones</div>;
    case "weather": return (
      <div style={base} className={`${box} whitespace-pre-line`}>
        {it.variant === "compact" ? "🌤️ 29°" : it.variant === "forecast" ? "🌤️ 29°  Your city\nToday  🌧️ 31° / 24°\nTomorrow  🌤️ 32° / 25°\nThursday  ☀️ 30° / 23°" : "🌤️ 29°  Partly cloudy\nYour city  ↑31° ↓24°"}
      </div>
    );
    case "nowplaying": return (
      <div style={base} className="flex items-center gap-[4%] overflow-hidden px-1">
        <span className="flex aspect-square h-[80%] items-center justify-center rounded-full bg-white/20">⏯</span>
        <span className="truncate">▶ Sample song — Artist · Spotify</span>
      </div>
    );
    case "shape": {
      const stroke = it.options?.stroke ?? 0;
      const common: CSSProperties = { width: "100%", height: "100%", opacity: it.style.opacity, boxSizing: "border-box" };
      if (it.variant === "line") return <div style={{ ...common, height: stroke > 0 ? cq(stroke) : "2px", background: fill ?? c, borderRadius: 999, margin: "auto 0", alignSelf: "center" }} />;
      if (it.variant === "circle") return <div style={{ ...common, background: fill, borderRadius: "50%", border: stroke ? `${cq(stroke)} solid ${c}` : undefined }} />;
      if (it.variant === "ring") return <div style={{ ...common, borderRadius: "50%", border: `${cq(Math.max(2, stroke))} solid ${fill ?? c}` }} />;
      return <div style={{ ...common, background: fill, borderRadius: cq(it.style.radius), border: stroke ? `${cq(stroke)} solid ${c}` : undefined }} />;
    }
    case "apps": case "recent": case "watchnext": return <Tiles it={it} apps={apps} base={base} box={box} />;
  }
}

function Clock({ it, now, base, box }: { it: LayoutItem; now: Date; base: CSSProperties; box: string }) {
  const p = clockParts(it, now);
  if (it.variant === "stacked") {
    return <div style={{ ...base, lineHeight: 0.88, alignItems: it.style.align === "end" ? "flex-end" : it.style.align === "center" ? "center" : "flex-start" }} className={box}><div>{p.hh.padStart(2, "0")}</div><div>{p.mm}</div>{p.ampm && <div style={{ fontSize: "0.3em", lineHeight: 1.6 }}>{p.ampm}</div>}</div>;
  }
  if (it.variant === "analog") {
    const s = now.getSeconds(), m = now.getMinutes() + s / 60, h = (now.getHours() % 12) + m / 60;
    const hand = (deg: number, len: number, w: number) => <line x1="50" y1="50" x2={50 + len * Math.sin((deg * Math.PI) / 180)} y2={50 - len * Math.cos((deg * Math.PI) / 180)} stroke={it.style.color} strokeWidth={w} strokeLinecap="round" />;
    return (
      <svg viewBox="0 0 100 100" style={{ width: "100%", height: "100%", opacity: it.style.opacity }} preserveAspectRatio="xMidYMid meet">
        <circle cx="50" cy="50" r="46" fill={it.style.fill ?? "none"} stroke={it.style.color} strokeWidth={Math.max(1, (it.options?.stroke ?? 2))} />
        {Array.from({ length: 12 }, (_, i) => <line key={i} x1="50" y1="8" x2="50" y2={i % 3 === 0 ? "15" : "12"} stroke={it.style.color} strokeWidth="1.5" transform={`rotate(${i * 30} 50 50)`} />)}
        {hand(h * 30, 24, 3)}{hand(m * 6, 36, 2)}{it.options?.seconds && hand(s * 6, 40, 1)}
        <circle cx="50" cy="50" r="2.5" fill={it.style.color} />
      </svg>
    );
  }
  return <div style={base} className={box}>{p.hh}:{p.mm}{it.options?.seconds ? `:${p.ss}` : ""}{p.ampm ? ` ${p.ampm}` : ""}</div>;
}

function Stat({ it, label, long, value, unit, hist, max, base, box }: { it: LayoutItem; label: string; long: string; value: number; unit: string; hist: number[]; max: number; base: CSSProperties; box: string }) {
  const shown = unit === "%" ? `${Math.round(value)}%` : `${value.toFixed(1)} MB/s`;
  const frac = Math.max(0, Math.min(1, value / max));
  const name = it.options?.compact ? label : long;
  if (it.variant === "bar") {
    return <div style={base} className={box}><div className="flex justify-between"><span>{name}</span><span>{shown}</span></div>
      <div style={{ height: "0.18em", background: "rgba(255,255,255,.2)", borderRadius: 99, marginTop: "0.25em" }}><div style={{ width: `${frac * 100}%`, height: "100%", background: it.style.color, borderRadius: 99 }} /></div></div>;
  }
  if (it.variant === "ring") {
    const r = 40, circ = 2 * Math.PI * r;
    return <div style={{ ...base, position: "relative" }}><svg viewBox="0 0 100 100" style={{ width: "100%", height: "100%" }} preserveAspectRatio="xMidYMid meet">
      <circle cx="50" cy="50" r={r} fill="none" stroke="rgba(255,255,255,.2)" strokeWidth="8" />
      <circle cx="50" cy="50" r={r} fill="none" stroke={it.style.color} strokeWidth="8" strokeLinecap="round" strokeDasharray={`${circ * frac} ${circ}`} transform="rotate(-90 50 50)" />
      <text x="50" y="50" textAnchor="middle" dominantBaseline="central" fill={it.style.color} style={{ fontSize: "20px", fontFamily: "inherit", letterSpacing: "inherit" }}>{shown}</text></svg></div>;
  }
  if (it.variant === "graph") {
    const pts = hist.map((v, i) => `${(i / Math.max(1, hist.length - 1)) * 100},${100 - Math.max(0, Math.min(1, v / max)) * 100}`).join(" ");
    return <div style={base} className={box}><div className="flex justify-between"><span>{name}</span><span>{shown}</span></div>
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" style={{ width: "100%", flex: 1, minHeight: 0 }}><polyline points={pts} fill="none" stroke={it.style.color} strokeWidth="2" vectorEffect="non-scaling-stroke" /></svg></div>;
  }
  return <div style={base} className={box}>{name} {shown}</div>;
}

function Calendar({ it, now, base, box }: { it: LayoutItem; now: Date; base: CSSProperties; box: string }) {
  const mon = !!it.options?.weekStartsMonday;
  const rows = monthGrid(now, mon);
  const heads = mon ? ["M", "T", "W", "T", "F", "S", "S"] : ["S", "M", "T", "W", "T", "F", "S"];
  return (
    <div style={{ ...base, fontSize: cq(it.style.size * 0.8) }} className={box}>
      <div style={{ fontSize: "1.5em", marginBottom: "0.3em" }}>{now.toLocaleDateString("en-GB", { month: "long" })}</div>
      <table style={{ width: "100%", borderCollapse: "collapse", textAlign: "center", tableLayout: "fixed" }}>
        <thead><tr>{heads.map((h, i) => <th key={i} style={{ opacity: 0.6, fontWeight: 500 }}>{h}</th>)}</tr></thead>
        <tbody>{rows.map((r, i) => <tr key={i}>{r.map((d, j) => (
          <td key={j} style={{ padding: "0.12em 0" }}>{d && <span style={d === now.getDate() ? { display: "inline-block", minWidth: "1.7em", lineHeight: "1.7em", borderRadius: "50%", border: `0.1em solid ${it.style.color}` } : undefined}>{d}</span>}</td>))}</tr>)}</tbody>
      </table>
    </div>
  );
}

function Visualizer({ it, animate }: { it: LayoutItem; animate: boolean }) {
  const n = it.options?.bars ?? 48;
  const reduce = typeof window !== "undefined" && !!window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  const t = useTick(90, animate && !reduce);
  const mirror = it.variant === "mirror", dots = it.variant === "dots";
  const top = mirror ? 60 : 100;
  const bars: ReactNode[] = [];
  for (let i = 0; i < n; i++) {
    const lvl = barLevel(i, t, n), x = (i + 0.5) * (100 / n), w = (100 / n) * 0.55;
    if (dots) { bars.push(<circle key={i} cx={x} cy={top - lvl * top} r={w * 0.6} fill={it.style.color} />); continue; }
    bars.push(<rect key={i} x={x - w / 2} y={top - lvl * top} width={w} height={lvl * top} fill={it.style.color} rx={w / 3} />);
    if (mirror) bars.push(<rect key={`m${i}`} x={x - w / 2} y={top + 2} width={w} height={lvl * 38} fill={it.style.color} opacity={0.3} rx={w / 3} />);
  }
  return <svg viewBox="0 0 100 100" preserveAspectRatio="none" style={{ width: "100%", height: "100%", opacity: it.style.opacity }} aria-label="Animated bars (decoration)">{bars}</svg>;
}

function Tiles({ it, apps, base, box }: { it: LayoutItem; apps: PreviewApp[]; base: CSSProperties; box: string }) {
  if (it.kind === "watchnext") {
    return (
      <div style={base} className={box}><span className="mb-0.5 opacity-70">Continue watching</span>
        <div className="flex gap-[1cqw]">{["Sample title A", "Sample title B", "Sample title C"].slice(0, it.options?.max ?? 3).map((t, i) => (
          <div key={t} className="shrink-0" style={{ width: "12cqw" }}><div style={{ height: "6.5cqw", background: ["#46508a", "#8a4666", "#468a6c"][i % 3], borderRadius: cq(it.style.radius) }} /><span className="block truncate">{t}</span></div>
        ))}</div>
      </div>
    );
  }
  const picked = it.options?.packages?.length ? it.options.packages.map(toApp) : apps;
  const list = picked.slice(0, it.kind === "recent" ? (it.options?.max ?? 6) : it.icon?.columns ? it.icon.columns * 3 : 24);
  const size = ((it.icon?.size ?? 96) / 1920) * 100;
  const shape = it.icon?.shape;
  return (
    <div style={base} className={box}>
      {it.kind === "recent" && <span className="mb-0.5 opacity-70">Recently opened</span>}
      <div className="flex flex-wrap content-start gap-[0.8cqw]" style={{ alignItems: "flex-start" }}>
        {list.map((a) => (
          <div key={a.pkg} className="flex flex-col items-center gap-0.5" style={{ width: `${size}cqw` }}>
            <div style={{ width: `${size}cqw`, height: `${size * (shape === "circle" ? 1 : 0.56)}cqw`, background: a.color, borderRadius: shape === "circle" ? "50%" : shape === "square" ? 2 : cq(it.style.radius) }} />
            {it.icon?.labels && <span className="max-w-full truncate" style={{ fontSize: `${(it.style.size / 1920) * 80}cqw` }}>{a.label}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
