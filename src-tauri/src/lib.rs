//! Tauri entry point. Commands here are thin: they validate input, call the core crates and map errors
//! into a serializable shape. No device logic lives in this crate.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;
use tvs_store::{AppSettings, Store};
use std::time::Duration;
use tvs_core::Serial;
use tvs_device::{advice, probe_port, DeviceInfo, Endpoint, PortState, DEFAULT_PORT};
use tvs_transport::{find_adb, AdbExeTransport, DeviceEntry, DeviceState, MdnsService, PairingCode, Transport, TransportError};

mod debloat;
mod launcher_cmds;
mod deploy_cmds;
mod insight_cmds;
mod adb_setup;

pub struct AppState {
    pub(crate) store: Mutex<Store>,
    db_path: PathBuf,
    pub(crate) data_dir: PathBuf,
    pub(crate) resource_dir: Option<PathBuf>,
    pub(crate) cancel_deploy: std::sync::atomic::AtomicBool,
    /// The simulated TV world, created the first time demo mode is used and kept for the whole session
    /// so changes persist between screens (and Revert to original has something to undo).
    demo: OnceLock<Arc<tvs_transport::mock::MockTransport>>,
    /// What is changing a TV right now. One change at a time, so Revert can't interleave with Apply Everything.
    busy: Arc<Mutex<Option<&'static str>>>,
}

