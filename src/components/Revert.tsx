import { useEffect, useState } from "react";
import { History, Loader2, X } from "lucide-react";
import { api, errMsg, type ChangeReport, type RestorePreview } from "../lib/ipc";
import { DevicePicker } from "./DevicePicker";
import { Button, ErrorNote } from "./ui";
import { ReportPanel } from "../pages/DebloatPage";

type State =
  | { kind: "loading" }
  | { kind: "untouched" }
  | { kind: "matches"; created: string }
  | { kind: "ready"; preview: RestorePreview }
  | { kind: "error"; message: string };

/**
 * One place that returns a TV to how it was before this app changed anything: preview, confirm, apply,
 * verified report. Later stages (Studio launcher, themes) add their own undo steps behind the same panel.
 */
export function RevertPanel({ serial, onDone }: { serial: string; onDone?: () => void }) {
  const [state, setState] = useState<State>({ kind: "loading" });
  const [ask, setAsk] = useState(false);
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<ChangeReport | null>(null);

  const load = async () => {
    setState({ kind: "loading" });
    try {
      const st = await api.baselineStatus(serial);
      if (!st.created) { setState({ kind: "untouched" }); return; }
      const preview = await api.restorePreview(serial, null);
      const p = preview.plan;
      setState(p.enable.length || p.set_home || p.settings.length ? { kind: "ready", preview } : { kind: "matches", created: st.created });
    } catch (e) { setState({ kind: "error", message: errMsg(e) }); }
  };
  useEffect(() => { setReport(null); setAsk(false); void load(); }, [serial]);

  async function apply() {
    setAsk(false); setBusy(true);
    try { setReport(await api.restoreApply(serial, null)); onDone?.(); await load(); }
    catch (e) { setState({ kind: "error", message: errMsg(e) }); }
    finally { setBusy(false); }
  }

  return (
    <div className="space-y-4">
      {state.kind === "loading" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Comparing the TV with how it was originally…</p>}
      {state.kind === "untouched" && <p className="text-sm text-ink-300">Nothing to revert: this app has not changed this TV.</p>}
      {state.kind === "matches" && <p className="text-sm text-accent-400">This TV already matches its original state (recorded {state.created}).</p>}
      {state.kind === "error" && <ErrorNote message={state.message} />}
      {state.kind === "ready" && (() => {
        const p = state.preview.plan;
        return (
          <>
            <p className="text-sm text-ink-300">Original state recorded <span className="font-mono text-xs">{state.preview.baseline_created}</span>. Reverting will:</p>
            <ul className="space-y-2 text-sm">
              {p.enable.length > 0 && <li><div className="font-medium">Re-enable {p.enable.length} package{p.enable.length === 1 ? "" : "s"}</div><div className="selectable font-mono text-xs text-ink-400">{p.enable.join(", ")}</div></li>}
              {p.set_home && <li><div className="font-medium">Set the Home screen back to</div><div className="selectable font-mono text-xs text-ink-400">{p.set_home}</div></li>}
              {p.settings.map(([k, v]) => <li key={k}><span className="font-medium">{k}</span> <span className="text-ink-400">→ {v ?? "default (unset)"}</span></li>)}
            </ul>
            {p.left_disabled.length > 0 && <p className="text-xs text-ink-400">Stay disabled, because they were already disabled before this app changed anything: {p.left_disabled.join(", ")}</p>}
            {p.not_in_snapshot.length > 0 && <p className="text-xs text-ink-400">Not on the TV originally, left alone: {p.not_in_snapshot.join(", ")}</p>}
            <div className="flex flex-wrap items-center gap-3 pt-1">
              {ask ? (<>
                <span className="text-sm">Revert this TV now? Every change is read back from the TV.</span>
                <Button variant="primary" onClick={() => void apply()}>Yes, revert</Button>
                <Button onClick={() => setAsk(false)}>Cancel</Button>
              </>) : (
                <Button variant="primary" onClick={() => setAsk(true)} disabled={busy}>
                  {busy ? <Loader2 size={14} className="animate-spin" /> : <History size={14} />}Revert to original
                </Button>
              )}
            </div>
          </>
        );
      })()}
      {report && <ReportPanel title="Revert result" report={report} />}
    </div>
  );
}

/** Always-visible header button that opens the revert panel for the selected (or a chosen) TV. */
export function RevertButton({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") setOpen(false); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);
  return (
    <>
      <button onClick={() => setOpen(true)}
        className="inline-flex items-center gap-2 rounded-lg border border-ink-600 px-3 py-1.5 text-sm text-ink-100 hover:border-accent-500 hover:text-accent-400">
        <History size={15} />Revert to original
      </button>
      {open && (
        <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 p-8 pt-24" onClick={() => setOpen(false)}>
          <div role="dialog" aria-modal="true" aria-labelledby="revert-title" onClick={(e) => e.stopPropagation()}
            className="max-h-[80vh] w-full max-w-2xl overflow-y-auto rounded-xl border border-ink-700 bg-ink-900 shadow-2xl">
            <header className="flex items-center justify-between border-b border-ink-700 px-5 py-3">
              <h2 id="revert-title" className="text-sm font-semibold">Revert TV to original</h2>
              <button aria-label="Close" onClick={() => setOpen(false)} className="text-ink-400 hover:text-ink-100"><X size={18} /></button>
            </header>
            <div className="space-y-4 p-5">
              <DevicePicker serial={serial} onSelect={onSelect} />
              {serial ? <RevertPanel serial={serial} /> : <p className="text-sm text-ink-400">Choose the TV to revert.</p>}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
