import { useEffect, useRef, useState } from "react";
import { Download, ImagePlus, Redo2, Rocket, Save, Trash2, Undo2, Upload } from "lucide-react";
import { Preview } from "../theme/Preview";
import { Controls } from "../theme/Controls";
import { useTvApps } from "../theme/useTvApps";
import { validate } from "../theme/schema";
import type { ThemeEditor } from "../theme/useThemeEditor";
import { api, errMsg, inTauri, type ThemeIssue } from "../lib/ipc";
import { ApplyThemeDialog } from "../components/ApplyThemeDialog";
import { Button, ErrorNote } from "../components/ui";

const readAsDataUrl = (f: File) => new Promise<string>((resolve, reject) => {
  const r = new FileReader();
  r.onload = () => resolve(String(r.result));
  r.onerror = () => reject(new Error(`Could not read ${f.name}`));
  r.readAsDataURL(f);
});

const readAsText = (f: File) => new Promise<string>((resolve, reject) => {
  const r = new FileReader();
  r.onload = () => resolve(String(r.result));
  r.onerror = () => reject(new Error(`Could not read ${f.name}`));
  r.readAsText(f);
});

export function EditorPage({ serial, editor, advanced }: { serial: string | null; editor: ThemeEditor; advanced: boolean }) {
  const { apps, source, packages } = useTvApps(serial);
  const [width, setWidth] = useState(720);
  const box = useRef<HTMLDivElement>(null);
  const [applying, setApplying] = useState(false);
  const [notes, setNotes] = useState<ThemeIssue[]>([]);
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const importRef = useRef<HTMLInputElement>(null);
  const imageRef = useRef<HTMLInputElement>(null);
  const { draft } = editor;
  const problems = validate(draft.config);
  const problemCount = Object.keys(problems).length;

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(320, Math.floor(el.clientWidth))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || (e.target as HTMLElement)?.tagName === "INPUT") return;
      if (e.key.toLowerCase() === "z" && !e.shiftKey) { e.preventDefault(); editor.undo(); }
      if (e.key.toLowerCase() === "y" || (e.key.toLowerCase() === "z" && e.shiftKey)) { e.preventDefault(); editor.redo(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [editor]);

  async function onImport(f: File | undefined) {
    if (!f) return;
    setErr(null); setNotes([]);
    try {
      const r = await api.importTheme(await readAsText(f));
      editor.load({ ...draft, name: f.name.replace(/\.json$/i, ""), baseId: "imported", config: r.config as typeof draft.config });
      setNotes(r.issues);
      setMsg(`Imported ${Object.keys(r.config).length} settings from ${f.name}.`);
    } catch (e) { setErr(errMsg(e)); }
  }

  function onExport() {
    const blob = new Blob([JSON.stringify(draft.config, null, 2) + "\n"], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = `${draft.name.replace(/[^A-Za-z0-9_-]+/g, "-").toLowerCase() || "theme"}.config.json`;
    a.click();
    URL.revokeObjectURL(a.href);
  }

  async function onAddImages(files: FileList | null) {
    if (!files?.length) return;
    try {
      const added = await Promise.all(Array.from(files).slice(0, 30).map(async (f) => ({ name: f.name, url: await readAsDataUrl(f) })));
      editor.setWallpapers([...draft.wallpapers, ...added].slice(0, 30));
      if (!("wallpaper" in draft.config)) editor.set("wallpaper", true);
    } catch (e) { setErr(errMsg(e)); }
  }

  async function onSave() {
    setErr(null);
    try {
      const v = await api.saveThemeVersion(draft.name, draft);
      editor.markSaved(`${draft.name} v${v}`);
      setMsg(`Saved as ${draft.name}, version ${v}.`);
    } catch (e) { setErr(errMsg(e)); }
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <input aria-label="Theme name" value={draft.name} onChange={(e) => editor.rename(e.target.value)} maxLength={80}
          className="selectable w-56 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm font-medium" />
        <Button onClick={editor.undo} disabled={!editor.canUndo}><Undo2 size={14} />Undo</Button>
        <Button onClick={editor.redo} disabled={!editor.canRedo}><Redo2 size={14} />Redo</Button>
        <span className="mx-1 h-5 w-px bg-ink-700" />
        <Button onClick={() => importRef.current?.click()}><Upload size={14} />Import</Button>
        <input ref={importRef} type="file" accept=".json,application/json" hidden onChange={(e) => { void onImport(e.target.files?.[0]); e.target.value = ""; }} />
        <Button onClick={onExport}><Download size={14} />Export</Button>
        <Button onClick={() => void onSave()} disabled={!inTauri() || problemCount > 0}><Save size={14} />Save version</Button>
        <span className="flex-1" />
        <Button variant="primary" onClick={() => setApplying(true)} disabled={!serial || problemCount > 0}><Rocket size={14} />Apply to TV</Button>
      </div>
      {!serial && <p className="text-xs text-ink-400">Choose a connected TV (Connect TV or the Revert dialog) to apply. You can design without one.</p>}
      {editor.savedAs && <p className="text-xs text-ink-400">Based on saved version {editor.savedAs}.</p>}
      {msg && <p className="text-sm text-accent-400">{msg}</p>}
      {err && <ErrorNote message={err} />}
      {notes.length > 0 && <div className="rounded-lg border border-warn-400/40 bg-warn-400/10 px-3 py-2 text-xs text-warn-400">{notes.map((n) => <div key={n.key + n.message}>{n.key}: {n.message}</div>)}</div>}
      {problemCount > 0 && <ErrorNote message={`Fix before applying: ${Object.entries(problems).map(([k, v]) => `${k} ${v}`).join("; ")}`} />}

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_380px]">
        <div className="space-y-3">
          <div ref={box}>
            <Preview config={draft.config} apps={apps} wallpaperUrl={draft.wallpapers[0]?.url ?? null} interactive width={width} />
          </div>
          <p className="text-xs text-ink-400">Click the preview and use the arrow keys like a remote. {source === "tv" ? "Showing your TV's apps." : "Sample apps: connect a TV to see its own."} App artwork is drawn by the TV.</p>
          <section className="rounded-lg border border-ink-700 bg-ink-900 p-3">
            <div className="mb-2 flex items-center justify-between">
              <h3 className="text-sm font-medium">Wallpapers ({draft.wallpapers.length})</h3>
              <Button onClick={() => imageRef.current?.click()}><ImagePlus size={14} />Add images</Button>
              <input ref={imageRef} type="file" accept="image/jpeg,image/png,image/webp" multiple hidden onChange={(e) => { void onAddImages(e.target.files); e.target.value = ""; }} />
            </div>
            {draft.wallpapers.length === 0 ? <p className="text-xs text-ink-400">No picture: the background colour is used.</p> : (
              <ul className="flex flex-wrap gap-2">
                {draft.wallpapers.map((w, i) => (
                  <li key={w.name + i} className="group relative">
                    <img src={w.url} alt={w.name} className="h-16 w-28 rounded object-cover" />
                    {i === 0 && <span className="absolute left-1 top-1 rounded bg-black/70 px-1 text-[10px]">shown</span>}
                    <button aria-label={`Remove ${w.name}`} onClick={() => editor.setWallpapers(draft.wallpapers.filter((_, j) => j !== i))}
                      className="absolute right-1 top-1 rounded bg-black/70 p-0.5 opacity-0 group-hover:opacity-100"><Trash2 size={12} /></button>
                  </li>
                ))}
              </ul>
            )}
            <p className="mt-2 text-[11px] text-ink-400">Images are re-encoded on this PC to JPEG, at most 1920×1080, before they reach the TV. Several images rotate when “Wallpaper rotation minutes” is set.</p>
          </section>
        </div>
        <div className="max-h-[calc(100vh-220px)] overflow-y-auto pr-1">
          <Controls config={draft.config} onSet={editor.set} onUnset={editor.unset} advanced={advanced} apps={packages} />
        </div>
      </div>
      {applying && serial && <ApplyThemeDialog serial={serial} draft={draft} onClose={() => setApplying(false)} />}
    </div>
  );
}
