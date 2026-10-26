import type { ReactNode } from "react";
import { SCOPE_LABEL, type Scope } from "../app/sections";

export function Panel({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return (
    <section className="rounded-xl border border-ink-700 bg-ink-900">
      <header className="flex items-center justify-between border-b border-ink-700 px-5 py-3">
        <h2 className="text-[13px] font-semibold tracking-wide text-ink-300 uppercase">{title}</h2>
        {action}
      </header>
      <div className="p-5">{children}</div>
    </section>
  );
}

export function ScopeBadges({ scope }: { scope: Scope[] }) {
  return (
    <div className="flex flex-wrap gap-2">
      {scope.map((s) => (
        <span key={s} className="rounded-full border border-ink-600 px-2.5 py-0.5 text-xs text-ink-300">
          {SCOPE_LABEL[s]}
        </span>
      ))}
    </div>
  );
}

export function Button({ children, onClick, disabled, variant = "secondary", type = "button" }: {
  children: ReactNode; onClick?: () => void; disabled?: boolean; variant?: "primary" | "secondary"; type?: "button" | "submit";
}) {
  const base = "inline-flex items-center gap-2 rounded-lg px-3.5 py-1.5 text-sm font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed";
  const look = variant === "primary"
    ? "bg-accent-500 text-ink-950 hover:bg-accent-400"
    : "border border-ink-600 bg-ink-850 text-ink-100 hover:border-ink-400";
  return <button type={type} className={`${base} ${look}`} onClick={onClick} disabled={disabled}>{children}</button>;
}

export function ErrorNote({ message }: { message: string }) {
  return <p role="alert" className="rounded-lg border border-danger-400/40 bg-danger-400/10 px-3 py-2 text-sm text-danger-400">{message}</p>;
}

export function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[180px_1fr] gap-4 py-1.5 text-sm">
      <dt className="text-ink-400">{label}</dt>
      <dd className="selectable min-w-0 break-words text-ink-100">{children}</dd>
    </div>
  );
}
