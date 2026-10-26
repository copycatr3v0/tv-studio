//! Apply Everything: dry-run plan, journaled run with live events, cancel, resume after an
//! interruption, history and undo of one deployment.

use crate::debloat::now_iso;
use crate::launcher_cmds::{bundle_from, bundled_apk, ThemeIn};
use crate::{blocking, serial_arg, transport, AppState, CmdError, CmdResult};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tvs_deploy::{plan, run, DeployInput, Event, Hooks, Plan, Profile, RunReport};
use tvs_recovery::{apply_restore, restore_plan, ChangeReport, Snapshot};
use tvs_transport::Transport;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeployRequest {
    pub profile: Profile,
    #[serde(default)]
    pub theme: Option<ThemeIn>,
}

fn input(app: &tauri::AppHandle, req: &DeployRequest) -> CmdResult<DeployInput> {
    let theme = match (&req.theme, req.profile.apply_theme) { (Some(t), true) => Some(bundle_from(t.clone())?), _ => None };
    let batch = app.state::<AppState>().store.lock().unwrap().settings()?.batch_size as usize;
    Ok(DeployInput { profile: req.profile.clone(), theme, apk: bundled_apk(app), batch_size: batch, settle: Duration::from_millis(1500) })
}

impl From<tvs_deploy::PlanError> for CmdError { fn from(e: tvs_deploy::PlanError) -> Self { CmdError::Invalid(e.to_string()) } }

/// Dry run: read the TV and preview every step. Changes nothing.
#[tauri::command]
pub async fn deploy_plan(app: tauri::AppHandle, serial: String, request: DeployRequest) -> CmdResult<Plan> {
    let serial = serial_arg(&serial)?;
    let inp = input(&app, &request)?;
    let t = transport(&app.state::<AppState>())?;
    blocking(move || Ok(plan(&t, &serial, &inp)?)).await
}

struct AppHooks { app: tauri::AppHandle, deployment: i64 }

impl Hooks for AppHooks {
    fn event(&mut self, e: &Event) { let _ = self.app.emit("deploy-event", e); }
    fn journal(&mut self, step: &str, phase: &str, detail: Option<&str>) {
        let _ = self.app.state::<AppState>().store.lock().unwrap().journal(self.deployment, step, phase, detail);
    }
    fn save_snapshot(&mut self, identity: &str, snap: &Snapshot) -> Result<(), String> {
        let body = serde_json::to_value(snap).map_err(|e| e.to_string())?;
        let st = self.app.state::<AppState>();
        let store = st.store.lock().unwrap();
        store.upsert_device(identity, &snap.serial, None, None, None).map_err(|e| e.to_string())?;
        store.save_baseline_once(identity, &body).map_err(|e| e.to_string())?;
        let id = store.save_snapshot(identity, "pre_deploy", &body).map_err(|e| e.to_string())?;
        store.set_deployment_snapshot(self.deployment, id).map_err(|e| e.to_string())
    }
    fn pause(&mut self, d: Duration) { std::thread::sleep(d); }
    fn cancelled(&mut self) -> bool { self.app.state::<AppState>().cancel_deploy.load(Ordering::SeqCst) }
    fn now(&mut self) -> String { now_iso() }
}

#[derive(Debug, Serialize)]
pub struct RunResult { deployment_id: i64, plan: Plan, report: RunReport }

fn run_request(app: tauri::AppHandle, serial: tvs_core::Serial, req: DeployRequest) -> CmdResult<RunResult> {
    let inp = input(&app, &req)?;
    let t = transport(&app.state::<AppState>())?;
    let p = plan(&t, &serial, &inp)?;
    if !p.runnable { return Err(CmdError::Invalid("The plan has blocked steps. Run the dry run and resolve them; nothing was changed.".into())); }
    let body = serde_json::to_value(&req).map_err(|e| CmdError::Invalid(e.to_string()))?;
    let id = {
        let st = app.state::<AppState>();
        let store = st.store.lock().unwrap();
        store.upsert_device(&p.identity, serial.as_str(), None, None, None)?;
        store.start_deployment(&p.identity, &body, None)?
    };
    app.state::<AppState>().cancel_deploy.store(false, Ordering::SeqCst);
    let mut hooks = AppHooks { app: app.clone(), deployment: id };
    let report = run(&t, &serial, &p, &inp, &mut hooks);
    let status = if report.completed { "completed" } else if report.cancelled { "cancelled" } else if report.stopped.is_some() { "stopped" } else { "partial" };
    app.state::<AppState>().store.lock().unwrap().finish_deployment(id, status)?;
    Ok(RunResult { deployment_id: id, plan: p, report })
}

