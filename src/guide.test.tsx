import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { GuidePage, profileFor } from "./pages/GuidePage";
import { useThemeEditor } from "./theme/useThemeEditor";

const SERIAL = "192.168.1.50:5555";
const TV = {
  serial: SERIAL, fingerprint: "f", brand: "TCL", manufacturer: "TCL", model: "Smart TV Pro", device: "G09", product: "p", android_release: "12", sdk: 31,
  build_id: null, incremental: null, platform: null, adb_secure: true, ram_total_kb: null, ram_available_kb: null, data_total_kb: null, data_free_kb: null,
  package_count: 62, disabled_count: 0, home: { kind: "component", component: "com.google.android.apps.tv.launcherx/.home.HomeActivity" },
  home_candidates: [], oem: [{ family: "google_tv", label: "Google TV", note: "Setup wizard note" }],
  capabilities: { cmd_package: true, studio_launcher: true, wireless_pairing: true, private_dns: true, content_write: true, install_via_pm: false },
  findings: [], probe_errors: [],
};
const app = (name: string, safe: boolean, balanced: boolean) => ({ name, enabled: true, system: true, kb: null, verdict: { kind: "Allowed" }, preselect_safe: safe, preselect_balanced: balanced });
const INVENTORY = { kb_lists: 316, apps: [app("com.tcl.logkit", true, true), app("com.tcl.gamebar", false, true), app("com.google.android.tungsten.setupwraith", true, true), app("com.tcl.tv", false, false)] };
const PLAN = {
  identity: "id", sdk: 31, runnable: true, notes: [],
  steps: [
    { id: "identify", title: "Identify the TV", kind: { kind: "identify" }, depends_on: [], critical: true, preview: { state: "check", detail: "Smart TV Pro" } },
    { id: "debloat-1", title: "Disable apps, batch 1 of 1", kind: { kind: "debloat_batch" }, depends_on: [], critical: false, preview: { state: "will_change", detail: "com.tcl.logkit" } },
  ],
};

function Host({ serial = SERIAL as string | null, onFinish = () => {} }) {
  const editor = useThemeEditor();
  return <GuidePage serial={serial} onSelect={() => {}} editor={editor} onFinish={onFinish} onOpen={() => {}} />;
}

function mockTv(over: Record<string, unknown> = {}) {
  invoke.mockImplementation(async (cmd: string, args?: { packages?: string[] }) => {
    if (cmd in over) return typeof over[cmd] === "function" ? (over[cmd] as (a?: unknown) => unknown)(args) : over[cmd];
    switch (cmd) {
      case "app_info": return { version: "0.1.0", db_path: "x", schema_version: 3, adb: { path: "adb", version: "1.0.41", source: "downloaded" } };
      case "probe_device": return TV;
      case "launcher_status": return { status: { installed: false, enabled: false, version: null, resolved_home: null, is_resolved_home: false }, apk_bundled: true, schema_launcher_version: "3", focused: null, google_home_off: [] };
      case "inventory": return INVENTORY;
      case "evaluate_debloat": return { facts: {}, verdicts: Object.fromEntries((args?.packages ?? []).map((p) => [p, p.includes("setupwraith") ? { kind: "Blocked", reason: "It owns the saved Wi-Fi network" } : { kind: "Allowed" }])) };
      case "deploy_plan": return PLAN;
      case "list_devices": return [];
      case "discover_wireless": return [];
      default: throw new Error(`unexpected ${cmd}`);
    }
  });
}

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  invoke.mockReset();
});

describe("profileFor", () => {
  it("never disables the stock launcher or touches DNS, and only the full goal installs the launcher", () => {
    const tidy = profileFor("tidy", ["a.b"], true, "x");
    expect(tidy.launcher).toEqual({ install: false, make_home: false, disable_stock: false });
    expect(tidy.settings).toEqual({ animation_scale: "0.5", private_dns: { mode: "keep" } });
    const full = profileFor("full", [], false, "x");
    expect(full.launcher).toEqual({ install: true, make_home: true, disable_stock: false });
    expect(full.apply_theme).toBe(true);
    expect(full.settings.animation_scale).toBeNull();
  });
});

