import { useState } from "react";
import { ChevronDown, RotateCcw, X } from "lucide-react";
import { GROUP_LABEL, GROUP_ORDER, PREVIEWED, SCHEMA, SPEC, UNSUPPORTED_GROUPS, validateKey, value, type Group, type KeySpec, type ThemeConfig, type ThemeValue } from "./schema";

const field = "selectable rounded-md border border-ink-600 bg-ink-850 px-2 py-1 text-sm";

function Control({ spec, v, onChange, apps }: { spec: KeySpec; v: ThemeValue; onChange: (v: ThemeValue) => void; apps: string[] }) {
  const [text, setText] = useState<string | null>(null);
  switch (spec.type) {
    case "bool":
      return <input type="checkbox" aria-label={spec.label} className="h-4 w-4 accent-teal-400" checked={v === true} onChange={(e) => onChange(e.target.checked)} />;
    case "int": {
      const n = typeof v === "number" ? v : spec.min ?? 0;
      return (
        <span className="flex items-center gap-2">
          <input type="range" aria-label={spec.label} min={spec.min} max={spec.max} value={n} onChange={(e) => onChange(Number(e.target.value))} className="w-32 accent-teal-400" />
          <span className="w-10 text-right font-mono text-xs">{n}</span>
        </span>
      );
    }
    case "color":
      return (
        <span className="flex items-center gap-2">
          <input type="color" aria-label={spec.label} value={typeof v === "string" ? v : "#000000"} onChange={(e) => onChange(e.target.value.toUpperCase())} className="h-7 w-10 rounded border border-ink-600 bg-transparent" />
          <span className="font-mono text-xs">{typeof v === "string" ? v : "—"}</span>
        </span>
      );
    case "enum":
      return <select aria-label={spec.label} className={field} value={typeof v === "string" ? v : ""} onChange={(e) => onChange(e.target.value)}>{(spec.options ?? []).map((o) => <option key={o} value={o}>{o || "(none)"}</option>)}</select>;
    case "date":
      return <input type="date" aria-label={spec.label} className={field} value={typeof v === "string" ? v : ""} onChange={(e) => onChange(e.target.value)} />;
    case "string":
      return <input aria-label={spec.label} className={field + " w-48"} value={typeof v === "string" ? v : ""} maxLength={500} onChange={(e) => onChange(e.target.value)} />;
    case "packages": {
      const list = Array.isArray(v) ? (v as string[]) : [];
      const options = apps.filter((a) => !list.includes(a));
      return (
        <div className="w-full space-y-1.5">
          <div className="flex flex-wrap gap-1.5">
            {list.map((p, i) => (
              <span key={p} className="inline-flex items-center gap-1 rounded-full bg-ink-700 px-2 py-0.5 font-mono text-[11px]">
                {i + 1}. {p}<button aria-label={`Remove ${p}`} onClick={() => onChange(list.filter((x) => x !== p))}><X size={11} /></button>
              </span>
            ))}
            {list.length === 0 && <span className="text-xs text-ink-400">None</span>}
          </div>
          <select aria-label={`Add to ${spec.label}`} className={field + " w-full"} value="" onChange={(e) => e.target.value && onChange([...list, e.target.value])}>
            <option value="">{apps.length ? "Add an app…" : "Connect a TV to pick from its apps"}</option>
            {options.map((a) => <option key={a} value={a}>{a}</option>)}
          </select>
        </div>
      );
    }
    case "int_list":
    case "string_list":
    case "json": {
      const shown = text ?? (v == null ? "" : JSON.stringify(v));
      return (
        <input aria-label={spec.label} className={field + " w-full font-mono text-xs"} value={shown} placeholder={spec.type === "json" ? "JSON" : "[…]"}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => { if (text === null) return; try { onChange(text.trim() ? JSON.parse(text) : null); setText(null); } catch { /* keep text; error shown below */ } }} />
      );
    }
  }
}

export function Controls({ config, onSet, onUnset, advanced, apps }: {
  config: ThemeConfig; onSet: (k: string, v: ThemeValue) => void; onUnset: (k: string) => void; advanced: boolean; apps: string[];
}) {
  const [open, setOpen] = useState<Set<Group>>(new Set(["background", "dock", "apps"]));
  const [q, setQ] = useState("");
  const groups = GROUP_ORDER.filter((g) => advanced || !["inputs", "remote", "extras"].includes(g));
  return (
    <div className="space-y-2">
      <input aria-label="Find a setting" placeholder="Find a setting…" value={q} onChange={(e) => setQ(e.target.value)}
        className="selectable w-full rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm" />
      {groups.map((g) => {
        const keys = SCHEMA.keys.filter((k) => k.group === g && (advanced || !k.advanced) && (!q || (k.key + k.label + k.description).toLowerCase().includes(q.toLowerCase())));
        if (!keys.length) return null;
        const isOpen = open.has(g) || !!q;
        const unsupported = UNSUPPORTED_GROUPS.includes(g);
        return (
          <section key={g} className="rounded-lg border border-ink-700 bg-ink-900">
            <button className="flex w-full items-center justify-between px-3 py-2 text-left text-sm font-medium"
              aria-expanded={isOpen} onClick={() => { const n = new Set(open); if (n.has(g)) n.delete(g); else n.add(g); setOpen(n); }}>
              {GROUP_LABEL[g]} <span className="flex items-center gap-2 text-xs text-ink-400">{keys.length}<ChevronDown size={14} className={isOpen ? "rotate-180" : ""} /></span>
            </button>
            {isOpen && (
              <div className="divide-y divide-ink-800 border-t border-ink-700">
                {unsupported && <p className="px-3 py-2 text-xs text-warn-400">Shown for reference. These need the launcher's remote-key helpers, which this version does not set up yet, so they can't be deployed.</p>}
                {keys.map((spec) => {
                  const set = spec.key in config;
                  const v = value(config, spec.key);
                  const err = set ? validateKey(spec, config[spec.key] ?? null) : null;
                  return (
                    <div key={spec.key} className={`px-3 py-2 ${unsupported ? "opacity-50" : ""}`}>
                      <div className="flex items-center justify-between gap-3">
                        <label className="min-w-0 text-sm" title={spec.description}>
                          <span className={set ? "text-ink-100" : "text-ink-300"}>{spec.label}</span>
                          {!PREVIEWED.has(spec.key) && <span className="ml-1.5 rounded bg-ink-700 px-1 py-px text-[10px] text-ink-400">TV only</span>}
                        </label>
                        <span className="flex shrink-0 items-center gap-1.5">
                          {!unsupported && spec.type !== "packages" && <Control spec={spec} v={v} onChange={(nv) => onSet(spec.key, nv)} apps={apps} />}
                          {set && <button aria-label={`Reset ${spec.label}`} title="Back to the launcher default" onClick={() => onUnset(spec.key)} className="text-ink-400 hover:text-ink-100"><RotateCcw size={13} /></button>}
                        </span>
                      </div>
                      {spec.type === "packages" && !unsupported && <div className="mt-1.5"><Control spec={spec} v={v} onChange={(nv) => onSet(spec.key, nv)} apps={apps} /></div>}
                      <p className="mt-0.5 line-clamp-2 text-[11px] leading-snug text-ink-400">{spec.description}</p>
                      {err && <p className="mt-0.5 text-[11px] text-danger-400">{err}</p>}
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        );
      })}
      <p className="px-1 text-[11px] text-ink-400">Every setting changes the Studio launcher only. “TV only” settings are deployed but not drawn in this preview. Launcher version {SCHEMA.launcher_version}.</p>
    </div>
  );
}

export const _test = { SPEC };