/// Apply Everything. Re-plans on the TV first, so the run always starts from the TV's real state.
#[tauri::command]
pub async fn deploy_run(app: tauri::AppHandle, serial: String, request: DeployRequest) -> CmdResult<RunResult> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Apply Everything")?;
    let serial = serial_arg(&serial)?;
    blocking(move || run_request(app, serial, request)).await
}

#[tauri::command]
pub fn deploy_cancel(state: tauri::State<'_, AppState>) { state.cancel_deploy.store(true, Ordering::SeqCst); }

#[derive(Debug, Serialize)]
pub struct Interrupted { id: i64, started: String, name: String, last_step: Option<String> }

fn identity(app: &tauri::AppHandle, serial: &tvs_core::Serial) -> CmdResult<String> {
    let t = transport(&app.state::<AppState>())?;
    tvs_device::probe::quick_identity(&t, serial).ok_or_else(|| CmdError::Invalid("Could not read the TV's identity.".into()))
}

/// A deployment for this TV that never finished (app closed, crash, power cut).
#[tauri::command]
pub async fn deploy_interrupted(app: tauri::AppHandle, serial: String) -> CmdResult<Option<Interrupted>> {
    let serial = serial_arg(&serial)?;
    let app2 = app.clone();
    let id = blocking(move || identity(&app2, &serial)).await?;
    let st = app.state::<AppState>();
    let store = st.store.lock().unwrap();
    let Some(row) = store.interrupted_deployment(&id)? else { return Ok(None) };
    let last = store.journal_rows(row.id)?.into_iter().last().map(|j| format!("{} ({})", j.step_id, j.phase));
    let name = row.profile.as_ref().and_then(|p| p.pointer("/profile/name")).and_then(|v| v.as_str()).unwrap_or("deployment").to_string();
    Ok(Some(Interrupted { id: row.id, started: row.started, name, last_step: last }))
}

/// Resume = plan the same request again against the TV's real state; finished steps show as already done.
#[tauri::command]
pub async fn deploy_resume(app: tauri::AppHandle, serial: String, deployment_id: i64) -> CmdResult<RunResult> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Apply Everything")?;
    let serial = serial_arg(&serial)?;
    let row = app.state::<AppState>().store.lock().unwrap().deployment(deployment_id)?
        .ok_or_else(|| CmdError::Invalid("That deployment no longer exists.".into()))?;
    let req: DeployRequest = serde_json::from_value(row.profile.ok_or_else(|| CmdError::Invalid("This deployment has no saved request to resume.".into()))?)
        .map_err(|e| CmdError::Invalid(format!("Saved request unreadable: {e}")))?;
    app.state::<AppState>().store.lock().unwrap().finish_deployment(deployment_id, "interrupted")?;
    blocking(move || run_request(app, serial, req)).await
}

#[tauri::command]
pub fn deploy_discard(state: tauri::State<'_, AppState>, deployment_id: i64) -> CmdResult<()> {
    Ok(state.store.lock().unwrap().finish_deployment(deployment_id, "abandoned")?)
}

#[derive(Debug, Serialize)]
pub struct HistoryRow { id: i64, started: String, finished: Option<String>, status: String, name: String, can_undo: bool }

#[tauri::command]
pub async fn deploy_history(app: tauri::AppHandle, serial: String) -> CmdResult<Vec<HistoryRow>> {
    let serial = serial_arg(&serial)?;
    let app2 = app.clone();
    let id = blocking(move || identity(&app2, &serial)).await?;
    let rows = app.state::<AppState>().store.lock().unwrap().deployments(&id, 30)?;
    Ok(rows.into_iter().map(|r| HistoryRow {
        name: r.profile.as_ref().and_then(|p| p.pointer("/profile/name")).and_then(|v| v.as_str()).unwrap_or("deployment").to_string(),
        can_undo: r.snapshot_id.is_some(), id: r.id, started: r.started, finished: r.finished, status: r.status,
    }).collect())
}

