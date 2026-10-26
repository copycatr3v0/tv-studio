import { useEffect, useMemo, useRef, useState } from "react";
import { Check, CheckCircle2, Loader2, Rocket, ShieldCheck, Undo2, XCircle } from "lucide-react";
import {
  api, emptyProfile, errMsg, onDeployEvent, PLATFORM_TOOLS_TERMS,
  type AppInfo, type ChangeReport, type DeployEvent, type DeviceInfo, type LauncherInfo, type Plan, type Profile, type RunResult,
} from "../lib/ipc";
import type { ThemeEditor } from "../theme/useThemeEditor";
import { Button, ErrorNote, Panel, Row } from "../components/ui";
import { ConnectPage } from "./ConnectPage";
import { themeIn } from "./DeploymentPage";
import { ReportPanel } from "./DebloatPage";

const STEPS = ["Get ready", "Connect", "Your TV", "Choose", "Check", "Apply"] as const;
export type Goal = "tidy" | "balanced" | "full";

export const GOALS: { id: Goal; title: string; text: string }[] = [
  { id: "tidy", title: "Gentle clean-up", text: "Turns off apps that tested TVs commonly turn off for ads, tracking and demo or retail features. The most careful choice." },
  { id: "balanced", title: "Standard clean-up", text: "Everything in the gentle clean-up, plus pre-installed extras that most people never open." },
  { id: "full", title: "Clean-up and a new look", text: "Standard clean-up, plus the lightweight Studio launcher and your current theme. Your original launcher stays installed." },
];

/** Packages a goal would disable: enabled apps the tested-TV evidence recommends for that mode, and that the TV's live checks allow. */
export function profileFor(goal: Goal, packages: string[], animations: boolean, name: string): Profile {
  const p = emptyProfile(name);
  p.debloat.packages = packages;
  p.settings.animation_scale = animations ? "0.5" : null;
  if (goal === "full") { p.launcher = { install: true, make_home: true, disable_stock: false }; p.apply_theme = true; }
  return p;
}

