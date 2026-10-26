//! Benchmarks, privacy audit, firmware-drift check and the diagnostics export.

use crate::debloat::now_iso;
use crate::deploy_cmds::DeployRequest;
use crate::launcher_cmds::{bundle_from, bundled_apk};
use crate::{blocking, serial_arg, transport, AppState, CmdError, CmdResult};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tvs_deploy::DeployInput;
use tvs_insight::bench::{self, BenchConfig, BenchRun, Metric, Pacer};

struct AppPacer { app: tauri::AppHandle }
impl Pacer for AppPacer {
    fn pause(&mut self, d: Duration) { std::thread::sleep(d); }
    fn progress(&mut self, what: &str, done: u32, total: u32) {
        let _ = self.app.emit("bench-progress", serde_json::json!({ "what": what, "done": done, "total": total }));
    }
    fn cancelled(&mut self) -> bool { self.app.state::<AppState>().cancel_deploy.load(Ordering::SeqCst) }
}

fn identity(t: &dyn tvs_transport::Transport, s: &tvs_core::Serial) -> CmdResult<String> {
    tvs_device::probe::quick_identity(t, s).ok_or_else(|| CmdError::Invalid("Could not read the TV's identity.".into()))
}

#[derive(Debug, Serialize)]
pub struct BenchSaved { id: i64, run: BenchRun }

/// Measure the launcher that currently answers Home. Changes nothing on the TV except which app is in front.
#[tauri::command]
pub async fn bench_run(app: tauri::AppHandle, serial: String, label: String, quick: bool) -> CmdResult<BenchSaved> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "A benchmark")?;
    let serial = serial_arg(&serial)?;
    let label = label.trim().chars().take(40).collect::<String>();
    if label.is_empty() { return Err(CmdError::Invalid("Give the run a label, e.g. “before”.".into())); }
    let t = transport(&app.state::<AppState>())?;
    let app2 = app.clone();
    blocking(move || {
        let id = identity(&t, &serial)?;
        app2.state::<AppState>().cancel_deploy.store(false, Ordering::SeqCst);
        let cfg = if quick { BenchConfig { keys: 12, cold_starts: 3, home_returns: 3, idle_seconds: 5, ..Default::default() } } else { BenchConfig::default() };
        let run = bench::run(&t, &serial, &cfg, &now_iso(), &mut AppPacer { app: app2.clone() })?;
        let st = app2.state::<AppState>();
        let store = st.store.lock().unwrap();
        store.upsert_device(&id, serial.as_str(), None, None, None)?;
        let saved = store.save_bench(&id, &label, &serde_json::to_value(&run).map_err(|e| CmdError::Invalid(e.to_string()))?)?;
        Ok(BenchSaved { id: saved, run })
    }).await
}

#[derive(Debug, Serialize)]
pub struct BenchListItem { id: i64, label: String, created: String, launcher: Option<String> }

#[tauri::command]
pub async fn bench_list(app: tauri::AppHandle, serial: String) -> CmdResult<Vec<BenchListItem>> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let id = blocking(move || identity(&t, &serial)).await?;
    let st = app.state::<AppState>();
    let store = st.store.lock().unwrap();
    store.list_bench(&id)?.into_iter().map(|m| {
        let launcher = store.get_bench(m.id)?.and_then(|(_, b)| b.get("launcher").and_then(|l| l.as_str()).map(str::to_string));
        Ok(BenchListItem { id: m.id, label: m.label, created: m.created, launcher })
    }).collect()
}

#[derive(Debug, Serialize)]
pub struct BenchComparison { before: BenchRun, after: BenchRun, metrics: Vec<Metric>, same_launcher: bool }

#[tauri::command]
pub fn bench_compare(state: tauri::State<'_, AppState>, before_id: i64, after_id: i64) -> CmdResult<BenchComparison> {
    let store = state.store.lock().unwrap();
    let load = |id| -> CmdResult<(String, BenchRun)> {
        let (fp, body) = store.get_bench(id)?.ok_or_else(|| CmdError::Invalid("That benchmark run no longer exists.".into()))?;
        Ok((fp, serde_json::from_value(body).map_err(|e| CmdError::Invalid(e.to_string()))?))
    };
    let (fa, before) = load(before_id)?;
    let (fb, after) = load(after_id)?;
    if fa != fb { return Err(CmdError::Invalid("Those runs are from different TVs; compare runs from the same TV.".into())); }
    let metrics = bench::comparison(&before, &after);
    Ok(BenchComparison { same_launcher: before.launcher == after.launcher, before, after, metrics })
}

#[tauri::command]
pub async fn privacy_audit(app: tauri::AppHandle, serial: String) -> CmdResult<tvs_insight::privacy::Audit> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    blocking(move || Ok(tvs_insight::privacy::audit(&t, &serial)?)).await
}

#[derive(Debug, Serialize)]
pub struct DriftResult { deployment_id: i64, deployment_started: String, report: tvs_deploy::drift::DriftReport, request: DeployRequest }

