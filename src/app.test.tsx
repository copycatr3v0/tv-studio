import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import { SECTIONS, visibleSections } from "./app/sections";
import { AppSettings, DeviceEntry, __test } from "./lib/ipc";
import { Planned } from "./pages/Planned";
import { App } from "./App";

describe("navigation", () => {
  it("has the 15 sections from the spec, in order, with unique ids", () => {
    expect(SECTIONS.map((s) => s.label)).toEqual([
      "Dashboard", "Connect TV", "Device Inspector", "Themes", "Live Theme Editor", "Launcher Builder",
      "Applications", "Debloat Center", "Privacy & Performance", "Optimization Benchmarks", "Deployment",
      "Profiles", "Restore", "Diagnostics", "Settings",
    ]);
    expect(new Set(SECTIONS.map((s) => s.id)).size).toBe(15);
  });

  it("beginner mode hides only advanced sections", () => {
    const hidden = SECTIONS.filter((s) => !visibleSections("beginner").includes(s)).map((s) => s.id);
    expect(hidden).toEqual(["launcher", "benchmarks", "diagnostics"]);
    expect(visibleSections("advanced")).toHaveLength(15);
  });

  it("every section declares where its changes land", () => {
    for (const s of SECTIONS) expect(s.scope.length).toBeGreaterThan(0);
  });
});

describe("planned sections", () => {
  it("render no interactive controls", () => {
    const s = { ...SECTIONS[0]!, milestone: "M9" as const };
    const { container } = render(<Planned section={s} />);
    expect(container.querySelectorAll("button, input, select, textarea")).toHaveLength(0);
    expect(screen.getByText("M9")).toBeInTheDocument();
  });
});

describe("ipc schemas", () => {
  it("accept the Rust shapes and reject drift", () => {
    expect(AppSettings.safeParse({ ui_mode: "beginner", adb_path: "", default_dry_run: true, batch_size: 5, check_updates: false, reduced_motion_ui: false, demo_mode: false }).success).toBe(true);
    expect(AppSettings.safeParse({ ui_mode: "expert", adb_path: "", default_dry_run: true, batch_size: 5, check_updates: false, reduced_motion_ui: false, demo_mode: false }).success).toBe(false);
    expect(DeviceEntry.safeParse({ serial: "192.168.1.50:5555", state: "unauthorized", model: null, product: null }).success).toBe(true);
  });

  it("turn transport errors into readable messages", () => {
    const e = __test.describe({ kind: "transport", message: { kind: "unauthorized" } });
    expect(e.kind).toBe("transport:unauthorized");
    expect(e.message).toMatch(/Allow USB debugging/);
    expect(__test.describe({ kind: "invalid", message: "No file at X" }).message).toBe("No file at X");
  });
});

describe("outside Tauri", () => {
  it("shows an honest error instead of fake device data", async () => {
    render(<App />);
    expect(await screen.findAllByText(/outside the desktop app/i)).not.toHaveLength(0);
  });
});
