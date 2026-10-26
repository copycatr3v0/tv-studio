// Typed, validated bridge to the Rust commands. Every response is parsed with Zod so a Rust/TS drift
// fails loudly instead of rendering wrong device state.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

export const PortState = z.discriminatedUnion("state", [
  z.object({ state: z.literal("open") }),
  z.object({ state: z.literal("refused") }),
  z.object({ state: z.literal("timeout") }),
  z.object({ state: z.literal("unreachable"), detail: z.string() }),
]);
export const Diagnosis = z.object({ endpoint: z.string(), port_state: PortState, advice: z.string() });
export type Diagnosis = z.infer<typeof Diagnosis>;

export const ConnectResult = z.object({ serial: z.string(), state: DeviceState.nullable(), model: z.string().nullable() });
export type ConnectResult = z.infer<typeof ConnectResult>;

export const MdnsService = z.object({ name: z.string(), kind: z.enum(["connect", "pairing", "legacy"]), endpoint: z.string() });
export type MdnsService = z.infer<typeof MdnsService>;

const opt = <T extends z.ZodTypeAny>(t: T) => t.nullable();
export const DeviceInfo = z.object({
  serial: z.string(),
  fingerprint: opt(z.string()), brand: opt(z.string()), manufacturer: opt(z.string()), model: opt(z.string()),
  device: opt(z.string()), product: opt(z.string()), android_release: opt(z.string()), sdk: opt(z.number().int()),
  build_id: opt(z.string()), incremental: opt(z.string()), platform: opt(z.string()), adb_secure: opt(z.boolean()),
  ram_total_kb: opt(z.number()), ram_available_kb: opt(z.number()), data_total_kb: opt(z.number()), data_free_kb: opt(z.number()),
  package_count: opt(z.number().int()), disabled_count: opt(z.number().int()),
  home: z.discriminatedUnion("kind", [
    z.object({ kind: z.literal("component"), component: z.string() }),
    z.object({ kind: z.literal("chooser") }),
    z.object({ kind: z.literal("unknown") }),
  ]),
  home_candidates: z.array(z.object({ component: z.string(), usable: z.boolean() })),
  oem: z.array(z.object({ family: z.string(), label: z.string(), note: z.string() })),
  capabilities: z.object({
    cmd_package: z.boolean(), studio_launcher: z.boolean(), wireless_pairing: z.boolean(),
    private_dns: z.boolean(), content_write: z.boolean(), install_via_pm: z.boolean(),
  }),
  findings: z.array(z.object({ severity: z.enum(["info", "warning", "blocker"]), message: z.string() })),
  probe_errors: z.array(z.string()),
});
export type DeviceInfo = z.infer<typeof DeviceInfo>;

export const Verdict = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("Allowed") }),
  z.object({ kind: z.literal("NeedsConfirm"), reason: z.string() }),
  z.object({ kind: z.literal("Blocked"), reason: z.string() }),
]);
export type Verdict = z.infer<typeof Verdict>;

export const KbCategory = z.enum(["telemetry", "ads_content", "retail_demo", "preinstalled", "streaming", "voice", "casting", "updater", "launcher", "factory", "privacy", "other"]);
export type KbCategory = z.infer<typeof KbCategory>;
export const KbEntry = z.object({
  disabled_in: z.number().int(), kept_in: z.number().int(), category: KbCategory,
  notes: z.array(z.string()), devices: z.array(z.string()),
  privacy: z.object({ category: z.string(), note: z.string() }).optional(),
});
export type KbEntry = z.infer<typeof KbEntry>;
export const AppRow = z.object({
  name: z.string(), enabled: z.boolean(), system: z.boolean(), kb: KbEntry.nullable(), verdict: Verdict,
  preselect_safe: z.boolean(), preselect_balanced: z.boolean(),
});
export type AppRow = z.infer<typeof AppRow>;
export const Inventory = z.object({ apps: z.array(AppRow), kb_lists: z.number().int() });
export type Inventory = z.infer<typeof Inventory>;

export const Evaluation = z.object({ verdicts: z.record(Verdict), facts: z.record(z.unknown()) });
export type Evaluation = z.infer<typeof Evaluation>;

export const ChangeReport = z.object({
  changed: z.array(z.string()), refused: z.array(z.tuple([z.string(), z.string()])),
  unchanged: z.array(z.string()), skipped: z.array(z.string()), stopped: z.string().nullable(),
});
export type ChangeReport = z.infer<typeof ChangeReport>;
export const ApplyResult = z.object({ report: ChangeReport, baseline_created: z.boolean(), identity: z.string() });
export type ApplyResult = z.infer<typeof ApplyResult>;

