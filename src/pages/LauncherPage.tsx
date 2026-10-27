import { useEffect, useState } from "react";
import { Home, Loader2, PackagePlus, RefreshCw } from "lucide-react";
import { api, errMsg, type HomeCheck, type LauncherInfo, type ThemeReport } from "../lib/ipc";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel, Row } from "../components/ui";

export function LauncherPage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [info, setInfo] = useState<LauncherInfo | null>(null);
  const [home, setHome] = useState<HomeCheck | null>(null);
  const [reset, setReset] = useState<ThemeReport | null>(null);
  const [google, setGoogle] = useState<HomeCheck | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const load = async (s: string) => { setErr(null); try { setInfo(await api.launcherStatus(s)); } catch (e) { setInfo(null); setErr(errMsg(e)); } };
  useEffect(() => { if (serial) { setHome(null); setReset(null); setGoogle(null); void load(serial); } }, [serial]);

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
          <GoogleTvSwitch info={info} busy={busy} onToggle={(off) => void act("google", async () => setGoogle(await api.setGoogleHome(serial!, off)))} result={google} />
          <Panel title="What stays the same">
            <ul className="list-disc space-y-1 pl-5 text-sm text-ink-300">
              <li>The TV's own launcher stays installed. It stays switched on unless you use the switch above.</li>
              <li>The launcher uses the internet only for the weather widget, and only after you agree to it in the editor. No analytics.</li>
              <li>On Google TV, a Home-button helper (an accessibility service) sends the Home button to the Studio launcher. Revert removes it.</li>
              <li>Revert to original switches Google TV back on, hands Home back, confirms it on screen, then uninstalls the launcher.</li>
            </ul>
          </Panel>
        </>
      )}
    </div>
  );
}

/** Opt-in: Google TV's launcher off, so the Studio launcher is the real Home (no Google TV screen at power-on, and
 *  Google TV no longer kept in memory behind it). Asks for a confirmation that names what changes. */
function GoogleTvSwitch({ info, busy, onToggle, result }: { info: LauncherInfo; busy: string | null; onToggle: (off: boolean) => void; result: HomeCheck | null }) {
  const [sure, setSure] = useState(false);
  const off = info.google_home_off.length > 0;
  const google = off || (info.status.resolved_home ?? "").startsWith("com.google.android.");
  if (!google || !info.status.installed) return null;
  return (
    <Panel title="Google TV's launcher">
      <p className="text-sm text-ink-300">
        {off ? "Off: the Studio launcher is the TV's real Home." : "On: Android keeps Google TV running behind the Studio launcher (about 70–80 MB on a typical TV) and shows it for a few seconds after a restart."}
      </p>
      {!off && (
        <div className="mt-3 rounded-lg border border-warn-400/40 bg-ink-900 p-3 text-xs text-ink-300" data-testid="google-tv-warning">
          <p className="font-medium text-warn-400">Before you switch Google TV off</p>
          <ul className="mt-1 list-disc space-y-0.5 pl-4">
            <li>Google TV's For you, Library and profile screens, and the Google TV home recommendations, go away until it is back on.</li>
            <li>A system update can switch it back on; the Home-button helper still sends Home to the Studio launcher then.</li>
            <li>It is switched off, not removed. Switch it on here, or use Revert to original, any time.</li>
            <li>Studio checks the TV shows the Studio launcher afterwards, and puts Google TV straight back if it doesn't.</li>
            <li>Some TVs don't allow it: on a Sony BRAVIA, Google's setup service takes Home and turns Google TV back on at the next start. Studio notices and puts it back.</li>
          </ul>
          <label className="mt-2 flex items-center gap-2"><input type="checkbox" checked={sure} onChange={(e) => setSure(e.target.checked)} />I understand, switch it off</label>
        </div>
      )}
      <div className="mt-3">
        {off
          ? <Button disabled={!!busy} onClick={() => onToggle(false)}>{busy === "google" && <Loader2 size={14} className="animate-spin" />}Switch Google TV back on</Button>
          : <Button variant="primary" disabled={!!busy || !sure} onClick={() => onToggle(true)}>{busy === "google" && <Loader2 size={14} className="animate-spin" />}Switch Google TV off</Button>}
      </div>
      {result && <p className={`mt-3 text-sm ${result.verified ? "text-accent-400" : "text-warn-400"}`}>{result.verified ? (result.note ?? "Done: the TV now goes straight to the Studio launcher.") : result.note}</p>}
    </Panel>
  );
}
