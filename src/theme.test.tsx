/// <reference types="vite/client" />
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));

import { CATALOG } from "./theme/catalog";
import { SCHEMA, validate } from "./theme/schema";
import { useThemeEditor } from "./theme/useThemeEditor";
import { EditorPage } from "./pages/EditorPage";
import { ApplyThemeDialog } from "./components/ApplyThemeDialog";
import { Preview } from "./theme/Preview";
import { SAMPLE_APPS } from "./theme/apps";

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  invoke.mockReset();
});

const SHIPPED = import.meta.glob("/public/themes/*.jpg", { query: "?url", eager: false });

describe("theme schema", () => {
  it("is the generated v1 schema with CleanYourTV's core keys", () => {
    const keys = SCHEMA.keys.map((k) => k.key);
    for (const k of ["dock_apps", "hidden_apps", "columns", "rows_below_dock", "background_color", "wallpaper", "reduced_motion", "app_library", "uniform_tiles", "show_network_pill", "top_bar_apps", "always_show_names"]) {
      expect(keys).toContain(k);
    }
  });

  it("flags the same problems as the Rust validator", () => {
    const p = validate({ columns: 12, background_color: "red", dock_apps: ["not a pkg"], glass_style: "neon", greeting_name: "a\nb", made_up: 1, app_keys: ["190:com.x.y"] });
    expect(Object.keys(p).sort()).toEqual(["app_keys", "background_color", "columns", "dock_apps", "glass_style", "greeting_name", "made_up"]);
  });

  it("ships 25 original themes that all validate", () => {
    expect(CATALOG).toHaveLength(25);
    expect(new Set(CATALOG.map((t) => t.id)).size).toBe(25);
    for (const t of CATALOG) expect(validate(t.config), t.id).toEqual({});
    // Every wallpaper a theme names is shipped with the app.
    for (const t of CATALOG.filter((x) => x.wallpaper)) expect(Object.keys(SHIPPED), t.id).toContain(`/public/themes/${t.wallpaper}`);
  });
});

describe("editor state", () => {
  it("undoes and redoes, and a new edit clears redo", () => {
    const { result } = renderHook(() => useThemeEditor("blank"));
    act(() => result.current.set("columns", 5));
    act(() => result.current.set("columns", 6));
    expect(result.current.draft.config.columns).toBe(6);
    act(() => result.current.undo());
    expect(result.current.draft.config.columns).toBe(5);
    act(() => result.current.redo());
    expect(result.current.draft.config.columns).toBe(6);
    act(() => result.current.undo());
    act(() => result.current.unset("columns"));
    expect(result.current.canRedo).toBe(false);
    expect("columns" in result.current.draft.config).toBe(false);
  });
});

describe("preview", () => {
  it("moves focus like a remote and shows the dock title", () => {
    render(<Preview config={{ columns: 4, dock_apps: ["com.netflix.ninja"] }} apps={SAMPLE_APPS} wallpaperUrl={null} interactive width={960} />);
    const tv = screen.getByRole("application");
    expect(screen.getAllByText("Netflix").length).toBeGreaterThan(0);
    fireEvent.keyDown(tv, { key: "ArrowRight" });
    expect(screen.getAllByText("YouTube").length).toBeGreaterThan(1);
  });
});

describe("editor page", () => {
  it("edits, blocks Apply on invalid values, and imports with dropped-key notes", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "import_theme") return { config: { columns: 6 }, issues: [{ level: "warning", key: "future_key", message: "Dropped: Unknown key" }] };
      return [];
    });
    function Host() { const ed = useThemeEditor("blank"); return <EditorPage serial="tv:5555" editor={ed} advanced={false} />; }
    render(<Host />);
    const apply = screen.getByRole("button", { name: /Apply to TV/ });
    expect(apply).toBeEnabled();
    fireEvent.change(screen.getByLabelText("Columns"), { target: { value: "7" } });
    expect((screen.getByLabelText("Columns") as HTMLInputElement).value).toBe("7");
    fireEvent.click(screen.getByRole("button", { name: /Undo/ }));
    expect((screen.getByLabelText("Columns") as HTMLInputElement).value).toBe("4");

    const file = new File([JSON.stringify({ columns: 6, future_key: true })], "living-room.json", { type: "application/json" });
    const input = document.querySelector('input[type="file"][accept*="json"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [file] } });
    expect(await screen.findByText(/Imported 1 settings/)).toBeInTheDocument();
    expect(screen.getByText(/future_key: Dropped/)).toBeInTheDocument();
    expect((screen.getByLabelText("Theme name") as HTMLInputElement).value).toBe("living-room");
  });
});

