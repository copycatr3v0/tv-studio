//! Launcher Builder and theme commands. Every change records the baseline first (so Revert can undo it).

use crate::debloat::{ensure_baseline, now_iso};
use crate::{blocking, serial_arg, transport, AppState, CmdError, CmdResult};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;
use tvs_launcher::assets::{normalize, safe_name, MAX_WALLPAPERS};
use tvs_launcher::install::{self, HomeCheck, LauncherError, LauncherStatus};
use tvs_launcher::schema::{self, Issue};
use tvs_launcher::theme::{self, ThemeBundle, ThemeError, ThemeReport};

impl From<LauncherError> for CmdError { fn from(e: LauncherError) -> Self { CmdError::Invalid(e.to_string()) } }
impl From<ThemeError> for CmdError {
    fn from(e: ThemeError) -> Self {
        match e {
            ThemeError::Invalid(issues) => CmdError::Invalid(format!("Nothing was sent to the TV. {}",
                issues.iter().map(|i| format!("{}: {}", i.key, i.message)).collect::<Vec<_>>().join(" | "))),
            other => CmdError::Invalid(other.to_string()),
        }
    }
}

/// The APK shipped inside the app (or `TVS_LAUNCHER_APK` for development builds).
pub(crate) fn bundled_apk(app: &tauri::AppHandle) -> Option<PathBuf> {
    if crate::demo_on(&app.state::<AppState>()) {
        // The simulated TV "installs" any file that looks like an APK; never a real APK on a real TV.
        let p = std::env::temp_dir().join("tv-studio-demo-launcher.apk");
        if !p.is_file() { std::fs::write(&p, b"PK\x03\x04 simulated studio launcher").ok()?; }
        return Some(p);
    }
    if let Some(p) = std::env::var_os("TVS_LAUNCHER_APK").map(PathBuf::from).filter(|p| p.is_file()) { return Some(p); }
    let dir = app.state::<AppState>().resource_dir.clone()?;
    [dir.join("resources").join("launcher").join("studio-home.apk"), dir.join("launcher").join("studio-home.apk")]
        .into_iter().find(|p| p.is_file())
}

#[derive(Debug, Serialize)]
pub struct LauncherInfo {
    status: LauncherStatus,
    apk_bundled: bool,
    schema_launcher_version: String,
    focused: Option<String>,
    /// Google TV launchers switched off on this TV (the opt-in switch).
    google_home_off: Vec<String>,
}

#[tauri::command]
pub async fn launcher_status(app: tauri::AppHandle, serial: String) -> CmdResult<LauncherInfo> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let apk_bundled = bundled_apk(&app).is_some();
    blocking(move || Ok(LauncherInfo {
        status: install::status(&t, &serial)?,
        focused: install::focused(&t, &serial),
        google_home_off: install::google_home_off(&t, &serial),
        apk_bundled,
        schema_launcher_version: schema::SCHEMA.launcher_version.clone(),
    })).await
}

#[tauri::command]
pub async fn install_launcher(app: tauri::AppHandle, serial: String) -> CmdResult<LauncherStatus> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Launcher install")?;
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let apk = bundled_apk(&app).ok_or_else(|| CmdError::Invalid("This build of the app does not include the launcher APK. Build it from launcher/ (see launcher/NOTICE.md) or use an official release.".into()))?;
    let app2 = app.clone();
    blocking(move || {
        let sdk = tvs_device::probe::quick_sdk(&t, &serial).ok_or_else(|| CmdError::Invalid("Could not read the TV's Android version.".into()))?;
        if sdk < install::MIN_SDK { return Err(LauncherError::OlderSdk(sdk).into()); }
        ensure_baseline(&app2, &t, &serial)?;
        Ok(install::install(&t, &serial, &apk, sdk)?)
    }).await
}

