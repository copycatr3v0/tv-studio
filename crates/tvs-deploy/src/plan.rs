//! Turn a profile into an ordered, dependency-aware plan, previewed against the live TV (dry run).
//! Planning only reads the TV. A plan with any blocked step cannot run.

use crate::profile::{valid_dns_hostname, PrivateDns, Profile, ANIMATION_VALUES};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tvs_core::{PackageName, Serial};
use tvs_device::inventory::{gather_facts, list_packages};
use tvs_launcher::install::{self, STUDIO_PKG};
use tvs_launcher::theme::ThemeBundle;
use tvs_safety::{evaluate_selection, Verdict, KNOWN_STOCK_LAUNCHERS};
use tvs_transport::{ShellCommand, Transport};

/// Everything a run needs besides the profile.
#[derive(Debug, Clone)]
pub struct DeployInput {
    pub profile: Profile,
    pub theme: Option<ThemeBundle>,
    pub apk: Option<PathBuf>,
    pub batch_size: usize,
    /// How long to wait after pressing Home before reading the screen.
    pub settle: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepKind {
    Identify,
    Snapshot,
    InstallLauncher,
    MakeHome,
    DebloatBatch { packages: Vec<String> },
    Settings { values: Vec<(String, Option<String>)> },
    Theme,
    DisableStockLauncher { package: String },
    FinalVerify,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", content = "detail", rename_all = "snake_case")]
pub enum Preview {
    /// Always runs (identity check, snapshot, verification).
    Check(String),
    WillChange(String),
    AlreadyDone(String),
    Blocked(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub id: String,
    pub title: String,
    pub kind: StepKind,
    pub depends_on: Vec<String>,
    /// A failed critical step stops the whole run.
    pub critical: bool,
    pub preview: Preview,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub identity: String,
    pub sdk: u32,
    pub steps: Vec<Step>,
    pub runnable: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum PlanError {
    Unreachable(String),
    WrongDevice { expected: String, found: String },
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::Unreachable(m) => write!(f, "Could not read the TV: {m}"),
            PlanError::WrongDevice { expected, found } => write!(f, "This profile belongs to another TV ({expected}); the connected one is {found}."),
        }
    }
}

pub(crate) fn get_setting(t: &dyn Transport, s: &Serial, key: &str) -> Option<String> {
    let c = ShellCommand::new("settings").arg("get").arg("global").checked(key).ok()?;
    let v = t.shell(s, &c).ok()?.stdout.trim().to_string();
    if v.is_empty() || v == "null" { None } else { Some(v) }
}

fn step(id: &str, title: &str, kind: StepKind, deps: &[&str], critical: bool, preview: Preview) -> Step {
    Step { id: id.into(), title: title.into(), kind, depends_on: deps.iter().map(|d| d.to_string()).collect(), critical, preview }
}

pub fn plan(t: &dyn Transport, s: &Serial, input: &DeployInput) -> Result<Plan, PlanError> {
    let p = &input.profile;
    let identity = tvs_device::probe::quick_identity(t, s).ok_or_else(|| PlanError::Unreachable("no identity (getprop)".into()))?;
    if let Some(exp) = &p.device_identity {
        if exp != &identity { return Err(PlanError::WrongDevice { expected: exp.clone(), found: identity }); }
    }
    let sdk = tvs_device::probe::quick_sdk(t, s).ok_or_else(|| PlanError::Unreachable("no Android version".into()))?;
    let installed = list_packages(t, s).map_err(|e| PlanError::Unreachable(e.to_string()))?;
    let enabled = |n: &str| installed.iter().any(|x| x.name == n && x.enabled);
    let launcher = install::status(t, s).map_err(|e| PlanError::Unreachable(e.to_string()))?;
    let mut notes = vec![];
    let fire_os = installed.iter().any(|x| x.name == "com.amazon.tv.launcher");

    let mut steps = vec![
        step("identify", "Identify the TV", StepKind::Identify, &[], true, Preview::Check(identity.clone())),
        step("snapshot", "Save the TV's current state", StepKind::Snapshot, &["identify"], true,
             Preview::Check("Baseline kept if one exists; a pre-change snapshot is always saved".into())),
    ];

    // Launcher.
    let launcher_ready = launcher.installed || p.launcher.install;
    if p.launcher.install {
        let pv = if sdk < install::MIN_SDK { Preview::Blocked(format!("Android API {sdk}: the launcher needs API 26")) }
            else if fire_os { Preview::Blocked("Fire OS does not allow replacing Home".into()) }
            else if launcher.installed { Preview::AlreadyDone(format!("Installed, version {}", launcher.version.clone().unwrap_or_default())) }
            else if input.apk.is_none() { Preview::Blocked("This build does not include the launcher APK".into()) }
            else { Preview::WillChange("Install the Studio launcher (your current launcher stays enabled)".into()) };
        steps.push(step("launcher-install", "Install the Studio launcher", StepKind::InstallLauncher, &["snapshot"], false, pv));
    }
    if p.launcher.make_home {
        let pv = if sdk < install::MIN_SDK || fire_os { Preview::Blocked("The Studio launcher cannot run on this TV".into()) }
            else if !launcher_ready { Preview::Blocked("The launcher is not installed and not selected for install".into()) }
            else { Preview::WillChange("Set as Home, then press Home and confirm on screen".into()) };
        let deps: Vec<&str> = if p.launcher.install { vec!["snapshot", "launcher-install"] } else { vec!["snapshot"] };
        steps.push(step("launcher-home", "Make the Studio launcher Home", StepKind::MakeHome, &deps, false, pv));
    }

    // Debloat: everything blocked or unconfirmed blocks the plan; already-disabled packages drop out.
    let mut names = vec![];
    let mut invalid = vec![];
    for n in &p.debloat.packages {
        match PackageName::new(n.clone()) {
            Ok(pn) => names.push(pn),
            Err(_) => invalid.push(format!("{n:?} is not a valid package name")),
        }
    }
    let stock = stock_launcher(&installed, launcher.resolved_home.as_deref());
    if p.launcher.disable_stock { if let Some(st) = &stock { names.retain(|n| n.as_str() != st); } }
    let mut sel: Vec<PackageName> = names.clone();
    if p.launcher.disable_stock { if let Some(st) = &stock { sel.push(PackageName::new(st.clone()).unwrap()); } }
    let facts = gather_facts(t, s, &installed, &sel);
    let verdicts = evaluate_selection(&names, &facts);
    // An invalid name in an imported profile is a broken profile: refuse it rather than run the rest.
    let mut problems = invalid;
    // Launchers never go through a debloat batch: batches don't wait for Home to be confirmed.
    for n in &names {
        if facts.usable_launchers.contains(n.as_str()) || KNOWN_STOCK_LAUNCHERS.contains(&n.as_str()) || n.as_str() == STUDIO_PKG {
            problems.push(format!("{n} is a launcher: use \u{201c}Disable the stock launcher\u{201d}, which runs only after the Studio launcher is confirmed as Home"));
        }
    }
    names.retain(|n| !(facts.usable_launchers.contains(n.as_str()) || KNOWN_STOCK_LAUNCHERS.contains(&n.as_str()) || n.as_str() == STUDIO_PKG));
    let mut todo = vec![];
    let mut already = vec![];
    for n in &names {
        match verdicts.get(n) {
            _ if problems.iter().any(|p| p.starts_with(&format!("{n} "))) => {}
            Some(Verdict::Blocked(r)) => problems.push(format!("{n}: {r}")),
            Some(Verdict::NeedsConfirm(r)) if !p.debloat.confirmed.iter().any(|c| c == n.as_str()) => problems.push(format!("{n} needs confirmation: {r}")),
            _ if !enabled(n.as_str()) => already.push(n.to_string()),
            _ => todo.push(n.to_string()),
        }
    }
    if !problems.is_empty() {
        steps.push(step("debloat-review", "Review the app selection", StepKind::DebloatBatch { packages: vec![] }, &["snapshot"], false,
                        Preview::Blocked(problems.join(" | "))));
    } else if !names.is_empty() {
        if todo.is_empty() {
            steps.push(step("debloat-1", "Disable selected apps", StepKind::DebloatBatch { packages: vec![] }, &["snapshot"], false,
                            Preview::AlreadyDone(format!("All {} already disabled", already.len()))));
        }
        let size = input.batch_size.clamp(1, 10);
        let batches: Vec<Vec<String>> = todo.chunks(size).map(|c| c.to_vec()).collect();
        let n = batches.len();
        for (i, b) in batches.into_iter().enumerate() {
            let id = format!("debloat-{}", i + 1);
            let prev = if i == 0 { "snapshot".to_string() } else { format!("debloat-{i}") };
            let title = format!("Disable apps, batch {} of {n}", i + 1);
            let detail = b.join(", ");
            steps.push(Step { id, title, kind: StepKind::DebloatBatch { packages: b }, depends_on: vec![prev], critical: false, preview: Preview::WillChange(detail) });
        }
        if !already.is_empty() { notes.push(format!("Already disabled, skipped: {}", already.join(", "))); }
    }

    // Settings.
    if let Some(scale) = &p.settings.animation_scale {
        let pv = if !ANIMATION_VALUES.contains(&scale.as_str()) { Preview::Blocked(format!("Unsupported animation speed {scale}")) } else {
            let cur: Vec<Option<String>> = tvs_recovery::ANIMATION_SETTINGS.iter().map(|k| get_setting(t, s, k)).collect();
            if cur.iter().all(|c| c.as_deref() == Some(scale.as_str()) || (scale == "1.0" && c.is_none())) { Preview::AlreadyDone(format!("Already {scale}×")) }
            else { Preview::WillChange(format!("Animation speed → {scale}× (now {})", cur[0].clone().unwrap_or_else(|| "1.0 default".into()))) }
        };
        let values = tvs_recovery::ANIMATION_SETTINGS.iter().map(|k| (k.to_string(), Some(scale.clone()))).collect();
        steps.push(step("settings-animation", "Animation speed", StepKind::Settings { values }, &["snapshot"], false, pv));
    }
    match &p.settings.private_dns {
        PrivateDns::Keep => {}
        dns => {
            let values: Vec<(String, Option<String>)> = match dns {
                PrivateDns::Automatic => vec![("private_dns_mode".into(), Some("opportunistic".into()))],
                PrivateDns::Off => vec![("private_dns_mode".into(), Some("off".into()))],
                PrivateDns::Hostname(h) => vec![("private_dns_specifier".into(), Some(h.clone())), ("private_dns_mode".into(), Some("hostname".into()))],
                PrivateDns::Keep => unreachable!(),
            };
            let pv = if sdk < 28 { Preview::Blocked(format!("Private DNS needs Android 9 (API 28); this TV is API {sdk}")) }
                else if matches!(dns, PrivateDns::Hostname(h) if !valid_dns_hostname(h)) { Preview::Blocked("Not a valid DNS hostname".into()) }
                else if values.iter().all(|(k, v)| &get_setting(t, s, k) == v) { Preview::AlreadyDone("Already set".into()) }
                else { Preview::WillChange(values.iter().map(|(k, v)| format!("{k} → {}", v.clone().unwrap_or_default())).collect::<Vec<_>>().join(", ")) };
            steps.push(step("settings-dns", "Private DNS", StepKind::Settings { values }, &["snapshot"], false, pv));
            notes.push("Private DNS bypasses DNS filtering done by your router (Pi-hole, AdGuard Home, VPN domain rules), and it doesn't block YouTube ads.".into());
        }
    }

    // Theme.
    if p.apply_theme {
        let pv = match &input.theme {
            None => Preview::Blocked("No theme was supplied".into()),
            Some(b) => {
                let issues = tvs_launcher::schema::validate(&b.config);
                if !issues.is_empty() { Preview::Blocked(issues.iter().map(|i| format!("{}: {}", i.key, i.message)).collect::<Vec<_>>().join(" | ")) }
                else if !launcher_ready { Preview::Blocked("The theme needs the Studio launcher: select Install".into()) }
                else { Preview::WillChange(format!("{} settings, {} wallpaper(s); read back after writing", b.config.as_object().map(|o| o.len()).unwrap_or(0), b.wallpapers.len())) }
            }
        };
        let deps: Vec<&str> = if p.launcher.install { vec!["snapshot", "launcher-install"] } else { vec!["snapshot"] };
        steps.push(step("theme", "Send the theme", StepKind::Theme, &deps, false, pv));
    }

    // Stock launcher, only after the Studio launcher is confirmed as Home.
    if p.launcher.disable_stock {
        let pv = if !p.launcher.make_home { Preview::Blocked("Only possible together with \u{201c}Make the Studio launcher Home\u{201d}".into()) }
            else { match &stock {
                None => Preview::AlreadyDone("No enabled stock launcher found".into()),
                Some(st) if facts.cvte && st.starts_with("com.cvte.tv.launcher") => Preview::Blocked("On CVTE boards the stock launcher stays enabled".into()),
                Some(st) => Preview::WillChange(format!("Disable {st} after Home is confirmed; checked again on the TV first")),
            } };
        let pkg = stock.clone().unwrap_or_default();
        steps.push(step("launcher-disable-stock", "Disable the stock launcher", StepKind::DisableStockLauncher { package: pkg }, &["launcher-home"], false, pv));
    }

    steps.push(step("verify", "Check Home and Settings still work", StepKind::FinalVerify, &["identify"], false,
                    Preview::Check("Press Home and confirm what opens; confirm Settings can still open".into())));
    if fire_os { notes.push("Fire OS detected: only inspection and a few settings are possible.".into()); }
    let runnable = !steps.iter().any(|s| matches!(s.preview, Preview::Blocked(_)));
    Ok(Plan { identity, sdk, steps, runnable, notes })
}

/// The enabled stock launcher: the current Home if it isn't ours, else a known stock launcher still enabled.
pub(crate) fn stock_launcher(installed: &[tvs_device::inventory::PackageInfo], resolved_home: Option<&str>) -> Option<String> {
    let enabled = |n: &str| installed.iter().any(|x| x.name == n && x.enabled);
    if let Some(h) = resolved_home.and_then(|h| h.split('/').next()) {
        if h != STUDIO_PKG && h != "android" && enabled(h) && !tvs_safety::NOT_LAUNCHERS.contains(&h) { return Some(h.to_string()); }
    }
    KNOWN_STOCK_LAUNCHERS.iter().find(|k| enabled(k)).map(|k| k.to_string())
}

/// Plan preview counts for the UI.
pub fn summary(plan: &Plan) -> BTreeMap<&'static str, usize> {
    let mut m = BTreeMap::new();
    for s in &plan.steps {
        let k = match s.preview { Preview::Check(_) => "check", Preview::WillChange(_) => "change", Preview::AlreadyDone(_) => "done", Preview::Blocked(_) => "blocked" };
        *m.entry(k).or_insert(0) += 1;
    }
    m
}
