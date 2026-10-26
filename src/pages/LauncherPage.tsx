import { useEffect, useState } from "react";
import { Home, Loader2, PackagePlus, RefreshCw } from "lucide-react";
import { api, errMsg, type HomeCheck, type LauncherInfo, type ThemeReport } from "../lib/ipc";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel, Row } from "../components/ui";

export function LauncherPage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [info, setInfo] = useState<LauncherInfo | null>(null);
  const [home, setHome] = useState<HomeCheck | null>(null);
  const [reset, setReset] = useState<ThemeReport | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const load = async (s: string) => { setErr(null); try { setInfo(await api.launcherStatus(s)); } catch (e) { setInfo(null); setErr(errMsg(e)); } };
  useEffect(() => { if (serial) { setHome(null); setReset(null); void load(serial); } }, [serial]);

  async function act(label: string, f: () => Promise<unknown>) {
    if (!serial) return;
    setBusy(label); setErr(null);
    try { await f(); await load(serial); } catch (e) { setErr(errMsg(e)); } finally { setBusy(null); }
  }

  const st = info?.status;
  return (
    <div className="max-w-4xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {err && <ErrorNote message={err} />}
      {info && st && (
        <>
          <Panel title="Studio launcher" action={<Button onClick={() => serial && void load(serial)}><RefreshCw size={14} />Refresh</Button>}>
            <dl>
              <Row label="On this TV">{st.installed ? `Installed, version ${st.version ?? "unknown"}${st.enabled ? "" : " (disabled)"}` : "Not installed"}</Row>
              <Row label="Default Home"><span className="font-mono text-xs break-all">{st.resolved_home ?? "No default (chooser) or unknown"}</span></Row>
              <Row label="On screen now"><span className="font-mono text-xs break-all">{info.focused ?? "Unknown"}</span></Row>
              <Row label="Bundled with this app">{info.apk_bundled ? `Yes (theme schema for ${info.schema_launcher_version})` : "No: build it from launcher/ (launcher/NOTICE.md)"}</Row>
            </dl>
            <div className="mt-4 flex flex-wrap gap-3">
              <Button variant={st.installed ? "secondary" : "primary"} disabled={!!busy || !info.apk_bundled} onClick={() => void act("install", () => api.installLauncher(serial!))}>
                {busy === "install" ? <Loader2 size={14} className="animate-spin" /> : <PackagePlus size={14} />}{st.installed ? "Reinstall / update" : "Install"}
              </Button>
              <Button variant="primary" disabled={!!busy || !st.installed} onClick={() => void act("home", async () => setHome(await api.makeLauncherHome(serial!)))}>
                {busy === "home" ? <Loader2 size={14} className="animate-spin" /> : <Home size={14} />}Make it Home
              </Button>
              <Button disabled={!!busy || !st.installed} onClick={() => void act("reset", async () => setReset(await api.resetTheme(serial!)))}>Reset to built-in look</Button>
            </div>
            {home && <p className={`mt-3 text-sm ${home.verified ? "text-accent-400" : "text-warn-400"}`}>{home.verified ? "Confirmed: pressing Home on the TV now opens the Studio launcher." : home.note}</p>}
            {reset && <p className="mt-3 text-sm text-accent-400">Theme files removed{reset.restarted ? "; Home restarted." : "."}</p>}
          </Panel>
          <Panel title="What stays the same">
            <ul className="list-disc space-y-1 pl-5 text-sm text-ink-300">
              <li>The TV's own launcher stays installed and enabled. Disabling it is a separate, guarded choice in the Debloat Center, and only after the Studio launcher is confirmed as Home.</li>
              <li>The launcher has no internet permission, no analytics and no background polling.</li>
              <li>Remote-button helpers (accessibility services) are not enabled by this version.</li>
              <li>Revert to original hands Home back, confirms it on screen, then uninstalls the launcher.</li>
            </ul>
          </Panel>
        </>
      )}
    </div>
  );
}