export const RestorePlan = z.object({
  enable: z.array(z.string()), left_disabled: z.array(z.string()), not_in_snapshot: z.array(z.string()),
  set_home: z.string().nullable(), settings: z.array(z.tuple([z.string(), z.string().nullable()])),
});
export type RestorePlan = z.infer<typeof RestorePlan>;
export const RestorePreview = z.object({ plan: RestorePlan, baseline_created: z.string(), studio_launcher: z.boolean() });
export type RestorePreview = z.infer<typeof RestorePreview>;

export const Progress = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("package"), name: z.string(), index: z.number(), total: z.number() }),
  z.object({ kind: z.literal("verified"), batch: z.number(), batches: z.number() }),
]);
export type Progress = z.infer<typeof Progress>;

export const LauncherStatus = z.object({
  installed: z.boolean(), enabled: z.boolean(), version: z.string().nullable(),
  resolved_home: z.string().nullable(), is_resolved_home: z.boolean(),
});
export type LauncherStatus = z.infer<typeof LauncherStatus>;
export const LauncherInfo = z.object({ status: LauncherStatus, apk_bundled: z.boolean(), schema_launcher_version: z.string(), focused: z.string().nullable() });
export type LauncherInfo = z.infer<typeof LauncherInfo>;
export const HomeCheck = z.object({ resolved: z.string().nullable(), focused: z.string().nullable(), verified: z.boolean(), note: z.string().nullable() });
export type HomeCheck = z.infer<typeof HomeCheck>;
export const ThemeIssue = z.object({ level: z.enum(["error", "warning"]), key: z.string(), message: z.string() });
export type ThemeIssue = z.infer<typeof ThemeIssue>;
export const ThemeReport = z.object({
  written: z.array(z.tuple([z.string(), z.enum(["provider", "copy"])])), config_verified: z.boolean(),
  restarted: z.boolean(), focused_after: z.string().nullable(),
});
export type ThemeReport = z.infer<typeof ThemeReport>;
export const SavedTheme = z.object({ name: z.string(), version: z.number().int(), created: z.string() });
export type SavedTheme = z.infer<typeof SavedTheme>;

export const PrivateDns = z.discriminatedUnion("mode", [
  z.object({ mode: z.literal("keep") }), z.object({ mode: z.literal("automatic") }), z.object({ mode: z.literal("off") }),
  z.object({ mode: z.literal("hostname"), hostname: z.string() }),
]);
export const Profile = z.object({
  version: z.number().int(),
  name: z.string(),
  device_identity: z.string().nullable(),
  launcher: z.object({ install: z.boolean(), make_home: z.boolean(), disable_stock: z.boolean() }),
  debloat: z.object({ packages: z.array(z.string()), confirmed: z.array(z.string()) }),
  settings: z.object({ animation_scale: z.string().nullable(), private_dns: PrivateDns }),
  apply_theme: z.boolean(),
});
export type Profile = z.infer<typeof Profile>;
export const emptyProfile = (name = "My TV setup"): Profile => ({
  version: 1, name, device_identity: null,
  launcher: { install: false, make_home: false, disable_stock: false },
  debloat: { packages: [], confirmed: [] },
  settings: { animation_scale: null, private_dns: { mode: "keep" } },
  apply_theme: false,
});

export const PlanPreview = z.discriminatedUnion("state", [
  z.object({ state: z.literal("check"), detail: z.string() }),
  z.object({ state: z.literal("will_change"), detail: z.string() }),
  z.object({ state: z.literal("already_done"), detail: z.string() }),
  z.object({ state: z.literal("blocked"), detail: z.string() }),
]);
export const PlanStep = z.object({ id: z.string(), title: z.string(), kind: z.object({ kind: z.string() }).passthrough(), depends_on: z.array(z.string()), critical: z.boolean(), preview: PlanPreview });
export type PlanStep = z.infer<typeof PlanStep>;
export const Plan = z.object({ identity: z.string(), sdk: z.number(), steps: z.array(PlanStep), runnable: z.boolean(), notes: z.array(z.string()) });
export type Plan = z.infer<typeof Plan>;
export const StepStatus = z.enum(["running", "done", "already_done", "failed", "skipped", "reconnecting"]);
export type StepStatus = z.infer<typeof StepStatus>;
export const DeployEvent = z.object({ step: z.string(), title: z.string(), status: StepStatus, message: z.string(), finished: z.number(), total: z.number() });
export type DeployEvent = z.infer<typeof DeployEvent>;
export const RunReport = z.object({
  results: z.array(z.object({ id: z.string(), title: z.string(), status: StepStatus, message: z.string() })),
  completed: z.boolean(), cancelled: z.boolean(), stopped: z.string().nullable(),
});
export type RunReport = z.infer<typeof RunReport>;
export const RunResult = z.object({ deployment_id: z.number(), plan: Plan, report: RunReport });
export type RunResult = z.infer<typeof RunResult>;
export const Interrupted = z.object({ id: z.number(), started: z.string(), name: z.string(), last_step: z.string().nullable() });
export const HistoryRow = z.object({ id: z.number(), started: z.string(), finished: z.string().nullable(), status: z.string(), name: z.string(), can_undo: z.boolean() });
export type HistoryRow = z.infer<typeof HistoryRow>;
export type ThemeIn = { config: Record<string, unknown>; wallpapers: { name: string; data_base64: string }[]; captions: string | null };

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
  pairing_failed: "Pairing failed. The code and pairing port change each time the pairing screen opens: check both on the TV and try again.",
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

