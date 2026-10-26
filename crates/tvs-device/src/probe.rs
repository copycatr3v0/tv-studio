//! Read-only device inspection. Every query is independent: one that fails is reported in `probe_errors`
//! and leaves its field empty, it never aborts the whole probe or gets guessed.

use crate::oem::{self, OemFamily, OemNote};
use serde::Serialize;
use std::collections::BTreeMap;
use tvs_core::{Component, Serial};
use tvs_transport::{ShellCommand, Transport};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "component", rename_all = "snake_case")]
pub enum HomeState {
    /// A single default Home activity.
    Component(String),
    /// Android's chooser: no default Home is set.
    Chooser,
    /// The TV did not say (old firmware without `cmd`, or the query failed).
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomeCandidate {
    pub component: String,
    /// False for setup wizards, settings apps, FallbackHome placeholders and factory tests.
    pub usable: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Capabilities {
    pub cmd_package: bool,
    pub studio_launcher: bool,
    pub wireless_pairing: bool,
    pub private_dns: bool,
    pub content_write: bool,
    pub install_via_pm: bool,
}

impl Capabilities {
    pub fn from_sdk(sdk: Option<u32>) -> Self {
        let s = sdk.unwrap_or(0);
        Self {
            cmd_package: s >= 24,
            studio_launcher: s >= 26,
            wireless_pairing: s >= 30,
            private_dns: s >= 28,
            content_write: s >= 29,
            install_via_pm: s >= 34,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity { Info, Warning, Blocker }

#[derive(Debug, Clone, Serialize)]
pub struct Finding { pub severity: Severity, pub message: String }

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub serial: Serial,
    pub fingerprint: Option<String>,
    pub brand: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub device: Option<String>,
    pub product: Option<String>,
    pub android_release: Option<String>,
    pub sdk: Option<u32>,
    pub build_id: Option<String>,
    pub incremental: Option<String>,
    pub platform: Option<String>,
    pub adb_secure: Option<bool>,
    pub ram_total_kb: Option<u64>,
    pub ram_available_kb: Option<u64>,
    pub data_total_kb: Option<u64>,
    pub data_free_kb: Option<u64>,
    pub package_count: Option<usize>,
    pub disabled_count: Option<usize>,
    pub home: HomeState,
    pub home_candidates: Vec<HomeCandidate>,
    pub oem: Vec<OemNote>,
    pub capabilities: Capabilities,
    pub findings: Vec<Finding>,
    pub probe_errors: Vec<String>,
}

impl DeviceInfo {
    /// Stable identity for snapshots: the build fingerprint plus the serial's host is not stable
    /// (DHCP), so the fingerprint plus model is used; a TV without a fingerprint falls back to the serial.
    pub fn identity(&self) -> String { identity_from(self.fingerprint.as_deref(), self.model.as_deref(), &self.serial) }
}

pub fn identity_from(fingerprint: Option<&str>, model: Option<&str>, serial: &Serial) -> String {
    match (fingerprint, model) {
        (Some(f), Some(m)) => format!("{m}|{f}"),
        (Some(f), None) => f.to_string(),
        _ => format!("serial:{serial}"),
    }
}

/// Identity from two getprop calls; None when the TV does not answer (never guessed).
pub fn quick_identity(t: &dyn Transport, serial: &Serial) -> Option<String> {
    let get = |k: &'static str| t.shell(serial, &ShellCommand::new("getprop").arg(k)).ok()
        .filter(|o| o.code == Some(0)).map(|o| o.stdout.trim().to_string()).filter(|v| !v.is_empty());
    let fp = get("ro.build.fingerprint")?;
    Some(identity_from(Some(&fp), get("ro.product.model").as_deref(), serial))
}

pub fn parse_getprop(out: &str) -> BTreeMap<String, String> {
    out.lines()
        .filter_map(|l| {
            let l = l.trim();
            let (k, v) = l.strip_prefix('[')?.split_once("]: [")?;
            Some((k.to_string(), v.strip_suffix(']')?.to_string()))
        })
        .collect()
}

pub fn parse_meminfo(out: &str) -> (Option<u64>, Option<u64>) {
    let get = |key: &str| out.lines()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|n| n.parse().ok());
    (get("MemTotal:"), get("MemAvailable:"))
}

fn parse_size_kb(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u64>() { return Some(n); }
    let (num, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: f64 = num.parse().ok()?;
    let mult = match unit { "K" => 1.0, "M" => 1024.0, "G" => 1024.0 * 1024.0, "T" => 1024.0 * 1024.0 * 1024.0, _ => return None };
    Some((n * mult) as u64)
}

/// `df /data` from toybox (`Filesystem 1K-blocks Used Available ...`) or old toolbox (`/data 12G 3G 9G 4096`).
pub fn parse_df(out: &str) -> (Option<u64>, Option<u64>) {
    let mut lines = out.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().unwrap_or_default();
    let Some(row) = lines.next() else { return (None, None) };
    let cols: Vec<&str> = row.split_whitespace().collect();
    if header.contains("1K-blocks") && cols.len() >= 4 {
        (parse_size_kb(cols[1]), parse_size_kb(cols[3]))
    } else if header.contains("Size") && cols.len() >= 4 {
        // toolbox: Filesystem Size Used Free Blksize
        (parse_size_kb(cols[1]), parse_size_kb(cols[3]))
    } else {
        (None, None)
    }
}

fn components(out: &str) -> Vec<String> {
    out.lines().map(str::trim).filter(|l| Component::new(*l).is_ok()).map(str::to_string).collect()
}

pub fn parse_resolve_home(out: &str) -> HomeState {
    match out.lines().map(str::trim).rfind(|l| !l.is_empty()) {
        Some(l) if l.starts_with("android/") => HomeState::Chooser,
        Some(l) if Component::new(l).is_ok() => HomeState::Component(l.to_string()),
        _ => HomeState::Unknown,
    }
}

/// `mHomeProcess: ProcessRecord{1a2b3c 1234:com.google.android.tvlauncher/u0a56}` (pre-Android 7 fallback).
pub fn parse_home_process(out: &str) -> Option<String> {
    let line = out.lines().find(|l| l.trim_start().starts_with("mHomeProcess:"))?;
    let after = line.split(':').nth(2)?;
    let pkg = after.split('/').next()?.trim();
    tvs_core::PackageName::new(pkg).ok().map(|p| p.to_string())
}

fn package_lines(out: &str) -> Vec<String> {
    out.lines().filter_map(|l| l.trim().strip_prefix("package:")).map(str::to_string).collect()
}

const HOME_ARGS: [&str; 4] = ["-a", "android.intent.action.MAIN", "-c", "android.intent.category.HOME"];

fn with_home(cmd: ShellCommand) -> ShellCommand {
    HOME_ARGS.iter().fold(cmd, |c, a| c.arg(a))
}

pub fn probe(t: &dyn Transport, serial: &Serial) -> DeviceInfo {
    let mut errors = Vec::new();
    let mut run = |label: &str, cmd: ShellCommand| -> Option<String> {
        match t.shell(serial, &cmd) {
            Ok(o) if o.code == Some(0) || (o.code.is_none() && !o.stdout.is_empty()) => Some(o.stdout),
            Ok(o) => { errors.push(format!("{label}: {}", o.stderr.trim())); None }
            Err(e) => { errors.push(format!("{label}: {e}")); None }
        }
    };

    let props = run("getprop", ShellCommand::new("getprop")).map(|o| parse_getprop(&o)).unwrap_or_default();
    let p = |k: &str| props.get(k).filter(|v| !v.is_empty()).cloned();
    let sdk = p("ro.build.version.sdk").and_then(|s| s.parse().ok());

    let (ram_total_kb, ram_available_kb) = run("meminfo", ShellCommand::new("cat").arg("/proc/meminfo")).map(|o| parse_meminfo(&o)).unwrap_or((None, None));
    let (data_total_kb, data_free_kb) = run("storage", ShellCommand::new("df").arg("/data")).map(|o| parse_df(&o)).unwrap_or((None, None));

    let all = run("packages", ShellCommand::new("pm").arg("list").arg("packages").arg("--user").arg("0")).map(|o| package_lines(&o));
    let disabled = run("disabled packages", ShellCommand::new("pm").arg("list").arg("packages").arg("-d").arg("--user").arg("0")).map(|o| package_lines(&o));

    let caps = Capabilities::from_sdk(sdk);
    let (home, home_candidates) = if caps.cmd_package {
        let home = run("home", with_home(ShellCommand::new("cmd").arg("package").arg("resolve-activity").arg("--brief")))
            .map(|o| parse_resolve_home(&o)).unwrap_or(HomeState::Unknown);
        let cands = run("home candidates", with_home(ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief")))
            .map(|o| components(&o)).unwrap_or_default()
            .into_iter()
            .map(|c| HomeCandidate { usable: Component::new(c.clone()).map(|x| tvs_safety::is_usable_launcher(&x)).unwrap_or(false), component: c })
            .collect();
        (home, cands)
    } else {
        let home = run("home (dumpsys)", ShellCommand::new("dumpsys").arg("activity").arg("activities"))
            .and_then(|o| parse_home_process(&o))
            .map(|pkg| HomeState::Component(format!("{pkg}/?")))
            .unwrap_or(HomeState::Unknown);
        (home, Vec::new())
    };

    let oem = all.as_deref().map(oem::detect).unwrap_or_default();
    let adb_secure = p("ro.adb.secure").map(|v| v != "0");

    let mut findings = Vec::new();
    let mut add = |severity, message: String| findings.push(Finding { severity, message });
    if oem.iter().any(|n| n.family == OemFamily::FireOs) {
        add(Severity::Blocker, "Fire OS detected: Home replacement and most changes are not supported. Inspection only.".into());
    }
    match sdk {
        Some(s) if s < 26 => add(Severity::Warning, format!("Android API {s}: the Studio launcher needs Android 8.0 (API 26). ADB-only mode is available.")),
        None => add(Severity::Warning, "Android version could not be read; changes stay disabled until it can.".into()),
        _ => {}
    }
    if sdk.is_some_and(|s| s < 24) {
        add(Severity::Warning, "No `cmd package` on this firmware (before Android 7): the default Home cannot be checked reliably, so the stock launcher will never be disabled.".into());
    }
    if adb_secure == Some(false) {
        add(Severity::Warning, "ro.adb.secure=0: network ADB accepts any device on your network without a prompt. Turn debugging off when you finish.".into());
    }
    if matches!(home, HomeState::Chooser) {
        add(Severity::Info, "No default Home is set; the TV asks which launcher to use.".into());
    }
    if !home_candidates.is_empty() && !home_candidates.iter().any(|c| c.usable) {
        add(Severity::Warning, "No usable launcher answers Home right now.".into());
    }
    if ram_total_kb.is_some_and(|k| k < 1_200_000) {
        add(Severity::Info, "About 1 GB of RAM: lightweight themes and reduced motion are recommended.".into());
    }

    DeviceInfo {
        serial: serial.clone(),
        fingerprint: p("ro.build.fingerprint"),
        brand: p("ro.product.brand"),
        manufacturer: p("ro.product.manufacturer"),
        model: p("ro.product.model"),
        device: p("ro.product.device"),
        product: p("ro.product.name"),
        android_release: p("ro.build.version.release"),
        sdk,
        build_id: p("ro.build.id"),
        incremental: p("ro.build.version.incremental"),
        platform: p("ro.board.platform"),
        adb_secure,
        ram_total_kb,
        ram_available_kb,
        data_total_kb,
        data_free_kb,
        package_count: all.as_ref().map(Vec::len),
        disabled_count: disabled.as_ref().map(Vec::len),
        home,
        home_candidates,
        oem,
        capabilities: caps,
        findings,
        probe_errors: errors,
    }
}
