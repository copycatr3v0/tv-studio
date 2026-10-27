import { useEffect, useState } from "react";
import { CheckCircle2, Circle, Loader2, X, XCircle } from "lucide-react";
import { api, errMsg, type HomeCheck, type LauncherInfo, type ThemeReport } from "../lib/ipc";
import type { Draft } from "../theme/useThemeEditor";
import { Button, ErrorNote } from "./ui";
import { hasVideo, themePayload } from "../theme/payload";
import { heavyFeatures, LOW_RAM_KB } from "../theme/layout";

type StepState = "todo" | "busy" | "done" | "failed" | "skipped";
const Icon = ({ s }: { s: StepState }) => s === "done" ? <CheckCircle2 size={16} className="text-accent-400" />
  : s === "busy" ? <Loader2 size={16} className="animate-spin" /> : s === "failed" ? <XCircle size={16} className="text-danger-400" />
  : <Circle size={16} className="text-ink-600" />;

/** Install the launcher if needed → optionally make it Home (verified on screen) → push the theme (verified). */
export function ApplyThemeDialog({ serial, draft, onClose }: { serial: string; draft: Draft; onClose: () => void }) {
  const [info, setInfo] = useState<LauncherInfo | null>(null);
  const [makeHome, setMakeHome] = useState(true);
  const [steps, setSteps] = useState<Record<"install" | "home" | "theme", StepState>>({ install: "todo", home: "todo", theme: "todo" });
  const [home, setHome] = useState<HomeCheck | null>(null);
  const [report, setReport] = useState<ThemeReport | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [withVideo, setWithVideo] = useState(false);
  const canvas = !!draft.layout?.enabled;
  const [ramKb, setRamKb] = useState<number | null>(null);
  const [keepHeavy, setKeepHeavy] = useState(!!draft.layout?.heavyOnLowRam);
  const heavy = canvas && draft.layout ? heavyFeatures(draft.layout, /\.(gif|webp)$/i.test(draft.wallpapers[0]?.name ?? "")) : [];
  const lowRam = ramKb !== null && ramKb < LOW_RAM_KB;

  useEffect(() => { api.probe(serial).then((d) => setRamKb(d.ram_total_kb ?? null), () => setRamKb(null)); }, [serial]);
  useEffect(() => { api.launcherStatus(serial).then((i) => { setInfo(i); setMakeHome(!i.status.is_resolved_home); }, (e: unknown) => setErr(errMsg(e))); }, [serial]);
  const set = (k: keyof typeof steps, v: StepState) => setSteps((s) => ({ ...s, [k]: v }));

  async function run() {
    if (!info) return;
    setRunning(true); setErr(null);
    try {
      if (!info.status.installed) {
        set("install", "busy");
        await api.installLauncher(serial);
        set("install", "done");
      } else set("install", "skipped");
      if (makeHome) {
        set("home", "busy");
        const h = await api.makeLauncherHome(serial);
        setHome(h);
        set("home", h.verified ? "done" : "failed");
      } else set("home", "skipped");
      set("theme", "busy");
      const sent = draft.layout && keepHeavy !== !!draft.layout.heavyOnLowRam ? { ...draft, layout: { ...draft.layout, heavyOnLowRam: keepHeavy } } : draft;
      const r = await api.deployTheme(serial, await themePayload(sent, withVideo));
      setReport(r);
      set("theme", r.config_verified ? "done" : "failed");
    } catch (e) {
      setErr(errMsg(e));
      setSteps((s) => Object.fromEntries(Object.entries(s).map(([k, v]) => [k, v === "busy" ? "failed" : v])) as typeof s);
    } finally { setRunning(false); }
  }

  const done = steps.theme === "done" || steps.theme === "failed";
  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 p-8 pt-20" onClick={() => !running && onClose()}>
      <div role="dialog" aria-modal="true" aria-labelledby="apply-title" onClick={(e) => e.stopPropagation()} className="w-full max-w-xl rounded-xl border border-ink-700 bg-ink-900 shadow-2xl">
        <header className="flex items-center justify-between border-b border-ink-700 px-5 py-3">
          <h2 id="apply-title" className="text-sm font-semibold">Apply “{draft.name}” to the TV</h2>
          <button aria-label="Close" disabled={running} onClick={onClose} className="text-ink-400 hover:text-ink-100"><X size={18} /></button>
        </header>
        <div className="space-y-4 p-5 text-sm">
          {!info && !err && <p className="flex items-center gap-2 text-ink-400"><Loader2 size={14} className="animate-spin" />Checking the launcher on the TV…</p>}
          {info && (
            <ol className="space-y-3">
              <li className="flex gap-3"><Icon s={steps.install} /><div>
                <div className="font-medium">{info.status.installed ? `Studio launcher ${info.status.version ?? ""} is installed` : "Install the Studio launcher"}</div>
                {!info.status.installed && !info.apk_bundled && <div className="text-xs text-danger-400">This build doesn't include the launcher APK (see launcher/NOTICE.md).</div>}
                {!info.status.installed && <div className="text-xs text-ink-400">Your current launcher stays installed and enabled.</div>}
              </div></li>
              <li className="flex gap-3"><Icon s={steps.home} /><div className="flex-1">
                <label className="flex items-center gap-2 font-medium">
                  <input type="checkbox" className="accent-teal-400" disabled={running || done} checked={makeHome} onChange={(e) => setMakeHome(e.target.checked)} />
                  Make it the Home screen
                </label>
                <div className="text-xs text-ink-400">{info.status.is_resolved_home ? "It is already the default Home." : "The app presses Home on the TV and checks what actually appears."}</div>
                {home && !home.verified && <div className="mt-1 text-xs text-warn-400">{home.note}</div>}
                {home?.verified && <div className="mt-1 text-xs text-accent-400">Confirmed on screen.</div>}
              </div></li>
              <li className="flex gap-3"><Icon s={steps.theme} /><div>
                <div className="font-medium">Send the theme ({Object.keys(draft.config).length} settings, {draft.wallpapers.length} wallpaper{draft.wallpapers.length === 1 ? "" : "s"})</div>
                {canvas && <div className="text-xs text-ink-300">Includes your canvas layout (positions, fonts, motion, clock and numbers). Revert to original removes it.</div>}
                {hasVideo(draft) && (
                  <label className="mt-1 flex items-start gap-2 text-xs">
                    <input type="checkbox" className="mt-0.5 accent-teal-400" disabled={running || done} checked={withVideo} onChange={(e) => setWithVideo(e.target.checked)} />
                    <span>Also send the video background “{draft.layout?.background.video}”. Off by default: video uses extra CPU and memory and can make a weak TV stutter.
                      Without it the canvas keeps your still wallpaper. After sending it, run a Benchmark and turn it off if the TV feels slower.</span>
                  </label>
                )}
                {lowRam && heavy.length > 0 && (
                  <div className="mt-1 rounded border border-warn-400/40 px-2 py-1 text-xs" data-testid="low-ram-warning">
                    <div className="text-warn-400">This TV has {(ramKb! / 1024 / 1024).toFixed(1)} GB of memory. The {heavy.join(", ")} will stay off on it (still picture, no motion) so Home stays quick.</div>
                    <label className="mt-1 flex items-start gap-2">
                      <input type="checkbox" className="mt-0.5 accent-teal-400" disabled={running || done} checked={keepHeavy} onChange={(e) => setKeepHeavy(e.target.checked)} />
                      <span>Keep them on anyway. Home may stutter and apps may reload more often; untick and apply again to undo.</span>
                    </label>
                  </div>
                )}
                {report && <div className="text-xs text-ink-400">
                  {report.config_verified ? "Read back from the TV and matches." : "Written, but the read-back did not match."}{" "}
                  {report.restarted ? "Home restarted with the new look." : "It shows the next time the launcher opens."}
                  {report.written.some(([, m]) => m === "copy") && " (Copied directly: this Android version has no theme provider write.)"}
                </div>}
              </div></li>
            </ol>
          )}
          {err && <ErrorNote message={err} />}
          <div className="flex items-center gap-3 pt-1">
            {!done ? <Button variant="primary" onClick={() => void run()} disabled={!info || running || (!info.status.installed && !info.apk_bundled)}>{running && <Loader2 size={14} className="animate-spin" />}Apply now</Button>
              : <Button variant="primary" onClick={onClose}>Done</Button>}
            <span className="text-xs text-ink-400">Revert to original undoes all of this.</span>
          </div>
        </div>
      </div>
    </div>
  );
}
