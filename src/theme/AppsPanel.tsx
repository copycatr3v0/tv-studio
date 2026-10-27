import { useRef, useState } from "react";
import { ArrowDown, ArrowUp, ImagePlus, Plus, Trash2 } from "lucide-react";
import type { Layout } from "./layout";
import type { PreviewApp } from "./apps";
import type { Wallpaper } from "./useThemeEditor";
import { Button } from "../components/ui";
import { IMAGE_ACCEPT, pictureForTheme } from "./media";

type Row = NonNullable<Layout["rows"]>[number];
type Folder = NonNullable<Layout["folders"]>[number];
type Look = NonNullable<Layout["apps"]>[string];

export const ROW_SOURCE_LABEL: Record<Row["source"], string> = {
  favorites: "Hand-picked", recent: "Recently used", games: "Games", video: "Video & streaming", music: "Music", news: "News", social: "Social", tools: "Tools & photos",
};

const readAsDataUrl = (f: File) => new Promise<string>((resolve, reject) => {
  const r = new FileReader();
  r.onload = () => resolve(String(r.result));
  r.onerror = () => reject(new Error(`Could not read ${f.name}`));
  r.readAsDataURL(f);
});

/** On-TV file name for an app's picture: from its package, so one app has one picture. */
export const tileNameFor = (pkg: string) => `${pkg.toLowerCase().replace(/[^a-z0-9]+/g, "-").slice(0, 55)}.jpg`;

function Picker({ apps, picked, onChange, max, label }: { apps: PreviewApp[]; picked: string[]; onChange: (p: string[]) => void; max: number; label: string }) {
  return (
    <details className="text-xs">
      <summary className="cursor-pointer">{label} ({picked.length ? `${picked.length} picked` : "none"})</summary>
      <div className="mt-1 max-h-40 overflow-auto rounded border border-ink-700 p-1">
        {apps.map((a) => (
          <label key={a.pkg} className="block">
            <input type="checkbox" checked={picked.includes(a.pkg)}
              onChange={(e) => onChange(e.target.checked ? [...picked, a.pkg].slice(0, max) : picked.filter((p) => p !== a.pkg))} /> {a.label}
          </label>
        ))}
      </div>
    </details>
  );
}

