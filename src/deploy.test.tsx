import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";

const invoke = vi.fn();
let deployListener: ((e: { payload: unknown }) => void) | null = null;
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async (name: string, cb: (e: { payload: unknown }) => void) => { if (name === "deploy-event") deployListener = cb; return () => {}; } }));

import { DeploymentPage } from "./pages/DeploymentPage";
import { useThemeEditor } from "./theme/useThemeEditor";
import { emptyProfile, Profile, type Profile as P } from "./lib/ipc";
import { useState } from "react";

const PLAN = {
  identity: "Smart TV Pro|fp", sdk: 31, runnable: true, notes: ["Private DNS bypasses DNS filtering done by your router"],
  steps: [
    { id: "identify", title: "Identify the TV", kind: { kind: "identify" }, depends_on: [], critical: true, preview: { state: "check", detail: "Smart TV Pro|fp" } },
    { id: "snapshot", title: "Save the TV's current state", kind: { kind: "snapshot" }, depends_on: ["identify"], critical: true, preview: { state: "check", detail: "x" } },
    { id: "settings-animation", title: "Animation speed", kind: { kind: "settings" }, depends_on: ["snapshot"], critical: false, preview: { state: "will_change", detail: "Animation speed → 0.5×" } },
  ],
};

function Host({ requireDryRun = true, initial }: { requireDryRun?: boolean; initial?: P }) {
  const editor = useThemeEditor("blank");
  const [profile, setProfile] = useState<P>(initial ?? emptyProfile());
  return <DeploymentPage serial="tv:5555" onSelect={() => {}} profile={profile} setProfile={setProfile} editor={editor} advanced requireDryRun={requireDryRun} onOpenDebloat={() => {}} />;
}

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  invoke.mockReset();
  deployListener = null;
});

describe("profile shape", () => {
  it("emptyProfile matches the schema shared with Rust", () => {
    expect(Profile.safeParse(emptyProfile()).success).toBe(true);
  });
});

describe("Deployment", () => {
  it("needs a choice and a dry run before Apply Everything", async () => {
    invoke.mockImplementation(async (cmd: string) => (cmd === "deploy_interrupted" ? null : cmd === "list_devices" ? [] : cmd === "deploy_plan" ? PLAN : null));
    render(<Host />);
    const applyBtn = screen.getByRole("button", { name: /Apply Everything/ });
    expect(applyBtn).toBeDisabled();
    expect(screen.getByText("Choose at least one change.")).toBeInTheDocument();
    fireEvent.change(screen.getByRole("combobox", { name: "Animation speed" }), { target: { value: "0.5" } });
    expect(applyBtn).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: /Dry run/ }));
    expect(await screen.findByText("Animation speed → 0.5×")).toBeInTheDocument();
    expect(screen.getByText("will change")).toBeInTheDocument();
    expect(screen.getByText(/router/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Apply Everything/ })).toBeEnabled();
    const sent = invoke.mock.calls.find((c) => c[0] === "deploy_plan")![1] as { request: { profile: P } };
    expect(sent.request.profile.settings.animation_scale).toBe("0.5");
    // Changing the plan after the dry run requires a new one.
    fireEvent.change(screen.getByRole("combobox", { name: "Animation speed" }), { target: { value: "0" } });
    expect(screen.getByRole("button", { name: /Apply Everything/ })).toBeDisabled();
  });

  it("blocked plans cannot be applied", async () => {
    const blocked = { ...PLAN, runnable: false, steps: [...PLAN.steps, { id: "settings-dns", title: "Private DNS", kind: { kind: "settings" }, depends_on: [], critical: false, preview: { state: "blocked", detail: "Private DNS needs Android 9" } }] };
    invoke.mockImplementation(async (cmd: string) => (cmd === "deploy_interrupted" ? null : cmd === "list_devices" ? [] : cmd === "deploy_plan" ? blocked : null));
    render(<Host initial={{ ...emptyProfile(), settings: { animation_scale: null, private_dns: { mode: "automatic" } } }} />);
    fireEvent.click(screen.getByRole("button", { name: /Dry run/ }));
    expect(await screen.findByText("Private DNS needs Android 9")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Apply Everything/ })).toBeDisabled();
    expect(screen.getByText(/Resolve the blocked steps/)).toBeInTheDocument();
  });

  it("shows live step events, the result, and undoes the deployment", async () => {
    let finishRun: (v: unknown) => void = () => {};
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "deploy_interrupted") return null;
      if (cmd === "list_devices") return [];
      if (cmd === "deploy_run") return new Promise((r) => { finishRun = r; });
      if (cmd === "undo_deployment") return { changed: ["window_animation_scale"], refused: [], unchanged: [], skipped: [], stopped: null };
      return null;
    });
    render(<Host requireDryRun={false} initial={{ ...emptyProfile(), settings: { animation_scale: "0.5", private_dns: { mode: "keep" } } }} />);
    fireEvent.click(screen.getByRole("button", { name: /Apply Everything/ }));
    await waitFor(() => expect(deployListener).not.toBeNull());
    act(() => deployListener!({ payload: { step: "identify", title: "Identify the TV", status: "done", message: "Same TV as planned", finished: 0, total: 3 } }));
    expect(await screen.findByText("1 of 3 steps finished")).toBeInTheDocument();
    await act(async () => finishRun({ deployment_id: 7, plan: PLAN, report: { completed: true, cancelled: false, stopped: null, results: [
      { id: "identify", title: "Identify the TV", status: "done", message: "Same TV as planned" },
      { id: "snapshot", title: "Save the TV's current state", status: "done", message: "412 packages recorded" },
      { id: "settings-animation", title: "Animation speed", status: "done", message: "window_animation_scale = 0.5" },
    ] } }));
    expect(await screen.findByText("Done")).toBeInTheDocument();
    expect(screen.getByText("window_animation_scale = 0.5")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Undo this deployment/ }));
    expect(await screen.findByText("Undo result")).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith("undo_deployment", { serial: "tv:5555", deploymentId: 7 });
  });

  it("offers to resume an interrupted deployment", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "deploy_interrupted") return { id: 3, started: "2026-10-26T11:00:00Z", name: "Living room", last_step: "debloat-2 (begin)" };
      if (cmd === "list_devices") return [];
      if (cmd === "deploy_resume") return { deployment_id: 4, plan: PLAN, report: { completed: true, cancelled: false, stopped: null, results: [] } };
      return null;
    });
    render(<Host />);
    expect(await screen.findByText(/did not finish \(last step: debloat-2/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Resume" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("deploy_resume", { serial: "tv:5555", deploymentId: 3 }));
  });
});

describe("editor hook reuse", () => {
  it("is independent per render", () => {
    const { result } = renderHook(() => useThemeEditor("blank"));
    expect(result.current.draft.baseId).toBe("blank");
  });
});
