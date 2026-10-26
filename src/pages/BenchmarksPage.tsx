import { useEffect, useState } from "react";
import { Gauge, Loader2 } from "lucide-react";
import { api, errMsg, onBenchProgress, type BenchComparison, type BenchListItem, type Metric } from "../lib/ipc";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";

const VERDICT = {
  better: { text: "Better", cls: "text-accent-400" },
  worse: { text: "Worse", cls: "text-danger-400" },
  no_clear_change: { text: "No clear change", cls: "text-ink-300" },
  not_enough_data: { text: "Not enough data", cls: "text-ink-400" },
} as const;

const fmt = (m: Metric, which: "before" | "after") => {
  const s = m[which];
  if (!s) return "—";
  const d = (x: number) => (Math.abs(x) >= 100 ? x.toFixed(0) : x.toFixed(1));
  return s.n > 1 ? `${d(s.median)} (${d(s.p25)}–${d(s.p75)}, n=${s.n})` : d(s.median);
};

export function BenchmarksPage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [runs, setRuns] = useState<BenchListItem[]>([]);
  const [label, setLabel] = useState("before");
  const [quick, setQuick] = useState(false);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{ what: string; done: number; total: number } | null>(null);
  const [pick, setPick] = useState<{ before: number | null; after: number | null }>({ before: null, after: null });
  const [cmp, setCmp] = useState<BenchComparison | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const refresh = (s: string) => api.benchList(s).then(setRuns, (e: unknown) => setErr(errMsg(e)));
  useEffect(() => { setRuns([]); setCmp(null); if (serial) void refresh(serial); }, [serial]);
  useEffect(() => { let off = () => {}; void onBenchProgress(setProgress).then((f) => { off = f; }); return () => off(); }, []);

  async function measure() {
    if (!serial) return;
    setBusy(true); setErr(null); setProgress(null);
    try { await api.benchRun(serial, label, quick); await refresh(serial); if (label === "before") setLabel("after"); }
    catch (e) { setErr(errMsg(e)); } finally { setBusy(false); setProgress(null); }
  }

  async function compare() {
    if (pick.before === null || pick.after === null) return;
    setErr(null);
    try { setCmp(await api.benchCompare(pick.before, pick.after)); } catch (e) { setErr(errMsg(e)); }
  }

  return (
    <div className="max-w-5xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {err && <ErrorNote message={err} />}
      <Panel title="Measure the Home screen">
        <p className="text-sm text-ink-300">Measures whichever launcher answers Home now: remote key response, focus movement, cold start, return to Home, idle CPU and memory. Run it once before customizing (“before”) and again afterwards (“after”) on the same TV. It switches between Home and Settings and presses the arrow keys; it changes nothing else. Leave the remote alone while it runs ({quick ? "about 1 minute" : "about 2–3 minutes"}).</p>
        <div className="mt-4 flex flex-wrap items-center gap-3">
          <input aria-label="Run label" value={label} maxLength={40} onChange={(e) => setLabel(e.target.value)} className="selectable w-40 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm" />
          <label className="flex items-center gap-2 text-sm"><input type="checkbox" className="accent-teal-400" checked={quick} onChange={(e) => setQuick(e.target.checked)} />Quick (fewer samples)</label>
          <Button variant="primary" onClick={() => void measure()} disabled={!serial || busy || !label.trim()}>{busy ? <Loader2 size={14} className="animate-spin" /> : <Gauge size={14} />}Measure</Button>
          {busy && <Button onClick={() => void api.deployCancel()}>Stop</Button>}
        </div>
        {busy && progress && <p className="mt-3 text-xs text-ink-400">{progress.what}: {progress.done + 1} of {progress.total}</p>}
      </Panel>

      <Panel title="Runs on this TV">
        {runs.length === 0 ? <p className="text-sm text-ink-400">No runs yet.</p> : (
          <>
            <table className="w-full text-sm">
              <thead className="text-left text-xs text-ink-400"><tr><th className="py-1">Before</th><th>After</th><th>Label</th><th>Launcher</th><th>When</th></tr></thead>
              <tbody className="divide-y divide-ink-800">
                {runs.map((r) => (
                  <tr key={r.id}>
                    <td className="py-1.5"><input type="radio" name="before" aria-label={`Use ${r.label} as before`} checked={pick.before === r.id} onChange={() => setPick({ ...pick, before: r.id })} /></td>
                    <td><input type="radio" name="after" aria-label={`Use ${r.label} as after`} checked={pick.after === r.id} onChange={() => setPick({ ...pick, after: r.id })} /></td>
                    <td className="font-medium">{r.label}</td>
                    <td className="font-mono text-xs text-ink-400">{r.launcher ?? "—"}</td>
                    <td className="text-xs text-ink-400">{r.created}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <div className="mt-3"><Button onClick={() => void compare()} disabled={pick.before === null || pick.after === null || pick.before === pick.after}>Compare</Button></div>
          </>
        )}
      </Panel>

      {cmp && (
        <Panel title="Before vs after">
          {!cmp.same_launcher && <p className="mb-3 text-xs text-ink-400">Different launchers ({cmp.before.launcher ?? "?"} → {cmp.after.launcher ?? "?"}): this compares the Home screen experience, not the same app.</p>}
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-ink-400"><tr><th className="py-1">Metric</th><th>Before · median (IQR)</th><th>After · median (IQR)</th><th>Verdict</th></tr></thead>
            <tbody className="divide-y divide-ink-800">
              {cmp.metrics.map((m) => (
                <tr key={m.key} title={m.note}>
                  <td className="py-2"><div className="font-medium">{m.label}</div><div className="text-[11px] text-ink-400">{m.unit} · {m.note}</div></td>
                  <td className="font-mono text-xs">{fmt(m, "before")}</td>
                  <td className="font-mono text-xs">{fmt(m, "after")}</td>
                  <td className={VERDICT[m.verdict].cls}>{VERDICT[m.verdict].text}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="mt-3 text-xs text-ink-400">Lower is better for every row. “Better” or “Worse” only appears when both runs have at least 5 samples and their middle halves (IQR) don't overlap; one-off readings need a clear margin. Shorter animations make “focus movement” faster without the TV being faster. Results are for this TV only.</p>
          {[...cmp.before.notes, ...cmp.after.notes].length > 0 && <ul className="mt-2 text-xs text-warn-400">{[...new Set([...cmp.before.notes, ...cmp.after.notes])].map((n) => <li key={n}>• {n}</li>)}</ul>}
        </Panel>
      )}
    </div>
  );
}
