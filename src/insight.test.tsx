import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { BenchmarksPage } from "./pages/BenchmarksPage";
import { PrivacyPage } from "./pages/PrivacyPage";
import { DiagnosticsPage } from "./pages/DiagnosticsPage";
import { emptyProfile } from "./lib/ipc";

beforeEach(() => { (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {}; invoke.mockReset(); });

const run = (launcher: string) => ({ created: "t", launcher, launcher_version: null, key_response_ms: [], key_movement_ms: [], keys_without_reaction: 0, frames: 0, janky_frames: 0, cold_start_ms: [], home_return_ms: [], idle_cpu_device_pct: null, idle_cpu_launcher_pct: null, launcher_pss_mb: null, mem_available_mb: null, notes: ["This TV doesn't log injected keys"] });
const stats = (m: number) => ({ n: 6, median: m, p25: m - 1, p75: m + 1, p95: m + 2, min: m - 2, max: m + 3 });

describe("Benchmarks", () => {
  it("compares two runs with cautious verdicts", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_devices") return [];
      if (cmd === "bench_list") return [{ id: 2, label: "after", created: "t2", launcher: "studio.tvcustomizer.home" }, { id: 1, label: "before", created: "t1", launcher: "com.google.android.apps.tv.launcherx" }];
      if (cmd === "bench_compare") return { same_launcher: false, before: run("com.google.android.apps.tv.launcherx"), after: run("studio.tvcustomizer.home"), metrics: [
        { key: "key_response", label: "Remote key response", unit: "ms", before: stats(42), after: stats(20), verdict: "better", note: "n" },
        { key: "pss", label: "Launcher memory (PSS)", unit: "MB", before: { ...stats(150), n: 1 }, after: { ...stats(148), n: 1 }, verdict: "no_clear_change", note: "One reading per run." },
      ] };
      return null;
    });
    render(<BenchmarksPage serial="tv:5555" onSelect={() => {}} />);
    fireEvent.click(await screen.findByLabelText("Use before as before"));
    fireEvent.click(screen.getByLabelText("Use after as after"));
    fireEvent.click(screen.getByRole("button", { name: "Compare" }));
    expect(await screen.findByText("Better")).toBeInTheDocument();
    expect(screen.getByText("No clear change")).toBeInTheDocument();
    expect(screen.getByText(/compares the Home screen experience/)).toBeInTheDocument();
    expect(screen.getByText("42.0 (41.0–43.0, n=6)")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("bench_compare", { beforeId: 1, afterId: 2 });
  });
});

describe("Privacy audit", () => {
  it("groups entries with their evidence and never claims listening", async () => {
    invoke.mockImplementation(async (cmd: string) => cmd === "list_devices" ? [] : cmd === "privacy_audit" ? { installed: 400, skipped: 0, entries: [
      { package: "com.google.android.katniss", enabled: true, category: "voice", note: "Google Assistant", evidence: ["named by the TV's settings: voice_interaction_service", "microphone access appears in the TV's returned history"], mic_history: "recorded" },
      { package: "tv.inscape.tvclient.inscapeacr", enabled: false, category: "acr", note: "Inscape ACR", evidence: ["tested TVs: disabled on 1, kept on 0"], mic_history: "unknown" },
    ] } : null);
    render(<PrivacyPage serial="tv:5555" onSelect={() => {}} onOpenDebloat={() => {}} onOpenDeployment={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Run audit/ }));
    expect(await screen.findByText(/Voice assistants and wake word · 1/)).toBeInTheDocument();
    expect(screen.getByText(/ACR: recognises what is on screen/)).toBeInTheDocument();
    expect(screen.getByText("Disabled")).toBeInTheDocument();
    expect(screen.getAllByRole("listitem").map((li) => li.textContent ?? "").join(" ").toLowerCase()).not.toMatch(/listening|listens/);
  });
});

describe("Diagnostics", () => {
  const plan = { identity: "id", sdk: 31, runnable: true, notes: [], steps: [] };
  it("reports drift and repairs with the same request", async () => {
    const request = { profile: { ...emptyProfile("Living room"), debloat: { packages: ["com.tcl.logkit"], confirmed: [] } }, theme: null };
    let checks = 0;
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_devices") return [];
      if (cmd === "drift_check") { checks++; return { deployment_id: 9, deployment_started: "2026-10-20T10:00:00Z", request,
        report: checks === 1 ? { plan, reenabled: ["com.tcl.logkit"], launcher_missing: false, home_moved: true, new_packages: ["com.tcl.newpromo"], would_change: ["Disable apps, batch 1 of 1"], in_sync: false }
          : { plan, reenabled: [], launcher_missing: false, home_moved: false, new_packages: ["com.tcl.newpromo"], would_change: [], in_sync: true } }; }
      if (cmd === "deploy_run") return { deployment_id: 10, plan, report: { completed: true, cancelled: false, stopped: null, results: [] } };
      return null;
    });
    render(<DiagnosticsPage serial="tv:5555" onSelect={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Check for changes/ }));
    expect(await screen.findByText("com.tcl.logkit")).toBeInTheDocument();
    expect(screen.getByText(/Home no longer opens the Studio launcher/)).toBeInTheDocument();
    expect(screen.getByText("com.tcl.newpromo")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Repair…/ }));
    fireEvent.click(screen.getByRole("button", { name: "Repair" }));
    expect(await screen.findByText("Repaired and verified.")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("deploy_run", { serial: "tv:5555", request: { profile: request.profile, theme: null } });
  });

  it("says when there is nothing to compare with, and exports a bundle", async () => {
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    (URL as unknown as { createObjectURL: unknown }).createObjectURL = vi.fn(() => "blob:x");
    (URL as unknown as { revokeObjectURL: unknown }).revokeObjectURL = vi.fn();
    invoke.mockImplementation(async (cmd: string) => cmd === "drift_check" ? null : cmd === "diagnostics_bundle" ? { app_version: "0.1.0" } : []);
    render(<DiagnosticsPage serial="tv:5555" onSelect={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Check for changes/ }));
    expect(await screen.findByText(/No deployment on this TV yet/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
    await waitFor(() => expect(click).toHaveBeenCalled());
    expect(invoke).toHaveBeenCalledWith("diagnostics_bundle", { serial: "tv:5555" });
  });
});
