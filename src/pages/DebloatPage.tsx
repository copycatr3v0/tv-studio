import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, CheckCircle2, Loader2, OctagonX, ShieldCheck, Undo2 } from "lucide-react";
import { api, errMsg, onProgress, type AppRow, type ApplyResult, type ChangeReport, type Evaluation, type Inventory, type KbCategory, type Progress } from "../lib/ipc";
import { CATEGORY_LABEL, evidence } from "../lib/categories";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";

type Mode = "safe" | "balanced" | "advanced";
const MODE_TEXT: Record<Mode, string> = {
  safe: "Only telemetry, ads and demo packages that at least 90% of tested TVs disabled. Nothing a household feature depends on.",
  balanced: "Adds preinstalled extras that most tested TVs disabled. Still nothing sensitive pre-selected.",
  advanced: "Shows every enabled package, including ones no tested TV mentions. Each risky one asks for confirmation.",
};
const SENSITIVE: KbCategory[] = ["streaming", "voice", "casting", "updater", "launcher"];
const ORDER: KbCategory[] = ["telemetry", "ads_content", "retail_demo", "privacy", "preinstalled", "streaming", "voice", "casting", "updater", "launcher", "factory", "other"];

export function DebloatPage({ serial, onSelect, onAddToPlan }: { serial: string | null; onSelect: (s: string) => void; onAddToPlan?: (packages: string[], confirmed: string[]) => void }) {
  const [inv, setInv] = useState<Inventory | null>(null);
  const [mode, setMode] = useState<Mode>("safe");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [evaluation, setEvaluation] = useState<{ key: string; data: Evaluation } | null>(null);
  const [confirmed, setConfirmed] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState<"load" | "eval" | "apply" | "undo" | null>(null);
  const [askApply, setAskApply] = useState(false);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [result, setResult] = useState<ApplyResult | null>(null);
  const [undo, setUndo] = useState<ChangeReport | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const load = async (s: string) => {
    setBusy("load"); setErr(null); setResult(null); setUndo(null);
    try { setInv(await api.inventory(s)); } catch (e) { setInv(null); setErr(errMsg(e)); } finally { setBusy(null); }
  };
  useEffect(() => { if (serial) void load(serial); }, [serial]);
  useEffect(() => { let off = () => {}; void onProgress(setProgress).then((f) => { off = f; }); return () => off(); }, []);

  // Pre-select for the mode from evidence; the TV check still decides.
  useEffect(() => {
    if (!inv) return;
    setSelected(new Set(inv.apps.filter((a) => (mode === "safe" ? a.preselect_safe : a.preselect_balanced)).map((a) => a.name)));
    setConfirmed(new Set());
  }, [inv, mode]);

  const candidates = useMemo(() => (inv?.apps ?? []).filter((a) => a.enabled && (mode === "advanced" || a.kb !== null)), [inv, mode]);
  const groups = useMemo(() => {
    const m = new Map<KbCategory, AppRow[]>();
    for (const a of candidates) {
      const c = a.kb?.category ?? "other";
      m.set(c, [...(m.get(c) ?? []), a]);
    }
    return ORDER.filter((c) => m.has(c)).map((c) => [c, m.get(c)!] as const);
  }, [candidates]);

  const selKey = [...selected].sort().join(",");
  const evalNow = evaluation?.key === selKey ? evaluation.data : null;
  const verdictOf = (n: string) => evalNow?.verdicts[n];
  const applySet = [...selected].filter((n) => verdictOf(n)?.kind !== "Blocked");
  const needConfirm = applySet.filter((n) => verdictOf(n)?.kind === "NeedsConfirm");
  const ready = !!evalNow && applySet.length > 0 && needConfirm.every((n) => confirmed.has(n));

  function toggle(n: string) {
    const s = new Set(selected);
    if (s.has(n)) s.delete(n); else s.add(n);
    setSelected(s);
  }

  async function check() {
    if (!serial) return;
    setBusy("eval"); setErr(null);
    try { setEvaluation({ key: selKey, data: await api.evaluate(serial, [...selected]) }); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  async function apply() {
    if (!serial) return;
    setAskApply(false); setBusy("apply"); setErr(null); setProgress(null); setUndo(null);
    try {
      const r = await api.applyDebloat(serial, applySet, needConfirm);
      setResult(r);
      setInv(await api.inventory(serial));
    } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); setProgress(null); }
  }

  async function undoRun() {
    if (!serial || !result) return;
    setBusy("undo"); setErr(null);
    try { setUndo(await api.restoreApply(serial, result.report.changed)); setInv(await api.inventory(serial)); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  return (
    <div className="max-w-5xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {err && <ErrorNote message={err} />}
      {busy === "load" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Reading installed packages…</p>}

      {inv && (
        <>
          <Panel title="Mode">
            <div className="flex gap-2" role="radiogroup">
              {(["safe", "balanced", "advanced"] as const).map((m) => (
                <button key={m} role="radio" aria-checked={mode === m} onClick={() => setMode(m)}
                  className={`rounded-lg border px-3 py-1.5 text-sm capitalize ${mode === m ? "border-accent-500 bg-accent-900 text-accent-400" : "border-ink-600 text-ink-300"}`}>{m}</button>
              ))}
            </div>
            <p className="mt-3 text-sm text-ink-300">{MODE_TEXT[mode]}</p>
            <p className="mt-1 text-xs text-ink-400">Every mode keeps the same protection list. Packages are disabled for user 0, never uninstalled, and can be re-enabled.</p>
          </Panel>

          {groups.map(([cat, apps]) => (
            <Panel key={cat} title={`${CATEGORY_LABEL[cat]} · ${apps.length}`}>
              {SENSITIVE.includes(cat) && <p className="mb-3 flex gap-2 text-xs text-warn-400"><AlertTriangle size={14} className="shrink-0" />Never pre-selected. Tick only what nobody in the household uses.</p>}
              <ul className="divide-y divide-ink-800">
                {apps.map((a) => {
                  const v = verdictOf(a.name);
                  const staticBlock = a.verdict.kind === "Blocked";
                  const blocked = staticBlock || v?.kind === "Blocked";
                  return (
                    <li key={a.name} className="py-2">
                      <label className={`flex items-start gap-3 ${staticBlock ? "opacity-60" : "cursor-pointer"}`}>
                        <input type="checkbox" className="mt-1 accent-teal-400" disabled={staticBlock} checked={selected.has(a.name) && !staticBlock} onChange={() => toggle(a.name)} />
                        <span className="min-w-0 flex-1">
                          <span className="selectable font-mono text-xs">{a.name}</span>
                          <span className="ml-2 text-xs text-ink-400">{evidence(a.kb, inv.kb_lists)}</span>
                          {a.kb?.notes[0] && <span className="block text-xs text-ink-400">“{a.kb.notes[0]}”</span>}
                          {blocked && <span className="mt-1 flex gap-1.5 text-xs text-danger-400"><OctagonX size={13} className="mt-0.5 shrink-0" />{staticBlock ? (a.verdict as { reason: string }).reason : (v as { reason: string }).reason}</span>}
                        </span>
                        {v?.kind === "Allowed" && selected.has(a.name) && <CheckCircle2 size={16} className="mt-0.5 text-accent-400" aria-label="checked on TV" />}
                      </label>
                      {v?.kind === "NeedsConfirm" && selected.has(a.name) && (
                        <label className="mt-2 ml-7 flex items-start gap-2 rounded-lg border border-warn-400/40 bg-warn-400/10 px-3 py-2 text-xs text-warn-400">
                          <input type="checkbox" className="mt-0.5" checked={confirmed.has(a.name)} onChange={() => { const c = new Set(confirmed); if (c.has(a.name)) c.delete(a.name); else c.add(a.name); setConfirmed(c); }} />
                          <span>I understand: {v.reason}</span>
                        </label>
                      )}
                    </li>
                  );
                })}
              </ul>
            </Panel>
          ))}

          <div className="sticky bottom-0 -mx-8 border-t border-ink-700 bg-ink-950/95 px-8 py-4 backdrop-blur">
            {busy === "apply" ? (
              <div className="text-sm">
                <div className="flex items-center gap-2"><Loader2 size={14} className="animate-spin" />
                  {progress?.kind === "package" ? `Disabling ${progress.index} of ${progress.total}: ${progress.name}` : progress?.kind === "verified" ? `Batch ${progress.batch} of ${progress.batches} verified on the TV` : "Saving the TV's current state…"}
                </div>
                {progress?.kind === "package" && <div className="mt-2 h-1.5 rounded bg-ink-800"><div className="h-1.5 rounded bg-accent-500" style={{ width: `${(progress.index / progress.total) * 100}%` }} /></div>}
              </div>
            ) : askApply ? (
              <div className="flex flex-wrap items-center gap-3 text-sm">
                <span>Disable {applySet.length} package{applySet.length === 1 ? "" : "s"} now? The TV's current state is saved first, and each batch is read back from the TV.</span>
                <Button variant="primary" onClick={() => void apply()}>Disable now</Button>
                <Button onClick={() => setAskApply(false)}>Cancel</Button>
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-3">
                <span className="text-sm text-ink-300">{selected.size} selected</span>
                <Button onClick={() => void check()} disabled={!selected.size || busy !== null}>
                  {busy === "eval" ? <Loader2 size={14} className="animate-spin" /> : <ShieldCheck size={14} />}Check on TV (dry run)
                </Button>
                <Button variant="primary" onClick={() => setAskApply(true)} disabled={!ready || busy !== null}>Disable {applySet.length || ""} selected</Button>
                {onAddToPlan && <Button onClick={() => onAddToPlan(applySet, needConfirm)} disabled={!ready || busy !== null}>Add to deployment plan</Button>}
                {!evalNow && selected.size > 0 && <span className="text-xs text-ink-400">Run the check first: it reads Home, inputs and Wi-Fi ownership from the TV.</span>}
                {evalNow && needConfirm.some((n) => !confirmed.has(n)) && <span className="text-xs text-warn-400">Confirm the highlighted items to continue.</span>}
              </div>
            )}
          </div>
        </>
      )}

      {result && <ReportPanel title="Result" report={result.report} extra={result.baseline_created ? "The TV's original state was saved as its baseline. Restore can always return to it." : "This TV's original baseline from an earlier run is kept unchanged."}
        action={result.report.changed.length > 0 && !undo && <Button onClick={() => void undoRun()} disabled={busy !== null}>{busy === "undo" ? <Loader2 size={14} className="animate-spin" /> : <Undo2 size={14} />}Undo this run</Button>} />}
      {undo && <ReportPanel title="Undo result" report={undo} />}
    </div>
  );
}

export function ReportPanel({ title, report, extra, action }: { title: string; report: ChangeReport; extra?: string; action?: React.ReactNode }) {
  return (
    <Panel title={title} action={action || undefined}>
      {report.stopped && <div className="mb-3"><ErrorNote message={`Stopped: ${report.stopped}`} /></div>}
      <ul className="space-y-1 text-sm">
        <li className="text-accent-400">{report.changed.length} verified on the TV</li>
        {report.refused.length > 0 && <li className="text-warn-400">{report.refused.length} refused by the TV</li>}
        {report.unchanged.length > 0 && <li className="text-warn-400">{report.unchanged.length} reported success but did not change: {report.unchanged.join(", ")}</li>}
        {report.skipped.length > 0 && <li className="text-ink-400">{report.skipped.length} not attempted</li>}
      </ul>
      {report.refused.length > 0 && <ul className="selectable mt-2 space-y-0.5 font-mono text-xs text-ink-400">{report.refused.map(([n, m]) => <li key={n}>{n}: {m}</li>)}</ul>}
      {extra && <p className="mt-3 text-xs text-ink-400">{extra}</p>}
    </Panel>
  );
}
