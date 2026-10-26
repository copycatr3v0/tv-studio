import { useEffect, useRef, useState } from "react";
import { CheckCircle2, Loader2, Radar, ShieldAlert, Stethoscope } from "lucide-react";
import { api, errMsg, type Diagnosis, type MdnsService } from "../lib/ipc";
import { FIND_IP, GUIDES } from "../lib/guides";
import { Button, ErrorNote, Panel } from "../components/ui";

type Step = 1 | 2 | 3 | 4;
type Method = "legacy" | "wireless";
const AUTH_TIMEOUT_MS = 90_000;
const input = "selectable rounded-lg border border-ink-600 bg-ink-850 px-3 py-1.5 font-mono text-sm placeholder:text-ink-400";

function Stepper({ step }: { step: Step }) {
  const names = ["Prepare the TV", "Connect", "Allow on the TV", "Done"];
  return (
    <ol className="mb-6 flex gap-2">
      {names.map((n, i) => {
        const k = (i + 1) as Step;
        const tone = k < step ? "border-accent-500 text-accent-400" : k === step ? "border-ink-100 text-ink-100" : "border-ink-700 text-ink-400";
        return <li key={n} className={`flex-1 border-t-2 pt-2 text-xs font-medium ${tone}`}>{k}. {n}</li>;
      })}
    </ol>
  );
}

function DiagnosisNote({ d }: { d: Diagnosis }) {
  const ok = d.port_state.state === "open";
  return (
    <div className={`rounded-lg border px-3 py-2 text-sm ${ok ? "border-accent-500/40 bg-accent-900/40" : "border-warn-400/40 bg-warn-400/10"}`}>
      <div className="font-mono text-xs text-ink-300">{d.endpoint} → {d.port_state.state}</div>
      <div className="mt-1">{d.advice}</div>
    </div>
  );
}

