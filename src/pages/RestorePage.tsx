import { DevicePicker } from "../components/DevicePicker";
import { Panel } from "../components/ui";
import { RevertPanel } from "../components/Revert";

export function RestorePage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  return (
    <div className="max-w-4xl space-y-5">
      <DevicePicker serial={serial} onSelect={onSelect} />
      {serial && <Panel title="Back to the original state"><RevertPanel serial={serial} /></Panel>}
      <p className="text-xs text-ink-400">The same revert is always available from the button at the top right. If the TV can no longer be reached over ADB, see RECOVERY.md for the manual path; this app never reports a revert it could not verify.</p>
    </div>
  );
}
