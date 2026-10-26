import { useState, type FormEvent } from "react";
import { api, IpcError, type AppSettings } from "../lib/ipc";
import { Button, ErrorNote, Panel } from "../components/ui";

export const DEMO_ADDRESS = "192.168.1.50";
export const DEMO_CODE = "123456";

export function SettingsPage({ settings, onSaved }: { settings: AppSettings; onSaved: (s: AppSettings) => void }) {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [err, setErr] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const dirty = JSON.stringify(draft) !== JSON.stringify(settings);
  const set = <K extends keyof AppSettings>(k: K, v: AppSettings[K]) => { setDraft({ ...draft, [k]: v }); setSaved(false); };

  async function submit(e: FormEvent) {
    e.preventDefault();
    setErr(null);
    try {
      const s = await api.saveSettings(draft);
      setDraft(s);
      onSaved(s);
      setSaved(true);
    } catch (x) {
      setErr(x instanceof IpcError ? x.message : String(x));
    }
  }

  const label = "block text-sm font-medium text-ink-100";
  const hint = "mt-1 text-xs text-ink-400";
  return (
    <form onSubmit={(e) => void submit(e)} className="max-w-2xl space-y-5">
      <Panel title="General">
        <div className="space-y-5">
          <fieldset>
            <legend className={label}>Interface mode</legend>
            <div className="mt-2 flex gap-2">
              {(["beginner", "advanced"] as const).map((m) => (
                <label key={m} className={`cursor-pointer rounded-lg border px-3 py-1.5 text-sm capitalize ${draft.ui_mode === m ? "border-accent-500 bg-accent-900 text-accent-400" : "border-ink-600 text-ink-300"}`}>
                  <input type="radio" name="ui_mode" className="sr-only" checked={draft.ui_mode === m} onChange={() => set("ui_mode", m)} />
                  {m}
                </label>
              ))}
            </div>
            <p className={hint}>Beginner hides Launcher Builder, Benchmarks and Diagnostics. The safety rules are the same in both modes.</p>
          </fieldset>

          <label className="flex items-start gap-3">
            <input type="checkbox" className="mt-1 accent-teal-400" checked={draft.default_dry_run} onChange={(e) => set("default_dry_run", e.target.checked)} />
            <span><span className={label}>Start every deployment as a dry run</span><span className={hint + " block"}>You see the full plan before anything is sent to the TV.</span></span>
          </label>

          <div>
            <label htmlFor="batch" className={label}>Packages per verified batch</label>
            <input id="batch" type="number" min={1} max={10} value={draft.batch_size}
              onChange={(e) => set("batch_size", Math.min(10, Math.max(1, Number(e.target.value) || 1)))}
              className="mt-2 w-24 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 text-sm" />
            <p className={hint}>Each batch is read back from the TV before the next one starts (1–10).</p>
          </div>
        </div>
      </Panel>

      <Panel title="adb">
        <label htmlFor="adb" className={label}>Custom adb path</label>
        <input id="adb" value={draft.adb_path} placeholder="Leave empty to use the bundled adb"
          onChange={(e) => set("adb_path", e.target.value)}
          className="selectable mt-2 w-full rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 font-mono text-sm" />
      </Panel>

      <Panel title="Demo">
        <label className="flex items-start gap-3">
          <input type="checkbox" className="mt-1 accent-teal-400" checked={draft.demo_mode} onChange={(e) => set("demo_mode", e.target.checked)} />
          <span><span className={label}>Practice on a simulated TV</span><span className={hint + " block"}>
            Every screen works against a built-in simulated TCL Google TV instead of adb. Nothing real is changed. Connect to {DEMO_ADDRESS} (pairing code {DEMO_CODE}).
          </span></span>
        </label>
      </Panel>

      <Panel title="Privacy">
        <label className="flex items-start gap-3">
          <input type="checkbox" className="mt-1 accent-teal-400" checked={draft.check_updates} onChange={(e) => set("check_updates", e.target.checked)} />
          <span><span className={label}>Check for app updates</span><span className={hint + " block"}>Off by default. The app sends no telemetry.</span></span>
        </label>
      </Panel>

      {err && <ErrorNote message={err} />}
      <div className="flex items-center gap-3">
        <Button type="submit" variant="primary" disabled={!dirty}>Save settings</Button>
        {saved && !dirty && <span className="text-sm text-accent-400">Saved</span>}
      </div>
    </form>
  );
}
