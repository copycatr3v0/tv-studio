import { useEffect, useState } from "react";
import { History, Paintbrush } from "lucide-react";
import { CATALOG } from "../theme/catalog";
import { Preview } from "../theme/Preview";
import { useTvApps } from "../theme/useTvApps";
import { draftFromCatalog, type Draft, type ThemeEditor } from "../theme/useThemeEditor";
import { api, errMsg, inTauri, type SavedTheme } from "../lib/ipc";
import { Button, ErrorNote, Panel } from "../components/ui";

export function ThemesPage({ serial, editor, onEdit }: { serial: string | null; editor: ThemeEditor; onEdit: () => void }) {
  const { apps, source } = useTvApps(serial);
  const [saved, setSaved] = useState<SavedTheme[]>([]);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => { if (inTauri()) api.listThemeVersions().then(setSaved, (e: unknown) => setErr(errMsg(e))); }, []);

  async function openSaved(t: SavedTheme) {
    try {
      const body = await api.loadThemeVersion(t.name, t.version) as Draft;
      editor.load({ ...body, name: t.name }, `${t.name} v${t.version}`);
      onEdit();
    } catch (e) { setErr(errMsg(e)); }
  }

  return (
    <div className="max-w-6xl space-y-6">
      <p className="text-sm text-ink-400">
        Previews use {source === "tv" ? "your TV's apps" : "sample apps (connect a TV to preview its own)"}. Every theme here is original and
        only uses settings the Studio launcher supports. Pick one to customize, then apply it to the TV.
      </p>
      {err && <ErrorNote message={err} />}
      {saved.length > 0 && (
        <Panel title="Your saved themes">
          <ul className="flex flex-wrap gap-2">
            {saved.map((t) => (
              <li key={`${t.name}-${t.version}`}>
                <Button onClick={() => void openSaved(t)}><History size={14} />{t.name} <span className="text-ink-400">v{t.version}</span></Button>
              </li>
            ))}
          </ul>
        </Panel>
      )}
      <div className="grid gap-5 sm:grid-cols-2 xl:grid-cols-3">
        {CATALOG.map((t) => {
          const d = draftFromCatalog(t.id);
          return (
            <article key={t.id} className="overflow-hidden rounded-xl border border-ink-700 bg-ink-900">
              <Preview config={d.config} apps={apps} wallpaperUrl={d.wallpapers[0]?.url ?? null} width={352} />
              <div className="space-y-2 p-4">
                <h3 className="font-semibold">{t.name}</h3>
                <p className="min-h-10 text-sm text-ink-400">{t.blurb}</p>
                <Button variant={editor.draft.baseId === t.id ? "secondary" : "primary"} onClick={() => { editor.load(d); onEdit(); }}>
                  <Paintbrush size={14} />Customize
                </Button>
              </div>
            </article>
          );
        })}
      </div>
    </div>
  );
}
