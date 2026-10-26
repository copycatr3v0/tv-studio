//! Tauri entry point. Commands here are thin: they validate input, call the core crates and map errors
//! into a serializable shape. No device logic lives in this crate.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use tvs_store::{AppSettings, Store};
use tvs_transport::{find_adb, AdbExeTransport, DeviceEntry, Transport, TransportError};

pub struct AppState {
    store: Mutex<Store>,
    db_path: PathBuf,
    resource_dir: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CmdError {
    Store(String),
    Transport(TransportError),
    Invalid(String),
}
impl From<tvs_store::StoreError> for CmdError { fn from(e: tvs_store::StoreError) -> Self { CmdError::Store(e.to_string()) } }
impl From<TransportError> for CmdError { fn from(e: TransportError) -> Self { CmdError::Transport(e) } }
type CmdResult<T> = Result<T, CmdError>;

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
    find_adb(None).map(|p| (p, "system"))
}

fn transport(state: &AppState) -> CmdResult<AdbExeTransport> {
    let settings = state.store.lock().unwrap().settings()?;
    resolve_adb(state, &settings).map(|(p, _)| AdbExeTransport::new(p)).ok_or(CmdError::Transport(TransportError::AdbMissing))
}

#[tauri::command]
fn app_info(state: tauri::State<'_, AppState>) -> CmdResult<AppInfo> {
    let (settings, schema_version) = {
        let store = state.store.lock().unwrap();
        (store.settings()?, store.schema_version()?)
    };
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let db_path = data_dir.join("studio.db");
            let store = Store::open(&db_path)?;
            let resource_dir = app.path().resource_dir().ok();
            app.manage(AppState { store: Mutex::new(store), db_path, resource_dir });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_info, get_settings, save_settings, list_devices])
        .run(tauri::generate_context!())
        .expect("error while running Android TV Customizer Studio");
}
