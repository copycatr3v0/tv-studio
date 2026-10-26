//! Snapshots, verified batch disabling and exact restore.
//!
//! Design from CleanYourTV `tools/recovery.py` (MIT, (c) 2026 Gathin): the state is recorded before the
//! first change and the first record wins; undo puts back what was there (a package that was already
//! disabled stays disabled) instead of enabling a whole list. Every change is read back from the TV;
//! an exit code is never taken as proof.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tvs_core::{Component, PackageName, Serial};
use tvs_device::inventory::{enabled_packages, list_packages, InventoryError, PackageInfo};
use tvs_device::probe::{parse_resolve_home, HomeState};
use tvs_transport::{ShellCommand, Transport, TransportError};

pub const SNAPSHOT_VERSION: u32 = 1;
pub const ANIMATION_SETTINGS: [&str; 3] = ["window_animation_scale", "transition_animation_scale", "animator_duration_scale"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PkgState { Enabled, Disabled }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub identity: String,
    pub serial: String,
    pub created: String,
    pub packages: BTreeMap<String, PkgState>,
    /// Default Home component; None for a chooser or when it could not be read.
    pub home: Option<String>,
    /// Global settings; None means the key was unset ("null").
    pub settings: BTreeMap<String, Option<String>>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum RecoveryError {
    #[error("{0}")]
    Inventory(#[from] InventoryError),
    #[error("{0}")]
    Transport(#[from] TransportError),
}
pub type Result<T> = std::result::Result<T, RecoveryError>;

fn home_component(t: &dyn Transport, serial: &Serial) -> Option<String> {
    let cmd = ShellCommand::new("cmd").arg("package").arg("resolve-activity").arg("--brief")
        .arg("-a").arg("android.intent.action.MAIN").arg("-c").arg("android.intent.category.HOME");
    let o = t.shell(serial, &cmd).ok().filter(|o| o.code == Some(0))?;
    match parse_resolve_home(&o.stdout) { HomeState::Component(c) => Some(c), _ => None }
}

/// Read the full state that undo needs. Fails if the package list cannot be read.
pub fn capture(t: &dyn Transport, serial: &Serial, identity: &str, now: &str) -> Result<Snapshot> {
    let pkgs = list_packages(t, serial)?;
    let mut settings = BTreeMap::new();
    for key in ANIMATION_SETTINGS {
        let o = t.shell(serial, &ShellCommand::new("settings").arg("get").arg("global").arg(key))?;
        let v = o.stdout.trim().to_string();
        settings.insert(key.to_string(), if v == "null" || v.is_empty() { None } else { Some(v) });
    }
    Ok(Snapshot {
        version: SNAPSHOT_VERSION,
        identity: identity.to_string(),
        serial: serial.to_string(),
        created: now.to_string(),
        packages: pkgs.iter().map(|p| (p.name.clone(), if p.enabled { PkgState::Enabled } else { PkgState::Disabled })).collect(),
        home: home_component(t, serial),
        settings,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Progress {
    Package { name: String, index: usize, total: usize },
    Verified { batch: usize, batches: usize },
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ChangeReport {
    /// Verified by reading the TV back.
    pub changed: Vec<String>,
    /// The TV answered with an error for these.
    pub refused: Vec<(String, String)>,
    /// The command "succeeded" but the readback shows no change.
    pub unchanged: Vec<String>,
    /// Never attempted because the run stopped first.
    pub skipped: Vec<String>,
    pub stopped: Option<String>,
}

fn verify_batch(t: &dyn Transport, serial: &Serial, batch: &[PackageName], want_enabled: bool, attempted: &BTreeSet<String>, r: &mut ChangeReport) -> Result<()> {
    let enabled = enabled_packages(t, serial)?;
    for p in batch {
        let name = p.as_str();
        if !attempted.contains(name) { continue; }
        if enabled.contains(name) == want_enabled { r.changed.push(name.into()) } else { r.unchanged.push(name.into()) }
    }
    Ok(())
}

/// Disable packages one shell call each, in batches, reading the enabled list back after every batch.
/// Stops (without retrying) on a lost connection, or when the whole first batch is refused, which means
/// this firmware does not allow `pm disable-user` at all.
pub fn disable_in_batches(t: &dyn Transport, serial: &Serial, pkgs: &[PackageName], batch_size: usize, progress: &mut dyn FnMut(Progress)) -> ChangeReport {
    change_in_batches(t, serial, pkgs, batch_size, false, progress)
}

fn change_in_batches(t: &dyn Transport, serial: &Serial, pkgs: &[PackageName], batch_size: usize, enable: bool, progress: &mut dyn FnMut(Progress)) -> ChangeReport {
    let mut r = ChangeReport::default();
    let size = batch_size.clamp(1, 10);
    let batches: Vec<&[PackageName]> = pkgs.chunks(size).collect();
    let total = pkgs.len();
    let mut index = 0;
    for (bi, batch) in batches.iter().enumerate() {
        let mut attempted = BTreeSet::new();
        let mut refused_here = 0;
        for p in batch.iter() {
            index += 1;
            progress(Progress::Package { name: p.to_string(), index, total });
            let cmd = if enable {
                ShellCommand::new("pm").arg("enable").arg("--user").arg("0").package(p)
            } else {
                ShellCommand::new("pm").arg("disable-user").arg("--user").arg("0").package(p)
            };
            match t.shell(serial, &cmd) {
                Ok(o) if o.code == Some(0) => { attempted.insert(p.to_string()); }
                Ok(o) => { refused_here += 1; r.refused.push((p.to_string(), format!("{} {}", o.stderr.trim(), o.stdout.trim()).trim().to_string())); }
                Err(e) => {
                    // The command may or may not have reached the TV: the readback decides, never a retry.
                    attempted.insert(p.to_string());
                    r.stopped = Some(format!("Connection problem at {p}: {e}"));
                    break;
                }
            }
        }
        if let Err(e) = verify_batch(t, serial, batch, enable, &attempted, &mut r) {
            r.stopped.get_or_insert(format!("Could not read the TV back after batch {}: {e}", bi + 1));
            r.unchanged.extend(attempted.iter().filter(|a| !r.changed.contains(a)).cloned());
        } else {
            progress(Progress::Verified { batch: bi + 1, batches: batches.len() });
        }
        if r.stopped.is_none() && bi == 0 && !enable && refused_here == batch.len() {
            r.stopped = Some("The TV refused every package in the first batch: this firmware does not allow disabling packages over ADB.".into());
        }
        if r.stopped.is_some() {
            let done: BTreeSet<&String> = r.changed.iter().chain(r.unchanged.iter()).chain(r.refused.iter().map(|(n, _)| n)).collect();
            r.skipped = pkgs.iter().map(|p| p.to_string()).filter(|p| !done.contains(p)).collect();
            break;
        }
    }
    r
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RestorePlan {
    /// Recorded enabled, currently disabled.
    pub enable: Vec<String>,
    /// Recorded disabled; left alone (the owner had disabled them before).
    pub left_disabled: Vec<String>,
    /// Not installed when the snapshot was taken; left alone.
    pub not_in_snapshot: Vec<String>,
    pub set_home: Option<String>,
    pub settings: Vec<(String, Option<String>)>,
}

/// What undo would do, optionally limited to `scope`. Never disables anything.
pub fn restore_plan(snap: &Snapshot, current: &[PackageInfo], current_home: Option<&str>, current_settings: &BTreeMap<String, Option<String>>, scope: Option<&[String]>) -> RestorePlan {
    let in_scope = |n: &str| scope.is_none_or(|s| s.iter().any(|x| x == n));
    let mut plan = RestorePlan { enable: vec![], left_disabled: vec![], not_in_snapshot: vec![], set_home: None, settings: vec![] };
    for p in current.iter().filter(|p| !p.enabled && in_scope(&p.name)) {
        match snap.packages.get(&p.name) {
            Some(PkgState::Enabled) => plan.enable.push(p.name.clone()),
            Some(PkgState::Disabled) => plan.left_disabled.push(p.name.clone()),
            None => plan.not_in_snapshot.push(p.name.clone()),
        }
    }
    if scope.is_none() {
        if let Some(h) = &snap.home {
            if current_home != Some(h.as_str()) && Component::new(h.clone()).is_ok() { plan.set_home = Some(h.clone()); }
        }
        for (k, v) in &snap.settings {
            if current_settings.get(k) != Some(v) { plan.settings.push((k.clone(), v.clone())); }
        }
    }
    plan
}

/// Re-enable the plan's packages (verified), then Home and settings. Returns the package report.
pub fn apply_restore(t: &dyn Transport, serial: &Serial, plan: &RestorePlan, batch_size: usize, progress: &mut dyn FnMut(Progress)) -> ChangeReport {
    let pkgs: Vec<PackageName> = plan.enable.iter().filter_map(|n| PackageName::new(n.clone()).ok()).collect();
    let mut r = change_in_batches(t, serial, &pkgs, batch_size, true, progress);
    if r.stopped.is_some() { return r; }
    if let Some(h) = plan.set_home.as_ref().and_then(|h| Component::new(h.clone()).ok()) {
        let _ = t.shell(serial, &ShellCommand::new("cmd").arg("package").arg("set-home-activity").arg("--user").arg("0").component(&h));
        if home_component(t, serial).as_deref() != Some(h.as_str()) {
            r.unchanged.push(format!("Home → {h}"));
        } else {
            r.changed.push(format!("Home → {h}"));
        }
    }
    for (k, v) in &plan.settings {
        let base = ShellCommand::new("settings");
        let cmd = match v {
            Some(val) => base.arg("put").arg("global").checked(k).and_then(|c| c.checked(val)),
            None => base.arg("delete").arg("global").checked(k),
        };
        if let Ok(cmd) = cmd {
            let _ = t.shell(serial, &cmd);
            let now = t.shell(serial, &ShellCommand::new("settings").arg("get").arg("global").checked(k).expect("checked above"))
                .map(|o| o.stdout.trim().to_string()).unwrap_or_default();
            let ok = match v { Some(val) => &now == val, None => now == "null" };
            if ok { r.changed.push(k.to_string()) } else { r.unchanged.push(k.to_string()) }
        }
    }
    r
}

#[cfg(test)]
mod tests;
