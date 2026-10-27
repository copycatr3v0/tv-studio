import { useEffect, useRef } from "react";
import type { SceneId } from "./layout";

/** Studio's preview of the launcher's ambient scenes (AmbientScene.java): same scenes, sky colours and particle
 *  counts, drawn on a canvas. `animate` false draws one still frame (looks gallery, low-RAM preview). */

type Phase = "sunrise" | "day" | "sunset" | "night";
export const phaseFor = (hour: number): Phase => (hour >= 5 && hour < 8 ? "sunrise" : hour >= 8 && hour < 17 ? "day" : hour >= 17 && hour < 20 ? "sunset" : "night");

const hex = (n: number) => `#${(n & 0xffffff).toString(16).padStart(6, "0")}`;
function mix(a: number, b: number, t: number) {
  const ch = (x: number, s: number) => (x >> s) & 0xff;
  const m = (s: number) => Math.round(ch(a, s) + (ch(b, s) - ch(a, s)) * t);
  return (m(16) << 16) | (m(8) << 8) | m(0);
}

export function skyColors(scene: SceneId, phase: Phase): [string, string] {
  switch (scene) {
    case "space": return ["#02030a", "#0b1030"];
    case "embers": return ["#0b0503", phase === "night" ? "#1e0a04" : "#2e1206"];
    case "underwater": return phase === "night" ? ["#01121a", "#03303d"] : phase === "day" ? ["#0a5a6e", "#022b3a"] : ["#06404f", "#021e29"];
    case "aurora": return phase === "day" ? ["#10233f", "#2a4a6a"] : ["#020614", "#0c1a2e"];
    default: break;
  }
  let [top, bottom] = phase === "sunrise" ? [0x2b2f5a, 0xf29e7a] : phase === "day" ? [0x4f8cc9, 0xb9dcf2] : phase === "sunset" ? [0x2a1f4d, 0xe8735a] : [0x050816, 0x141a33];
  if (scene === "rain") { top = mix(top, 0x2a2f38, 0.55); bottom = mix(bottom, 0x3a4048, 0.55); }
  if (scene === "snow") { top = mix(top, 0x8a97a8, 0.3); bottom = mix(bottom, 0xd5dee8, 0.35); }
  if (scene === "sakura" && phase !== "night") bottom = mix(bottom, 0xf6c6d4, 0.35);
  return [hex(top), hex(bottom)];
}

const COUNT: Record<SceneId, number> = { rain: 220, snow: 170, fireflies: 46, space: 240, sakura: 42, embers: 80, underwater: 50, aurora: 160 };

interface P { x: number; y: number; s: number; z: number; p: number; e: number }

export function SceneCanvas({ scene, time = "auto", animate = true }: { scene: SceneId; time?: string; animate?: boolean }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    const W = (c.width = 480), H = (c.height = 270);
    const u = H / 1080;
    let seed = 7;
    const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647);
    const ps: P[] = Array.from({ length: Math.round(COUNT[scene] * 0.6) }, () => {
      const p: P = { x: rnd() * W, y: rnd() * H, s: 50 * u, z: 2 * u, p: rnd() * 6.283, e: rnd() };
      switch (scene) {
        case "rain": p.s = (900 + rnd() * 600) * u; p.z = (16 + rnd() * 22) * u * 1.6; break;
        case "snow": p.s = (35 + rnd() * 80) * u; p.z = Math.max(0.7, (1.5 + rnd() * 3.5) * u * 1.6); break;
        case "fireflies": p.s = (10 + rnd() * 25) * u; p.z = Math.max(0.8, (2 + rnd() * 2.5) * u * 1.6); p.y = H * (0.35 + rnd() * 0.65); break;
        case "space": case "aurora": p.s = (2 + rnd() * 6) * u; p.z = Math.max(0.5, (0.6 + rnd() * 1.4) * u * 1.6); if (scene === "aurora") p.y = rnd() * H * 0.7; break;
        case "sakura": p.s = (40 + rnd() * 50) * u; p.z = (6 + rnd() * 7) * u * 1.6; break;
        case "embers": p.s = (40 + rnd() * 90) * u; p.z = Math.max(0.7, (1.5 + rnd() * 2.5) * u * 1.6); break;
        case "underwater": p.s = (30 + rnd() * 60) * u; p.z = (3 + rnd() * 9) * u * 1.6; break;
      }
      return p;
    });
    const phase: Phase = time === "auto" ? phaseFor(new Date().getHours()) : (time as Phase);
    const [top, bottom] = skyColors(scene, phase);
    const sky = ctx.createLinearGradient(0, 0, 0, H);
    sky.addColorStop(0, top); sky.addColorStop(1, bottom);
    const night = phase === "night";
    let last = performance.now(), raf = 0;
    const frame = (now: number) => {
      const dt = animate ? Math.min(0.1, (now - last) / 1000) : 0;
      last = now;
      const t = now / 1000;
      ctx.fillStyle = sky; ctx.fillRect(0, 0, W, H);
      draw(ctx, scene, ps, W, H, dt, t, night, u, rnd);
      if (animate) raf = requestAnimationFrame(frame);
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, [scene, time, animate]);
  return <canvas ref={ref} aria-hidden className="absolute inset-0 h-full w-full" data-scene={scene} />;
}