describe("Guided setup", () => {
  it("blocks Next until adb is ready, and offers Google's download", async () => {
    mockTv({ app_info: { version: "0.1.0", db_path: "x", schema_version: 3, adb: null } });
    render(<Host />);
    expect(await screen.findByRole("button", { name: /get adb from google/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /next: connect/i })).toBeDisabled();
  });

  it("walks from your TV to a plan, leaving alone anything the TV's own checks block, and changes nothing before Apply", async () => {
    mockTv();
    render(<Host />);
    fireEvent.click(await screen.findByRole("button", { name: /next: connect/i }));
    fireEvent.click(await screen.findByRole("button", { name: /next: look at your tv/i }));
    expect(await screen.findByText("Smart TV Pro", { exact: false })).toBeInTheDocument();
    expect(screen.getByText(/Setup wizard note/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /next: choose/i }));
    expect(screen.getByRole("radio", { name: /Gentle clean-up/ })).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /next: preview/i }));
    expect(await screen.findByText(/would be turned off/)).toBeInTheDocument();
    const evaluated = invoke.mock.calls.find((c) => c[0] === "evaluate_debloat")![1] as { packages: string[] };
    expect(evaluated.packages).toEqual(["com.tcl.logkit", "com.google.android.tungsten.setupwraith"]);
    const planned = invoke.mock.calls.find((c) => c[0] === "deploy_plan")![1] as { request: { profile: { debloat: { packages: string[] } } } };
    expect(planned.request.profile.debloat.packages).toEqual(["com.tcl.logkit"]);
    expect(screen.getByText(/left alone because this TV needs it/)).toBeInTheDocument();
    expect(invoke.mock.calls.some((c) => ["deploy_run", "apply_debloat", "install_launcher"].includes(c[0] as string))).toBe(false);
  });

  it("applies, shows plain results, and offers undo", async () => {
    mockTv({
      deploy_run: { deployment_id: 7, plan: PLAN, report: { completed: true, cancelled: false, stopped: null, results: [{ id: "debloat-1", title: "Disable apps, batch 1 of 1", status: "done", message: "1 verified off" }] } },
      undo_deployment: { changed: ["com.tcl.logkit"], refused: [], unchanged: [], skipped: [], stopped: null },
    });
    const finish = vi.fn();
    render(<Host onFinish={finish} />);
    fireEvent.click(await screen.findByRole("button", { name: /next: connect/i }));
    fireEvent.click(await screen.findByRole("button", { name: /next: look at your tv/i }));
    await screen.findByText(/Setup wizard note/);
    fireEvent.click(screen.getByRole("button", { name: /next: choose/i }));
    fireEvent.click(screen.getByRole("button", { name: /next: preview/i }));
    fireEvent.click(await screen.findByRole("button", { name: /looks right/i }));
    fireEvent.click(screen.getByRole("button", { name: /apply to my tv/i }));
    expect(await screen.findByText("All done")).toBeInTheDocument();
    expect(screen.getByText("1 verified off")).toBeInTheDocument();
    expect(screen.getByText(/Revert to original/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /undo this run/i }));
    await waitFor(() => expect(invoke.mock.calls.some((c) => c[0] === "undo_deployment")).toBe(true));
    expect(await screen.findByText("Undo result")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Finish" }));
    expect(finish).toHaveBeenCalled();
  });

  it("won't continue from a blocked plan, and disables the launcher goal when the APK isn't bundled", async () => {
    mockTv({
      deploy_plan: { ...PLAN, runnable: false, steps: [{ id: "debloat-review", title: "Review the app selection", kind: { kind: "debloat_batch" }, depends_on: [], critical: false, preview: { state: "blocked", detail: "x is a launcher" } }] },
      launcher_status: { status: { installed: false, enabled: false, version: null, resolved_home: null, is_resolved_home: false }, apk_bundled: false, schema_launcher_version: "3", focused: null, google_home_off: [] },
    });
    render(<Host />);
    fireEvent.click(await screen.findByRole("button", { name: /next: connect/i }));
    fireEvent.click(await screen.findByRole("button", { name: /next: look at your tv/i }));
    await screen.findByText(/Setup wizard note/);
    fireEvent.click(screen.getByRole("button", { name: /next: choose/i }));
    expect(screen.getByRole("radio", { name: /Clean-up and a new look/ })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: /next: preview/i }));
    expect(await screen.findByText("x is a launcher")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /looks right/i })).toBeDisabled();
  });
});