export function ConnectPage({ onConnected }: { onConnected: (serial: string) => void }) {
  const [step, setStep] = useState<Step>(1);
  const [guideId, setGuideId] = useState(GUIDES[0]!.id);
  const guide = GUIDES.find((g) => g.id === guideId)!;
  const [method, setMethod] = useState<Method>("legacy");
  const [address, setAddress] = useState("");
  const [pairAddress, setPairAddress] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [diag, setDiag] = useState<Diagnosis | null>(null);
  const [mdns, setMdns] = useState<MdnsService[] | null>(null);
  const [paired, setPaired] = useState(false);
  const [serial, setSerial] = useState<string | null>(null);
  const [model, setModel] = useState<string | null>(null);
  const [authDeadline, setAuthDeadline] = useState<number | null>(null);
  const pollRef = useRef<number | null>(null);

  async function run<T>(label: string, f: () => Promise<T>): Promise<T | undefined> {
    setBusy(label); setErr(null);
    try { return await f(); } catch (e) { setErr(errMsg(e)); return undefined; } finally { setBusy(null); }
  }

  const diagnose = (addr: string) => run("diagnose", async () => setDiag(await api.diagnose(addr)));

  const scan = () => run("scan", async () => {
    const found = await api.discover();
    setMdns(found);
    const pairing = found.find((s) => s.kind === "pairing");
    const connect = found.find((s) => s.kind === "connect");
    if (pairing && !pairAddress) setPairAddress(pairing.endpoint);
    if (connect && !address) setAddress(connect.endpoint);
  });

  const pair = () => run("pair", async () => { await api.pair(pairAddress, code); setPaired(true); setCode(""); });

  const connect = () => run("connect", async () => {
    setDiag(null);
    const r = await api.connect(address);
    setSerial(r.serial); setModel(r.model);
    if (r.state === "device") { setStep(4); } else { setStep(3); setAuthDeadline(Date.now() + AUTH_TIMEOUT_MS); }
  });

  // Step 3: poll adb until the TV reports the computer as authorized, bounded by AUTH_TIMEOUT_MS.
  useEffect(() => {
    if (step !== 3 || !serial || authDeadline === null) return;
    const tick = async () => {
      try {
        const d = (await api.listDevices()).find((x) => x.serial === serial);
        if (d?.state === "device") { setModel(d.model); setStep(4); return; }
      } catch (e) { setErr(errMsg(e)); }
      if (Date.now() < authDeadline) pollRef.current = window.setTimeout(() => void tick(), 2000);
      else setAuthDeadline(null);
    };
    void tick();
    return () => { if (pollRef.current) window.clearTimeout(pollRef.current); };
  }, [step, serial, authDeadline]);

  return (
    <div className="max-w-3xl">
      <Stepper step={step} />

      {step === 1 && (
        <div className="space-y-5">
          <Panel title="Which TV do you have?">
            <select value={guideId} onChange={(e) => setGuideId(e.target.value)} className="w-full rounded-lg border border-ink-600 bg-ink-850 px-3 py-2 text-sm">
              {GUIDES.map((g) => <option key={g.id} value={g.id}>{g.label}</option>)}
            </select>
            <div className="mt-5 grid gap-5 md:grid-cols-2">
              <div>
                <h3 className="mb-2 text-sm font-semibold">1 · Unlock Developer options</h3>
                <ol className="list-decimal space-y-1.5 pl-5 text-sm text-ink-300">{guide.developerOptions.map((s) => <li key={s}>{s}</li>)}</ol>
              </div>
              <div>
                <h3 className="mb-2 text-sm font-semibold">2 · Turn on debugging</h3>
                <ol className="list-decimal space-y-1.5 pl-5 text-sm text-ink-300">{guide.debugging.map((s) => <li key={s}>{s}</li>)}</ol>
              </div>
            </div>
            {guide.note && <p className="mt-4 text-xs text-ink-400">{guide.note}</p>}
            <div className="mt-5">
              <h3 className="mb-2 text-sm font-semibold">3 · Find the TV's IP address</h3>
              <ol className="list-decimal space-y-1.5 pl-5 text-sm text-ink-300">{FIND_IP.map((s) => <li key={s}>{s}</li>)}</ol>
            </div>
            <p className="mt-4 text-xs text-ink-400">Menu names vary between firmware versions. Keep the TV switched on during the whole session.</p>
          </Panel>
          <Button variant="primary" onClick={() => setStep(2)}>I've done this, continue</Button>
        </div>
      )}

      {step === 2 && (
        <div className="space-y-5">
          <div className="flex gap-2" role="tablist">
            {([["legacy", "Network debugging (port 5555)"], ["wireless", "Wireless debugging (pairing code)"]] as const).map(([m, l]) => (
              <button key={m} role="tab" aria-selected={method === m} onClick={() => { setMethod(m); setErr(null); setDiag(null); }}
                className={`rounded-lg border px-3 py-1.5 text-sm ${method === m ? "border-accent-500 bg-accent-900 text-accent-400" : "border-ink-600 text-ink-300"}`}>{l}</button>
            ))}
          </div>

          {method === "wireless" && (
            <Panel title="Pair this computer" action={<Button onClick={() => void scan()} disabled={!!busy}><Radar size={14} />Scan network</Button>}>
              <p className="mb-3 text-sm text-ink-300">On the TV open Developer options → Wireless debugging → <em>Pair device with pairing code</em>. Keep that screen open.</p>
              {mdns && mdns.length === 0 && <p className="mb-3 text-xs text-ink-400">Nothing advertised on this network. Type the address and port from the TV screen instead.</p>}
              <div className="flex flex-wrap items-end gap-3">
                <label className="text-xs text-ink-400">Pairing address (IP:port)<br />
                  <input className={input + " mt-1 w-56"} value={pairAddress} onChange={(e) => setPairAddress(e.target.value)} placeholder="192.168.1.50:41234" /></label>
                <label className="text-xs text-ink-400">6-digit code<br />
                  <input className={input + " mt-1 w-28"} inputMode="numeric" maxLength={6} value={code} onChange={(e) => setCode(e.target.value.replace(/\D/g, ""))} placeholder="000000" /></label>
                <Button onClick={() => void pair()} disabled={!!busy || code.length !== 6 || !pairAddress}>{busy === "pair" && <Loader2 size={14} className="animate-spin" />}Pair</Button>
              </div>
              {paired && <p className="mt-3 flex items-center gap-2 text-sm text-accent-400"><CheckCircle2 size={16} />Paired. Now connect with the address shown on the main Wireless debugging screen (its port differs from the pairing port).</p>}
            </Panel>
          )}

          <Panel title="Connect">
            <div className="flex flex-wrap items-end gap-3">
              <label className="text-xs text-ink-400">{method === "legacy" ? "TV IP address" : "Connect address (IP:port)"}<br />
                <input className={input + " mt-1 w-64"} value={address} onChange={(e) => setAddress(e.target.value)}
                  placeholder={method === "legacy" ? "192.168.1.50" : "192.168.1.50:37123"} /></label>
              <Button variant="primary" onClick={() => void connect()} disabled={!!busy || !address}>{busy === "connect" && <Loader2 size={14} className="animate-spin" />}Connect</Button>
              <Button onClick={() => void diagnose(address)} disabled={!!busy || !address}><Stethoscope size={14} />Diagnose</Button>
            </div>
            <div className="mt-4 space-y-3">
              {diag && <DiagnosisNote d={diag} />}
              {err && <ErrorNote message={err} />}
            </div>
          </Panel>
          <Button onClick={() => setStep(1)}>Back</Button>
        </div>
      )}

      {step === 3 && (
        <Panel title="Allow this computer on the TV">
          <p className="text-sm text-ink-300">The TV shows <strong>“Allow USB debugging?”</strong> with this computer's key fingerprint. Tick <em>Always allow from this computer</em> and choose <strong>Allow</strong>.</p>
          {authDeadline !== null
            ? <p className="mt-4 flex items-center gap-2 text-sm text-ink-400"><Loader2 size={14} className="animate-spin" />Waiting for your answer on the TV…</p>
            : <div className="mt-4 space-y-3"><ErrorNote message="No answer within 90 seconds. If no prompt appeared, turn USB debugging off and on again on the TV, then reconnect." />
                <Button onClick={() => setStep(2)}>Back to Connect</Button></div>}
          {err && <div className="mt-3"><ErrorNote message={err} /></div>}
        </Panel>
      )}

      {step === 4 && serial && (
        <div className="space-y-5">
          <Panel title="Connected">
            <p className="flex items-center gap-2 text-sm"><CheckCircle2 size={18} className="text-accent-400" />{model ?? "Your TV"} is connected and has authorized this computer.</p>
            <p className="selectable mt-1 font-mono text-xs text-ink-400">{serial}</p>
            <div className="mt-4 flex gap-3">
              <Button variant="primary" onClick={() => onConnected(serial)}>Inspect this TV</Button>
              <Button onClick={() => { setStep(2); setSerial(null); }}>Connect another TV</Button>
            </div>
          </Panel>
        </div>
      )}

      <div className="mt-8"><FinishSession defaultAddress={serial ?? address} /></div>
    </div>
  );
}

