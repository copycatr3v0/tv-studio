//! Package inventory and the live facts the safety guards need. Everything here is read-only.

use crate::probe::{parse_resolve_home, HomeState};
use serde::Serialize;
use std::collections::BTreeSet;
use tvs_core::{Component, PackageName, Serial};
use tvs_safety::DeviceFacts;
use tvs_transport::{ShellCommand, Transport, TransportError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub enabled: bool,
    pub system: bool,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum InventoryError {
    #[error("{0}")]
    Transport(#[from] TransportError),
    #[error("the TV refused to list packages: {0}")]
    Refused(String),
}

fn run_ok(t: &dyn Transport, serial: &Serial, cmd: &ShellCommand) -> Result<String, InventoryError> {
    let o = t.shell(serial, cmd)?;
    if o.code == Some(0) { Ok(o.stdout) } else { Err(InventoryError::Refused(o.stderr.trim().to_string())) }
}

fn packages(out: &str) -> BTreeSet<String> {
    out.lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .filter(|p| PackageName::new(*p).is_ok())
        .map(str::to_string)
        .collect()
}

fn pm_list(flags: &[&'static str]) -> ShellCommand {
    flags.iter().fold(ShellCommand::new("pm").arg("list").arg("packages"), |c, f| c.arg(f)).arg("--user").arg("0")
}

/// All packages for user 0 with enabled and system flags. Any failed query fails the whole call:
/// a partial list must never look like the TV's real state.
pub fn list_packages(t: &dyn Transport, serial: &Serial) -> Result<Vec<PackageInfo>, InventoryError> {
    let all = packages(&run_ok(t, serial, &pm_list(&[]))?);
    if all.is_empty() {
        return Err(InventoryError::Refused("the package list came back empty".into()));
    }
    let disabled = packages(&run_ok(t, serial, &pm_list(&["-d"]))?);
    let system = packages(&run_ok(t, serial, &pm_list(&["-s"]))?);
    Ok(all.into_iter().map(|name| PackageInfo { enabled: !disabled.contains(&name), system: system.contains(&name), name }).collect())
}

/// Enabled packages for user 0 (used to read back the result of a change).
pub fn enabled_packages(t: &dyn Transport, serial: &Serial) -> Result<BTreeSet<String>, InventoryError> {
    let e = packages(&run_ok(t, serial, &pm_list(&["-e"]))?);
    if e.is_empty() { Err(InventoryError::Refused("the enabled list came back empty".into())) } else { Ok(e) }
}

/// Whether `dumpsys package` lists TV_INPUT_HARDWARE among the *requested* permissions.
pub fn requests_input_hardware(dumpsys: &str) -> bool {
    let mut inside = false;
    for line in dumpsys.lines() {
        let l = line.trim();
        if l.starts_with("requested permissions:") { inside = true; continue; }
        if inside && (l.starts_with("install permissions:") || l.starts_with("runtime permissions:") || l.starts_with("User ")) { inside = false; }
        if inside && l.starts_with("android.permission.TV_INPUT_HARDWARE") { return true; }
    }
    false
}

/// `None` when no saved network shows an owner (cannot decide), else whether setupwraith owns one.
pub fn setupwraith_owns_wifi(dumpsys_wifi: &str) -> Option<bool> {
    let owners: Vec<&str> = dumpsys_wifi.split_whitespace().filter_map(|w| w.strip_prefix("cname=")).collect();
    if owners.is_empty() { None } else { Some(owners.iter().any(|o| o.contains("setupwraith"))) }
}

/// Packages with a TV (LEANBACK_LAUNCHER) entry, in the TV's order: what a launcher can show as tiles.
pub fn launchable_packages(t: &dyn Transport, serial: &Serial) -> Result<Vec<String>, InventoryError> {
    let out = run_ok(t, serial, &ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief")
        .arg("-a").arg("android.intent.action.MAIN").arg("-c").arg("android.intent.category.LEANBACK_LAUNCHER"))?;
    let mut seen = BTreeSet::new();
    Ok(components(&out).into_iter().map(|c| c.package().to_string()).filter(|p| seen.insert(p.clone())).collect())
}

/// Current default Home component, if a single one is set.
pub fn current_home(t: &dyn Transport, serial: &Serial) -> Option<String> {
    let cmd = ShellCommand::new("cmd").arg("package").arg("resolve-activity").arg("--brief")
        .arg("-a").arg("android.intent.action.MAIN").arg("-c").arg("android.intent.category.HOME");
    match parse_resolve_home(&run_ok(t, serial, &cmd).ok()?) { HomeState::Component(c) => Some(c), _ => None }
}

fn components(out: &str) -> Vec<Component> {
    out.lines().filter_map(|l| Component::new(l.trim()).ok()).collect()
}

/// Read the facts the guards need for this selection. A query that fails leaves its "checked" flag
/// false, and the guards then block (fail closed).
pub fn gather_facts(t: &dyn Transport, serial: &Serial, installed: &[PackageInfo], selection: &[PackageName]) -> DeviceFacts {
    let enabled: BTreeSet<&str> = installed.iter().filter(|p| p.enabled).map(|p| p.name.as_str()).collect();
    let mut f = DeviceFacts { cvte: installed.iter().any(|p| p.name.starts_with("com.cvte.")), ..Default::default() };
    let home_args = |c: ShellCommand| c.arg("-a").arg("android.intent.action.MAIN").arg("-c").arg("android.intent.category.HOME");

    if let Ok(out) = run_ok(t, serial, &home_args(ShellCommand::new("cmd").arg("package").arg("resolve-activity").arg("--brief"))) {
        match parse_resolve_home(&out) {
            HomeState::Component(c) => { f.home_known = true; f.home_package = c.split('/').next().map(str::to_string); }
            HomeState::Chooser => f.home_known = true,
            HomeState::Unknown => {}
        }
    }
    if let Ok(out) = run_ok(t, serial, &home_args(ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief"))) {
        f.usable_launchers = components(&out).into_iter()
            .filter(tvs_safety::is_usable_launcher)
            .map(|c| c.package().to_string())
            .filter(|p| enabled.contains(p.as_str()))
            .collect();
    } else {
        f.home_known = false; // without the claimant list the Home guard cannot judge
    }

    if selection.iter().any(|p| p.as_str() == tvs_safety::SETUPWRAITH) {
        f.setupwraith_owns_wifi = run_ok(t, serial, &ShellCommand::new("dumpsys").arg("wifi")).ok().and_then(|o| setupwraith_owns_wifi(&o));
    }

    f.input_hardware_checked = true;
    for p in selection {
        match run_ok(t, serial, &ShellCommand::new("dumpsys").arg("package").package(p)) {
            Ok(o) => if requests_input_hardware(&o) { f.input_hardware.insert(p.to_string()); },
            Err(_) => { f.input_hardware_checked = false; break; }
        }
    }

    let channel = run_ok(t, serial, &ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief")
        .arg("-a").arg("android.intent.action.VIEW").arg("-t").arg("vnd.android.cursor.item/channel"));
    let settings = run_ok(t, serial, &ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief").arg("-a").arg("android.settings.SETTINGS"));
    if let (Ok(c), Ok(s)) = (channel, settings) {
        f.handlers_checked = true;
        f.input_handlers = components(&c).into_iter().map(|c| c.package().to_string()).collect();
        f.settings_handlers = components(&s).into_iter().map(|c| c.package().to_string()).collect();
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_hardware_only_in_requested_section() {
        let req = "Packages:\n  Package [com.tcl.tvinput]\n    requested permissions:\n      android.permission.TV_INPUT_HARDWARE\n    install permissions:\n      android.permission.INTERNET: granted=true\n";
        assert!(requests_input_hardware(req));
        let granted_only = "    requested permissions:\n      android.permission.INTERNET\n    install permissions:\n      android.permission.TV_INPUT_HARDWARE: granted=true\n";
        assert!(!requests_input_hardware(granted_only));
    }

    #[test]
    fn wifi_owner() {
        assert_eq!(setupwraith_owns_wifi("SSID: \"Home\" cname=com.google.android.tungsten.setupwraith uid=1000"), Some(true));
        assert_eq!(setupwraith_owns_wifi("SSID: \"Home\" cname=com.android.tv.settings"), Some(false));
        assert_eq!(setupwraith_owns_wifi("no networks"), None);
    }
}
