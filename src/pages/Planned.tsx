import type { Section } from "../app/sections";
import { Panel, ScopeBadges } from "../components/ui";

/** Honest placeholder: says what the section will do and when. Shows no controls and no fake data. */
export function Planned({ section }: { section: Section }) {
  return (
    <div className="max-w-2xl space-y-5">
      <ScopeBadges scope={section.scope} />
      <Panel title="Not built yet">
        <p className="text-sm leading-relaxed text-ink-300">{section.summary}</p>
        <p className="mt-3 text-sm text-ink-400">
          This section is scheduled for milestone <span className="font-mono text-accent-400">{section.milestone}</span> in
          docs/PLAN.md. Until then it shows nothing, so nothing here can be mistaken for a real device state.
        </p>
      </Panel>
    </div>
  );
}
