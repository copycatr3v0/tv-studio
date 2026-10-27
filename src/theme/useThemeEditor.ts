import { useCallback, useReducer } from "react";
import { CATALOG } from "./catalog";
import type { ThemeConfig, ThemeValue } from "./schema";
import { defaultLayout, type Layout } from "./layout";

export interface Wallpaper { name: string; url: string }
/** `layout` and `videos` are optional so themes saved before the canvas existed still load. */
export interface Draft { name: string; baseId: string; config: ThemeConfig; wallpapers: Wallpaper[]; layout?: Layout; videos?: Wallpaper[]; fonts?: Wallpaper[]; tiles?: Wallpaper[]; profileLayouts?: Record<string, Layout> }
interface State { draft: Draft; past: Draft[]; future: Draft[]; savedAs: string | null }

type Action =
  | { type: "set"; key: string; value: ThemeValue }
  | { type: "unset"; key: string }
  | { type: "load"; draft: Draft; savedAs?: string | null }
  | { type: "wallpapers"; wallpapers: Wallpaper[] }
  | { type: "layout"; layout: Layout }
  | { type: "videos"; videos: Wallpaper[] }
  | { type: "fonts"; fonts: Wallpaper[] }
  | { type: "tiles"; tiles: Wallpaper[] }
  | { type: "profileLayouts"; layouts: Record<string, Layout> }
  | { type: "rename"; name: string }
  | { type: "undo" } | { type: "redo" } | { type: "saved"; as: string };

const HISTORY = 100;

export function draftFromCatalog(id: string): Draft {
  const t = CATALOG.find((x) => x.id === id) ?? CATALOG[CATALOG.length - 1]!;
  return { name: t.name, baseId: t.id, config: { ...t.config }, wallpapers: t.wallpaper ? [{ name: t.wallpaper, url: `/themes/${t.wallpaper}` }] : [], layout: defaultLayout(), videos: [], fonts: [] };
}

function push(s: State, draft: Draft): State {
  return { ...s, draft, past: [...s.past, s.draft].slice(-HISTORY), future: [] };
}

export function reducer(s: State, a: Action): State {
  switch (a.type) {
    case "set": return push(s, { ...s.draft, config: { ...s.draft.config, [a.key]: a.value } });
    case "unset": { const c = { ...s.draft.config }; delete c[a.key]; return push(s, { ...s.draft, config: c }); }
    case "wallpapers": return push(s, { ...s.draft, wallpapers: a.wallpapers });
    case "layout": return push(s, { ...s.draft, layout: a.layout });
    case "fonts": return push(s, { ...s.draft, fonts: a.fonts });
    case "tiles": return push(s, { ...s.draft, tiles: a.tiles });
    case "profileLayouts": return push(s, { ...s.draft, profileLayouts: a.layouts });
    case "videos": return push(s, { ...s.draft, videos: a.videos });
    case "rename": return { ...s, draft: { ...s.draft, name: a.name } };
    case "load": return { draft: a.draft, past: [], future: [], savedAs: a.savedAs ?? null };
    case "undo": { const prev = s.past[s.past.length - 1]; return prev ? { ...s, draft: prev, past: s.past.slice(0, -1), future: [s.draft, ...s.future] } : s; }
    case "redo": { const next = s.future[0]; return next ? { ...s, draft: next, past: [...s.past, s.draft], future: s.future.slice(1) } : s; }
    case "saved": return { ...s, savedAs: a.as };
  }
}

export function useThemeEditor(initial = "premium") {
  const [state, dispatch] = useReducer(reducer, undefined, () => ({ draft: draftFromCatalog(initial), past: [], future: [], savedAs: null }));
  return {
    ...state,
    canUndo: state.past.length > 0,
    canRedo: state.future.length > 0,
    set: useCallback((key: string, value: ThemeValue) => dispatch({ type: "set", key, value }), []),
    unset: useCallback((key: string) => dispatch({ type: "unset", key }), []),
    load: useCallback((draft: Draft, savedAs?: string | null) => dispatch({ type: "load", draft, savedAs: savedAs ?? null }), []),
    setWallpapers: useCallback((w: Wallpaper[]) => dispatch({ type: "wallpapers", wallpapers: w }), []),
    setLayout: useCallback((layout: Layout) => dispatch({ type: "layout", layout }), []),
    setFonts: useCallback((fonts: Wallpaper[]) => dispatch({ type: "fonts", fonts }), []),
    setTiles: useCallback((tiles: Wallpaper[]) => dispatch({ type: "tiles", tiles }), []),
    setProfileLayouts: useCallback((layouts: Record<string, Layout>) => dispatch({ type: "profileLayouts", layouts }), []),
    setVideos: useCallback((videos: Wallpaper[]) => dispatch({ type: "videos", videos }), []),
    rename: useCallback((name: string) => dispatch({ type: "rename", name }), []),
    undo: useCallback(() => dispatch({ type: "undo" }), []),
    redo: useCallback(() => dispatch({ type: "redo" }), []),
    markSaved: useCallback((as: string) => dispatch({ type: "saved", as }), []),
  };
}
export type ThemeEditor = ReturnType<typeof useThemeEditor>;
