import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));

import { App } from "./App";

const settings = { ui_mode: "beginner", adb_path: "", default_dry_run: true, batch_size: 5, check_updates: false, reduced_motion_ui: false, demo_mode: true };

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  invoke.mockReset();
});

describe("demo mode", () => {
  it("shows a banner that says nothing real is changed, and turns off in one click", async () => {
    let current = { ...settings };
    invoke.mockImplementation(async (cmd: string, args?: { settings?: typeof settings }) => {
      if (cmd === "get_settings") return current;
      if (cmd === "save_settings") { current = args!.settings!; return current; }
      if (cmd === "app_info") return { version: "0.1.0", db_path: "x", schema_version: 3, adb: { path: "Simulated TCL Google TV", version: "demo", source: "demo" } };
      if (cmd === "list_devices") return [];
      throw new Error(`unexpected ${cmd}`);
    });
    render(<App />);
    const banner = await screen.findByRole("status");
    expect(banner).toHaveTextContent(/nothing real is changed/i);
    expect(banner).toHaveTextContent("192.168.1.50");
    fireEvent.click(screen.getByRole("button", { name: /use a real tv/i }));
    await waitFor(() => expect(screen.queryByText(/Demo mode:/)).not.toBeInTheDocument());
    expect(current.demo_mode).toBe(false);
  });

  it("no banner with a real TV", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "get_settings") return { ...settings, demo_mode: false };
      if (cmd === "app_info") return { version: "0.1.0", db_path: "x", schema_version: 3, adb: null };
      if (cmd === "list_devices") return [];
      throw new Error(`unexpected ${cmd}`);
    });
    render(<App />);
    await screen.findAllByText("Dashboard");
    expect(screen.queryByText(/Demo mode:/)).not.toBeInTheDocument();
  });
});
