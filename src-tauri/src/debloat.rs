//! Inventory, debloat evaluation, verified apply and exact restore commands.

use crate::{blocking, serial_arg, transport, AppState, CmdError, CmdResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::{Emitter, Manager};
use tvs_core::PackageName;
use tvs_device::inventory::{gather_facts, list_packages, PackageInfo};
use tvs_kb::{KbEntry, Mode};
use tvs_recovery::{apply_restore, capture, disable_in_batches, restore_plan, ChangeReport, RestorePlan, Snapshot};
use tvs_safety::{evaluate_selection, DeviceFacts, Verdict};

#[derive(Debug, Serialize)]
pub struct AppRow {
    name: String,
    enabled: bool,
    system: bool,
    kb: Option<&'static KbEntry>,
    /// Static verdict (core packages); live guards run in `evaluate_debloat`.
    verdict: Verdict,
    preselect_safe: bool,
    preselect_balanced: bool,
}

#[derive(Debug, Serialize)]
pub struct Inventory { apps: Vec<AppRow>, kb_lists: u32 }

impl From<tvs_device::inventory::InventoryError> for CmdError { fn from(e: tvs_device::inventory::InventoryError) -> Self { CmdError::Invalid(e.to_string()) } }
impl From<tvs_recovery::RecoveryError> for CmdError { fn from(e: tvs_recovery::RecoveryError) -> Self { CmdError::Invalid(e.to_string()) } }

#[tauri::command]
pub async fn inventory(state: tauri::State<'_, AppState>, serial: String) -> CmdResult<Inventory> {
    let serial = serial_arg(&serial)?;
    let t = transport(&state)?;
    blocking(move || {
        let pkgs = list_packages(&t, &serial)?;
        let apps = pkgs.into_iter().map(|p| {
            let kb = tvs_kb::get(&p.name);
            let verdict = PackageName::new(p.name.clone()).map(|n| tvs_safety::can_disable(&n)).unwrap_or(Verdict::Blocked("invalid name".into()));
            let ok = p.enabled && !matches!(verdict, Verdict::Blocked(_));
            AppRow {
                preselect_safe: ok && tvs_kb::preselect(kb, Mode::Safe),
                preselect_balanced: ok && tvs_kb::preselect(kb, Mode::Balanced),
                name: p.name, enabled: p.enabled, system: p.system, kb, verdict,
            }
        }).collect();
        Ok(Inventory { apps, kb_lists: tvs_kb::list_count() })
    }).await
}

fn names(pkgs: &[String]) -> CmdResult<Vec<PackageName>> {
    pkgs.iter().map(|p| PackageName::new(p.clone()).map_err(|e| CmdError::Invalid(e.to_string()))).collect()
}

#[derive(Debug, Serialize)]
pub struct Evaluation { verdicts: BTreeMap<String, Verdict>, facts: DeviceFacts }

fn evaluate(t: &dyn tvs_transport::Transport, serial: &tvs_core::Serial, sel: &[PackageName]) -> CmdResult<(Vec<PackageInfo>, Evaluation)> {
    let installed = list_packages(t, serial)?;
    let facts = gather_facts(t, serial, &installed, sel);
    let verdicts = evaluate_selection(sel, &facts).into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    Ok((installed, Evaluation { verdicts, facts }))
}

/// Dry run: live guards for a selection. Changes nothing.
#[tauri::command]
pub async fn evaluate_debloat(state: tauri::State<'_, AppState>, serial: String, packages: Vec<String>) -> CmdResult<Evaluation> {
    let serial = serial_arg(&serial)?;
    let sel = names(&packages)?;
    let t = transport(&state)?;
    blocking(move || Ok(evaluate(&t, &serial, &sel)?.1)).await
}

#[derive(Debug, Deserialize)]
pub struct ApplyRequest { serial: String, packages: Vec<String>, confirmed: Vec<String> }

#[derive(Debug, Serialize)]
pub struct ApplyResult { report: ChangeReport, baseline_created: bool, identity: String }

/// Re-evaluates on the TV, refuses anything blocked or unconfirmed, records the baseline (first wins)
/// and a pre-change snapshot, then disables in verified batches.
#[tauri::command]
pub async fn apply_debloat(app: tauri::AppHandle, request: ApplyRequest) -> CmdResult<ApplyResult> {
    let serial = serial_arg(&request.serial)?;
    let sel = names(&request.packages)?;
    if sel.is_empty() { return Err(CmdError::Invalid("Nothing selected.".into())); }
    let t = transport(&app.state::<AppState>())?;
    let batch = app.state::<AppState>().store.lock().unwrap().settings()?.batch_size as usize;
    let app2 = app.clone();
    blocking(move || {
        let (_, eval) = evaluate(&t, &serial, &sel)?;
        let mut problems = vec![];
        for (pkg, v) in &eval.verdicts {
            match v {
                Verdict::Blocked(r) => problems.push(format!("{pkg}: {r}")),
                Verdict::NeedsConfirm(r) if !request.confirmed.contains(pkg) => problems.push(format!("{pkg} needs confirmation: {r}")),
                _ => {}
            }
        }
        if !problems.is_empty() {
            return Err(CmdError::Invalid(format!("Nothing was changed. {}", problems.join(" | "))));
        }
        let (identity, baseline_created) = ensure_baseline(&app2, &t, &serial)?;
        let report = disable_in_batches(&t, &serial, &sel, batch, &mut |p| { let _ = app2.emit("debloat-progress", &p); });
        Ok(ApplyResult { report, baseline_created, identity })
    }).await
}

/// Record the TV's state before any change this app makes: the baseline (first one ever, never
/// replaced) and a pre-change snapshot. Fails, and the caller changes nothing, if the TV can't be read.
pub(crate) fn ensure_baseline(app: &tauri::AppHandle, t: &dyn tvs_transport::Transport, serial: &tvs_core::Serial) -> CmdResult<(String, bool)> {
    let identity = tvs_device::probe::quick_identity(t, serial)
        .ok_or_else(|| CmdError::Invalid("Could not read the TV's identity, so no snapshot could be saved. Nothing was changed.".into()))?;
    let snap = capture(t, serial, &identity, &now_iso())?;
    let body = serde_json::to_value(&snap).map_err(|e| CmdError::Invalid(e.to_string()))?;
    let st = app.state::<AppState>();
    let store = st.store.lock().unwrap();
    store.upsert_device(&identity, serial.as_str(), None, None, None)?;
    let created = store.save_baseline_once(&identity, &body)?;
    store.save_snapshot(&identity, "pre_deploy", &body)?;
    Ok((identity, created))
}

#[derive(Debug, Serialize)]
pub struct RestorePreview { plan: RestorePlan, baseline_created: String, studio_launcher: bool }

fn load_baseline(app: &tauri::AppHandle, identity: &str) -> CmdResult<Snapshot> {
    let v = app.state::<AppState>().store.lock().unwrap().baseline(identity)?
        .ok_or_else(|| CmdError::Invalid("No baseline saved for this TV yet: nothing has been changed by this app.".into()))?;
    serde_json::from_value(v).map_err(|e| CmdError::Invalid(format!("Baseline unreadable: {e}")))
}

fn current_settings(t: &dyn tvs_transport::Transport, serial: &tvs_core::Serial) -> BTreeMap<String, Option<String>> {
    tvs_recovery::TRACKED_SETTINGS.iter().filter_map(|k| {
        let o = t.shell(serial, &tvs_transport::ShellCommand::new("settings").arg("get").arg("global").arg(k)).ok()?;
        let v = o.stdout.trim().to_string();
        Some((k.to_string(), if v == "null" || v.is_empty() { None } else { Some(v) }))
    }).collect()
}

fn preview(app: &tauri::AppHandle, t: &dyn tvs_transport::Transport, serial: &tvs_core::Serial, scope: Option<&[String]>) -> CmdResult<(Snapshot, RestorePlan)> {
    let identity = tvs_device::probe::quick_identity(t, serial).ok_or_else(|| CmdError::Invalid("Could not read the TV's identity.".into()))?;
    let snap = load_baseline(app, &identity)?;
    let current = list_packages(t, serial)?;
    let home = tvs_device::inventory::current_home(t, serial);
    let plan = restore_plan(&snap, &current, home.as_deref(), &current_settings(t, serial), scope);
    Ok((snap, plan))
}

#[derive(Debug, Serialize)]
pub struct BaselineStatus { identity: String, created: Option<String> }

/// Whether this app has ever changed this TV (a baseline exists), without touching the TV beyond getprop.
#[tauri::command]
pub async fn baseline_status(app: tauri::AppHandle, serial: String) -> CmdResult<BaselineStatus> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let identity = blocking(move || tvs_device::probe::quick_identity(&t, &serial)
        .ok_or_else(|| CmdError::Invalid("Could not read the TV's identity. Check that it is connected and authorized.".into()))).await?;
    let created = app.state::<AppState>().store.lock().unwrap().baseline(&identity)?
        .and_then(|v| v.get("created").and_then(|c| c.as_str()).map(str::to_string));
    Ok(BaselineStatus { identity, created })
}

