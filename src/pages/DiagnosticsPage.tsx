import { useState } from "react";
import { CheckCircle2, Download, Loader2, Search, Wrench } from "lucide-react";
import { api, errMsg, type DriftResult, type RunResult } from "../lib/ipc";
import { downloadJson } from "../lib/files";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";

export function DiagnosticsPage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [drift, setDrift] = useState<DriftResult | null | undefined>(undefined);
  const [repair, setRepair] = useState<RunResult | null>(null);
  const [busy, setBusy] = useState<"drift" | "repair" | "export" | null>(null);
  const [ask, setAsk] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function check() {
    if (!serial) return;
    setBusy("drift"); setErr(null); setRepair(null);
    try { setDrift(await api.driftCheck(serial)); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }
  async function doRepair() {
    if (!serial || !drift) return;
    setAsk(false); setBusy("repair"); setErr(null);
    try {
      const theme = (drift.request.theme ?? null) as Parameters<typeof api.deployRun>[2];
      setRepair(await api.deployRun(serial, drift.request.profile, theme));
      setDrift(await api.driftCheck(serial));
    } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }
  async function exportBundle() {
    setBusy("export"); setErr(null);
    try { downloadJson(`tv-studio-diagnostics-${new Date().toISOString().slice(0, 10)}.json`, await api.diagnosticsBundle(serial)); }
    catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  const r = drift?.report;
  return (
    <div className="max-w-4xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {err && <ErrorNote message={err} />}
      <Panel title="After a firmware update">
        <p className="text-sm text-ink-300">Updates can re-enable apps, give Home back to the stock launcher, remove the Studio launcher or reset settings. This compares the TV with your last deployment. It only reads the TV.</p>
        <div className="mt-4"><Button variant="primary" onClick={() => void check()} disabled={!serial || !!busy}>{busy === "drift" ? <Loader2 size={14} className="animate-spin" /> : <Search size={14} />}Check for changes</Button></div>
        {drift === null && <p className="mt-3 text-sm text-ink-400">No deployment on this TV yet, so there is nothing to compare with.</p>}
        {r && drift && (
          <div className="mt-4 space-y-3 text-sm">
            <p className="text-xs text-ink-400">Compared with the deployment from {drift.deployment_started}.</p>
            {r.in_sync && r.new_packages.length === 0 ? <p className="flex items-center gap-2 text-accent-400"><CheckCircle2 size={16} />The TV still matches.</p> : (
              <ul className="space-y-1.5">
                {r.reenabled.length > 0 && <li><span className="font-medium">Re-enabled:</span> <span className="selectable font-mono text-xs">{r.reenabled.join(", ")}</span></li>}
                {r.launcher_missing && <li className="text-warn-400">The Studio launcher is no longer installed.</li>}
                {r.home_moved && <li className="text-warn-400">Home no longer opens the Studio launcher.</li>}
                {r.would_change.length > 0 && <li><span className="font-medium">Repair would change:</span> {r.would_change.join(", ")}</li>}
                {r.new_packages.length > 0 && <li><span className="font-medium">New preinstalled apps since the baseline:</span> <span className="selectable font-mono text-xs">{r.new_packages.join(", ")}</span> <span className="text-xs text-ink-400">(not touched; review them in the Debloat Center)</span></li>}
              </ul>
            )}
            {!r.in_sync && (
              <div className="flex flex-wrap items-center gap-3 pt-1">
                {!r.plan.runnable && <span className="text-danger-400">The repair plan has blocked steps: {r.plan.steps.filter((s) => s.preview.state === "blocked").map((s) => s.preview.detail).join(" | ")}</span>}
                {r.plan.runnable && (ask ? (<>
                  <span>Repair now? The same checks as a normal deployment run first; a new snapshot is saved.</span>
                  <Button variant="primary" onClick={() => void doRepair()}>Repair</Button><Button onClick={() => setAsk(false)}>Cancel</Button>
                </>) : <Button variant="primary" onClick={() => setAsk(true)} disabled={!!busy}>{busy === "repair" ? <Loader2 size={14} className="animate-spin" /> : <Wrench size={14} />}Repair…</Button>)}
              </div>
            )}
          </div>
        )}
        {repair && <p className={`mt-3 text-sm ${repair.report.completed ? "text-accent-400" : "text-warn-400"}`}>{repair.report.completed ? "Repaired and verified." : `Repair finished with problems: ${repair.report.results.filter((x) => x.status === "failed").map((x) => `${x.title}: ${x.message}`).join("; ")}`}</p>}
      </Panel>
      <Panel title="Diagnostics bundle">
        <p className="text-sm text-ink-300">A JSON file with the app version, settings, this TV's details, recent deployments with their step logs, and benchmark run names. IP addresses and the ADB serial are replaced. Nothing is uploaded; you choose whether to share it.</p>
        <div className="mt-3"><Button onClick={() => void exportBundle()} disabled={!!busy}>{busy === "export" ? <Loader2 size={14} className="animate-spin" /> : <Download size={14} />}Export</Button></div>
      </Panel>
    </div>
  );
}
