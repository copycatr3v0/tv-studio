import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { DebloatPage } from "./pages/DebloatPage";
import { RestorePage } from "./pages/RestorePage";

const kb = (category: string, disabled_in: number, kept_in = 0) => ({ disabled_in, kept_in, category, notes: [], devices: [] });
const row = (name: string, extra: Record<string, unknown> = {}) => ({
  name, enabled: true, system: true, kb: null, verdict: { kind: "Allowed" }, preselect_safe: false, preselect_balanced: false, ...extra,
});
const INVENTORY = {
  kb_lists: 316,
  apps: [
    row("com.tcl.logkit", { kb: kb("telemetry", 29, 1), preselect_safe: true, preselect_balanced: true }),
    row("com.tcl.gamecenter", { kb: kb("preinstalled", 8, 1), preselect_balanced: true }),
    row("com.google.android.katniss", { kb: kb("voice", 104, 43) }),
    row("com.android.vending", { kb: kb("other", 0, 1) }),
    row("com.android.systemui", { verdict: { kind: "Blocked", reason: "core Android package" } }),
    row("com.tcl.off", { enabled: false, kb: kb("telemetry", 3) }),
  ],
};

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  invoke.mockReset();
});

describe("Debloat Center", () => {
  it("pre-selects by mode, never sensitive ones, and hides unknown packages outside Advanced", async () => {
    invoke.mockImplementation(async (cmd: string) => (cmd === "inventory" ? INVENTORY : []));
    render(<DebloatPage serial="tv:5555" onSelect={() => {}} />);
    const logkit = await screen.findByRole("checkbox", { name: /com\.tcl\.logkit/ });
    expect(logkit).toBeChecked();
    expect(screen.getByRole("checkbox", { name: /gamecenter/ })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: /katniss/ })).not.toBeChecked();
    expect(screen.queryByText("com.android.systemui")).toBeNull();
    expect(screen.queryByText("com.tcl.off")).toBeNull();
    fireEvent.click(screen.getByRole("radio", { name: "balanced" }));
    expect(screen.getByRole("checkbox", { name: /gamecenter/ })).toBeChecked();
    fireEvent.click(screen.getByRole("radio", { name: "advanced" }));
    const core = screen.getByRole("checkbox", { name: /systemui/ });
    expect(core).toBeDisabled();
    expect(screen.getByText(/core Android package/)).toBeInTheDocument();
  });

  it("requires the TV check and per-item confirmation, skips blocked items, then applies", async () => {
    invoke.mockImplementation(async (cmd: string, args: { packages?: string[] }) => {
      if (cmd === "inventory") return INVENTORY;
      if (cmd === "evaluate_debloat") return { facts: {}, verdicts: {
        "com.tcl.logkit": { kind: "Allowed" },
        "com.google.android.katniss": { kind: "NeedsConfirm", reason: "voice search stops" },
        "com.android.vending": { kind: "Blocked", reason: "test block" },
      } };
      if (cmd === "apply_debloat") return { identity: "id", baseline_created: true,
        report: { changed: (args as unknown as { request: { packages: string[] } }).request.packages, refused: [], unchanged: [], skipped: [], stopped: null } };
      return [];
    });
    render(<DebloatPage serial="tv:5555" onSelect={() => {}} />);
    await screen.findByRole("checkbox", { name: /com\.tcl\.logkit/ });
    fireEvent.click(screen.getByRole("checkbox", { name: /katniss/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /vending/ }));
    const applyBtn = () => screen.getByRole("button", { name: /^Disable/ });
    expect(applyBtn()).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: /Check on TV/ }));
    expect(await screen.findByText("test block")).toBeInTheDocument();
    expect(applyBtn()).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: /I understand: voice search stops/ }));
    expect(applyBtn()).toHaveTextContent("Disable 2 selected");
    fireEvent.click(applyBtn());
    fireEvent.click(screen.getByRole("button", { name: "Disable now" }));
    await waitFor(() => expect(screen.getByText("2 verified on the TV")).toBeInTheDocument());
    const call = invoke.mock.calls.find((c) => c[0] === "apply_debloat")![1] as { request: { packages: string[]; confirmed: string[] } };
    expect(call.request.packages.sort()).toEqual(["com.google.android.katniss", "com.tcl.logkit"]);
    expect(call.request.confirmed).toEqual(["com.google.android.katniss"]);
    expect(screen.getByText(/original state was saved/)).toBeInTheDocument();
  });

  it("invalidates the check when the selection changes", async () => {
    invoke.mockImplementation(async (cmd: string) => (cmd === "inventory" ? INVENTORY : cmd === "evaluate_debloat" ? { facts: {}, verdicts: { "com.tcl.logkit": { kind: "Allowed" } } } : []));
    render(<DebloatPage serial="tv:5555" onSelect={() => {}} />);
    await screen.findByRole("checkbox", { name: /com\.tcl\.logkit/ });
    fireEvent.click(screen.getByRole("button", { name: /Check on TV/ }));
    await waitFor(() => expect(screen.getByRole("button", { name: /^Disable/ })).toBeEnabled());
    fireEvent.click(screen.getByRole("checkbox", { name: /katniss/ }));
    expect(screen.getByRole("button", { name: /^Disable/ })).toBeDisabled();
  });
});

describe("Restore", () => {
  it("previews the plan, explains what stays, and reports the verified result", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_devices") return [];
      if (cmd === "restore_preview") return { baseline_created: "2026-10-26T10:00:00Z", plan: { enable: ["com.tcl.logkit"], left_disabled: ["com.tcl.off"], not_in_snapshot: [], set_home: null, settings: [["window_animation_scale", "0.5"]] } };
      if (cmd === "restore_apply") return { changed: ["com.tcl.logkit", "window_animation_scale"], refused: [], unchanged: [], skipped: [], stopped: null };
      throw new Error(cmd);
    });
    render(<RestorePage serial="tv:5555" onSelect={() => {}} />);
    expect(await screen.findByText(/Re-enable 1 package/)).toBeInTheDocument();
    expect(screen.getByText(/already disabled before/)).toHaveTextContent("com.tcl.off");
    fireEvent.click(screen.getByRole("button", { name: /Restore everything/ }));
    fireEvent.click(screen.getByRole("button", { name: "Restore" }));
    const panel = await screen.findByText("Restore result");
    expect(within(panel.closest("section")!).getByText("2 verified on the TV")).toBeInTheDocument();
  });
});
