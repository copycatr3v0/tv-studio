import type { KbCategory, KbEntry } from "./ipc";

export const CATEGORY_LABEL: Record<KbCategory, string> = {
  telemetry: "Telemetry & tracking",
  ads_content: "Ads, promos & content rows",
  retail_demo: "Retail demo & hotel mode",
  preinstalled: "Preinstalled extras",
  streaming: "Streaming apps",
  voice: "Voice assistant",
  casting: "Casting & mirroring",
  updater: "Firmware updates",
  launcher: "Launchers & setup",
  factory: "Factory & test tools",
  privacy: "Privacy-relevant",
  other: "Other",
};

export function evidence(kb: KbEntry | null, lists: number): string {
  if (!kb) return "Not seen on any of the tested TVs";
  const parts = [];
  if (kb.disabled_in) parts.push(`disabled on ${kb.disabled_in}`);
  if (kb.kept_in) parts.push(`kept on ${kb.kept_in}`);
  return parts.length ? `${parts.join(", ")} of ${lists} tested TVs` : `Listed in the privacy catalogue`;
}