/// Make the Studio launcher Home and confirm it on screen. The stock launcher stays enabled.
#[tauri::command]
pub async fn make_launcher_home(app: tauri::AppHandle, serial: String) -> CmdResult<HomeCheck> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Making the launcher Home")?;
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let app2 = app.clone();
    blocking(move || {
        if !install::status(&t, &serial)?.installed { return Err(CmdError::Invalid("Install the launcher first.".into())); }
        ensure_baseline(&app2, &t, &serial)?;
        Ok(install::make_home(&t, &serial, std::time::Duration::from_millis(1500))?)
    }).await
}

/// Opt-in: switch Google TV's launcher off (the Studio launcher becomes the real Home, confirmed on screen, with
/// Google TV put straight back if not) or back on. Revert and hand-back always switch it back on.
#[tauri::command]
pub async fn set_google_home(app: tauri::AppHandle, serial: String, off: bool) -> CmdResult<HomeCheck> {
    let _busy = crate::begin_change(&app.state::<AppState>(), if off { "Switching Google TV off" } else { "Switching Google TV on" })?;
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    let app2 = app.clone();
    blocking(move || {
        let settle = std::time::Duration::from_millis(1500);
        if off {
            ensure_baseline(&app2, &t, &serial)?;
            return Ok(install::google_home_turn_off(&t, &serial, settle)?);
        }
        let back = install::google_home_turn_on(&t, &serial)?;
        Ok(HomeCheck { resolved: install::resolved_home(&t, &serial), focused: install::focused(&t, &serial), verified: true,
            note: Some(if back.is_empty() { "Google TV's launcher was already on.".into() } else { format!("Switched back on: {}.", back.join(", ")) }) })
    }).await
}