/// Compare the TV with the last deployment that changed it. None when there is nothing to compare with.
#[tauri::command]
pub async fn drift_check(app: tauri::AppHandle, serial: String) -> CmdResult<Option<DriftResult>> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let app2 = app.clone();
    blocking(move || {
        let id = identity(&t, &serial)?;
        let (row, baseline) = {
            let st = app2.state::<AppState>();
            let store = st.store.lock().unwrap();
            (store.last_applied_deployment(&id)?, store.baseline(&id)?)
        };
        let Some(row) = row else { return Ok(None) };
        let request: DeployRequest = serde_json::from_value(row.profile.clone().unwrap_or_default())
            .map_err(|e| CmdError::Invalid(format!("Saved deployment unreadable: {e}")))?;
        let theme = match (&request.theme, request.profile.apply_theme) { (Some(t), true) => Some(bundle_from(t.clone())?), _ => None };
        let batch = app2.state::<AppState>().store.lock().unwrap().settings()?.batch_size as usize;
        let input = DeployInput { profile: request.profile.clone(), theme, apk: bundled_apk(&app2), batch_size: batch, settle: Duration::from_millis(1500) };
        let baseline: Option<tvs_recovery::Snapshot> = baseline.and_then(|b| serde_json::from_value(b).ok());
        let report = tvs_deploy::drift::check(&t, &serial, &input, baseline.as_ref())?;
        Ok(Some(DriftResult { deployment_id: row.id, deployment_started: row.started, report, request }))
    }).await
}

/// A JSON bundle for troubleshooting. IP addresses and the ADB serial are replaced; nothing is sent anywhere.
#[tauri::command]
pub async fn diagnostics_bundle(app: tauri::AppHandle, serial: Option<String>) -> CmdResult<serde_json::Value> {
    let settings = app.state::<AppState>().store.lock().unwrap().settings()?;
    let mut out = serde_json::json!({
        "app_version": env!("CARGO_PKG_VERSION"),
        "created": now_iso(),
        "settings": { "ui_mode": settings.ui_mode, "batch_size": settings.batch_size, "default_dry_run": settings.default_dry_run, "custom_adb": !settings.adb_path.is_empty() },
        "launcher_apk_bundled": bundled_apk(&app).is_some(),
    });
    if let Some(serial) = serial {
        let serial = serial_arg(&serial)?;
        let t = transport(&app.state::<AppState>())?;
        let s2 = serial.clone();
        let info = blocking(move || Ok(tvs_device::probe(&t, &s2))).await?;
        let id = info.identity();
        let st = app.state::<AppState>();
        let store = st.store.lock().unwrap();
        let deployments: Vec<serde_json::Value> = store.deployments(&id, 10)?.into_iter().map(|d| {
            let journal = store.journal_rows(d.id).unwrap_or_default();
            serde_json::json!({ "id": d.id, "started": d.started, "finished": d.finished, "status": d.status,
                "profile": d.profile.as_ref().and_then(|p| p.get("profile")).cloned(), "journal": journal })
        }).collect();
        let mut device = serde_json::to_value(&info).map_err(|e| CmdError::Invalid(e.to_string()))?;
        device["serial"] = serde_json::Value::String("<redacted>".into());
        out["device"] = device;
        out["baseline_recorded"] = serde_json::Value::Bool(store.baseline(&id)?.is_some());
        out["deployments"] = serde_json::Value::Array(deployments);
        out["bench_runs"] = serde_json::to_value(store.list_bench(&id)?).unwrap_or_default();
        let text = redact(&out.to_string(), serial.as_str());
        out = serde_json::from_str(&text).map_err(|e| CmdError::Invalid(e.to_string()))?;
    }
    Ok(out)
}

/// Replace the serial and anything that looks like an IPv4/IPv6 or MAC address.
pub(crate) fn redact(text: &str, serial: &str) -> String {
    let mac = regex::Regex::new(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b").unwrap();
    let v6 = regex::Regex::new(r"\b(?:[0-9A-Fa-f]{1,4}:){2,7}(?::|[0-9A-Fa-f]{1,4})\b|\b(?:[0-9A-Fa-f]{1,4}:)+:(?:[0-9A-Fa-f]{1,4}:?)*").unwrap();
    let v4 = regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}(?::\d+)?\b").unwrap();
    let t = text.replace(serial, "<tv>");
    let t = mac.replace_all(&t, "<mac>");
    let t = v6.replace_all(&t, "<ip>");
    v4.replace_all(&t, "<ip>").into_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn redacts_addresses_and_serial() {
        let r = super::redact(r#"{"a":"192.168.1.50:5555","b":"adb-ABC._adb-tls-connect._tcp","c":"10.0.0.2","v":"Android 12"}"#, "adb-ABC._adb-tls-connect._tcp");
        assert!(!r.contains("192.168") && !r.contains("10.0.0.2") && !r.contains("adb-ABC"));
        assert!(r.contains("Android 12"));
        let r = super::redact("wlan a4:5e:60:12:34:56 fe80::1c2a:3bff:fe4d:5e6f 2001:db8:0:0:1:0:0:1 at 2026-10-26T12:00:00Z", "x:5555");
        assert!(!r.contains("a4:5e") && !r.contains("fe80") && !r.contains("2001:db8"), "{r}");
        assert!(r.contains("2026-10-26T12:00:00Z"), "timestamps survive: {r}");
    }
}
