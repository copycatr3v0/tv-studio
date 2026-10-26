import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, CheckCircle2, Circle, Loader2, MinusCircle, OctagonX, Rocket, ShieldCheck, Undo2, Wifi, XCircle } from "lucide-react";
import { api, errMsg, onDeployEvent, type ChangeReport, type DeployEvent, type Plan, type Profile, type RunResult, type StepStatus, type ThemeIn } from "../lib/ipc";
import { urlToBase64 } from "../lib/files";
import type { ThemeEditor } from "../theme/useThemeEditor";
import { Preview } from "../theme/Preview";
import { SAMPLE_APPS } from "../theme/apps";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";
import { ReportPanel } from "./DebloatPage";

const ADBLOCK_DNS = "dns.adguard-dns.com";
type DnsChoice = "keep" | "automatic" | "off" | "adblock" | "custom";

function dnsChoice(p: Profile): DnsChoice {
  const d = p.settings.private_dns;
  if (d.mode === "hostname") return d.hostname === ADBLOCK_DNS ? "adblock" : "custom";
  return d.mode;
}

const PREVIEW_STYLE = {
  check: { label: "check", cls: "bg-ink-700 text-ink-300" },
  will_change: { label: "will change", cls: "bg-accent-900 text-accent-400" },
  already_done: { label: "already done", cls: "bg-ink-700 text-ink-400" },
  blocked: { label: "blocked", cls: "bg-danger-400/15 text-danger-400" },
} as const;

const STATUS_ICON: Record<StepStatus, JSX.Element> = {
  running: <Loader2 size={16} className="animate-spin text-ink-100" />,
  reconnecting: <Wifi size={16} className="animate-pulse text-warn-400" />,
  done: <CheckCircle2 size={16} className="text-accent-400" />,
  already_done: <CheckCircle2 size={16} className="text-ink-400" />,
  failed: <XCircle size={16} className="text-danger-400" />,
  skipped: <MinusCircle size={16} className="text-ink-400" />,
};

export async function themeIn(editor: ThemeEditor): Promise<ThemeIn> {
  const wallpapers = await Promise.all(editor.draft.wallpapers.map(async (w) => ({ name: w.name, data_base64: await urlToBase64(w.url) })));
  return { config: editor.draft.config as Record<string, unknown>, wallpapers, captions: null };
}

