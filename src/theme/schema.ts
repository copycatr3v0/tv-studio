// The same schema/theme.v1.json the Rust validator embeds: one source for editor controls and validation.
import raw from "../../schema/theme.v1.json";

export type KeyType = "bool" | "int" | "color" | "enum" | "string" | "packages" | "int_list" | "string_list" | "date" | "json";
export type Group = "dock" | "apps" | "background" | "focus" | "motion" | "header" | "extras" | "inputs" | "remote";
export type ThemeValue = boolean | number | string | string[] | number[] | Record<string, unknown> | unknown[] | null;
export type ThemeConfig = Record<string, ThemeValue>;

export interface KeySpec {
  key: string;
  type: KeyType;
  min?: number;
  max?: number;
  options?: string[];
  default: ThemeValue;
  group: Group;
  advanced: boolean;
  label: string;
  description: string;
  since: string;
}

export const SCHEMA = raw as unknown as { version: number; launcher_version: string; keys: KeySpec[] };
export const SPEC: Record<string, KeySpec> = Object.fromEntries(SCHEMA.keys.map((k) => [k.key, k]));

export const GROUP_LABEL: Record<Group, string> = {
  dock: "Dock", apps: "Apps & grid", background: "Background", focus: "Focus & glow", motion: "Motion",
  header: "Top bar & clock", extras: "Extras", inputs: "TV inputs (advanced)", remote: "Remote buttons (advanced)",
};
export const GROUP_ORDER: Group[] = ["background", "dock", "apps", "focus", "header", "motion", "extras", "inputs", "remote"];

/** Mirrors NOT_YET_SUPPORTED_GROUPS in crates/tvs-launcher/src/schema.rs. */
export const UNSUPPORTED_GROUPS: Group[] = ["remote"];

/** Keys the desktop preview draws. Others are deployed but only visible on the TV. */
export const PREVIEWED = new Set([
  "background_color", "wallpaper", "wallpaper_dim", "sky_intensity", "columns", "rows_below_dock", "dock_apps", "fill_dock",
  "hidden_apps", "dock_row", "dock_band", "dock_glass", "dock_opacity", "dock_title", "always_show_names", "uniform_tiles",
  "show_greeting", "greeting_name", "date_below_clock", "show_network_pill", "show_inputs_pill", "icon_only_buttons",
  "top_menu", "glass_style", "ambient_glow", "app_glow", "single_row", "app_library", "show_library_pill", "top_bar_apps",
  "show_stats", "header_white", "reduced_motion", "animations", "focus_overshoot", "cinematic_home", "dock_band_all",
]);

export function value<T extends ThemeValue>(config: ThemeConfig, key: string): T {
  return (key in config ? config[key] : SPEC[key]?.default ?? null) as T;
}

/** Client-side mirror of the Rust validator for instant feedback; the Rust check still runs before any push. */
export function validateKey(spec: KeySpec, v: ThemeValue): string | null {
  switch (spec.type) {
    case "bool": return typeof v === "boolean" ? null : "must be on or off";
    case "int": return typeof v === "number" && Number.isInteger(v) && (spec.min === undefined || v >= spec.min) && (spec.max === undefined || v <= spec.max) ? null : `must be ${spec.min}–${spec.max}`;
    case "color": return typeof v === "string" && /^#[0-9A-Fa-f]{6}$/.test(v) ? null : "must be a colour like #1A2B3C";
    case "enum": return typeof v === "string" && (spec.options ?? []).includes(v) ? null : "not an allowed value";
    case "date": return typeof v === "string" && /^\d{4}-\d{2}-\d{2}$/.test(v) ? null : "must be YYYY-MM-DD";
    case "string": return typeof v === "string" && v.length <= 500 && !/[\u0000-\u001f]/.test(v) ? null : "plain text, up to 500 characters";
    case "packages": return Array.isArray(v) && v.every((p) => typeof p === "string" && /^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$/.test(p)) ? null : "must be package names";
    case "int_list": return Array.isArray(v) && v.every((n) => Number.isInteger(n)) ? null : "must be whole numbers";
    case "string_list": return Array.isArray(v) && v.every((s) => typeof s === "string") ? null : "must be text entries";
    case "json": return null;
  }
}

export function validate(config: ThemeConfig): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(config)) {
    const s = SPEC[k];
    if (!s) out[k] = "unknown key";
    else if (UNSUPPORTED_GROUPS.includes(s.group)) out[k] = "remote-button keys are not supported by this version yet";
    else { const e = validateKey(s, v); if (e) out[k] = e; }
  }
  return out;
}
