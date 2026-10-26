import { useEffect, useState } from "react";
import { Loader2, Undo2 } from "lucide-react";
import { api, errMsg, type ChangeReport, type RestorePreview } from "../lib/ipc";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";
import { ReportPanel } from "./DebloatPage";

export function RestorePage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [preview, setPreview] = useState<RestorePreview | null>(null);
  const [report, setReport] = useState<ChangeReport | null>(null);
  const [busy, setBusy] = useState<"preview" | "apply" | null>(null);
  const [ask, setAsk] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const load = async (s: string) => {
    setBusy("preview"); setErr(null);
    try { setPreview(await api.restorePreview(s, null)); } catch (e) { setPreview(null); setErr(errMsg(e)); } finally { setBusy(null); }
  };
  useEffect(() => { if (serial) { setReport(null); void load(serial); } }, [serial]);

  async function apply() {
    if (!serial) return;
    setAsk(false); setBusy("apply"); setErr(null);
    try { setReport(await api.restoreApply(serial, null)); await load(serial); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  const p = preview?.plan;
  const nothing = p && !p.enable.length && !p.set_home && !p.settings.length;
  return (
    <div className="max-w-4xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {err && <ErrorNote message={err} />}
      {busy === "preview" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Comparing the TV with its baseline…</p>}
      {p && preview && (
        <Panel title={`Back to the baseline from ${preview.baseline_created}`}>
          {nothing ? <p className="text-sm text-accent-400">The TV already matches its baseline.</p> : (
            <ul className="space-y-3 text-sm">
              {p.enable.length > 0 && <li><div className="font-medium">Re-enable {p.enable.length} package{p.enable.length === 1 ? "" : "s"}</div><div className="selectable font-mono text-xs text-ink-400">{p.enable.join(", ")}</div></li>}
              {p.set_home && <li><div className="font-medium">Set Home back to</div><div className="selectable font-mono text-xs text-ink-400">{p.set_home}</div></li>}
              {p.settings.map(([k, v]) => <li key={k}><span className="font-medium">{k}</span> <span className="text-ink-400">→ {v ?? "default (unset)"}</span></li>)}
            </ul>
          )}
          {p.left_disabled.length > 0 && <p className="mt-4 text-xs text-ink-400">Left disabled because they were already disabled before this app changed anything: {p.left_disabled.join(", ")}</p>}
          {p.not_in_snapshot.length > 0 && <p className="mt-2 text-xs text-ink-400">Not on the TV when the baseline was taken, left alone: {p.not_in_snapshot.join(", ")}</p>}
          {!nothing && (
            <div className="mt-5 flex items-center gap-3">
              {ask ? (<>
                <span className="text-sm">Restore now? Each change is read back from the TV.</span>
                <Button variant="primary" onClick={() => void apply()}>Restore</Button>
                <Button onClick={() => setAsk(false)}>Cancel</Button>
              </>) : (
                <Button variant="primary" onClick={() => setAsk(true)} disabled={busy !== null}>
                  {busy === "apply" ? <Loader2 size={14} className="animate-spin" /> : <Undo2 size={14} />}Restore everything
                </Button>
              )}
            </div>
          )}
        </Panel>
      )}
      {report && <ReportPanel title="Restore result" report={report} />}
      <p className="text-xs text-ink-400">If the TV can no longer be reached over ADB, see RECOVERY.md for the manual path (Safe Mode or factory reset); this app never claims a restore it could not verify.</p>
    </div>
  );
}