describe("apply dialog", () => {
  it("installs, confirms Home on screen, then sends the theme with its wallpaper", async () => {
    globalThis.fetch = vi.fn(async () => ({ blob: async () => new Blob([new Uint8Array([0xff, 0xd8, 1])]) })) as unknown as typeof fetch;
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "launcher_status") return { status: { installed: false, enabled: false, version: null, resolved_home: "com.stock/.Home", is_resolved_home: false }, apk_bundled: true, schema_launcher_version: "3.0-studio.1", focused: null };
      if (cmd === "install_launcher") return { installed: true, enabled: true, version: "3.0-studio.1", resolved_home: "com.stock/.Home", is_resolved_home: false };
      if (cmd === "make_launcher_home") return { resolved: "studio.tvcustomizer.home/com.gathin.tvhome.MainActivity", focused: "studio.tvcustomizer.home/com.gathin.tvhome.MainActivity", verified: true, note: null };
      if (cmd === "deploy_theme") return { written: [["config.json", "provider"]], config_verified: true, restarted: true, focused_after: "studio.tvcustomizer.home/com.gathin.tvhome.MainActivity" };
      throw new Error(cmd);
    });
    render(<ApplyThemeDialog serial="tv:5555" draft={{ name: "Aurora", baseId: "aurora", config: { columns: 5 }, wallpapers: [{ name: "aurora.jpg", url: "/themes/aurora.jpg" }] }} onClose={() => {}} />);
    fireEvent.click(await screen.findByRole("button", { name: /Apply now/ }));
    await waitFor(() => expect(screen.getByText("Confirmed on screen.")).toBeInTheDocument());
    await waitFor(() => expect(screen.getByText(/Read back from the TV and matches/)).toBeInTheDocument());
    const order = invoke.mock.calls.map((c) => c[0]);
    expect(order.indexOf("install_launcher")).toBeLessThan(order.indexOf("make_launcher_home"));
    expect(order.indexOf("make_launcher_home")).toBeLessThan(order.indexOf("deploy_theme"));
    const sent = invoke.mock.calls.find((c) => c[0] === "deploy_theme")![1] as { theme: { wallpapers: { name: string; data_base64: string }[] } };
    expect(sent.theme.wallpapers[0]).toEqual({ name: "aurora.jpg", data_base64: "/9gB" });
  });

  it("reports an unconfirmed Home honestly", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "launcher_status") return { status: { installed: true, enabled: true, version: "3.0-studio.1", resolved_home: "x.y/.Z", is_resolved_home: false }, apk_bundled: true, schema_launcher_version: "3.0-studio.1", focused: null };
      if (cmd === "make_launcher_home") return { resolved: null, focused: "com.google.android.tungsten.setupwraith/.RecoveryActivity", verified: false, note: "Google TV's setup wizard still answers Home." };
      if (cmd === "deploy_theme") return { written: [], config_verified: true, restarted: false, focused_after: null };
      throw new Error(cmd);
    });
    render(<ApplyThemeDialog serial="tv:5555" draft={{ name: "Blank", baseId: "blank", config: {}, wallpapers: [] }} onClose={() => {}} />);
    fireEvent.click(await screen.findByRole("button", { name: /Apply now/ }));
    expect(await screen.findByText(/setup wizard still answers Home/)).toBeInTheDocument();
    expect(invoke.mock.calls.some((c) => c[0] === "install_launcher")).toBe(false);
  });
});
