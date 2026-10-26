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
    let resolved = resolved_home(t, s);
    let focused = press_home_and_look(t, s, settle);
    let verified = focused.as_deref().is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/")));
    let note = (!verified).then(|| match &focused {
        Some(f) if f.contains("setupwraith") => "Google TV's setup wizard still answers Home. The stock launcher stays enabled; this TV needs the setupwraith steps (Wi-Fi ownership check) before Home can move.".to_string(),
        Some(f) => format!("The TV reported success but still shows {f} for Home. Nothing else was changed."),
        None => "Could not read what the TV shows after pressing Home.".to_string(),
    });
    Ok(HomeCheck { resolved, focused, verified, note })
}

/// Give Home back to `stock` (or whatever the TV resolves once ours is not preferred) and prove it.
pub fn hand_back_home(t: &dyn Transport, s: &Serial, stock: Option<&Component>, settle: Duration) -> Result<HomeCheck, TransportError> {
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