#[tauri::command]
pub async fn restore_preview(app: tauri::AppHandle, serial: String, scope: Option<Vec<String>>) -> CmdResult<RestorePreview> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let app2 = app.clone();
    blocking(move || {
        let (snap, plan) = preview(&app2, &t, &serial, scope.as_deref())?;
        let studio_launcher = scope.is_none() && tvs_launcher::install::status(&t, &serial).map(|s| s.installed).unwrap_or(false);
        Ok(RestorePreview { plan, baseline_created: snap.created, studio_launcher })
    }).await
}

#[tauri::command]
pub async fn restore_apply(app: tauri::AppHandle, serial: String, scope: Option<Vec<String>>) -> CmdResult<ChangeReport> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let batch = app.state::<AppState>().store.lock().unwrap().settings()?.batch_size as usize;
    let app2 = app.clone();
    blocking(move || {
        let (snap, plan) = preview(&app2, &t, &serial, scope.as_deref())?;
        let mut report = apply_restore(&t, &serial, &plan, batch, &mut |p| { let _ = app2.emit("debloat-progress", &p); });
        if scope.is_none() && report.stopped.is_none() {
            let (changed, kept) = tvs_launcher::revert::remove_studio_launcher(&t, &serial, snap.home.as_deref(), std::time::Duration::from_millis(1500));
            report.changed.extend(changed);
            report.unchanged.extend(kept);
        }
        Ok(report)
    }).await
}

pub(crate) fn now_iso() -> String {
    // Seconds since the epoch, formatted as UTC without pulling in a date crate.
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_dates() {
        assert_eq!(super::civil_from_days(0), (1970, 1, 1));
        assert_eq!(super::civil_from_days(20_752), (2026, 10, 26));
    }
}
