import { useCallback, useReducer } from "react";
import { CATALOG } from "./catalog";
import type { ThemeConfig, ThemeValue } from "./schema";

export interface Wallpaper { name: string; url: string }
export interface Draft { name: string; baseId: string; config: ThemeConfig; wallpapers: Wallpaper[] }
interface State { draft: Draft; past: Draft[]; future: Draft[]; savedAs: string | null }

type Action =
  | { type: "set"; key: string; value: ThemeValue }
  | { type: "unset"; key: string }
  | { type: "load"; draft: Draft; savedAs?: string | null }
  | { type: "wallpapers"; wallpapers: Wallpaper[] }
  | { type: "rename"; name: string }
  | { type: "undo" } | { type: "redo" } | { type: "saved"; as: string };

const HISTORY = 100;

export function draftFromCatalog(id: string): Draft {
  const t = CATALOG.find((x) => x.id === id) ?? CATALOG[CATALOG.length - 1]!;
  return { name: t.name, baseId: t.id, config: { ...t.config }, wallpapers: t.wallpaper ? [{ name: t.wallpaper, url: `/themes/${t.wallpaper}` }] : [] };
}

function push(s: State, draft: Draft): State {
  return { ...s, draft, past: [...s.past, s.draft].slice(-HISTORY), future: [] };
}

export function reducer(s: State, a: Action): State {
  switch (a.type) {
    case "set": return push(s, { ...s.draft, config: { ...s.draft.config, [a.key]: a.value } });
    case "unset": { const c = { ...s.draft.config }; delete c[a.key]; return push(s, { ...s.draft, config: c }); }
    case "wallpapers": return push(s, { ...s.draft, wallpapers: a.wallpapers });
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
    rename: useCallback((name: string) => dispatch({ type: "rename", name }), []),
    undo: useCallback(() => dispatch({ type: "undo" }), []),
    redo: useCallback(() => dispatch({ type: "redo" }), []),
    markSaved: useCallback((as: string) => dispatch({ type: "saved", as }), []),
  };
}
export type ThemeEditor = ReturnType<typeof useThemeEditor>;