export const errMsg = (e: unknown): string => (e instanceof IpcError ? e.message : String(e));

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
  diagnose: (address: string) => call("diagnose", Diagnosis, { address }),
  connect: (address: string) => call("connect_tv", ConnectResult, { address }),
  pair: (address: string, code: string) => call("pair_tv", z.null(), { address, code }),
  discover: () => call("discover_wireless", z.array(MdnsService)),
  disconnect: (serial: string) => call("disconnect_tv", z.null(), { serial }),
  probe: (serial: string) => call("probe_device", DeviceInfo, { serial }),
  inventory: (serial: string) => call("inventory", Inventory, { serial }),
  evaluate: (serial: string, packages: string[]) => call("evaluate_debloat", Evaluation, { serial, packages }),
  applyDebloat: (serial: string, packages: string[], confirmed: string[]) =>
    call("apply_debloat", ApplyResult, { request: { serial, packages, confirmed } }),
  restorePreview: (serial: string, scope: string[] | null) => call("restore_preview", RestorePreview, { serial, scope }),
  baselineStatus: (serial: string) => call("baseline_status", z.object({ identity: z.string(), created: z.string().nullable() }), { serial }),
  launchableApps: (serial: string) => call("launchable_apps", z.array(z.string()), { serial }),
  launcherStatus: (serial: string) => call("launcher_status", LauncherInfo, { serial }),
  installLauncher: (serial: string) => call("install_launcher", LauncherStatus, { serial }),
  makeLauncherHome: (serial: string) => call("make_launcher_home", HomeCheck, { serial }),
  importTheme: (jsonText: string) => call("import_theme", z.object({ config: z.record(z.unknown()), issues: z.array(ThemeIssue) }), { jsonText }),
  deployTheme: (serial: string, theme: { config: Record<string, unknown>; wallpapers: { name: string; data_base64: string }[]; captions: string | null }) =>
    call("deploy_theme", ThemeReport, { serial, theme }),
  resetTheme: (serial: string) => call("reset_theme", ThemeReport, { serial }),
  saveThemeVersion: (name: string, theme: unknown) => call("save_theme_version", z.number().int(), { name, theme }),
  listThemeVersions: () => call("list_theme_versions", z.array(SavedTheme)),
  loadThemeVersion: (name: string, version: number) => call("load_theme_version", z.unknown(), { name, version }),
  deployPlan: (serial: string, profile: Profile, theme: ThemeIn | null) => call("deploy_plan", Plan, { serial, request: { profile, theme } }),
  deployRun: (serial: string, profile: Profile, theme: ThemeIn | null) => call("deploy_run", RunResult, { serial, request: { profile, theme } }),
  deployCancel: () => call("deploy_cancel", z.null()),
  deployInterrupted: (serial: string) => call("deploy_interrupted", Interrupted.nullable(), { serial }),
  deployResume: (serial: string, deploymentId: number) => call("deploy_resume", RunResult, { serial, deploymentId }),
  deployDiscard: (deploymentId: number) => call("deploy_discard", z.null(), { deploymentId }),
  deployHistory: (serial: string) => call("deploy_history", z.array(HistoryRow), { serial }),
  undoDeployment: (serial: string, deploymentId: number) => call("undo_deployment", ChangeReport, { serial, deploymentId }),
  saveDeployProfile: (profile: Profile) => call("save_deploy_profile", z.number().int(), { profile }),
  listDeployProfiles: () => call("list_deploy_profiles", z.array(SavedTheme)),
  loadDeployProfile: (name: string, version: number) => call("load_deploy_profile", Profile, { name, version }),
  importDeployProfile: (jsonText: string) => call("import_deploy_profile", Profile, { jsonText }),
  restoreApply: (serial: string, scope: string[] | null) => call("restore_apply", ChangeReport, { serial, scope }),
};

/** Subscribe to deployment step events. */
export async function onDeployEvent(cb: (e: DeployEvent) => void): Promise<() => void> {
  if (!inTauri()) return () => {};
  return listen("deploy-event", (e) => { const r = DeployEvent.safeParse(e.payload); if (r.success) cb(r.data); });
}

/** Subscribe to batch progress; returns an unsubscribe function (no-op outside Tauri). */
export async function onProgress(cb: (p: Progress) => void): Promise<() => void> {
  if (!inTauri()) return () => {};
  return listen("debloat-progress", (e) => { const r = Progress.safeParse(e.payload); if (r.success) cb(r.data); });
}

export const __test = { describe };