export function GuidePage({ serial, onSelect, editor, onFinish, onOpen }: {
  serial: string | null; onSelect: (s: string) => void; editor: ThemeEditor;
  onFinish: () => void; onOpen: (id: "benchmarks" | "restore" | "privacy" | "deploy") => void;
}) {
  const [step, setStep] = useState(0);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [tv, setTv] = useState<DeviceInfo | null>(null);
  const [launcher, setLauncher] = useState<LauncherInfo | null>(null);
  const [goal, setGoal] = useState<Goal>("tidy");
  const [animations, setAnimations] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [skipped, setSkipped] = useState<[string, string][]>([]);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [events, setEvents] = useState<Record<string, DeployEvent>>({});
  const [progress, setProgress] = useState<{ finished: number; total: number } | null>(null);
  const [result, setResult] = useState<RunResult | null>(null);
  const [undo, setUndo] = useState<ChangeReport | null>(null);
  const running = useRef(false);

  const refreshInfo = () => api.appInfo().then(setInfo, (e: unknown) => setErr(errMsg(e)));
  useEffect(() => { void refreshInfo(); }, []);
  useEffect(() => {
    let off = () => {};
    void onDeployEvent((e) => { setEvents((m) => ({ ...m, [e.step]: e })); setProgress({ finished: e.finished + (["done", "already_done", "failed", "skipped"].includes(e.status) ? 1 : 0), total: e.total }); }).then((f) => { off = f; });
    return () => off();
  }, []);

  const go = (n: number) => { setErr(null); setStep(n); };

  async function getAdb() {
    setBusy("adb"); setErr(null);
    try { await api.installAdb(); await refreshInfo(); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  async function lookAtTv() {
    if (!serial) return;
    setBusy("tv"); setErr(null); setTv(null);
    try {
      const [d, l] = await Promise.all([api.probe(serial), api.launcherStatus(serial).catch(() => null)]);
      setTv(d); setLauncher(l);
    } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }
  useEffect(() => { if (step === 2 && serial && !tv && !busy) void lookAtTv(); /* eslint-disable-next-line react-hooks/exhaustive-deps */ }, [step, serial]);

  async function check() {
    if (!serial) return;
    setBusy("check"); setErr(null); setPlan(null); setProfile(null); setSkipped([]);
    try {
      const inv = await api.inventory(serial);
      const wanted = inv.apps.filter((a) => a.enabled && (goal === "tidy" ? a.preselect_safe : a.preselect_balanced)).map((a) => a.name);
      // The TV's live checks decide: anything not plainly allowed (Home, Wi-Fi owner, TV inputs, protected) is left alone.
      const ev = wanted.length ? await api.evaluate(serial, wanted) : { verdicts: {}, facts: {} };
      const keep: string[] = []; const left: [string, string][] = [];
      for (const n of wanted) {
        const v = ev.verdicts[n];
        if (v?.kind === "Allowed") keep.push(n); else left.push([n, v && "reason" in v ? v.reason : "Not confirmed safe on this TV"]);
      }
      const p = profileFor(goal, keep, animations, `Guided setup (${GOALS.find((g) => g.id === goal)!.title})`);
      setProfile(p); setSkipped(left);
      setPlan(await api.deployPlan(serial, p, p.apply_theme ? await themeIn(editor) : null));
    } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }
  useEffect(() => { if (step === 4 && serial && !plan && !busy) void check(); /* eslint-disable-next-line react-hooks/exhaustive-deps */ }, [step]);

  async function apply() {
    if (!serial || !profile || running.current) return;
    running.current = true;
    setBusy("apply"); setErr(null); setEvents({}); setProgress(null); setResult(null); setUndo(null);
    try { setResult(await api.deployRun(serial, profile, profile.apply_theme ? await themeIn(editor) : null)); }
    catch (e) { setErr(errMsg(e)); } finally { setBusy(null); running.current = false; }
  }

  async function undoRun() {
    if (!serial || !result) return;
    setBusy("undo"); setErr(null);
    try { setUndo(await api.undoDeployment(serial, result.deployment_id)); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  const changes = useMemo(() => plan?.steps.filter((s) => s.preview.state === "will_change") ?? [], [plan]);
  const blocked = useMemo(() => plan?.steps.filter((s) => s.preview.state === "blocked") ?? [], [plan]);
  const apkMissing = launcher !== null && !launcher.apk_bundled;
  const locked = busy === "apply" || busy === "undo";

  return (
    <div className="max-w-3xl space-y-5">
      <ol className="flex flex-wrap gap-2" aria-label="Steps">
        {STEPS.map((s, i) => (
          <li key={s} aria-current={i === step ? "step" : undefined}
            className={`flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs ${i === step ? "border-accent-500 bg-accent-900 text-accent-400" : i < step ? "border-ink-600 text-ink-300" : "border-ink-700 text-ink-500"}`}>
            {i < step ? <Check size={12} /> : <span>{i + 1}</span>}{s}
          </li>
        ))}
      </ol>
      {err && <ErrorNote message={err} />}

      {step === 0 && (
        <Panel title="Get ready">
          <p className="text-sm text-ink-300">This guide takes you from nothing to a cleaned-up TV in six steps. Nothing is changed on your TV until the last one, and you can undo everything afterwards.</p>
          <p className="mt-3 text-sm text-ink-300">On the TV, turn on <strong>Developer options</strong> (Settings → Device preferences → About → press <em>Build</em> seven times), then switch on <strong>USB debugging</strong> or <strong>Wireless debugging</strong>.</p>
          <dl className="mt-4"><Row label="adb (the tool that talks to the TV)">
            {info?.adb ? <span className="text-accent-400">Ready ({info.adb.source})</span> : info ? <span className="text-warn-400">Not set up yet</span> : "Checking…"}
          </Row></dl>
          {info && !info.adb && (
            <div className="mt-3 rounded-lg border border-ink-600 p-4 text-sm">
              <p className="text-ink-400">
                Google&rsquo;s licence doesn&rsquo;t let other apps ship adb, so the app downloads the official copy from Google (about 7 MB) when you ask.
                By downloading it you accept Google&rsquo;s <a href={PLATFORM_TOOLS_TERMS} onClick={(e) => { e.preventDefault(); api.openPlatformToolsTerms().catch((x: unknown) => setErr(errMsg(x))); }} className="text-accent-400 underline">platform-tools terms</a>.
              </p>
              <div className="mt-3"><Button variant="primary" onClick={() => void getAdb()} disabled={busy === "adb"}>{busy === "adb" ? "Downloading from Google…" : "Get adb from Google"}</Button></div>
            </div>
          )}
          <div className="mt-5"><Button variant="primary" onClick={() => go(1)} disabled={!info?.adb}>Next: connect to your TV</Button></div>
        </Panel>
      )}

      {step === 1 && (
        <>
          {serial && (
            <div className="flex flex-wrap items-center gap-3 rounded-lg border border-accent-500/40 bg-accent-900/40 px-4 py-3 text-sm">
              <CheckCircle2 size={16} className="text-accent-400" /><span className="flex-1">Connected to <span className="selectable font-mono">{serial}</span></span>
              <Button variant="primary" onClick={() => go(2)}>Next: look at your TV</Button>
            </div>
          )}
          <ConnectPage onConnected={(s) => { onSelect(s); setTv(null); go(2); }} />
        </>
      )}

      {step === 2 && (
        <Panel title="Your TV">
          {busy === "tv" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Reading the TV (nothing is changed)…</p>}
          {tv && (
            <>
              <dl>
                <Row label="Model">{[tv.brand, tv.model].filter(Boolean).join(" ") || "Unknown"}</Row>
                <Row label="Android">{tv.android_release ?? "?"}{tv.sdk ? ` (API ${tv.sdk})` : ""}</Row>
                <Row label="Home screen">{tv.home.kind === "component" ? <span className="selectable font-mono text-xs">{tv.home.component}</span> : tv.home.kind === "chooser" ? "Asks which launcher to use" : "Unknown"}</Row>
                <Row label="Apps">{tv.package_count ?? "?"} installed, {tv.disabled_count ?? 0} already off</Row>
              </dl>
              {tv.oem.map((o) => <p key={o.family} className="mt-3 text-xs text-ink-400"><strong>{o.label}:</strong> {o.note}</p>)}
              {tv.findings.map((f) => <p key={f.message} className={`mt-2 text-xs ${f.severity === "info" ? "text-ink-400" : "text-warn-400"}`}>{f.message}</p>)}
              <p className="mt-4 text-sm text-ink-300">Before the first change, the app saves this TV&rsquo;s original state: which apps were on, the Home screen, and the animation and DNS settings. <strong>Revert to original</strong> (top right) puts it all back.</p>
            </>
          )}
          <div className="mt-5 flex gap-3"><Button onClick={() => go(1)}>Back</Button><Button variant="primary" onClick={() => go(3)} disabled={!tv}>Next: choose what to do</Button></div>
        </Panel>
      )}

      {step === 3 && (
        <Panel title="What would you like?">
          <fieldset className="space-y-2">
            <legend className="sr-only">Goal</legend>
            {GOALS.map((g) => {
              const off = g.id === "full" && apkMissing;
              return (
                <label key={g.id} className={`flex items-start gap-3 rounded-lg border p-3 text-sm ${goal === g.id ? "border-accent-500 bg-accent-900/40" : "border-ink-700"} ${off ? "opacity-50" : "cursor-pointer"}`}>
                  <input type="radio" name="goal" className="mt-1 accent-teal-400" disabled={off} checked={goal === g.id} onChange={() => { setGoal(g.id); setPlan(null); }} />
                  <span><span className="font-medium">{g.title}{g.id === "tidy" ? " (recommended)" : ""}</span><span className="block text-xs text-ink-400">{g.text}</span>
                    {off && <span className="block text-xs text-warn-400">This build of the app doesn&rsquo;t include the launcher.</span>}</span>
                </label>
              );
            })}
          </fieldset>
          <label className="mt-4 flex items-start gap-3 text-sm">
            <input type="checkbox" className="mt-1 accent-teal-400" checked={animations} onChange={(e) => { setAnimations(e.target.checked); setPlan(null); }} />
            <span>Make menu animations shorter (0.5×)<span className="block text-xs text-ink-400">Feels snappier. It doesn&rsquo;t make apps load faster.</span></span>
          </label>
          <p className="mt-4 text-xs text-ink-400">Private DNS, the stock launcher and individual app choices are in the Deployment and Debloat Center pages if you want them.</p>
          <div className="mt-5 flex gap-3"><Button onClick={() => go(2)}>Back</Button><Button variant="primary" onClick={() => { setPlan(null); go(4); }}>Next: preview the changes</Button></div>
        </Panel>
      )}

      {step === 4 && (
        <Panel title="Check before anything changes">
          {busy === "check" && <p className="flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Checking every app against this TV…</p>}
          {plan && profile && (
            <>
              <p className="text-sm"><strong>{profile.debloat.packages.length}</strong> app{profile.debloat.packages.length === 1 ? "" : "s"} would be turned off{profile.apply_theme ? ", the Studio launcher installed as Home, and your theme sent" : ""}{profile.settings.animation_scale ? ", with shorter animations" : ""}.
                Turned-off apps stay on the TV and come back with one click.</p>
              {profile.debloat.packages.length > 0 && (
                <details className="mt-2 text-xs text-ink-400"><summary className="cursor-pointer">Show the apps</summary><p className="selectable mt-1 font-mono">{profile.debloat.packages.join(", ")}</p></details>
              )}
              {skipped.length > 0 && (
                <details className="mt-2 text-xs text-ink-400"><summary className="cursor-pointer">{skipped.length} app{skipped.length === 1 ? "" : "s"} left alone because this TV needs {skipped.length === 1 ? "it" : "them"}</summary>
                  <ul className="mt-1 space-y-0.5">{skipped.map(([n, why]) => <li key={n}><span className="selectable font-mono">{n}</span>: {why}</li>)}</ul></details>
              )}
              <ul className="mt-4 space-y-1.5 text-sm">
                {changes.map((s) => <li key={s.id} className="flex gap-2"><ShieldCheck size={15} className="mt-0.5 shrink-0 text-accent-400" /><span><span className="font-medium">{s.title}</span> <span className="text-xs text-ink-400">{s.preview.detail}</span></span></li>)}
                {changes.length === 0 && blocked.length === 0 && <li className="text-ink-400">Nothing to change: this TV already matches.</li>}
                {blocked.map((s) => <li key={s.id} className="flex gap-2 text-danger-400"><XCircle size={15} className="mt-0.5 shrink-0" /><span><span className="font-medium">{s.title}</span> <span className="text-xs">{s.preview.detail}</span></span></li>)}
              </ul>
              {plan.notes.length > 0 && <ul className="mt-3 space-y-1 text-xs text-warn-400">{plan.notes.map((n) => <li key={n}>• {n}</li>)}</ul>}
            </>
          )}
          <div className="mt-5 flex flex-wrap gap-3">
            <Button onClick={() => go(3)}>Back</Button>
            <Button variant="primary" onClick={() => go(5)} disabled={!plan?.runnable || changes.length === 0}>Looks right: continue</Button>
            {plan && !plan.runnable && <span className="self-center text-xs text-danger-400">Something is blocked. Go back and pick a gentler option.</span>}
          </div>
        </Panel>
      )}

      {step === 5 && (
        <Panel title={result ? (result.report.completed ? "All done" : "Finished with problems") : "Apply"}>
          {!busy && !result && (
            <>
              <p className="text-sm text-ink-300">This will change your TV now. The app re-reads the TV first, saves its current state, then changes things in small steps and checks each one.</p>
              <div className="mt-4 flex gap-3"><Button onClick={() => go(4)}>Back</Button><Button variant="primary" onClick={() => void apply()}><Rocket size={14} />Apply to my TV</Button></div>
            </>
          )}
          {busy === "apply" && (
            <div>
              <div className="h-1.5 rounded bg-ink-800"><div className="h-1.5 rounded bg-accent-500 transition-all" style={{ width: `${((progress?.finished ?? 0) / Math.max(1, progress?.total ?? 1)) * 100}%` }} /></div>
              <p className="mt-2 text-sm text-ink-300">{Object.values(events).filter((e) => e.status === "running" || e.status === "reconnecting").map((e) => e.title)[0] ?? "Re-reading the TV before the first change…"}</p>
              <p className="mt-1 text-xs text-ink-400">Keep the TV on. If it drops off Wi-Fi, the app reconnects and carries on.</p>
              <div className="mt-3"><Button onClick={() => void api.deployCancel()}>Stop after this step</Button></div>
            </div>
          )}
          {result && (
            <>
              <ul className="space-y-1.5 text-sm">
                {result.report.results.map((r) => (
                  <li key={r.id} className="flex gap-2">
                    {r.status === "failed" ? <XCircle size={15} className="mt-0.5 shrink-0 text-danger-400" /> : <CheckCircle2 size={15} className="mt-0.5 shrink-0 text-accent-400" />}
                    <span><span className="font-medium">{r.title}</span> <span className={`text-xs ${r.status === "failed" ? "text-danger-400" : "text-ink-400"}`}>{r.message}</span></span>
                  </li>
                ))}
              </ul>
              {result.report.stopped && <p className="mt-3 text-sm text-warn-400">Stopped: {result.report.stopped}</p>}
              <div className="mt-5 rounded-lg border border-ink-700 p-4 text-sm">
                <p className="font-medium">What next</p>
                <ul className="mt-2 space-y-1 text-xs text-ink-300">
                  <li>• Use the TV normally for a day. If anything you use is missing, <strong>Revert to original</strong> (top right) or <button className="underline" onClick={() => onOpen("restore")}>Restore</button> brings it back.</li>
                  <li>• <button className="underline" onClick={() => onOpen("benchmarks")}>Run a benchmark</button> to compare responsiveness before and after. Results vary by TV; there are no promises.</li>
                  <li>• <button className="underline" onClick={() => onOpen("privacy")}>Review the privacy audit</button> for voice and screen-recognition components.</li>
                </ul>
              </div>
              <div className="mt-4 flex flex-wrap gap-3">
                <Button onClick={() => void undoRun()} disabled={!!undo || busy === "undo"}><Undo2 size={14} />Undo this run</Button>
                <Button variant="primary" onClick={onFinish}>Finish</Button>
              </div>
              {busy === "undo" && <p className="mt-3 flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Undoing…</p>}
              {undo && <div className="mt-4"><ReportPanel title="Undo result" report={undo} /></div>}
            </>
          )}
        </Panel>
      )}
      {locked && <p className="sr-only" role="status">Working on your TV</p>}
    </div>
  );
}