function draw(c: CanvasRenderingContext2D, scene: SceneId, ps: P[], W: number, H: number, dt: number, t: number, night: boolean, u: number, rnd: () => number) {
  const rgba = (r: number, g: number, b: number, a: number) => `rgba(${r},${g},${b},${Math.max(0, Math.min(1, a))})`;
  switch (scene) {
    case "rain":
      c.lineWidth = 1; c.lineCap = "round";
      for (const p of ps) {
        p.y += p.s * dt; p.x += p.s * 0.18 * dt;
        if (p.y > H) { p.y = -p.z; p.x = rnd() * W * 1.2 - W * 0.1; }
        c.strokeStyle = rgba(200, 215, 235, (60 + p.e * 90) / 255);
        c.beginPath(); c.moveTo(p.x, p.y); c.lineTo(p.x - p.z * 0.18, p.y - p.z); c.stroke();
      }
      break;
    case "snow":
      for (const p of ps) {
        p.y += p.s * dt; p.x += Math.sin(t * 0.8 + p.p) * 18 * u * dt;
        if (p.y > H + 6) { p.y = -6; p.x = rnd() * W; }
        c.fillStyle = rgba(255, 255, 255, (150 + p.e * 100) / 255);
        c.beginPath(); c.arc(p.x, p.y, p.z, 0, 6.283); c.fill();
      }
      break;
    case "fireflies":
      for (const p of ps) {
        p.x = (p.x + Math.cos(t * 0.3 + p.p) * p.s * dt + W) % W;
        p.y = Math.min(H, Math.max(H * 0.3, p.y + Math.sin(t * 0.23 + p.p * 1.7) * p.s * dt));
        const g = 0.5 + 0.5 * Math.sin(t * (1.2 + p.e) + p.p);
        c.fillStyle = rgba(255, 230, 120, (40 * g) / 255); c.beginPath(); c.arc(p.x, p.y, p.z * 5, 0, 6.283); c.fill();
        c.fillStyle = rgba(255, 240, 150, (230 * g) / 255); c.beginPath(); c.arc(p.x, p.y, p.z, 0, 6.283); c.fill();
      }
      break;
    case "space": case "aurora":
      for (const p of ps) {
        p.x -= p.s * dt; if (p.x < 0) p.x += W;
        const tw = 0.6 + 0.4 * Math.sin(t * (0.7 + p.e * 2) + p.p);
        c.fillStyle = rgba(255, 255, 255, ((night || scene === "space" ? 220 : 90) * tw) / 255);
        c.beginPath(); c.arc(p.x, p.y, p.z, 0, 6.283); c.fill();
      }
      if (scene === "aurora") {
        const cols: [number, number, number, number][] = [[52, 245, 160, 0.4], [34, 200, 232, 0.33], [160, 96, 240, 0.27]];
        cols.forEach(([r, g, b, a], k) => {
          const base = H * (0.22 + k * 0.08), amp = H * 0.06, height = H * (0.22 - k * 0.03);
          const grad = c.createLinearGradient(0, base + height, 0, base - amp);
          grad.addColorStop(0, rgba(r, g, b, 0)); grad.addColorStop(1, rgba(r, g, b, a));
          c.fillStyle = grad; c.beginPath();
          for (let x = 0; x <= W; x += 6) {
            const xs = x * 4; // the TV draws at 1920 wide; keep the same wave shape
            const y = base + amp * Math.sin(xs * 0.004 + t * (0.25 + k * 0.07) + k) + amp * 0.5 * Math.sin(xs * 0.011 - t * 0.4);
            if (x === 0) c.moveTo(x, y); else c.lineTo(x, y);
          }
          for (let x = W; x >= 0; x -= 6) c.lineTo(x, base + height + amp * 0.6 * Math.sin(x * 4 * 0.005 + t * 0.3 + k * 2));
          c.closePath(); c.fill();
        });
      }
      break;
    case "sakura":
      for (const p of ps) {
        p.y += p.s * dt; p.x += (Math.sin(t * 0.9 + p.p) * 40 + 20) * u * dt;
        if (p.y > H + 20 || p.x > W + 20) { p.y = -20; p.x = rnd() * W - W * 0.2; }
        c.save(); c.translate(p.x, p.y); c.rotate(((t * 60 * (0.5 + p.e) + p.p * 57) % 360) * Math.PI / 180);
        c.fillStyle = "rgba(248,183,205,0.86)"; c.beginPath(); c.ellipse(0, 0, p.z, p.z * 0.55, 0, 0, 6.283); c.fill(); c.restore();
      }
      break;
    case "embers": {
      for (const p of ps) {
        p.y -= p.s * dt; p.x += Math.sin(t * 1.3 + p.p) * 22 * u * dt;
        if (p.y < -6) { p.y = H + 6; p.x = rnd() * W; }
        const life = Math.max(0, Math.min(1, p.y / H)), fl = 0.7 + 0.3 * Math.sin(t * 9 + p.p * 3);
        c.fillStyle = rgba(255, Math.round(120 + 80 * p.e), 40, life * fl); c.beginPath(); c.arc(p.x, p.y, p.z, 0, 6.283); c.fill();
      }
      const g = c.createLinearGradient(0, H * 0.7, 0, H); g.addColorStop(0, "rgba(255,106,26,0)"); g.addColorStop(1, "rgba(255,106,26,0.33)");
      c.fillStyle = g; c.fillRect(0, H * 0.7, W, H * 0.3);
      break;
    }
    case "underwater": {
      const g = c.createLinearGradient(0, 0, 0, H * 0.8); g.addColorStop(0, night ? "rgba(255,255,255,0.09)" : "rgba(191,246,255,0.19)"); g.addColorStop(1, "rgba(191,246,255,0)");
      c.fillStyle = g;
      for (let r = 0; r < 5; r++) {
        const x = W * (0.1 + r * 0.2) + Math.sin(t * 0.2 + r) * 40 * u;
        c.beginPath(); c.moveTo(x - 30 * u, 0); c.lineTo(x + 30 * u, 0); c.lineTo(x + 150 * u, H * 0.8); c.lineTo(x - 10 * u, H * 0.8); c.closePath(); c.fill();
      }
      c.lineWidth = 1;
      for (const p of ps) {
        p.y -= p.s * dt; p.x += Math.sin(t * 2 + p.p) * 14 * u * dt;
        if (p.y < -p.z) { p.y = H + p.z; p.x = rnd() * W; }
        c.strokeStyle = rgba(210, 245, 255, (90 + p.e * 80) / 255); c.beginPath(); c.arc(p.x, p.y, p.z, 0, 6.283); c.stroke();
      }
      break;
    }
  }
}