/** Scroll mode's apps: rows on the apps page, folders, hidden apps, and per-app name, colour and picture. */
export function AppsPanel({ layout, onChange, apps, tiles, onTiles }: {
  layout: Layout; onChange: (l: Layout) => void; apps: PreviewApp[]; tiles: Wallpaper[]; onTiles: (t: Wallpaper[]) => void;
}) {
  const rows = layout.rows ?? [], folders = layout.folders ?? [], hidden = layout.hidden ?? [], looks = layout.apps ?? {};
  const [lookPkg, setLookPkg] = useState<string>(apps[0]?.pkg ?? "");
  const [err, setErr] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const setRows = (r: Row[]) => onChange({ ...layout, rows: r });
  const setFolders = (f: Folder[]) => onChange({ ...layout, folders: f });
  const setLook = (pkg: string, l: Look | null) => {
    const next = { ...looks };
    if (l && (l.name || l.color || l.image)) next[pkg] = l; else delete next[pkg];
    onChange({ ...layout, apps: next });
  };
  const look = looks[lookPkg] ?? {};
  const label = (pkg: string) => apps.find((a) => a.pkg === pkg)?.label ?? pkg;

  async function addPicture(f: File | undefined) {
    if (!f || !lookPkg) return;
    setErr(null);
    try {
      const pic = await pictureForTheme(f, readAsDataUrl);
      const name = tileNameFor(lookPkg);
      onTiles([...tiles.filter((t) => t.name !== name), { name, url: pic.url }].slice(-60));
      setLook(lookPkg, { ...look, image: name });
    } catch (e) { setErr(e instanceof Error ? e.message : String(e)); }
  }

  return (
    <details className="rounded-lg border border-ink-700 bg-ink-900 p-3 text-sm" open data-testid="apps-panel">
      <summary className="cursor-pointer font-medium">Apps: rows, folders, hidden apps, app looks</summary>

      <h4 className="mt-2 text-xs font-medium text-ink-300">Rows on the apps page ({rows.length}/8)</h4>
      <p className="text-[11px] text-ink-400">Shown after Down, above All apps. Category rows fill themselves from what each app declares on Google Play; empty rows are left out on the TV.</p>
      <ul className="space-y-2">
        {rows.map((r, i) => (
          <li key={i} className="rounded border border-ink-700 p-2">
            <div className="flex flex-wrap items-center gap-1">
              <input aria-label={`Row ${i + 1} title`} value={r.title} maxLength={40} onChange={(e) => setRows(rows.map((x, j) => (j === i ? { ...x, title: e.target.value } : x)))}
                className="w-40 rounded border border-ink-600 bg-ink-850 px-2 py-0.5 text-xs" />
              <select aria-label={`Row ${i + 1} source`} value={r.source} onChange={(e) => setRows(rows.map((x, j) => (j === i ? { ...x, source: e.target.value as Row["source"] } : x)))}
                className="rounded border border-ink-600 bg-ink-850 px-1 py-0.5 text-xs">
                {Object.entries(ROW_SOURCE_LABEL).map(([v, l]) => <option key={v} value={v}>{l}</option>)}
              </select>
              <button aria-label={`Move row ${i + 1} up`} disabled={i === 0} onClick={() => { const n = [...rows]; [n[i - 1], n[i]] = [n[i]!, n[i - 1]!]; setRows(n); }}><ArrowUp size={13} /></button>
              <button aria-label={`Move row ${i + 1} down`} disabled={i === rows.length - 1} onClick={() => { const n = [...rows]; [n[i + 1], n[i]] = [n[i]!, n[i + 1]!]; setRows(n); }}><ArrowDown size={13} /></button>
              <button aria-label={`Delete row ${i + 1}`} onClick={() => setRows(rows.filter((_, j) => j !== i))}><Trash2 size={13} /></button>
            </div>
            {r.source === "favorites" && <Picker apps={apps} max={24} label="Apps in this row" picked={r.packages ?? []} onChange={(p) => setRows(rows.map((x, j) => (j === i ? { ...x, packages: p } : x)))} />}
          </li>
        ))}
      </ul>
      <Button disabled={rows.length >= 8} onClick={() => setRows([...rows, { title: rows.length ? "Games" : "Favourites", source: rows.length ? "games" : "favorites", packages: [] }])}><Plus size={12} />Add row</Button>

      <h4 className="mt-3 text-xs font-medium text-ink-300">Folders ({folders.length}/8)</h4>
      <ul className="space-y-2">
        {folders.map((f, i) => (
          <li key={i} className="rounded border border-ink-700 p-2">
            <div className="flex items-center gap-1">
              <input aria-label={`Folder ${i + 1} name`} value={f.name} maxLength={30} onChange={(e) => setFolders(folders.map((x, j) => (j === i ? { ...x, name: e.target.value } : x)))}
                className="w-40 rounded border border-ink-600 bg-ink-850 px-2 py-0.5 text-xs" />
              <button aria-label={`Delete folder ${i + 1}`} onClick={() => setFolders(folders.filter((_, j) => j !== i))}><Trash2 size={13} /></button>
            </div>
            <Picker apps={apps} max={24} label="Apps in this folder" picked={f.packages} onChange={(p) => setFolders(folders.map((x, j) => (j === i ? { ...x, packages: p } : x)))} />
          </li>
        ))}
      </ul>
      <Button disabled={folders.length >= 8} onClick={() => setFolders([...folders, { name: `Folder ${folders.length + 1}`, packages: [] }])}><Plus size={12} />Add folder</Button>

      <h4 className="mt-3 text-xs font-medium text-ink-300">Hidden apps</h4>
      <Picker apps={apps} max={100} label="Hide from Home (still in Settings → Apps)" picked={hidden} onChange={(p) => onChange({ ...layout, hidden: p })} />

      <h4 className="mt-3 text-xs font-medium text-ink-300">App looks</h4>
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <select aria-label="App to change" value={lookPkg} onChange={(e) => setLookPkg(e.target.value)} className="rounded border border-ink-600 bg-ink-850 px-1 py-0.5">
          {apps.map((a) => <option key={a.pkg} value={a.pkg}>{a.label}{looks[a.pkg] ? " •" : ""}</option>)}
        </select>
        <input aria-label="Name shown on the TV" placeholder={label(lookPkg)} value={look.name ?? ""} maxLength={40}
          onChange={(e) => setLook(lookPkg, { ...look, name: e.target.value || null })} className="w-36 rounded border border-ink-600 bg-ink-850 px-2 py-0.5" />
        <label>Card colour <input aria-label="Card colour" type="color" value={(look.color ?? "#333333").slice(0, 7)} onChange={(e) => setLook(lookPkg, { ...look, color: e.target.value })} /></label>
        {look.color && <button className="underline" onClick={() => setLook(lookPkg, { ...look, color: null })}>default colour</button>}
        <Button onClick={() => fileRef.current?.click()}><ImagePlus size={12} />{look.image ? "Replace picture" : "Own picture"}</Button>
        {look.image && <button className="underline" onClick={() => setLook(lookPkg, { ...look, image: null })}>remove picture</button>}
        <input ref={fileRef} type="file" accept={IMAGE_ACCEPT} hidden onChange={(e) => { void addPicture(e.target.files?.[0]); e.target.value = ""; }} />
      </div>
      {look.image && <img alt={`Picture for ${label(lookPkg)}`} src={tiles.find((t) => t.name === look.image)?.url} className="mt-1 h-14 rounded object-cover" />}
      {err && <p className="text-xs text-warn-400">{err}</p>}
    </details>
  );
}
