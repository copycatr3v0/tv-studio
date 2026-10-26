// Typed, validated bridge to the Rust commands. Every response is parsed with Zod so a Rust/TS drift
// fails loudly instead of rendering wrong device state.
import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

export const UiMode = z.enum(["beginner", "advanced"]);
export type UiMode = z.infer<typeof UiMode>;

export const AppSettings = z.object({
  ui_mode: UiMode,
  adb_path: z.string(),
  default_dry_run: z.boolean(),
  batch_size: z.number().int().min(1).max(10),
  check_updates: z.boolean(),
  reduced_motion_ui: z.boolean(),
});
export type AppSettings = z.infer<typeof AppSettings>;

export const AdbInfo = z.object({
  path: z.string(),
  version: z.string().nullable(),
  source: z.enum(["bundled", "system", "custom"]),
});

export const AppInfo = z.object({
  version: z.string(),
  db_path: z.string(),
  schema_version: z.number().int(),
  adb: AdbInfo.nullable(),
});
export type AppInfo = z.infer<typeof AppInfo>;

export const DeviceState = z.enum(["device", "unauthorized", "offline", "no_permissions", "other"]);
export const DeviceEntry = z.object({
  serial: z.string(),
  state: DeviceState,
  model: z.string().nullable(),
  product: z.string().nullable(),
});
export type DeviceEntry = z.infer<typeof DeviceEntry>;

export const CmdError = z.object({
  kind: z.enum(["store", "transport", "invalid"]),
  message: z.unknown(),
});

export class IpcError extends Error {
  constructor(message: string, readonly kind: string) {
    super(message);
  }
}

const TRANSPORT_MESSAGES: Record<string, string> = {
  adb_missing: "adb was not found. Reinstall the app, or set a custom adb path in Settings.",
  unauthorized: "The TV has not authorized this computer yet. Accept the “Allow USB debugging?” prompt on the TV.",
  timeout: "The TV did not answer in time.",
};

function describe(err: unknown): IpcError {
  const parsed = CmdError.safeParse(err);
  if (!parsed.success) return new IpcError(String(err), "unknown");
  const { kind, message } = parsed.data;
  if (kind === "transport" && message && typeof message === "object" && "kind" in message) {
    const t = message as { kind: string; detail?: unknown };
    const text = TRANSPORT_MESSAGES[t.kind] ?? (typeof t.detail === "string" ? t.detail : t.kind);
    return new IpcError(text, `transport:${t.kind}`);
  }
  return new IpcError(typeof message === "string" ? message : JSON.stringify(message), kind);
}

export const inTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, schema: z.ZodType<T>, args?: Record<string, unknown>): Promise<T> {
  if (!inTauri()) throw new IpcError("Running outside the desktop app: device features are unavailable.", "no_tauri");
  try {
    return schema.parse(await invoke(cmd, args));
  } catch (e) {
    if (e instanceof z.ZodError) throw new IpcError(`Unexpected response from ${cmd}`, "schema");
    throw describe(e);
  }
}

export const api = {
  appInfo: () => call("app_info", AppInfo),
  getSettings: () => call("get_settings", AppSettings),
  saveSettings: (settings: AppSettings) => call("save_settings", AppSettings, { settings }),
  listDevices: () => call("list_devices", z.array(DeviceEntry)),
};

export const __test = { describe };
