import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));

import { ConnectPage } from "./pages/ConnectPage";
import { InspectorPage } from "./pages/InspectorPage";

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  invoke.mockReset();
});
afterEach(() => { vi.useRealTimers(); });

const base = {
  serial: "192.168.1.50:5555", fingerprint: null, brand: "TCL", manufacturer: "TCL", model: "Smart TV Pro", device: "G09",
  product: "G09_4K_GB", android_release: "12", sdk: 31, build_id: null, incremental: null, platform: "t5m", adb_secure: true,
  ram_total_kb: 1956324, ram_available_kb: 803112, data_total_kb: null, data_free_kb: null, package_count: 412, disabled_count: 3,
  home: { kind: "component", component: "com.google.android.apps.tv.launcherx/.home.HomeActivity" },
  home_candidates: [
    { component: "com.google.android.apps.tv.launcherx/.home.HomeActivity", usable: true },
    { component: "com.google.android.tungsten.setupwraith/.RecoveryActivity", usable: false },
  ],
  oem: [{ family: "google_tv", label: "Google TV", note: "setupwraith note" }],
  capabilities: { cmd_package: true, studio_launcher: true, wireless_pairing: true, private_dns: true, content_write: true, install_via_pm: false },
  findings: [{ severity: "warning", message: "ro.adb.secure=0 example" }],
  probe_errors: ["storage: df: /data: unsupported"],
};

describe("Connect wizard", () => {
  it("connects, waits for authorization, then reports success", async () => {
    let authorized = false;
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "connect_tv") return { serial: "192.168.1.50:5555", state: "unauthorized", model: null };
      if (cmd === "list_devices") return [{ serial: "192.168.1.50:5555", state: authorized ? "device" : "unauthorized", model: authorized ? "Smart TV Pro" : null, product: null }];
      throw new Error(`unexpected ${cmd}`);
    });
    const onConnected = vi.fn();
    render(<ConnectPage onConnected={onConnected} />);
    fireEvent.click(screen.getByText(/I've done this/));
    fireEvent.change(screen.getByPlaceholderText("192.168.1.50"), { target: { value: "192.168.1.50" } });
    fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    expect(await screen.findByText(/Waiting for your answer on the TV/)).toBeInTheDocument();
    authorized = true;
    await waitFor(() => expect(screen.getByText(/is connected and has authorized/)).toBeInTheDocument(), { timeout: 4000 });
    fireEvent.click(screen.getByText("Inspect this TV"));
    expect(onConnected).toHaveBeenCalledWith("192.168.1.50:5555");
    expect(invoke).toHaveBeenCalledWith("connect_tv", { address: "192.168.1.50" });
  });

  it("shows the backend's advice when the TV cannot be reached", async () => {
    invoke.mockRejectedValue({ kind: "invalid", message: "The TV is reachable but network debugging is off." });
    render(<ConnectPage onConnected={() => {}} />);
    fireEvent.click(screen.getByText(/I've done this/));
    fireEvent.change(screen.getByPlaceholderText("192.168.1.50"), { target: { value: "192.168.1.50" } });
    fireEvent.click(screen.getByRole("button", { name: "Connect" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("network debugging is off");
  });

  it("pairs with a 6-digit code only", async () => {
    invoke.mockResolvedValue(null);
    render(<ConnectPage onConnected={() => {}} />);
    fireEvent.click(screen.getByText(/I've done this/));
    fireEvent.click(screen.getByRole("tab", { name: /Wireless debugging/ }));
    fireEvent.change(screen.getByPlaceholderText("192.168.1.50:41234"), { target: { value: "192.168.1.50:41234" } });
    const codeBox = screen.getByPlaceholderText("000000");
    fireEvent.change(codeBox, { target: { value: "12a34" } });
    expect(screen.getByRole("button", { name: "Pair" })).toBeDisabled();
    fireEvent.change(codeBox, { target: { value: "123456" } });
    await act(async () => { fireEvent.click(screen.getByRole("button", { name: "Pair" })); });
    expect(invoke).toHaveBeenCalledWith("pair_tv", { address: "192.168.1.50:41234", code: "123456" });
    expect(await screen.findByText(/^Paired\./)).toBeInTheDocument();
  });

  it("only calls the port closed when the TV refused", async () => {
    invoke.mockResolvedValueOnce({ endpoint: "192.168.1.50:5555", port_state: { state: "timeout" }, advice: "x" })
      .mockResolvedValueOnce({ endpoint: "192.168.1.50:5555", port_state: { state: "refused" }, advice: "x" });
    render(<ConnectPage onConnected={() => {}} />);
    const box = screen.getByLabelText("Address to check");
    fireEvent.change(box, { target: { value: "192.168.1.50" } });
    fireEvent.click(screen.getByText("Check port is closed"));
    expect(await screen.findByText(/does not prove the port is closed/)).toBeInTheDocument();
    fireEvent.click(screen.getByText("Check port is closed"));
    expect(await screen.findByText(/Closed: the TV refused/)).toBeInTheDocument();
  });
});

describe("Device Inspector", () => {
  it("renders real fields, dashes for missing ones, and failed queries", async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === "list_devices") return [{ serial: "192.168.1.50:5555", state: "device", model: "Smart TV Pro", product: null }];
      if (cmd === "probe_device") return base;
      throw new Error(cmd);
    });
    render(<InspectorPage serial="192.168.1.50:5555" onSelect={() => {}} />);
    expect(await screen.findByText("Smart TV Pro", { selector: "dd" })).toBeInTheDocument();
    expect(screen.getByText(/1.87 GB total/)).toBeInTheDocument();
    expect(screen.getByText("— total · — free")).toBeInTheDocument();
    expect(screen.getByText("Not a launcher")).toBeInTheDocument();
    expect(screen.getByText("storage: df: /data: unsupported")).toBeInTheDocument();
    expect(screen.getByText("ro.adb.secure=0 example")).toBeInTheDocument();
  });

  it("rejects a response that drifted from the Rust shape", async () => {
    invoke.mockImplementation(async (cmd: string) => (cmd === "list_devices" ? [] : { ...base, sdk: "31" }));
    render(<InspectorPage serial="192.168.1.50:5555" onSelect={() => {}} />);
    expect(await screen.findByRole("alert")).toHaveTextContent("Unexpected response from probe_device");
  });
});