/// Apps the TV can show as tiles, for the editor's preview and dock picker.
#[tauri::command]
pub async fn launchable_apps(app: tauri::AppHandle, serial: String) -> CmdResult<Vec<String>> {
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    blocking(move || Ok(tvs_device::inventory::launchable_packages(&t, &serial)?)).await
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WallpaperIn { name: String, data_base64: String }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeIn { config: serde_json::Value, #[serde(default)] wallpapers: Vec<WallpaperIn>, #[serde(default)] captions: Option<String>, #[serde(default)] layout: Option<serde_json::Value>, #[serde(default)] videos: Vec<VideoIn>, #[serde(default)] fonts: Vec<VideoIn>, #[serde(default)] tiles: Vec<VideoIn>, #[serde(default)] profiles: Vec<ProfileIn> }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileIn { id: String, layout: serde_json::Value }

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoIn { name: String, data_base64: String }

#[derive(Debug, Serialize)]
pub struct ThemeCheck { issues: Vec<Issue> }

/// Validate without touching the TV (the editor calls this on every change).
#[tauri::command]
pub fn validate_theme(config: serde_json::Value) -> ThemeCheck { ThemeCheck { issues: schema::validate(&config) } }

#[derive(Debug, Serialize)]
pub struct ImportResult { config: serde_json::Map<String, serde_json::Value>, issues: Vec<Issue> }

#[tauri::command]
pub fn import_theme(json_text: String) -> CmdResult<ImportResult> {
    if json_text.len() > 256 * 1024 { return Err(CmdError::Invalid("That file is too large to be a theme config.".into())); }
    let v: serde_json::Value = serde_json::from_str(&json_text).map_err(|e| CmdError::Invalid(format!("Not valid JSON: {e}")))?;
    let (config, issues) = schema::import(&v);
    Ok(ImportResult { config, issues })
}

pub(crate) fn bundle_from(input: ThemeIn) -> CmdResult<ThemeBundle> {
    if input.wallpapers.len() > MAX_WALLPAPERS { return Err(CmdError::Invalid("A theme can carry at most 30 wallpapers.".into())); }
    let mut wallpapers = vec![];
    let mut used = std::collections::BTreeSet::new();
    for (i, w) in input.wallpapers.into_iter().enumerate() {
        let bytes = base64::engine::general_purpose::STANDARD.decode(w.data_base64.as_bytes())
            .map_err(|_| CmdError::Invalid(format!("{}: could not read the image data", w.name)))?;
        let (out, ext) = normalize(&w.name, &bytes).map_err(|e| CmdError::Invalid(e.to_string()))?;
        let mut name = safe_name(&w.name, i, ext);
        if !used.insert(name.clone()) { name = format!("{}-{i}.{ext}", name.trim_end_matches(&format!(".{ext}"))); used.insert(name.clone()); }
        wallpapers.push((name, out));
    }
    let captions = input.captions.filter(|c| !c.trim().is_empty() && c.len() <= 16 * 1024);
    let mut videos = vec![];
    if input.videos.len() > tvs_launcher::theme::MAX_VIDEOS { return Err(CmdError::Invalid("A theme can carry at most 4 videos.".into())); }
    for v in input.videos {
        // Base64 of the size limit is the most a legitimate video can take; refuse anything larger before decoding.
        if v.data_base64.len() > tvs_launcher::theme::MAX_VIDEO_BYTES / 3 * 4 + 8 { return Err(CmdError::Invalid(format!("{}: the video is too large (60 MB at most).", v.name))); }
        let bytes = base64::engine::general_purpose::STANDARD.decode(v.data_base64.as_bytes())
            .map_err(|_| CmdError::Invalid(format!("{}: could not read the video data", v.name)))?;
        videos.push((v.name, bytes));
    }
    let mut fonts = vec![];
    if input.fonts.len() > tvs_launcher::theme::MAX_FONTS { return Err(CmdError::Invalid("A theme can carry at most 6 font files.".into())); }
    for f in input.fonts {
        if f.data_base64.len() > tvs_launcher::theme::MAX_FONT_BYTES / 3 * 4 + 8 { return Err(CmdError::Invalid(format!("{}: the font file is too large (4 MB at most).", f.name))); }
        let bytes = base64::engine::general_purpose::STANDARD.decode(f.data_base64.as_bytes())
            .map_err(|_| CmdError::Invalid(format!("{}: could not read the font data", f.name)))?;
        fonts.push((f.name, bytes));
    }
    // Pictures for single apps: re-encoded like wallpapers (metadata and payloads dropped), stills only.
    if input.tiles.len() > tvs_launcher::theme::MAX_TILES { return Err(CmdError::Invalid("A theme can carry at most 60 app pictures.".into())); }
    let mut tiles = vec![];
    for t in input.tiles {
        if !tvs_launcher::layout::valid_tile_image(&t.name) { return Err(CmdError::Invalid(format!("{}: not a safe picture name", t.name))); }
        let bytes = base64::engine::general_purpose::STANDARD.decode(t.data_base64.as_bytes())
            .map_err(|_| CmdError::Invalid(format!("{}: could not read the picture data", t.name)))?;
        let (out, ext) = normalize(&t.name, &bytes).map_err(|e| CmdError::Invalid(e.to_string()))?;
        if ext != "jpg" { return Err(CmdError::Invalid(format!("{}: an app picture must be a still picture", t.name))); }
        if out.len() > tvs_launcher::theme::MAX_TILE_BYTES { return Err(CmdError::Invalid(format!("{}: the picture is too large", t.name))); }
        tiles.push((t.name, out));
    }
    if input.profiles.len() > 3 { return Err(CmdError::Invalid("At most 3 more profiles.".into())); }
    let mut profiles = vec![];
    for p in input.profiles {
        if !tvs_launcher::layout::valid_profile_id(&p.id) { return Err(CmdError::Invalid(format!("{}: not a safe profile id", p.id))); }
        profiles.push((p.id, p.layout));
    }
    Ok(ThemeBundle { config: input.config, wallpapers, captions, layout: input.layout, videos, fonts, tiles, profiles })
}

#[tauri::command]
pub async fn deploy_theme(app: tauri::AppHandle, serial: String, theme: ThemeIn) -> CmdResult<ThemeReport> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Sending the theme")?;
    let serial = serial_arg(&serial)?;
    let bundle = bundle_from(theme)?;
    let issues = schema::validate(&bundle.config);
    if !issues.is_empty() { return Err(ThemeError::Invalid(issues).into()); }
    let t = transport(&app.state::<AppState>())?;
    let app2 = app.clone();
    blocking(move || {
        if !install::status(&t, &serial)?.installed { return Err(ThemeError::NoLauncher.into()); }
        ensure_baseline(&app2, &t, &serial)?;
        Ok(theme::deploy(&t, &serial, &bundle, &revision())?)
    }).await
}

/// Make the TV forget its PIN (locked apps and protected profiles ask for a new one).
#[tauri::command]
pub async fn reset_tv_pin(app: tauri::AppHandle, serial: String) -> CmdResult<()> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "PIN reset")?;
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    blocking(move || { theme::reset_pin(&t, &serial)?; Ok(()) }).await
}

