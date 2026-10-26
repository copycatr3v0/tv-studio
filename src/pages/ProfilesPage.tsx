import { useEffect, useRef, useState } from "react";
import { Download, FolderOpen, Save, Undo2, Upload } from "lucide-react";
import { api, errMsg, inTauri, type ChangeReport, type HistoryRow, type Profile, type SavedTheme } from "../lib/ipc";
import { downloadJson, readFileText } from "../lib/files";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";
import { ReportPanel } from "./DebloatPage";

export function ProfilesPage({ serial, onSelect, profile, setProfile, onOpenDeployment }: {
  serial: string | null; onSelect: (s: string) => void; profile: Profile; setProfile: (p: Profile) => void; onOpenDeployment: () => void;
}) {
  const [saved, setSaved] = useState<SavedTheme[]>([]);
  const [history, setHistory] = useState<HistoryRow[]>([]);
  const [msg, setMsg] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [undo, setUndo] = useState<ChangeReport | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const refresh = () => { if (inTauri()) api.listDeployProfiles().then(setSaved, (e: unknown) => setErr(errMsg(e))); };
  useEffect(refresh, []);
  useEffect(() => { setHistory([]); if (serial) api.deployHistory(serial).then(setHistory, () => {}); }, [serial]);

  async function save() {
    setErr(null);
    try { const v = await api.saveDeployProfile(profile); setMsg(`Saved “${profile.name}” version ${v}.`); refresh(); } catch (e) { setErr(errMsg(e)); }
  }
  async function load(name: string, version: number) {
    try { setProfile(await api.loadDeployProfile(name, version)); setMsg(`Loaded “${name}” v${version}.`); onOpenDeployment(); } catch (e) { setErr(errMsg(e)); }
  }
  async function importFile(f: File | undefined) {
    if (!f) return;
    try { setProfile(await api.importDeployProfile(await readFileText(f))); setMsg(`Imported ${f.name}.`); } catch (e) { setErr(errMsg(e)); }
  }
  async function undoRun(id: number) {
    if (!serial) return;
    try { setUndo(await api.undoDeployment(serial, id)); setHistory(await api.deployHistory(serial)); } catch (e) { setErr(errMsg(e)); }
  }

  return (
    <div className="max-w-4xl space-y-5">
      {msg && <p className="text-sm text-accent-400">{msg}</p>}
      {err && <ErrorNote message={err} />}
      <Panel title="Current plan">
        <div className="flex flex-wrap items-center gap-2">
          <input aria-label="Profile name" value={profile.name} maxLength={80} onChange={(e) => setProfile({ ...profile, name: e.target.value })}
            className="selectable w-56 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm" />
          <Button onClick={() => void save()} disabled={!inTauri()}><Save size={14} />Save version</Button>
          <Button onClick={() => downloadJson(`${profile.name.replace(/[^A-Za-z0-9_-]+/g, "-").toLowerCase() || "profile"}.profile.json`, profile)}><Download size={14} />Export</Button>
          <Button onClick={() => fileRef.current?.click()}><Upload size={14} />Import</Button>
          <input ref={fileRef} type="file" accept=".json,application/json" hidden onChange={(e) => { void importFile(e.target.files?.[0]); e.target.value = ""; }} />
          <Button variant="primary" onClick={onOpenDeployment}>Open in Deployment</Button>
        </div>
        <p className="mt-2 text-xs text-ink-400">A profile holds the launcher, app, settings and theme choices. The theme itself is saved separately in the Live Theme Editor.</p>
      </Panel>
      <Panel title="Saved profiles">
        {saved.length === 0 ? <p className="text-sm text-ink-400">None yet.</p> : (
          <ul className="divide-y divide-ink-800">{saved.map((p) => (
            <li key={`${p.name}-${p.version}`} className="flex items-center justify-between py-2 text-sm">
              <span>{p.name} <span className="text-ink-400">v{p.version} · {p.created}</span></span>
              <Button onClick={() => void load(p.name, p.version)}><FolderOpen size={14} />Load</Button>
            </li>))}</ul>
        )}
      </Panel>
      <Panel title="Deployment history for this TV">
        <DevicePicker serial={serial} onSelect={onSelect} />
        {history.length === 0 ? <p className="mt-3 text-sm text-ink-400">{serial ? "No deployments on this TV yet." : "Choose a TV."}</p> : (
          <ul className="mt-3 divide-y divide-ink-800">{history.map((h) => (
            <li key={h.id} className="flex items-center justify-between gap-3 py-2 text-sm">
              <span><span className="font-medium">{h.name}</span> <span className="text-ink-400">· {h.started} · {h.status}</span></span>
              {h.can_undo && <Button onClick={() => void undoRun(h.id)}><Undo2 size={14} />Undo</Button>}
            </li>))}</ul>
        )}
      </Panel>
      {undo && <ReportPanel title="Undo result" report={undo} />}
    </div>
  );
}
