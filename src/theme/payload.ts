import type { Draft } from "./useThemeEditor";
import type { ThemeIn } from "../lib/ipc";
import { urlToBase64 } from "../lib/files";
import { VIDEO_SAMPLES, isFontFile } from "./layout";

export const hasVideo = (d: Draft) => !!d.layout?.enabled && d.layout.background.kind === "video" && !!d.layout.background.video;

/**
 * What goes to the TV for a draft. The canvas layout is sent only when it is switched on. The video is sent only when
 * `withVideo` is true; otherwise the layout keeps its still wallpaper background, so the TV never plays video by accident.
 */
export async function themePayload(d: Draft, withVideo: boolean, fetchB64: (url: string) => Promise<string> = urlToBase64): Promise<ThemeIn> {
  const wallpapers = await Promise.all(d.wallpapers.map(async (w) => ({ name: w.name, data_base64: await fetchB64(w.url) })));
  const base: ThemeIn = { config: d.config as Record<string, unknown>, wallpapers, captions: null, layout: null, videos: [], fonts: [] };
  if (!d.layout?.enabled) return base;
  const bg = d.layout.background;
  // Every profile's layout travels too; fonts and app pictures are collected from all of them.
  const ids = (d.layout.profiles ?? []).map((p) => p.id);
  const others = ids.map((id) => ({ id, layout: d.profileLayouts?.[id] })).filter((p): p is { id: string; layout: NonNullable<typeof p.layout> } => !!p.layout);
  const all = [d.layout, ...others.map((p) => p.layout)];
  const wanted = [...new Set(all.flatMap((l) => l.items.filter((i) => i.enabled && isFontFile(i.style.font)).map((i) => i.style.font.slice(5))))];
  const fonts = await Promise.all(wanted.map(async (name) => {
    const f = d.fonts?.find((x) => x.name === name);
    if (!f) throw new Error(`The font file “${name}” is no longer in this theme. Add it again or pick another font.`);
    return { name, data_base64: await fetchB64(f.url) };
  }));
  // Pictures the layout gives single apps: only the ones it names, so a removed one never travels.
  const tileNames = [...new Set(all.flatMap((l) => Object.values(l.apps ?? {}).map((a) => a.image)).filter((n): n is string => !!n))];
  const tiles = await Promise.all(tileNames.map(async (name) => {
    const t = d.tiles?.find((x) => x.name === name);
    if (!t) throw new Error(`The picture “${name}” for an app is no longer in this theme. Add it again or remove it from the app.`);
    return { name, data_base64: await fetchB64(t.url) };
  }));
  // A profile carries no video of its own: it shows its still picture instead.
  const profiles = others.map((p) => ({ id: p.id, layout: { ...p.layout, enabled: true, profiles: undefined, background: { ...p.layout.background, kind: p.layout.background.kind === "video" ? "image" as const : p.layout.background.kind } } }));
  if (bg.kind !== "video" || !bg.video || !withVideo) {
    return { ...base, fonts, tiles, profiles, layout: { ...d.layout, background: { ...bg, kind: bg.kind === "video" ? "image" : bg.kind } } };
  }
  const sample = VIDEO_SAMPLES.find((s) => s.id === bg.video);
  const own = d.videos?.find((v) => v.name === bg.video);
  const url = own ? own.url : sample ? `/videos/${sample.id}.mp4` : null;
  if (!url) throw new Error(`The video “${bg.video}” is no longer in this theme. Pick another or add it again.`);
  const name = own ? own.name : `${sample!.id}.mp4`;
  return { ...base, fonts, tiles, profiles, layout: d.layout, videos: [{ name, data_base64: await fetchB64(url) }] };
}
