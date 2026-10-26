import { useState } from "react";
import { Loader2, ShieldCheck } from "lucide-react";
import { api, errMsg, type PrivacyAudit, type PrivacyEntry } from "../lib/ipc";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote, Panel } from "../components/ui";

const TITLES: [PrivacyEntry["category"], string][] = [
  ["voice", "Voice assistants and wake word"],
  ["acr", "ACR: recognises what is on screen (needs no microphone)"],
  ["telemetry", "Telemetry and analytics"],
  ["unknown", "Matched by name only, nothing else known"],
];

export function PrivacyPage({ serial, onSelect, onOpenDebloat, onOpenDeployment }: { serial: string | null; onSelect: (s: string) => void; onOpenDebloat: () => void; onOpenDeployment: () => void }) {
  const [audit, setAudit] = useState<PrivacyAudit | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function run() {
    if (!serial) return;
    setBusy(true); setErr(null);
    try { setAudit(await api.privacyAudit(serial)); } catch (e) { setErr(errMsg(e)); } finally { setBusy(false); }
  }

  return (
    <div className="max-w-4xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {err && <ErrorNote message={err} />}
      <Panel title="Privacy audit">
        <p className="text-sm text-ink-300">Lists the voice, screen-recognition (ACR) and telemetry components on this TV and <em>how</em> each is known: named by the TV's own settings, seen on tested TVs, or matched by name only. Read-only.</p>
        <p className="mt-2 text-xs text-ink-400">It never says a package is listening. On many TVs dozens of system packages “have” microphone permission because they share the system's identity; that is not evidence of recording.</p>
        <div className="mt-4 flex gap-3">
          <Button variant="primary" onClick={() => void run()} disabled={!serial || busy}>{busy ? <Loader2 size={14} className="animate-spin" /> : <ShieldCheck size={14} />}Run audit</Button>
          <Button onClick={onOpenDebloat}>Disable components in Debloat Center</Button>
        </div>
      </Panel>
      {audit && TITLES.map(([cat, title]) => {
        const group = audit.entries.filter((e) => e.category === cat);
        if (!group.length) return null;
        return (
          <Panel key={cat} title={`${title} · ${group.length}`}>
            <ul className="divide-y divide-ink-800">
              {group.map((e) => (
                <li key={e.package} className="py-2.5">
                  <div className="flex items-center justify-between gap-3">
                    <span className="selectable font-mono text-xs">{e.package}</span>
                    <span className={`text-xs ${e.enabled ? "text-ink-100" : "text-ink-400"}`}>{e.enabled ? "Enabled" : "Disabled"}</span>
                  </div>
                  <p className="mt-0.5 text-sm text-ink-300">{e.note}</p>
                  <ul className="mt-1 space-y-0.5 text-xs text-ink-400">{e.evidence.map((x) => <li key={x}>• {x}</li>)}</ul>
                </li>
              ))}
            </ul>
          </Panel>
        );
      })}
      {audit && audit.entries.length === 0 && <p className="text-sm text-ink-400">Nothing matched on this TV.</p>}
      {audit && audit.skipped > 0 && <p className="text-xs text-ink-400">{audit.skipped} more matches were not probed (limit 25 per audit).</p>}
      <Panel title="Performance and DNS settings">
        <p className="text-sm text-ink-300">Animation speed and ad-blocking Private DNS are part of a deployment, so they get a dry run, verification and undo.</p>
        <div className="mt-3"><Button onClick={onOpenDeployment}>Open Deployment</Button></div>
      </Panel>
    </div>
  );
}
