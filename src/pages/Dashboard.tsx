import { useCallback, useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { api, IpcError, type AppInfo, type DeviceEntry } from "../lib/ipc";
import { Button, ErrorNote, Panel, Row } from "../components/ui";

const STATE_TEXT: Record<DeviceEntry["state"], { label: string; tone: string }> = {
  device: { label: "Connected", tone: "text-accent-400" },
  unauthorized: { label: "Waiting for approval on the TV", tone: "text-warn-400" },
  offline: { label: "Offline", tone: "text-ink-400" },
  no_permissions: { label: "No permission", tone: "text-danger-400" },
  other: { label: "Unknown state", tone: "text-ink-400" },
};

export function Dashboard() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [infoErr, setInfoErr] = useState<string | null>(null);
  const [devices, setDevices] = useState<DeviceEntry[] | null>(null);
  const [devErr, setDevErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setBusy(true);
    setDevErr(null);
    try {
      setDevices(await api.listDevices());
    } catch (e) {
      setDevices(null);
      setDevErr(e instanceof IpcError ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    api.appInfo().then(setInfo, (e: unknown) => setInfoErr(e instanceof IpcError ? e.message : String(e)));
    void refresh();
  }, [refresh]);

  return (
    <div className="grid max-w-5xl gap-5 lg:grid-cols-2">
      <Panel
        title="TVs seen by adb"
        action={<Button onClick={() => void refresh()} disabled={busy}><RefreshCw size={14} className={busy ? "animate-spin" : ""} />Refresh</Button>}
      >
        {devErr && <ErrorNote message={devErr} />}
        {devices && devices.length === 0 && (
          <p className="text-sm text-ink-400">No TV connected yet. Use Connect TV to add one.</p>
        )}
        {devices && devices.length > 0 && (
          <ul className="divide-y divide-ink-700">
            {devices.map((d) => (
              <li key={d.serial} className="flex items-center justify-between py-2.5 text-sm">
                <div>
                  <div className="font-medium">{d.model ?? "Unknown model"}</div>
                  <div className="selectable font-mono text-xs text-ink-400">{d.serial}</div>
                </div>
                <span className={STATE_TEXT[d.state].tone}>{STATE_TEXT[d.state].label}</span>
              </li>
            ))}
          </ul>
        )}
      </Panel>

      <Panel title="App status">
        {infoErr && <ErrorNote message={infoErr} />}
        {info && (
          <dl>
            <Row label="Version">{info.version}</Row>
            <Row label="adb">
              {info.adb ? `${info.adb.version ?? "version unknown"} (${info.adb.source})` : <span className="text-danger-400">Not found</span>}
            </Row>
            {info.adb && <Row label="adb path"><span className="font-mono text-xs break-all">{info.adb.path}</span></Row>}
            <Row label="Database"><span className="font-mono text-xs break-all">{info.db_path}</span></Row>
            <Row label="Schema version">{info.schema_version}</Row>
          </dl>
        )}
      </Panel>
    </div>
  );
}
