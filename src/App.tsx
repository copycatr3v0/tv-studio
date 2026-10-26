import { useEffect, useMemo, useState } from "react";
import { SECTIONS, visibleSections, type SectionId } from "./app/sections";
import { api, IpcError, type AppSettings } from "./lib/ipc";
import { Dashboard } from "./pages/Dashboard";
import { Planned } from "./pages/Planned";
import { SettingsPage } from "./pages/SettingsPage";
import { ConnectPage } from "./pages/ConnectPage";
import { InspectorPage } from "./pages/InspectorPage";
import { ApplicationsPage } from "./pages/ApplicationsPage";
import { DebloatPage } from "./pages/DebloatPage";
import { RestorePage } from "./pages/RestorePage";
import { ErrorNote } from "./components/ui";
import { RevertButton } from "./components/Revert";
import { ThemesPage } from "./pages/ThemesPage";
import { EditorPage } from "./pages/EditorPage";
import { LauncherPage } from "./pages/LauncherPage";
import { useThemeEditor } from "./theme/useThemeEditor";

const GROUPS = ["Device", "Design", "Clean up", "Apply", "App"] as const;
const BUILT = new Set<SectionId>(["dashboard", "settings", "connect", "inspector", "apps", "debloat", "restore", "themes", "editor", "launcher"]);

export function App() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [active, setActive] = useState<SectionId>("dashboard");
  const [serial, setSerial] = useState<string | null>(null);
  const editor = useThemeEditor();

  useEffect(() => {
    api.getSettings().then(setSettings, (e: unknown) => setLoadErr(e instanceof IpcError ? e.message : String(e)));
  }, []);

  const mode = settings?.ui_mode ?? "beginner";
  const sections = useMemo(() => visibleSections(mode), [mode]);
  // A section hidden by switching to Beginner falls back to the dashboard.
  const current = sections.find((s) => s.id === active) ?? SECTIONS[0]!;

  async function toggleMode() {
    if (!settings) return;
    const next = { ...settings, ui_mode: settings.ui_mode === "beginner" ? "advanced" : "beginner" } as const;
    try { setSettings(await api.saveSettings(next)); } catch (e) { setLoadErr(e instanceof IpcError ? e.message : String(e)); }
  }

  return (
    <div className="flex h-full">
      <nav aria-label="Main" className="flex w-60 shrink-0 flex-col border-r border-ink-700 bg-ink-900">
        <div className="px-5 pt-5 pb-4">
          <div className="text-[15px] font-semibold">TV Customizer Studio</div>
          <div className="text-xs text-ink-400">Android TV · Google TV</div>
        </div>
        <div className="flex-1 overflow-y-auto px-3 pb-4">
          {GROUPS.map((g) => {
            const items = sections.filter((s) => s.group === g);
            if (items.length === 0) return null;
            return (
              <div key={g} className="mb-3">
                <div className="px-2 pb-1 text-[11px] font-semibold tracking-wider text-ink-400 uppercase">{g}</div>
                {items.map((s) => {
                  const Icon = s.icon;
                  const on = s.id === current.id;
                  return (
                    <button key={s.id} onClick={() => setActive(s.id)} aria-current={on ? "page" : undefined}
                      className={`flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left text-sm ${on ? "bg-ink-800 text-ink-100" : "text-ink-300 hover:bg-ink-850 hover:text-ink-100"}`}>
                      <Icon size={16} className={on ? "text-accent-400" : ""} />
                      {s.label}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
        <div className="border-t border-ink-700 p-3">
          <button onClick={() => void toggleMode()} disabled={!settings}
            className="w-full rounded-lg border border-ink-600 px-3 py-1.5 text-xs text-ink-300 hover:border-ink-400 disabled:opacity-40">
            {mode === "beginner" ? "Beginner mode · switch to Advanced" : "Advanced mode · switch to Beginner"}
          </button>
        </div>
      </nav>

      <main className="flex-1 overflow-y-auto">
        <header className="flex items-start justify-between gap-6 border-b border-ink-700 px-8 py-5">
          <div>
            <h1 className="text-xl font-semibold">{current.label}</h1>
            <p className="mt-0.5 text-sm text-ink-400">{current.summary}</p>
          </div>
          <RevertButton serial={serial} onSelect={setSerial} />
        </header>
        <div className="px-8 py-6">
          {loadErr && <div className="mb-5 max-w-2xl"><ErrorNote message={loadErr} /></div>}
          {current.id === "dashboard" && <Dashboard />}
          {current.id === "settings" && settings && <SettingsPage key={JSON.stringify(settings)} settings={settings} onSaved={setSettings} />}
          {current.id === "connect" && <ConnectPage onConnected={(s) => { setSerial(s); setActive("inspector"); }} />}
          {current.id === "inspector" && <InspectorPage serial={serial} onSelect={setSerial} />}
          {current.id === "apps" && <ApplicationsPage serial={serial} onSelect={setSerial} />}
          {current.id === "debloat" && <DebloatPage serial={serial} onSelect={setSerial} />}
          {current.id === "restore" && <RestorePage serial={serial} onSelect={setSerial} />}
          {current.id === "themes" && <ThemesPage serial={serial} editor={editor} onEdit={() => setActive("editor")} />}
          {current.id === "editor" && <EditorPage serial={serial} editor={editor} advanced={mode === "advanced"} />}
          {current.id === "launcher" && <LauncherPage serial={serial} onSelect={setSerial} />}
          {!BUILT.has(current.id) && <Planned section={current} />}
        </div>
      </main>
    </div>
  );
}
