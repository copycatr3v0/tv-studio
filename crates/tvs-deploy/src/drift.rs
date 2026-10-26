//! What a firmware update (or anything else) undid, compared with a target profile — CleanYourTV's
//! `tools/check.py` idea. Read-only. The repair is the same plan run again after the user reviews it:
//! every guard runs again, nothing is re-applied silently.

use crate::plan::{plan, DeployInput, Plan, PlanError, Preview};
use serde::Serialize;
use tvs_core::Serial;
use tvs_launcher::install::{self, STUDIO_PKG};
use tvs_recovery::Snapshot;
use tvs_transport::Transport;

#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub plan: Plan,
    /// Packages the profile disables that are enabled again.
    pub reenabled: Vec<String>,
    /// The profile installs the Studio launcher but it is gone.
    pub launcher_missing: bool,
    /// The profile makes the Studio launcher Home but Home resolves elsewhere.
    pub home_moved: bool,
    /// Enabled packages that were not on the TV when its baseline was recorded (new preloads).
    pub new_packages: Vec<String>,
    /// Step titles the repair would change.
    pub would_change: Vec<String>,
    pub in_sync: bool,
}

pub fn check(t: &dyn Transport, s: &Serial, input: &DeployInput, baseline: Option<&Snapshot>) -> Result<DriftReport, PlanError> {
    let p = plan(t, s, input)?;
    let installed = tvs_device::inventory::list_packages(t, s).map_err(|e| PlanError::Unreachable(e.to_string()))?;
    let enabled = |n: &str| installed.iter().any(|x| x.name == n && x.enabled);
    let reenabled = input.profile.debloat.packages.iter().filter(|n| enabled(n)).cloned().collect();
    let status = install::status(t, s).map_err(|e| PlanError::Unreachable(e.to_string()))?;
    let launcher_missing = (input.profile.launcher.install || input.profile.launcher.make_home) && !status.installed;
    let home_moved = input.profile.launcher.make_home && !status.resolved_home.as_deref().is_some_and(|h| h.starts_with(&format!("{STUDIO_PKG}/")));
    let new_packages = baseline.map(|b| installed.iter()
        .filter(|x| x.enabled && x.system && !b.packages.contains_key(&x.name))
        .map(|x| x.name.clone()).collect()).unwrap_or_default();
    // A theme is always re-sent and Home is always re-verified, so neither counts as drift on its own.
    let would_change: Vec<String> = p.steps.iter()
        .filter(|st| matches!(st.preview, Preview::WillChange(_)) && st.id != "theme" && st.id != "launcher-home")
        .map(|st| st.title.clone()).collect();
    let in_sync = would_change.is_empty() && !home_moved && !launcher_missing;
    Ok(DriftReport { plan: p, reenabled, launcher_missing, home_moved, new_packages, would_change, in_sync })
}
