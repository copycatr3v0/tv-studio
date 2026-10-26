import { Fragment, useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw, Search } from "lucide-react";
import { api, errMsg, type Inventory } from "../lib/ipc";
import { CATEGORY_LABEL, evidence } from "../lib/categories";
import { DevicePicker } from "../components/DevicePicker";
import { Button, ErrorNote } from "../components/ui";

type StateFilter = "all" | "enabled" | "disabled";
type KindFilter = "all" | "system" | "user";

export function ApplicationsPage({ serial, onSelect }: { serial: string | null; onSelect: (s: string) => void }) {
  const [inv, setInv] = useState<Inventory | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [q, setQ] = useState("");
  const [stateF, setStateF] = useState<StateFilter>("all");
  const [kindF, setKindF] = useState<KindFilter>("all");
  const [open, setOpen] = useState<string | null>(null);

  const load = async (s: string) => {
    setBusy(true); setErr(null);
    try { setInv(await api.inventory(s)); } catch (e) { setInv(null); setErr(errMsg(e)); } finally { setBusy(false); }
  };
  useEffect(() => { if (serial) void load(serial); }, [serial]);

  const rows = useMemo(() => (inv?.apps ?? []).filter((a) =>
    (stateF === "all" || (stateF === "enabled") === a.enabled) &&
    (kindF === "all" || (kindF === "system") === a.system) &&
    (!q || a.name.toLowerCase().includes(q.toLowerCase()))), [inv, q, stateF, kindF]);

  const seg = (on: boolean) => `rounded-md px-2.5 py-1 text-xs ${on ? "bg-ink-700 text-ink-100" : "text-ink-400 hover:text-ink-100"}`;
  return (
    <div className="max-w-6xl space-y-4">
      <div className="flex flex-wrap items-center gap-3">
        <DevicePicker serial={serial} onSelect={onSelect} />
        <Button onClick={() => serial && void load(serial)} disabled={!serial || busy}>
          {busy ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}Reload
        </Button>
      </div>
      {err && <ErrorNote message={err} />}
      {inv && (
        <>
          <div className="flex flex-wrap items-center gap-3">
            <label className="flex items-center gap-2 rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5">
              <Search size={14} className="text-ink-400" />
              <input aria-label="Search packages" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search package names" className="selectable w-64 bg-transparent text-sm outline-none" />
            </label>
            <div className="flex gap-1 rounded-lg border border-ink-700 p-0.5">
              {(["all", "enabled", "disabled"] as const).map((f) => <button key={f} className={seg(stateF === f)} onClick={() => setStateF(f)}>{f}</button>)}
            </div>
            <div className="flex gap-1 rounded-lg border border-ink-700 p-0.5">
              {(["all", "system", "user"] as const).map((f) => <button key={f} className={seg(kindF === f)} onClick={() => setKindF(f)}>{f}</button>)}
            </div>
            <span className="text-xs text-ink-400">{rows.length} of {inv.apps.length} packages</span>
          </div>
          <div className="overflow-hidden rounded-xl border border-ink-700">
            <table className="w-full text-sm">
              <thead className="bg-ink-900 text-left text-xs text-ink-400">
                <tr><th className="px-4 py-2 font-medium">Package</th><th className="px-4 py-2 font-medium">State</th><th className="px-4 py-2 font-medium">Type</th><th className="px-4 py-2 font-medium">Known as</th><th className="px-4 py-2 font-medium">Evidence</th></tr>
              </thead>
              <tbody className="divide-y divide-ink-800">
                {rows.map((a) => (
                  <Fragment key={a.name}>
                    <tr onClick={() => setOpen(open === a.name ? null : a.name)} className="cursor-pointer hover:bg-ink-900">
                      <td className="selectable px-4 py-2 font-mono text-xs">{a.name}{a.verdict.kind === "Blocked" && <span className="ml-2 rounded bg-danger-400/15 px-1.5 py-0.5 font-sans text-[10px] text-danger-400">protected</span>}</td>
                      <td className={`px-4 py-2 ${a.enabled ? "text-ink-100" : "text-ink-400"}`}>{a.enabled ? "Enabled" : "Disabled"}</td>
                      <td className="px-4 py-2 text-ink-300">{a.system ? "System" : "User"}</td>
                      <td className="px-4 py-2 text-ink-300">{a.kb ? CATEGORY_LABEL[a.kb.category] : "—"}</td>
                      <td className="px-4 py-2 text-xs text-ink-400">{evidence(a.kb, inv.kb_lists)}</td>
                    </tr>
                    {open === a.name && (
                      <tr className="bg-ink-900/60">
                        <td colSpan={5} className="px-4 py-3 text-sm text-ink-300">
                          {a.verdict.kind === "Blocked" && <p className="text-danger-400">{a.verdict.reason}</p>}
                          {a.kb?.privacy && <p className="mt-1">{a.kb.privacy.note}</p>}
                          {a.kb?.notes.map((n) => <p key={n} className="mt-1">“{n}”</p>)}
                          {a.kb?.devices.length ? <p className="mt-2 text-xs text-ink-400">Seen on: {a.kb.devices.join(", ")}</p> : null}
                          {!a.kb && a.verdict.kind !== "Blocked" && <p className="text-ink-400">No notes from tested TVs. Check what it does before disabling it.</p>}
                        </td>
                      </tr>
                    )}
                  </Fragment>
                ))}
              </tbody>
            </table>
          </div>
          <p className="text-xs text-ink-400">Notes and evidence come from CleanYourTV's tested-TV lists (MIT). They describe other TVs, not proof about this one.</p>
        </>
      )}
    </div>
  );
}