export function DeploymentPage({ serial, onSelect, profile, setProfile, editor, advanced, requireDryRun, onOpenDebloat }: {
  serial: string | null; onSelect: (s: string) => void; profile: Profile; setProfile: (p: Profile) => void;
  editor: ThemeEditor; advanced: boolean; requireDryRun: boolean; onOpenDebloat: () => void;
}) {
  const [plan, setPlan] = useState<{ key: string; plan: Plan } | null>(null);
  const [busy, setBusy] = useState<"plan" | "run" | "undo" | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [events, setEvents] = useState<Record<string, DeployEvent>>({});
  const [progress, setProgress] = useState<{ finished: number; total: number } | null>(null);
  const [result, setResult] = useState<RunResult | null>(null);
  const [undo, setUndo] = useState<ChangeReport | null>(null);
  const [interrupted, setInterrupted] = useState<{ id: number; started: string; name: string; last_step: string | null } | null>(null);
  const [customDns, setCustomDns] = useState(profile.settings.private_dns.mode === "hostname" && profile.settings.private_dns.hostname !== ADBLOCK_DNS ? profile.settings.private_dns.hostname : "");

  const key = JSON.stringify([serial, profile, profile.apply_theme ? editor.draft : null]);
  const planNow = plan?.key === key ? plan.plan : null;
  const upd = (f: (p: Profile) => void) => { const n = structuredClone(profile); f(n); setProfile(n); };

  useEffect(() => { let off = () => {}; void onDeployEvent((e) => { setEvents((m) => ({ ...m, [e.step]: e })); setProgress({ finished: e.finished + (["done", "already_done", "failed", "skipped"].includes(e.status) ? 1 : 0), total: e.total }); }).then((f) => { off = f; }); return () => off(); }, []);
  useEffect(() => { setInterrupted(null); if (serial) api.deployInterrupted(serial).then(setInterrupted, () => {}); }, [serial]);

  const nothingSelected = !profile.launcher.install && !profile.launcher.make_home && !profile.launcher.disable_stock && !profile.debloat.packages.length
    && profile.settings.animation_scale === null && profile.settings.private_dns.mode === "keep" && !profile.apply_theme;

  async function dryRun() {
    if (!serial) return;
    setBusy("plan"); setErr(null);
    try { setPlan({ key, plan: await api.deployPlan(serial, profile, profile.apply_theme ? await themeIn(editor) : null) }); }
    catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  async function apply(resumeId?: number) {
    if (!serial) return;
    setBusy("run"); setErr(null); setEvents({}); setProgress(null); setResult(null); setUndo(null);
    try {
      const r = resumeId !== undefined ? await api.deployResume(serial, resumeId) : await api.deployRun(serial, profile, profile.apply_theme ? await themeIn(editor) : null);
      setResult(r); setInterrupted(null);
    } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  async function undoRun() {
    if (!serial || !result) return;
    setBusy("undo"); setErr(null);
    try { setUndo(await api.undoDeployment(serial, result.deployment_id)); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  const canApply = !!serial && !busy && !nothingSelected && (planNow ? planNow.runnable : !requireDryRun);
  const runSteps = useMemo(() => (result?.plan ?? planNow)?.steps ?? [], [result, planNow]);
  const label = "flex items-start gap-3 text-sm";
  const hint = "block text-xs text-ink-400";

  return (
    <div className="max-w-5xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {interrupted && !busy && !result && (
        <div className="flex flex-wrap items-center gap-3 rounded-lg border border-warn-400/40 bg-warn-400/10 px-4 py-3 text-sm">
          <AlertTriangle size={16} className="text-warn-400" />
          <span className="flex-1">“{interrupted.name}” started {interrupted.started} did not finish{interrupted.last_step ? ` (last step: ${interrupted.last_step})` : ""}. Resuming re-checks the TV and only does what is left.</span>
          <Button variant="primary" onClick={() => void apply(interrupted.id)}>Resume</Button>
          <Button onClick={() => { void api.deployDiscard(interrupted.id); setInterrupted(null); }}>Discard</Button>
        </div>
      )}
      {err && <ErrorNote message={err} />}

      {!busy && !result && (
        <div className="grid gap-5 lg:grid-cols-2">
          <Panel title="Launcher & look">
            <div className="space-y-3">
              <label className={label}><input type="checkbox" className="mt-1 accent-teal-400" checked={profile.launcher.install} onChange={(e) => upd((p) => { p.launcher.install = e.target.checked; })} />
                <span>Install the Studio launcher<span className={hint}>Your current launcher stays installed.</span></span></label>
              <label className={label}><input type="checkbox" className="mt-1 accent-teal-400" checked={profile.launcher.make_home} onChange={(e) => upd((p) => { p.launcher.make_home = e.target.checked; if (!e.target.checked) p.launcher.disable_stock = false; })} />
                <span>Make it the Home screen<span className={hint}>Confirmed by pressing Home on the TV.</span></span></label>
              {advanced && (
                <label className={label}><input type="checkbox" className="mt-1 accent-teal-400" disabled={!profile.launcher.make_home} checked={profile.launcher.disable_stock} onChange={(e) => upd((p) => { p.launcher.disable_stock = e.target.checked; })} />
                  <span>Disable the stock launcher<span className={hint}>Only after Home is confirmed, re-checked on the TV, and put back at once if Home stops answering. On Google TV the stock launcher also runs the remote's app buttons.</span></span></label>
              )}
              <label className={label}><input type="checkbox" className="mt-1 accent-teal-400" checked={profile.apply_theme} onChange={(e) => upd((p) => { p.apply_theme = e.target.checked; })} />
                <span>Apply the theme “{editor.draft.name}”<span className={hint}>The theme currently in the Live Theme Editor.</span></span></label>
              {profile.apply_theme && <Preview config={editor.draft.config} apps={SAMPLE_APPS} wallpaperUrl={editor.draft.wallpapers[0]?.url ?? null} width={320} />}
            </div>
          </Panel>

          <div className="space-y-5">
            <Panel title="Apps to disable" action={<Button onClick={onOpenDebloat}>Choose in Debloat Center</Button>}>
              {profile.debloat.packages.length === 0 ? <p className="text-sm text-ink-400">None. Pick apps in the Debloat Center, run its check, then use “Add to plan”.</p> : (
                <>
                  <p className="text-sm">{profile.debloat.packages.length} app{profile.debloat.packages.length === 1 ? "" : "s"}{profile.debloat.confirmed.length ? `, ${profile.debloat.confirmed.length} confirmed individually` : ""}</p>
                  <p className="selectable mt-1 line-clamp-3 font-mono text-xs text-ink-400">{profile.debloat.packages.join(", ")}</p>
                  <button className="mt-2 text-xs text-ink-400 underline" onClick={() => upd((p) => { p.debloat = { packages: [], confirmed: [] }; })}>Clear</button>
                </>
              )}
            </Panel>
            <Panel title="Settings">
              <div className="space-y-4 text-sm">
                <label className="block">Animation speed
                  <select aria-label="Animation speed" className="ml-3 rounded-md border border-ink-600 bg-ink-850 px-2 py-1" value={profile.settings.animation_scale ?? "keep"}
                    onChange={(e) => upd((p) => { p.settings.animation_scale = e.target.value === "keep" ? null : e.target.value; })}>
                    <option value="keep">Keep the TV's</option><option value="1.0">Normal (1×)</option><option value="0.5">Faster (0.5×)</option>
                    <option value="0.25">Much faster (0.25×)</option><option value="0">Off</option>
                  </select>
                  <span className={hint}>Shorter animations feel snappier; they don't make apps load faster.</span>
                </label>
                <label className="block">Private DNS
                  <select aria-label="Private DNS" className="ml-3 rounded-md border border-ink-600 bg-ink-850 px-2 py-1" value={dnsChoice(profile)}
                    onChange={(e) => upd((p) => {
                      const v = e.target.value as DnsChoice;
                      p.settings.private_dns = v === "adblock" ? { mode: "hostname", hostname: ADBLOCK_DNS } : v === "custom" ? { mode: "hostname", hostname: customDns } : { mode: v };
                    })}>
                    <option value="keep">Keep the TV's</option><option value="automatic">Automatic (Android default)</option><option value="off">Off</option>
                    <option value="adblock">Ad-blocking (AdGuard DNS)</option><option value="custom">Custom hostname…</option>
                  </select>
                  {dnsChoice(profile) === "custom" && <input aria-label="DNS hostname" className="selectable ml-3 mt-2 w-56 rounded-md border border-ink-600 bg-ink-850 px-2 py-1 font-mono text-xs" placeholder="dns.example.com" value={customDns}
                    onChange={(e) => { setCustomDns(e.target.value); upd((p) => { p.settings.private_dns = { mode: "hostname", hostname: e.target.value.trim() }; }); }} />}
                  {profile.settings.private_dns.mode === "hostname" && <span className="mt-1 block text-xs text-warn-400">Android 9+. Goes around DNS filtering on your router (Pi-hole, AdGuard Home, VPN rules) and doesn't block YouTube ads. Check that your streaming apps still play afterwards.</span>}
                </label>
              </div>
            </Panel>
          </div>
        </div>
      )}

      {!busy && !result && (
        <div className="flex flex-wrap items-center gap-3">
          <Button onClick={() => void dryRun()} disabled={!serial || nothingSelected}><ShieldCheck size={14} />Dry run</Button>
          <Button variant="primary" onClick={() => void apply()} disabled={!canApply}><Rocket size={14} />Apply Everything</Button>
          {nothingSelected && <span className="text-xs text-ink-400">Choose at least one change.</span>}
          {!nothingSelected && requireDryRun && !planNow && <span className="text-xs text-ink-400">Run the dry run first (Settings: “Start every deployment as a dry run”).</span>}
          {planNow && !planNow.runnable && <span className="text-xs text-danger-400">Resolve the blocked steps first.</span>}
        </div>
      )}

      {busy === "plan" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Reading the TV and planning…</p>}

      {(planNow || busy === "run" || result) && (
        <Panel title={busy === "run" ? "Applying" : result ? (result.report.completed ? "Done" : "Finished with problems") : "Plan (dry run, nothing changed yet)"}
          action={busy === "run" ? <Button onClick={() => void api.deployCancel()}>Stop after this step</Button> : undefined}>
          {busy === "run" && progress && (
            <div className="mb-4">
              <div className="h-1.5 rounded bg-ink-800"><div className="h-1.5 rounded bg-accent-500 transition-all" style={{ width: `${(progress.finished / Math.max(1, progress.total)) * 100}%` }} /></div>
              <p className="mt-1 text-xs text-ink-400">{progress.finished} of {progress.total} steps finished</p>
            </div>
          )}
          {busy === "run" && runSteps.length === 0 && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Re-reading the TV before the first change…</p>}
          <ol className="space-y-2">
            {(result ? result.report.results.map((r) => ({ id: r.id, title: r.title, status: r.status as StepStatus | null, message: r.message, preview: null }))
              : runSteps.map((s) => ({ id: s.id, title: s.title, status: (events[s.id]?.status ?? null) as StepStatus | null, message: events[s.id]?.message ?? "", preview: s.preview }))
            ).map((s) => (
              <li key={s.id} className="flex gap-3 text-sm">
                <span className="mt-0.5">{s.status ? STATUS_ICON[s.status] : s.preview?.state === "blocked" ? <OctagonX size={16} className="text-danger-400" /> : <Circle size={16} className="text-ink-600" />}</span>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2"><span className="font-medium">{s.title}</span>
                    {!s.status && s.preview && <span className={`rounded px-1.5 py-0.5 text-[10px] ${PREVIEW_STYLE[s.preview.state].cls}`}>{PREVIEW_STYLE[s.preview.state].label}</span>}
                  </div>
                  <div className={`break-words text-xs ${s.status === "failed" || s.preview?.state === "blocked" ? "text-danger-400" : "text-ink-400"}`}>{s.message || s.preview?.detail}</div>
                </div>
              </li>
            ))}
          </ol>
          {planNow && !result && planNow.notes.length > 0 && <ul className="mt-4 space-y-1 text-xs text-warn-400">{planNow.notes.map((n) => <li key={n}>• {n}</li>)}</ul>}
          {result && (
            <div className="mt-5 flex flex-wrap items-center gap-3 border-t border-ink-700 pt-4">
              {result.report.stopped && <span className="text-sm text-warn-400">Stopped: {result.report.stopped}</span>}
              <Button onClick={() => void undoRun()} disabled={!!undo}><Undo2 size={14} />Undo this deployment</Button>
              <Button onClick={() => { setResult(null); setPlan(null); }}>New deployment</Button>
              <span className="text-xs text-ink-400">Undo returns to the state saved right before this run. “Revert to original” returns to the very first one.</span>
            </div>
          )}
        </Panel>
      )}
      {busy === "undo" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Undoing…</p>}
      {undo && <ReportPanel title="Undo result" report={undo} />}
    </div>
  );
}