/** End-of-session guidance: network ADB is a full shell for anyone on the LAN on some firmware. */
function FinishSession({ defaultAddress }: { defaultAddress: string }) {
  const [addr, setAddr] = useState(defaultAddress);
  const [result, setResult] = useState<Diagnosis | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => setAddr(defaultAddress), [defaultAddress]);
  const closed = result?.port_state.state === "refused";
  return (
    <Panel title="When you're finished">
      <div className="flex gap-3">
        <ShieldAlert size={18} className="mt-0.5 shrink-0 text-warn-400" />
        <div className="text-sm text-ink-300">
          <p>Leaving network debugging on lets other devices on your network ask for full access to the TV, and on some firmware get it without any prompt. After customizing:</p>
          <ol className="mt-2 list-decimal space-y-1 pl-5">
            <li>On the TV, turn off USB debugging, Network debugging and Wireless debugging.</li>
            <li>In Developer options choose “Revoke USB debugging authorizations”.</li>
            <li>Restart the TV, then check the port below. Only “refused” proves it is closed.</li>
          </ol>
        </div>
      </div>
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <input aria-label="Address to check" className={input + " w-64"} value={addr} onChange={(e) => setAddr(e.target.value)} placeholder="TV IP address" />
        <Button onClick={() => { setErr(null); api.diagnose(addr).then(setResult, (e: unknown) => setErr(errMsg(e))); }} disabled={!addr}>Check port is closed</Button>
        {result && <span className={`text-sm ${closed ? "text-accent-400" : "text-warn-400"}`}>
          {closed ? "Closed: the TV refused the connection." : result.port_state.state === "open" ? "Still open: debugging is on." : "No answer: this does not prove the port is closed. Check the TV is on and the address is right."}
        </span>}
      </div>
      {err && <div className="mt-3"><ErrorNote message={err} /></div>}
    </Panel>
  );
}