/// Put the TV back to how it was right before one deployment (its pre-change snapshot).
#[tauri::command]
pub async fn undo_deployment(app: tauri::AppHandle, serial: String, deployment_id: i64) -> CmdResult<ChangeReport> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Undo")?;
    let serial = serial_arg(&serial)?;
    let (snap_id, owner) = {
        let st = app.state::<AppState>();
        let store = st.store.lock().unwrap();
        let row = store.deployment(deployment_id)?.ok_or_else(|| CmdError::Invalid("That deployment no longer exists.".into()))?;
        (row.snapshot_id.ok_or_else(|| CmdError::Invalid("This deployment stopped before its snapshot, so nothing was changed by it.".into()))?, row.fingerprint)
    };
    let body = app.state::<AppState>().store.lock().unwrap().snapshot(snap_id)?.ok_or_else(|| CmdError::Invalid("Snapshot missing.".into()))?;
    let snap: Snapshot = serde_json::from_value(body).map_err(|e| CmdError::Invalid(e.to_string()))?;
    let t = transport(&app.state::<AppState>())?;
    let batch = app.state::<AppState>().store.lock().unwrap().settings()?.batch_size as usize;
    blocking(move || {
        if tvs_device::probe::quick_identity(&t, &serial).as_deref() != Some(owner.as_str()) {
            return Err(CmdError::Invalid("That deployment belongs to a different TV.".into()));
        }
        let current = tvs_device::inventory::list_packages(&t, &serial)?;
        let home = tvs_device::inventory::current_home(&t, &serial);
        let settings = tvs_recovery::TRACKED_SETTINGS.iter().filter_map(|k| {
            let o = t.shell(&serial, &tvs_transport::ShellCommand::new("settings").arg("get").arg("global").arg(k)).ok()?;
            let v = o.stdout.trim().to_string();
            Some((k.to_string(), if v == "null" || v.is_empty() { None } else { Some(v) }))
        }).collect();
        let plan = restore_plan(&snap, &current, home.as_deref(), &settings, None);
        let mut report = apply_restore(&t, &serial, &plan, batch, &mut |_| {});
        // The launcher came with this deployment: remove it the safe way (Home handed back and confirmed first).
        if report.stopped.is_none() && !snap.packages.contains_key(tvs_launcher::install::STUDIO_PKG) {
            let (c, k) = tvs_launcher::revert::remove_studio_launcher(&t, &serial, snap.home.as_deref(), Duration::from_millis(1500));
            report.changed.extend(c);
            report.unchanged.extend(k);
        }
        Ok(report)
    }).await
}

const PROFILE_PREFIX: &str = "deploy:";

#[tauri::command]
pub fn save_deploy_profile(state: tauri::State<'_, AppState>, profile: Profile) -> CmdResult<i64> {
    let name = profile.name.trim();
    if name.is_empty() || name.len() > 80 || name.chars().any(char::is_control) { return Err(CmdError::Invalid("Give the profile a name (up to 80 characters).".into())); }
    let body = serde_json::to_value(&profile).map_err(|e| CmdError::Invalid(e.to_string()))?;
    Ok(state.store.lock().unwrap().save_profile(&format!("{PROFILE_PREFIX}{name}"), &body)?)
}

#[derive(Debug, Serialize)]
pub struct SavedProfile { name: String, version: i64, created: String }

#[tauri::command]
pub fn list_deploy_profiles(state: tauri::State<'_, AppState>) -> CmdResult<Vec<SavedProfile>> {
    Ok(state.store.lock().unwrap().list_profiles(PROFILE_PREFIX)?.into_iter()
        .map(|p| SavedProfile { name: p.name.trim_start_matches(PROFILE_PREFIX).to_string(), version: p.version, created: p.created }).collect())
}

#[tauri::command]
pub fn load_deploy_profile(state: tauri::State<'_, AppState>, name: String, version: i64) -> CmdResult<Profile> {
    let v = state.store.lock().unwrap().get_profile(&format!("{PROFILE_PREFIX}{name}"), version)?
        .ok_or_else(|| CmdError::Invalid("That profile version no longer exists.".into()))?;
    serde_json::from_value(v).map_err(|e| CmdError::Invalid(format!("Profile unreadable: {e}")))
}

/// Parse an exported profile file; unknown fields are rejected so typos don't silently do nothing.
#[tauri::command]
pub fn import_deploy_profile(json_text: String) -> CmdResult<Profile> {
    if json_text.len() > 1024 * 1024 { return Err(CmdError::Invalid("That file is too large to be a profile.".into())); }
    serde_json::from_str::<Profile>(&json_text).map_err(|e| CmdError::Invalid(format!("Not a valid profile: {e}")))
}
