import { useEffect, useState } from "react";
import { api, errMsg, type DeviceEntry } from "../lib/ipc";
import { ErrorNote } from "./ui";

/** Picks one authorized TV. Every device action names this serial explicitly. */
export function DevicePicker({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [devices, setDevices] = useState<DeviceEntry[]>([]);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    api.listDevices().then((d) => {
      const ok = d.filter((x) => x.state === "device");
      setDevices(ok);
      if (!serial && ok.length === 1) onSelect(ok[0]!.serial);
    }, (e: unknown) => setErr(errMsg(e)));
  }, [serial, onSelect]);
  return (
    <div className="flex flex-wrap items-center gap-3">
      <select aria-label="TV" value={serial ?? ""} onChange={(e) => onSelect(e.target.value)}
        className="min-w-72 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm">
        <option value="" disabled>{devices.length ? "Choose a connected TV" : "No authorized TV connected"}</option>
        {devices.map((d) => <option key={d.serial} value={d.serial}>{d.model ?? "TV"} · {d.serial}</option>)}
      </select>
      {err && <ErrorNote message={err} />}
    </div>
  );
}
