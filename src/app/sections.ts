import type { LucideIcon } from "lucide-react";
import {
  Boxes, Cable, Compass, Gauge, HardDriveDownload, History, LayoutDashboard, MonitorSmartphone,
  Package, Palette, Paintbrush, Rocket, Settings, ShieldCheck, Stethoscope, Trash2,
} from "lucide-react";

export type SectionId =
  | "guide" | "dashboard" | "connect" | "inspector" | "themes" | "editor" | "launcher" | "apps" | "debloat"
  | "privacy" | "benchmarks" | "deploy" | "profiles" | "restore" | "diagnostics" | "settings";

/** Where a section's changes land, shown on every page (prompt §14). */
export type Scope = "android" | "launcher" | "manual" | "app";

export interface Section {
  id: SectionId;
  label: string;
  icon: LucideIcon;
  group: "Device" | "Design" | "Clean up" | "Apply" | "App";
  /** Milestone in docs/PLAN.md where the section becomes functional. */
  milestone: `M${number}`;
  scope: Scope[];
  advancedOnly?: boolean;
  summary: string;
}

export const SECTIONS: readonly Section[] = [
  { id: "guide", label: "Guided setup", icon: Compass, group: "Device", milestone: "M11", scope: ["manual", "android", "launcher"], summary: "From nothing to a cleaned-up TV in six steps, with every change checked and undoable." },
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard, group: "Device", milestone: "M3", scope: ["app"], summary: "App status and connected TVs." },
  { id: "connect", label: "Connect TV", icon: Cable, group: "Device", milestone: "M4", scope: ["manual", "android"], summary: "Developer options guide, pairing, legacy ADB connect and diagnostics." },
  { id: "inspector", label: "Device Inspector", icon: MonitorSmartphone, group: "Device", milestone: "M4", scope: ["android"], summary: "Model, Android version, fingerprint, RAM, storage, current Home and HOME candidates." },
  { id: "themes", label: "Themes", icon: Palette, group: "Design", milestone: "M6", scope: ["launcher"], summary: "Twenty-five original themes to start from, plus your saved versions." },
  { id: "editor", label: "Live Theme Editor", icon: Paintbrush, group: "Design", milestone: "M6", scope: ["launcher"], summary: "Live 16:9 preview with remote-style navigation, every launcher setting, undo/redo, import/export and saved versions." },
  { id: "launcher", label: "Launcher Builder", icon: Boxes, group: "Design", milestone: "M6", scope: ["launcher"], advancedOnly: true, summary: "Install the Studio launcher, make it Home with on-screen confirmation, or reset its look." },
  { id: "apps", label: "Applications", icon: Package, group: "Clean up", milestone: "M5", scope: ["android"], summary: "Every installed package with its real state, type and what tested TVs did with it." },
  { id: "debloat", label: "Debloat Center", icon: Trash2, group: "Clean up", milestone: "M5", scope: ["android"], summary: "Safe, Balanced and Advanced modes on top of a hard protection list." },
  { id: "privacy", label: "Privacy & Performance", icon: ShieldCheck, group: "Clean up", milestone: "M8", scope: ["android"], summary: "Evidence-graded audit of voice, screen-recognition and telemetry components." },
  { id: "benchmarks", label: "Optimization Benchmarks", icon: Gauge, group: "Clean up", milestone: "M8", scope: ["android"], advancedOnly: true, summary: "Before/after key latency, jank, cold start, memory and idle CPU, with spread." },
  { id: "deploy", label: "Deployment", icon: Rocket, group: "Apply", milestone: "M7", scope: ["android", "launcher"], summary: "Dry-run plan, Apply Everything, live log and verified results." },
  { id: "profiles", label: "Profiles", icon: HardDriveDownload, group: "Apply", milestone: "M7", scope: ["app"], summary: "Saved deployment profiles (import, export) and this TV's deployment history with undo." },
  { id: "restore", label: "Restore", icon: History, group: "Apply", milestone: "M5", scope: ["android"], summary: "Exact rollback of packages, Home and animation settings to the TV's recorded baseline." },
  { id: "diagnostics", label: "Diagnostics", icon: Stethoscope, group: "App", milestone: "M8", scope: ["android"], advancedOnly: true, summary: "Check what a firmware update undid, repair it, and export a redacted diagnostics bundle." },
  { id: "settings", label: "Settings", icon: Settings, group: "App", milestone: "M3", scope: ["app"], summary: "Mode, adb location, dry-run default and batch size." },
] as const;

export const visibleSections = (mode: "beginner" | "advanced"): Section[] =>
  SECTIONS.filter((s) => mode === "advanced" || !s.advancedOnly);

export const SCOPE_LABEL: Record<Scope, string> = {
  android: "Changes Android",
  launcher: "Changes the Studio launcher",
  manual: "Needs a step on the TV",
  app: "This app only",
};