/// Held for the whole of a command that changes a TV; released on drop (also on error or panic).
pub(crate) struct BusyGuard(Arc<Mutex<Option<&'static str>>>);
impl Drop for BusyGuard {
    fn drop(&mut self) { if let Ok(mut b) = self.0.lock() { *b = None; } }
}

/// Start a change, or refuse with what is already running. Read-only commands don't take this.
pub(crate) fn begin_change(state: &AppState, what: &'static str) -> CmdResult<BusyGuard> {
    let mut b = state.busy.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(running) = *b {
        return Err(CmdError::Invalid(format!("{running} is still running. Wait for it to finish (or cancel it), then try again; nothing was changed.")));
    }
    *b = Some(what);
    Ok(BusyGuard(state.busy.clone()))
}

/// Whatever the commands talk to: adb.exe, or the simulated TV in demo mode.
pub(crate) type Tx = Arc<dyn Transport>;

pub(crate) fn demo_on(state: &AppState) -> bool {
    std::env::var_os("TVS_DEMO").is_some_and(|v| v == "1") || state.store.lock().unwrap().settings().map(|s| s.demo_mode).unwrap_or(false)
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CmdError {
    Store(String),
    Transport(TransportError),
    Invalid(String),
}
impl From<tvs_store::StoreError> for CmdError { fn from(e: tvs_store::StoreError) -> Self { CmdError::Store(e.to_string()) } }
impl From<tvs_device::EndpointError> for CmdError { fn from(e: tvs_device::EndpointError) -> Self { CmdError::Invalid(e.to_string()) } }
impl From<TransportError> for CmdError { fn from(e: TransportError) -> Self { CmdError::Transport(e) } }
pub(crate) type CmdResult<T> = Result<T, CmdError>;

#[derive(Debug, Serialize)]
pub struct AdbInfo { path: String, version: Option<String>, source: &'static str }

#[derive(Debug, Serialize)]
pub struct AppInfo { version: &'static str, db_path: String, schema_version: i64, adb: Option<AdbInfo> }

fn resolve_adb(state: &AppState, settings: &AppSettings) -> Option<(PathBuf, &'static str)> {
    if !settings.adb_path.is_empty() {
        let p = PathBuf::from(&settings.adb_path);
        return p.is_file().then_some((p, "custom"));
    }
    let bundled = state.resource_dir.as_deref().and_then(|d| find_adb(Some(d)).filter(|p| p.starts_with(d)));
    if let Some(p) = bundled { return Some((p, "bundled")); }
    if let Some(p) = adb_setup::downloaded_adb(&state.data_dir) { return Some((p, "downloaded")); }
    find_adb(None).map(|p| (p, "system"))
}

pub(crate) fn transport(state: &AppState) -> CmdResult<Tx> {
    if demo_on(state) {
        let world = state.demo.get_or_init(|| Arc::new(tvs_demo::demo_transport()));
        return Ok(world.clone());
    }
    let settings = state.store.lock().unwrap().settings()?;
    resolve_adb(state, &settings).map(|(p, _)| Arc::new(AdbExeTransport::new(p)) as Tx).ok_or(CmdError::Transport(TransportError::AdbMissing))
}

#[tauri::command]
fn app_info(state: tauri::State<'_, AppState>) -> CmdResult<AppInfo> {
    let (settings, schema_version) = {
        let store = state.store.lock().unwrap();
        (store.settings()?, store.schema_version()?)
    };
    if demo_on(&state) {
        let adb = Some(AdbInfo { path: "Simulated TCL Google TV".into(), version: Some("demo".into()), source: "demo" });
        return Ok(AppInfo { version: env!("CARGO_PKG_VERSION"), db_path: state.db_path.display().to_string(), schema_version, adb });
    }
    let adb = resolve_adb(&state, &settings).map(|(p, source)| AdbInfo {
        version: AdbExeTransport::new(p.clone()).version().ok(),
        path: p.display().to_string(),
        source,
    });
    Ok(AppInfo { version: env!("CARGO_PKG_VERSION"), db_path: state.db_path.display().to_string(), schema_version, adb })
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> CmdResult<AppSettings> {
    Ok(state.store.lock().unwrap().settings()?)
}

#[tauri::command]
fn save_settings(state: tauri::State<'_, AppState>, settings: AppSettings) -> CmdResult<AppSettings> {
    if !settings.adb_path.trim().is_empty() && !PathBuf::from(settings.adb_path.trim()).is_file() {
        return Err(CmdError::Invalid(format!("No file at {}", settings.adb_path.trim())));
    }
    Ok(state.store.lock().unwrap().save_settings(&settings)?)
}

/// Blocking adb call, run off the main thread.
#[tauri::command]
async fn list_devices(state: tauri::State<'_, AppState>) -> CmdResult<Vec<DeviceEntry>> {
    let t = transport(&state)?;
    tauri::async_runtime::spawn_blocking(move || t.devices())
        .await
        .map_err(|e| CmdError::Invalid(e.to_string()))?
        .map_err(CmdError::from)
}

pub(crate) async fn blocking<T: Send + 'static>(f: impl FnOnce() -> CmdResult<T> + Send + 'static) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| CmdError::Invalid(e.to_string()))?
}

pub(crate) fn serial_arg(s: &str) -> CmdResult<Serial> { Serial::new(s).map_err(|e| CmdError::Invalid(e.to_string())) }

#[derive(Debug, Serialize)]
pub struct Diagnosis { endpoint: String, port_state: PortState, advice: &'static str }

/// Plain TCP check, no adb: tells "wrong IP / asleep" apart from "debugging off" apart from "listening".
/// Also used at the end of a session to prove the port is closed (only Refused counts).
#[tauri::command]
async fn diagnose(state: tauri::State<'_, AppState>, address: String) -> CmdResult<Diagnosis> {
    let ep = Endpoint::parse(&address, DEFAULT_PORT)?;
    if demo_on(&state) {
        let st = demo_port_state(&ep);
        return Ok(Diagnosis { endpoint: format!("{}:{}", ep.host, ep.port), advice: advice(&st, ep.port), port_state: st });
    }
    blocking(move || {
        let st = probe_port(&ep, Duration::from_secs(3));
        Ok(Diagnosis { endpoint: format!("{}:{}", ep.host, ep.port), advice: advice(&st, ep.port), port_state: st })
    }).await
}

/// In demo mode the simulated TV answers on its address; anything else looks like a TV that is off.
fn demo_port_state(ep: &Endpoint) -> PortState {
    if ep.host == tvs_demo::DEMO_HOST && [5555, 37123, 41234].contains(&ep.port) { PortState::Open } else { PortState::Timeout }
}

#[derive(Debug, Serialize)]
pub struct ConnectResult { serial: Serial, state: Option<DeviceState>, model: Option<String> }

/// `adb connect`, preceded by a TCP check so a dead address fails in 3 s with advice instead of adb's long timeout.
#[tauri::command]
async fn connect_tv(state: tauri::State<'_, AppState>, address: String) -> CmdResult<ConnectResult> {
    let ep = Endpoint::parse(&address, DEFAULT_PORT)?;
    let t = transport(&state)?;
    let demo = demo_on(&state);
    blocking(move || {
        let st = if demo { demo_port_state(&ep) } else { probe_port(&ep, Duration::from_secs(3)) };
        if st != PortState::Open {
            return Err(CmdError::Invalid(advice(&st, ep.port).to_string()));
        }
        let serial = ep.serial();
        t.connect(&serial)?;
        let entry = t.devices()?.into_iter().find(|d| d.serial == serial);
        Ok(ConnectResult { state: entry.as_ref().map(|d| d.state), model: entry.and_then(|d| d.model), serial })
    }).await
}

/// Android 11+ Wireless debugging: pair with the code and pairing port shown on the TV.
#[tauri::command]
async fn pair_tv(state: tauri::State<'_, AppState>, address: String, code: String) -> CmdResult<()> {
    let ep = Endpoint::parse(&address, 0)?;
    if ep.port == 0 {
        return Err(CmdError::Invalid("Add the pairing port shown on the TV, e.g. 192.168.1.50:41234".into()));
    }
    let code = PairingCode::new(&code).map_err(|e| CmdError::Invalid(e.to_string()))?;
    let t = transport(&state)?;
    blocking(move || Ok(t.pair(&ep.serial(), &code)?)).await
}

#[tauri::command]
async fn discover_wireless(state: tauri::State<'_, AppState>) -> CmdResult<Vec<MdnsService>> {
    let t = transport(&state)?;
    blocking(move || Ok(t.mdns_services()?)).await
}

#[tauri::command]
async fn disconnect_tv(state: tauri::State<'_, AppState>, serial: String) -> CmdResult<()> {
    let serial = serial_arg(&serial)?;
    let t = transport(&state)?;
    blocking(move || Ok(t.disconnect(&serial)?)).await
}

/// Read-only inspection; the device is remembered by its build identity, not its (DHCP) address.
#[tauri::command]
async fn probe_device(app: tauri::AppHandle, serial: String) -> CmdResult<DeviceInfo> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let info = blocking(move || Ok(tvs_device::probe(&t, &serial))).await?;
    if info.sdk.is_some() {
        let st = app.state::<AppState>();
        let store = st.store.lock().unwrap();
        store.upsert_device(&info.identity(), info.serial.as_str(), info.brand.as_deref(), info.model.as_deref(), info.sdk.map(i64::from))?;
    }
    Ok(info)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("studio.db");
            let store = Store::open(&db_path)?;
            let resource_dir = app.path().resource_dir().ok();
            app.manage(AppState { store: Mutex::new(store), db_path, data_dir, resource_dir, cancel_deploy: std::sync::atomic::AtomicBool::new(false), demo: OnceLock::new(), busy: Arc::new(Mutex::new(None)) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info, get_settings, save_settings, list_devices, diagnose, connect_tv, pair_tv,
            discover_wireless, disconnect_tv, probe_device, debloat::inventory, debloat::evaluate_debloat,
            debloat::apply_debloat, debloat::restore_preview, debloat::restore_apply, debloat::baseline_status,
            launcher_cmds::launcher_status, launcher_cmds::install_launcher, launcher_cmds::make_launcher_home, launcher_cmds::set_google_home,
            launcher_cmds::validate_theme, launcher_cmds::import_theme, launcher_cmds::deploy_theme, launcher_cmds::reset_theme, launcher_cmds::reset_tv_pin,
            launcher_cmds::save_theme_version, launcher_cmds::list_theme_versions, launcher_cmds::load_theme_version, launcher_cmds::launchable_apps,
            deploy_cmds::deploy_plan, deploy_cmds::deploy_run, deploy_cmds::deploy_cancel, deploy_cmds::deploy_interrupted,
            deploy_cmds::deploy_resume, deploy_cmds::deploy_discard, deploy_cmds::deploy_history, deploy_cmds::undo_deployment,
            deploy_cmds::save_deploy_profile, deploy_cmds::list_deploy_profiles, deploy_cmds::load_deploy_profile, deploy_cmds::import_deploy_profile,
            insight_cmds::bench_run, insight_cmds::bench_list, insight_cmds::bench_compare, insight_cmds::privacy_audit,
            insight_cmds::drift_check, insight_cmds::diagnostics_bundle, adb_setup::install_adb, adb_setup::open_platform_tools_terms
        ])
        .run(tauri::generate_context!())
        .expect("error while running Android TV Customizer Studio");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState { store: Mutex::new(Store::open_in_memory().unwrap()), db_path: PathBuf::new(), data_dir: PathBuf::new(), resource_dir: None,
            cancel_deploy: std::sync::atomic::AtomicBool::new(false), demo: OnceLock::new(), busy: Arc::new(Mutex::new(None)) }
    }

    #[test]
    fn one_change_at_a_time() {
        let st = state();
        let g = begin_change(&st, "Apply Everything").unwrap();
        let err = begin_change(&st, "Restore").err().unwrap();
        assert!(matches!(&err, CmdError::Invalid(m) if m.starts_with("Apply Everything is still running")));
        drop(g);
        assert!(begin_change(&st, "Restore").is_ok(), "released when the first change ends");
    }

    #[test]
    fn demo_mode_uses_one_persistent_simulated_tv() {
        let st = state();
        st.store.lock().unwrap().save_settings(&AppSettings { demo_mode: true, ..Default::default() }).unwrap();
        let t1 = transport(&st).unwrap();
        t1.connect(&Serial::new(tvs_demo::DEMO_SERIAL).unwrap()).unwrap();
        let t2 = transport(&st).unwrap();
        assert_eq!(t2.devices().unwrap().len(), 1, "the connection survives between commands");
        assert_eq!(demo_port_state(&Endpoint::parse("192.168.1.50", DEFAULT_PORT).unwrap()), PortState::Open);
        assert_eq!(demo_port_state(&Endpoint::parse("192.168.1.99", DEFAULT_PORT).unwrap()), PortState::Timeout);
    }
}
