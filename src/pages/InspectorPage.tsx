import { useCallback, useEffect, useState } from "react";
import { AlertTriangle, Check, Info, Loader2, OctagonX, ScanSearch, X } from "lucide-react";
import { api, errMsg, type DeviceEntry, type DeviceInfo } from "../lib/ipc";
import { Button, ErrorNote, Panel, Row } from "../components/ui";

const gb = (kb: number | null) => (kb == null ? "—" : `${(kb / 1024 / 1024).toFixed(kb < 10 * 1024 * 1024 ? 2 : 1)} GB`);
const dash = (v: string | number | null | undefined) => (v == null || v === "" ? "—" : String(v));

const CAPS: { key: keyof DeviceInfo["capabilities"]; label: string; meaning: string }[] = [
  { key: "studio_launcher", label: "Studio launcher", meaning: "Full home-screen customization (Android 8.0+)" },
  { key: "cmd_package", label: "Home queries", meaning: "Default Home can be read and set (Android 7+)" },
  { key: "wireless_pairing", label: "Wireless debugging", meaning: "Pairing-code connection (Android 11+)" },
  { key: "private_dns", label: "Private DNS", meaning: "Ad-blocking DNS option (Android 9+)" },
  { key: "content_write", label: "Theme provider transfer", meaning: "Theme files written as the launcher (Android 10+, else copy fallback)" },
  { key: "install_via_pm", label: "Install via pm", meaning: "Android 14+: installs go through pm to avoid an on-screen confirmation" },
];

const SEV_ICON = { info: Info, warning: AlertTriangle, blocker: OctagonX } as const;
const SEV_TONE = { info: "text-ink-300", warning: "text-warn-400", blocker: "text-danger-400" } as const;

export function InspectorPage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [devices, setDevices] = useState<DeviceEntry[]>([]);
  const [info, setInfo] = useState<DeviceInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => { api.listDevices().then((d) => setDevices(d.filter((x) => x.state === "device")), (e: unknown) => setErr(errMsg(e))); }, []);

  const inspect = useCallback(async (s: string) => {
    setBusy(true); setErr(null);
    try { setInfo(await api.probe(s)); } catch (e) { setInfo(null); setErr(errMsg(e)); } finally { setBusy(false); }
  }, []);

  useEffect(() => { if (serial) void inspect(serial); }, [serial, inspect]);

  return (
    <div className="max-w-5xl space-y-5">
      <div className="flex flex-wrap items-center gap-3">
        <select aria-label="TV" value={serial ?? ""} onChange={(e) => onSelect(e.target.value)} className="min-w-72 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm">
          <option value="" disabled>{devices.length ? "Choose a connected TV" : "No authorized TV connected"}</option>
          {devices.map((d) => <option key={d.serial} value={d.serial}>{d.model ?? "TV"} · {d.serial}</option>)}
        </select>
        <Button onClick={() => serial && void inspect(serial)} disabled={!serial || busy}>
          {busy ? <Loader2 size={14} className="animate-spin" /> : <ScanSearch size={14} />}Inspect again
        </Button>
        <span className="text-xs text-ink-400">Read-only: inspecting changes nothing on the TV.</span>
      </div>
      {err && <ErrorNote message={err} />}

      {info && (
        <>
          {info.findings.length > 0 && (
            <Panel title="Findings">
              <ul className="space-y-2">
                {info.findings.map((f) => {
                  const I = SEV_ICON[f.severity];
                  return <li key={f.message} className={`flex gap-2 text-sm ${SEV_TONE[f.severity]}`}><I size={16} className="mt-0.5 shrink-0" />{f.message}</li>;
                })}
              </ul>
            </Panel>
          )}

          <div className="grid gap-5 lg:grid-cols-2">
            <Panel title="Device">
              <dl>
                <Row label="Brand / maker">{dash(info.brand)}{info.manufacturer && info.manufacturer !== info.brand ? ` / ${info.manufacturer}` : ""}</Row>
                <Row label="Model">{dash(info.model)}</Row>
                <Row label="Device / product">{dash(info.device)} / {dash(info.product)}</Row>
                <Row label="Platform">{dash(info.platform)}</Row>
                <Row label="RAM">{gb(info.ram_total_kb)} total · {gb(info.ram_available_kb)} available</Row>
                <Row label="Storage (/data)">{gb(info.data_total_kb)} total · {gb(info.data_free_kb)} free</Row>
                <Row label="ADB authorization">{info.adb_secure == null ? "—" : info.adb_secure ? "Required (secure)" : "Not required: ro.adb.secure=0"}</Row>
              </dl>
            </Panel>
            <Panel title="Software">
              <dl>
                <Row label="Android">{dash(info.android_release)} (API {dash(info.sdk)})</Row>
                <Row label="Build">{dash(info.build_id)}</Row>
                <Row label="Incremental">{dash(info.incremental)}</Row>
                <Row label="Fingerprint"><span className="font-mono text-xs break-all">{dash(info.fingerprint)}</span></Row>
                <Row label="Packages">{dash(info.package_count)} installed · {dash(info.disabled_count)} disabled</Row>
              </dl>
            </Panel>
          </div>

          <Panel title="Home screen">
            <dl>
              <Row label="Current Home">
                <span className="font-mono text-xs break-all">
                  {info.home.kind === "component" ? info.home.component : info.home.kind === "chooser" ? "No default (chooser)" : "Unknown"}
                </span>
              </Row>
            </dl>
            {info.home_candidates.length > 0 && (
              <ul className="mt-3 divide-y divide-ink-700 rounded-lg border border-ink-700">
                {info.home_candidates.map((c) => (
                  <li key={c.component} className="flex items-center justify-between px-3 py-2 text-sm">
                    <span className="selectable font-mono text-xs break-all">{c.component}</span>
                    <span className={c.usable ? "text-accent-400" : "text-ink-400"}>{c.usable ? "Launcher" : "Not a launcher"}</span>
                  </li>
                ))}
              </ul>
            )}
          </Panel>

          <div className="grid gap-5 lg:grid-cols-2">
            <Panel title="What this TV supports">
              <ul className="space-y-2">
                {CAPS.map((c) => (
                  <li key={c.key} className="flex gap-2 text-sm">
                    {info.capabilities[c.key] ? <Check size={16} className="mt-0.5 shrink-0 text-accent-400" /> : <X size={16} className="mt-0.5 shrink-0 text-ink-400" />}
                    <span><span className="font-medium">{c.label}</span> <span className="text-ink-400">— {c.meaning}</span></span>
                  </li>
                ))}
              </ul>
              <p className="mt-3 text-xs text-ink-400">Based on the Android version. Firmware can still refuse an action; every change is verified on the TV.</p>
            </Panel>
            <Panel title="Firmware family">
              {info.oem.length === 0
                ? <p className="text-sm text-ink-400">No known family matched. Generic safety rules apply.</p>
                : <ul className="space-y-3">{info.oem.map((o) => <li key={o.family} className="text-sm"><div className="font-medium">{o.label}</div><div className="text-ink-300">{o.note}</div></li>)}</ul>}
            </Panel>
          </div>

          {info.probe_errors.length > 0 && (
            <Panel title="Queries that failed">
              <ul className="selectable space-y-1 font-mono text-xs text-ink-400">{info.probe_errors.map((e) => <li key={e}>{e}</li>)}</ul>
              <p className="mt-2 text-xs text-ink-400">Fields from these queries are shown as “—” rather than guessed.</p>
            </Panel>
          )}
        </>
      )}
    </div>
  );
}