#[tauri::command]
pub async fn reset_theme(app: tauri::AppHandle, serial: String) -> CmdResult<ThemeReport> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "Theme reset")?;
    let serial = serial_arg(&serial)?;
    let t = transport(&app.state::<AppState>())?;
    blocking(move || Ok(theme::reset(&t, &serial, &revision())?)).await
}

const THEME_PREFIX: &str = "theme:";
const MAX_SAVED_THEME_BYTES: usize = 40 * 1024 * 1024;

#[tauri::command]
pub fn save_theme_version(state: tauri::State<'_, AppState>, name: String, theme: serde_json::Value) -> CmdResult<i64> {
    let name = name.trim();
    if name.is_empty() || name.len() > 80 || name.chars().any(char::is_control) { return Err(CmdError::Invalid("Give the theme a name (up to 80 characters).".into())); }
    if serde_json::to_vec(&theme).map(|b| b.len()).unwrap_or(usize::MAX) > MAX_SAVED_THEME_BYTES {
        return Err(CmdError::Invalid("This theme is too large to save (over 40 MB of wallpapers).".into()));
    }
    let issues = schema::validate(theme.get("config").unwrap_or(&serde_json::Value::Null));
    if !issues.is_empty() { return Err(ThemeError::Invalid(issues).into()); }
    Ok(state.store.lock().unwrap().save_profile(&format!("{THEME_PREFIX}{name}"), &theme)?)
}

#[derive(Debug, Serialize)]
pub struct SavedTheme { name: String, version: i64, created: String }

#[tauri::command]
pub fn list_theme_versions(state: tauri::State<'_, AppState>) -> CmdResult<Vec<SavedTheme>> {
    Ok(state.store.lock().unwrap().list_profiles(THEME_PREFIX)?.into_iter()
        .map(|p| SavedTheme { name: p.name.trim_start_matches(THEME_PREFIX).to_string(), version: p.version, created: p.created }).collect())
}

#[tauri::command]
pub fn load_theme_version(state: tauri::State<'_, AppState>, name: String, version: i64) -> CmdResult<serde_json::Value> {
    state.store.lock().unwrap().get_profile(&format!("{THEME_PREFIX}{name}"), version)?
        .ok_or_else(|| CmdError::Invalid("That saved version no longer exists.".into()))
}

fn revision() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{}-{nanos:x}", now_iso())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_dedupes_names_and_rejects_garbage() {
        let png = {
            let img = image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]));
            let mut b = Vec::new();
            image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png).unwrap();
            base64::engine::general_purpose::STANDARD.encode(b)
        };
        let input = ThemeIn { config: serde_json::json!({}), captions: None, layout: None, videos: vec![], fonts: vec![], tiles: vec![], profiles: vec![], wallpapers: vec![
            WallpaperIn { name: "Sky.png".into(), data_base64: png.clone() },
            WallpaperIn { name: "sky.PNG".into(), data_base64: png },
        ] };
        let b = bundle_from(input).unwrap();
        assert_eq!(b.wallpapers.iter().map(|w| w.0.as_str()).collect::<Vec<_>>(), vec!["sky.jpg", "sky-1.jpg"]);
        let bad = ThemeIn { config: serde_json::json!({}), captions: None, layout: None, videos: vec![], fonts: vec![], tiles: vec![], profiles: vec![], wallpapers: vec![WallpaperIn { name: "x.png".into(), data_base64: "!!".into() }] };
        assert!(bundle_from(bad).is_err());
    }
}
