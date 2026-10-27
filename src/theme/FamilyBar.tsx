import { useState } from "react";
import { Lock, Plus, Trash2, Users } from "lucide-react";
import type { Layout } from "./layout";
import type { PreviewApp } from "./apps";
import { Button } from "../components/ui";
import { api, errMsg } from "../lib/ipc";

type Profile = NonNullable<Layout["profiles"]>[number];
const COLORS = ["#5b8def", "#f59e0b", "#10b981", "#ec4899"];

/** Profiles (who is using the TV) and the app lock. The main layout lists the profiles; each has its own layout. */
export function FamilyBar({ main, onMain, layouts, onLayouts, editing, onEditing, apps, serial }: {
  main: Layout; onMain: (l: Layout) => void;
  layouts: Record<string, Layout>; onLayouts: (l: Record<string, Layout>) => void;
  editing: string; onEditing: (id: string) => void;
  apps: PreviewApp[]; serial: string | null;
}) {
  const profiles = main.profiles ?? [];
  const current = editing ? layouts[editing] ?? main : main;
  const [note, setNote] = useState<string | null>(null);
  const setProfiles = (p: Profile[]) => onMain({ ...main, profiles: p });
  const setCurrent = (l: Layout) => (editing ? onLayouts({ ...layouts, [editing]: l }) : onMain(l));
  const locked = current.locked ?? [];

  function add() {
    const n = profiles.length + 2;
    let id = `profile-${n}`;
    while (profiles.some((p) => p.id === id)) id += "x";
    // A new profile starts as a copy of the main layout, without the family settings.
    const copy: Layout = { ...main, profiles: undefined, profileName: undefined, profileColor: undefined, locked: [] };
    setProfiles([...profiles, { id, name: n === 2 ? "Kids" : `Profile ${n}`, color: COLORS[(n - 1) % 4] }]);
    onLayouts({ ...layouts, [id]: copy });
    onEditing(id);
  }

  function remove(id: string) {
    const next = { ...layouts };
    delete next[id];
    onLayouts(next);
    setProfiles(profiles.filter((p) => p.id !== id));
    if (editing === id) onEditing("");
  }

  async function resetPin() {
    if (!serial) return;
    setNote(null);
    try { await api.resetTvPin(serial); setNote("The TV forgot its PIN. The next locked app asks for a new one."); }
    catch (e) { setNote(errMsg(e)); }
  }

  return (
    <section className="mb-3 rounded-lg border border-ink-700 bg-ink-900 p-3 text-sm" data-testid="family-bar">
      <div className="flex flex-wrap items-center gap-2">
        <Users size={14} />
        <span className="text-xs text-ink-300">Editing:</span>
        <div role="tablist" aria-label="Profiles" className="flex flex-wrap gap-1">
          <button role="tab" aria-selected={!editing} onClick={() => onEditing("")}
            className={`rounded px-2 py-0.5 text-xs ${!editing ? "bg-ink-700 font-medium" : "hover:bg-ink-800"}`}>{main.profileName ?? "Main"}</button>
          {profiles.map((p) => (
            <button key={p.id} role="tab" aria-selected={editing === p.id} onClick={() => onEditing(p.id)}
              className={`rounded px-2 py-0.5 text-xs ${editing === p.id ? "bg-ink-700 font-medium" : "hover:bg-ink-800"}`}>
              <span className="mr-1 inline-block h-2 w-2 rounded-full" style={{ background: p.color ?? "#888" }} />{p.name}{p.pin ? " 🔒" : ""}
            </button>
          ))}
        </div>
        <Button disabled={profiles.length >= 3} onClick={add}><Plus size={12} />Add profile</Button>
      </div>
      {!editing ? (
        <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
          <label>Main profile name <input aria-label="Main profile name" value={main.profileName ?? "Main"} maxLength={20} onChange={(e) => onMain({ ...main, profileName: e.target.value || undefined })}
            className="ml-1 w-28 rounded border border-ink-600 bg-ink-850 px-2 py-0.5" /></label>
          <input aria-label="Main profile colour" type="color" value={main.profileColor ?? "#5b8def"} onChange={(e) => onMain({ ...main, profileColor: e.target.value })} />
        </div>
      ) : (() => {
        const p = profiles.find((x) => x.id === editing);
        if (!p) return null;
        const patch = (q: Partial<Profile>) => setProfiles(profiles.map((x) => (x.id === editing ? { ...x, ...q } : x)));
        return (
          <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
            <input aria-label="Profile name" value={p.name} maxLength={20} onChange={(e) => patch({ name: e.target.value })} className="w-28 rounded border border-ink-600 bg-ink-850 px-2 py-0.5" />
            <input aria-label="Profile colour" type="color" value={p.color ?? "#888888"} onChange={(e) => patch({ color: e.target.value })} />
            <label><input type="checkbox" checked={!!p.pin} onChange={(e) => patch({ pin: e.target.checked })} /> Ask for the PIN to switch to it</label>
            <button aria-label={`Delete profile ${p.name}`} onClick={() => remove(p.id)}><Trash2 size={13} /></button>
          </div>
        );
      })()}
      {profiles.length > 0 && <p className="mt-1 text-[11px] text-ink-400">Each profile has its own canvas below. On the TV a profile button sits top-right in Canvas and Scroll mode (not in Classic).</p>}
      <details className="mt-2 text-xs">
        <summary className="cursor-pointer"><Lock size={12} className="mr-1 inline" />Locked apps in this layout ({locked.length})</summary>
        <div className="mt-1 max-h-40 overflow-auto rounded border border-ink-700 p-1">
          {apps.map((a) => (
            <label key={a.pkg} className="block">
              <input type="checkbox" checked={locked.includes(a.pkg)} onChange={(e) => setCurrent({ ...current, locked: e.target.checked ? [...locked, a.pkg].slice(0, 50) : locked.filter((x) => x !== a.pkg) })} /> {a.label}
            </label>
          ))}
        </div>
        <p className="mt-1 text-[11px] text-ink-400">The TV asks for a PIN when a locked app is opened from this Home. The PIN is chosen on the TV the first time and stays there; Studio never sees it. Apps opened another way (voice, the stock launcher) are not locked.</p>
        <Button disabled={!serial} onClick={() => void resetPin()}>Reset the TV's PIN</Button>
        {note && <span className="ml-2">{note}</span>}
      </details>
    </section>
  );
}
