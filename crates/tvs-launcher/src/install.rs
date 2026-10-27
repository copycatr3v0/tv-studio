//! Install, hand Home to, hand Home back from, and uninstall the Studio launcher.
//! Home is only ever "done" when the TV shows it after a real HOME key press: `set-home-activity`
//! printing Success proves nothing (some Google TV builds ignore it).

use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use tvs_core::{Component, PackageName, Serial};
use tvs_transport::{ShellCommand, Transport, TransportError};

pub const STUDIO_PKG: &str = "studio.tvcustomizer.home";
/// The fork keeps the upstream Java namespace, so the activity class is com.gathin.tvhome.MainActivity.
pub const STUDIO_HOME: &str = "studio.tvcustomizer.home/com.gathin.tvhome.MainActivity";
pub const MIN_SDK: u32 = 26;
const STAGED_APK: &str = "/data/local/tmp/tvs-studio-home.apk";

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum LauncherError {
    #[error("Android {0} is too old: the launcher needs Android 8.0 (API 26). ADB-only mode still works.")]
    OlderSdk(u32),
    #[error("This TV only accepts apps signed by its manufacturer (INSTALL_FAILED_VERIFICATION_FAILURE). The launcher cannot be installed here; debloating still works.")]
    VerificationRefused,
    #[error("Not enough free storage on the TV to install the launcher.")]
    Storage,
    #[error("A Studio launcher signed with a different key is already installed. Revert (which uninstalls it) and install again.")]
    SignatureMismatch,
    #[error("The TV refused the install: {0}")]
    Refused(String),
    #[error("The install reported success but the launcher is not on the TV.")]
    NotVerified,
    #[error("{0}")]
    Transport(#[from] TransportError),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LauncherStatus {
    pub installed: bool,
    pub enabled: bool,
    pub version: Option<String>,
    /// Default Home as resolved by the package manager.
    pub resolved_home: Option<String>,
    pub is_resolved_home: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HomeCheck {
    /// What `resolve-activity` says Home is after the change.
    pub resolved: Option<String>,
    /// What the screen actually shows after a HOME key press.
    pub focused: Option<String>,
    pub verified: bool,
    pub note: Option<String>,
}

fn sh(t: &dyn Transport, s: &Serial, c: &ShellCommand) -> Result<String, TransportError> {
    let o = t.shell(s, c)?;
    Ok(format!("{}{}", o.stdout, o.stderr))
}

fn home_query(c: ShellCommand) -> ShellCommand {
    c.arg("-a").arg("android.intent.action.MAIN").arg("-c").arg("android.intent.category.HOME")
}

pub fn resolved_home(t: &dyn Transport, s: &Serial) -> Option<String> {
    let out = sh(t, s, &home_query(ShellCommand::new("cmd").arg("package").arg("resolve-activity").arg("--brief"))).ok()?;
    out.lines().map(str::trim).rfind(|l| Component::new(*l).is_ok()).map(str::to_string)
}

/// The component in front, from `dumpsys activity activities` (mFocusedApp, topResumedActivity or mResumedActivity).
pub fn parse_focused(dumpsys: &str) -> Option<String> {
    for key in ["mFocusedApp=", "topResumedActivity=", "mResumedActivity:"] {
        for line in dumpsys.lines().filter(|l| l.contains(key)) {
            if let Some(tok) = line.split_whitespace().find(|w| w.contains('/') && Component::new(w.trim_end_matches('}')).is_ok()) {
                return Some(tok.trim_end_matches('}').to_string());
            }
        }
    }
    None
}

pub fn focused(t: &dyn Transport, s: &Serial) -> Option<String> {
    sh(t, s, &ShellCommand::new("dumpsys").arg("activity").arg("activities")).ok().and_then(|o| parse_focused(&o))
}

pub fn status(t: &dyn Transport, s: &Serial) -> Result<LauncherStatus, TransportError> {
    let pkg = PackageName::new(STUDIO_PKG).unwrap();
    let all = sh(t, s, &ShellCommand::new("pm").arg("list").arg("packages").arg("--user").arg("0"))?;
    let installed = all.lines().any(|l| l.trim() == format!("package:{STUDIO_PKG}"));
    let disabled = sh(t, s, &ShellCommand::new("pm").arg("list").arg("packages").arg("-d").arg("--user").arg("0"))?;
    let enabled = installed && !disabled.lines().any(|l| l.trim() == format!("package:{STUDIO_PKG}"));
    let version = if installed {
        sh(t, s, &ShellCommand::new("dumpsys").arg("package").package(&pkg)).ok()
            .and_then(|o| o.lines().find_map(|l| l.trim().strip_prefix("versionName=").map(str::to_string)))
    } else { None };
    let resolved_home = resolved_home(t, s);
    let is_resolved_home = resolved_home.as_deref().is_some_and(|h| h.starts_with(&format!("{STUDIO_PKG}/")));
    Ok(LauncherStatus { installed, enabled, version, resolved_home, is_resolved_home })
}

fn classify_install(out: &str) -> Option<LauncherError> {
    if out.contains("Success") && !out.contains("Failure") { return None; }
    Some(if out.contains("INSTALL_FAILED_VERIFICATION_FAILURE") { LauncherError::VerificationRefused }
    else if out.contains("INSTALL_FAILED_OLDER_SDK") { LauncherError::OlderSdk(0) }
    else if out.contains("INSUFFICIENT_STORAGE") { LauncherError::Storage }
    else if out.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") { LauncherError::SignatureMismatch }
    else { LauncherError::Refused(out.trim().chars().take(300).collect()) })
}

/// Push and install (via `pm install -r`, which avoids Android 14's on-screen confirmation), then verify.
pub fn install(t: &dyn Transport, s: &Serial, apk: &Path, sdk: u32) -> Result<LauncherStatus, LauncherError> {
    if sdk < MIN_SDK { return Err(LauncherError::OlderSdk(sdk)); }
    t.push(s, apk, STAGED_APK)?;
    let out = sh(t, s, &ShellCommand::new("pm").arg("install").arg("-r").arg(STAGED_APK));
    let _ = t.shell(s, &ShellCommand::new("rm").arg("-f").arg(STAGED_APK));
    if let Some(e) = classify_install(&out?) { return Err(e); }
    let st = status(t, s)?;
    if !st.installed { return Err(LauncherError::NotVerified); }
    // Lets the network pill show the Wi-Fi name (Android 10+). Optional: a refusal changes nothing else.
    let _ = t.shell(s, &ShellCommand::new("pm").arg("grant").package(&PackageName::new(STUDIO_PKG).unwrap()).arg("android.permission.ACCESS_FINE_LOCATION"));
    Ok(st)
}

fn press_home_and_look(t: &dyn Transport, s: &Serial, settle: Duration) -> Option<String> {
    let _ = t.shell(s, &ShellCommand::new("input").arg("keyevent").arg("KEYCODE_HOME"));
    std::thread::sleep(settle);
    focused(t, s)
}

/// Make the Studio launcher Home and prove it on screen.
pub fn make_home(t: &dyn Transport, s: &Serial, settle: Duration) -> Result<HomeCheck, TransportError> {
    let comp = Component::new(STUDIO_HOME).unwrap();
    t.shell(s, &ShellCommand::new("cmd").arg("package").arg("set-home-activity").arg("--user").arg("0").component(&comp))?;
    let mut resolved = resolved_home(t, s);
    let mut focused = press_home_and_look(t, s, settle);
    let mut verified = focused.as_deref().is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/")));
    if !verified && focused.as_deref().is_some_and(|f| !f.contains("setupwraith")) {
        // Android 10+ keeps Home in a role. Asking for the role is the supported route and is verified the same way.
        let _ = t.shell(s, &ShellCommand::new("cmd").arg("role").arg("add-role-holder").arg("--user").arg("0").arg("android.app.role.HOME").arg(STUDIO_PKG));
        resolved = resolved_home(t, s);
        focused = press_home_and_look(t, s, settle);
        verified = focused.as_deref().is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/")));
    }
    if !verified && focused.as_deref().is_some_and(|f| !f.contains("setupwraith")) {
        // Google TV's launcherx outranks every installed Home. The launcher's Home-button helper (an
        // accessibility service) answers the Home key instead; nothing on the TV is disabled.
        let _ = enable_home_key(t, s);
        std::thread::sleep(settle);
        // A press made while the stock Home is already in front brings no window change; one more press settles it.
        for _ in 0..2 {
            focused = press_home_and_look(t, s, settle + Duration::from_millis(400));
            verified = focused.as_deref().is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/")));
            if verified { break; }
        }
    }
    let note = (!verified).then(|| match &focused {
        Some(f) if f.contains("setupwraith") => "Google TV's setup wizard still answers Home. The stock launcher stays enabled; this TV needs the setupwraith steps (Wi-Fi ownership check) before Home can move.".to_string(),
        Some(f) => format!("The TV reported success but still shows {f} for Home. Nothing else was changed."),
        None => "Could not read what the TV shows after pressing Home.".to_string(),
    });
    Ok(HomeCheck { resolved, focused, verified, note })
}

/// The launcher's Home-button helper, as Android names it in `enabled_accessibility_services`.
pub const HOME_KEY_SERVICE: &str = "studio.tvcustomizer.home/com.gathin.tvhome.HomeKeyService";

fn secure_list(t: &dyn Transport, s: &Serial) -> Vec<String> {
    let out = sh(t, s, &ShellCommand::new("settings").arg("get").arg("secure").arg("enabled_accessibility_services")).unwrap_or_default();
    let v = out.trim();
    if v.is_empty() || v == "null" { return vec![]; }
    v.split(':').filter(|x| !x.is_empty()).map(str::to_string).collect()
}

fn put_list(t: &dyn Transport, s: &Serial, list: &[String]) -> Result<(), TransportError> {
    if list.is_empty() {
        t.shell(s, &ShellCommand::new("settings").arg("delete").arg("secure").arg("enabled_accessibility_services"))?;
        return Ok(());
    }
    let joined = list.join(":");
    let cmd = ShellCommand::new("settings").arg("put").arg("secure").arg("enabled_accessibility_services")
        .checked(&joined).map_err(|e| TransportError::Other(e.to_string()))?;
    t.shell(s, &cmd)?;
    Ok(())
}

/// Whether the Home-button helper is on the TV's enabled accessibility services.
pub fn home_key_enabled(t: &dyn Transport, s: &Serial) -> bool {
    secure_list(t, s).iter().any(|x| x == HOME_KEY_SERVICE)
}

/// Add the Home-button helper to the enabled services, keeping every other service the user has on.
pub fn enable_home_key(t: &dyn Transport, s: &Serial) -> Result<(), TransportError> {
    let mut list = secure_list(t, s);
    if list.iter().any(|x| x == HOME_KEY_SERVICE) {
        // Android binds again only when the list changes.
        let others: Vec<String> = list.iter().filter(|x| *x != HOME_KEY_SERVICE).cloned().collect();
        put_list(t, s, &others)?;
    } else {
        list.push(HOME_KEY_SERVICE.to_string());
    }
    put_list(t, s, &list)?;
    t.shell(s, &ShellCommand::new("settings").arg("put").arg("secure").arg("accessibility_enabled").arg("1"))?;
    Ok(())
}

/// Remove only the Home-button helper; other services stay as they were.
pub fn disable_home_key(t: &dyn Transport, s: &Serial) -> Result<bool, TransportError> {
    let list = secure_list(t, s);
    if !list.iter().any(|x| x == HOME_KEY_SERVICE) { return Ok(false); }
    let others: Vec<String> = list.into_iter().filter(|x| x != HOME_KEY_SERVICE).collect();
    put_list(t, s, &others)?;
    Ok(true)
}

/// Google's TV launchers, the only stock Home the "turn Google TV off" switch will touch.
pub const GOOGLE_HOMES: &[&str] = &["com.google.android.apps.tv.launcherx", "com.google.android.tvlauncher", "com.google.android.leanbacklauncher"];

fn disabled_packages(t: &dyn Transport, s: &Serial) -> Vec<String> {
    sh(t, s, &ShellCommand::new("pm").arg("list").arg("packages").arg("-d").arg("--user").arg("0")).unwrap_or_default()
        .lines().filter_map(|l| l.trim().strip_prefix("package:").map(str::to_string)).collect()
}

/// Google TV launchers that are installed but switched off (by this switch, or before).
pub fn google_home_off(t: &dyn Transport, s: &Serial) -> Vec<String> {
    let off = disabled_packages(t, s);
    GOOGLE_HOMES.iter().filter(|g| off.iter().any(|o| o == *g)).map(|g| g.to_string()).collect()
}

fn is_ours(f: Option<&str>) -> bool { f.is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/"))) }

/// Opt-in: switch Google's TV launcher off (`pm disable-user`, undone by `pm enable`) so the Studio launcher is the
/// TV's real Home: it opens straight away at power-on and Google TV no longer sits in memory behind it.
/// Only when the Studio launcher is installed and already answers Home; afterwards Home must show the Studio
/// launcher, otherwise Google TV is switched straight back on. Nothing is uninstalled.
pub fn google_home_turn_off(t: &dyn Transport, s: &Serial, settle: Duration) -> Result<HomeCheck, TransportError> {
    let refuse = |note: &str, resolved: Option<String>, focused: Option<String>| Ok(HomeCheck { resolved, focused, verified: false, note: Some(note.to_string()) });
    let st = status(t, s)?;
    if !st.installed || !st.enabled { return refuse("Install the Studio launcher first.", st.resolved_home, None); }
    let resolved = st.resolved_home.clone();
    let stock = match resolved.as_deref().and_then(|r| r.split('/').next()) {
        Some(p) if GOOGLE_HOMES.contains(&p) => p.to_string(),
        Some(p) if p == STUDIO_PKG => {
            let f = press_home_and_look(t, s, settle);
            return Ok(HomeCheck { verified: is_ours(f.as_deref()), resolved, focused: f, note: Some("Google TV's launcher is already off; the Studio launcher is Home.".into()) });
        }
        _ => return refuse("The TV's Home is not Google TV's launcher, so there is nothing for this switch to turn off.", resolved, None),
    };
    // Home must already reach the Studio launcher (the Home button helper), so the TV is never left without one.
    let before = press_home_and_look(t, s, settle);
    if !is_ours(before.as_deref()) {
        return refuse("Make the Studio launcher Home first (Apply with “Make Home” ticked) and check it opens on the TV.", resolved, before);
    }
    let pkg = PackageName::new(stock.clone()).map_err(|e| TransportError::Other(e.to_string()))?;
    t.shell(s, &ShellCommand::new("pm").arg("disable-user").arg("--user").arg("0").package(&pkg))?;
    let comp = Component::new(STUDIO_HOME).unwrap();
    let _ = t.shell(s, &ShellCommand::new("cmd").arg("role").arg("add-role-holder").arg("--user").arg("0").arg("android.app.role.HOME").arg(STUDIO_PKG));
    let _ = t.shell(s, &ShellCommand::new("cmd").arg("package").arg("set-home-activity").arg("--user").arg("0").component(&comp));
    let mut focused = None;
    for _ in 0..2 {
        focused = press_home_and_look(t, s, settle);
        if is_ours(focused.as_deref()) { break; }
    }
    let resolved = resolved_home(t, s);
    if is_ours(focused.as_deref()) && google_home_off(t, s).contains(&stock) {
        return Ok(HomeCheck { resolved, focused, verified: true, note: None });
    }
    // Something else took Home (the setup wizard, Settings): put Google TV straight back.
    let _ = t.shell(s, &ShellCommand::new("pm").arg("enable").arg("--user").arg("0").package(&pkg));
    let _ = press_home_and_look(t, s, settle);
    let note = match focused.as_deref() {
        Some(f) if f.contains("setupwraith") => "This TV doesn't allow it: with Google TV off, Google's setup service takes Home and switches Google TV back on at the next start. Google TV was switched straight back on. The Home button helper keeps sending Home to the Studio launcher.".to_string(),
        other => format!("After Google TV was switched off the TV showed {} for Home, so it was switched back on. Nothing else changed.", other.unwrap_or("nothing readable")),
    };
    Ok(HomeCheck { resolved, focused, verified: false, note: Some(note) })
}

/// Switch Google's TV launcher back on (the switch off, Revert and hand-back all come here). Verified by the
/// package manager listing it as enabled again; the Home button helper keeps sending Home to the Studio launcher.
pub fn google_home_turn_on(t: &dyn Transport, s: &Serial) -> Result<Vec<String>, TransportError> {
    let mut back = vec![];
    for g in google_home_off(t, s) {
        let pkg = PackageName::new(g.clone()).map_err(|e| TransportError::Other(e.to_string()))?;
        t.shell(s, &ShellCommand::new("pm").arg("enable").arg("--user").arg("0").package(&pkg))?;
        back.push(g);
    }
    let still = google_home_off(t, s);
    if let Some(g) = back.iter().find(|g| still.contains(g)) {
        return Err(TransportError::Other(format!("The TV kept {g} switched off.")));
    }
    Ok(back)
}

/// Give Home back to `stock` (or whatever the TV resolves once ours is not preferred) and prove it.
pub fn hand_back_home(t: &dyn Transport, s: &Serial, stock: Option<&Component>, settle: Duration) -> Result<HomeCheck, TransportError> {
    // Google TV first, if the opt-in switch turned it off: it must be there to take Home back.
    let _ = google_home_turn_on(t, s);
    let _ = disable_home_key(t, s);
    // Undo the Home role first if we asked for it; a refusal just means we never held it.
    let _ = t.shell(s, &ShellCommand::new("cmd").arg("role").arg("remove-role-holder").arg("--user").arg("0").arg("android.app.role.HOME").arg(STUDIO_PKG));
    if let Some(c) = stock {
        t.shell(s, &ShellCommand::new("cmd").arg("package").arg("set-home-activity").arg("--user").arg("0").component(c))?;
    }
    let resolved = resolved_home(t, s);
    let focused = press_home_and_look(t, s, settle);
    let not_ours = focused.as_deref().is_some_and(|f| !f.starts_with(&format!("{STUDIO_PKG}/")));
    let matches_stock = match (stock, focused.as_deref()) {
        (Some(c), Some(f)) => f.starts_with(&format!("{}/", c.package())),
        (None, Some(_)) => true,
        _ => false,
    };
    let verified = not_ours && matches_stock;
    let note = (!verified).then(|| "Home could not be confirmed on the stock launcher, so the Studio launcher was kept installed to avoid leaving the TV without a Home screen.".to_string());
    Ok(HomeCheck { resolved, focused, verified, note })
}

/// Uninstall the Studio launcher (a normal app, never a system package) and verify it is gone.
/// The caller must have handed Home back and verified it first.
pub fn uninstall(t: &dyn Transport, s: &Serial) -> Result<bool, TransportError> {
    let pkg = PackageName::new(STUDIO_PKG).unwrap();
    let _ = t.shell(s, &ShellCommand::new("pm").arg("uninstall").package(&pkg))?;
    Ok(!status(t, s)?.installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_focused_from_variants() {
        assert_eq!(parse_focused("  mFocusedApp=ActivityRecord{9f1 u0 studio.tvcustomizer.home/com.gathin.tvhome.MainActivity t12}\n"),
            Some(STUDIO_HOME.into()));
        assert_eq!(parse_focused("  topResumedActivity=ActivityRecord{1 u0 com.google.android.apps.tv.launcherx/.home.HomeActivity t3}"),
            Some("com.google.android.apps.tv.launcherx/.home.HomeActivity".into()));
        assert_eq!(parse_focused("    mResumedActivity: ActivityRecord{2 u0 com.tcl.tv/.Main t4}"), Some("com.tcl.tv/.Main".into()));
        assert_eq!(parse_focused("mFocusedApp=null"), None);
    }

    #[test]
    fn classifies_install_output() {
        assert!(classify_install("Performing Streamed Install\nSuccess").is_none());
        assert!(matches!(classify_install("Failure [INSTALL_FAILED_VERIFICATION_FAILURE]"), Some(LauncherError::VerificationRefused)));
        assert!(matches!(classify_install("Failure [INSTALL_FAILED_UPDATE_INCOMPATIBLE: ...]"), Some(LauncherError::SignatureMismatch)));
        assert!(matches!(classify_install("Failure [INSTALL_FAILED_INSUFFICIENT_STORAGE]"), Some(LauncherError::Storage)));
        assert!(matches!(classify_install("weird"), Some(LauncherError::Refused(_))));
    }
}
