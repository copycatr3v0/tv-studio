//! A scriptable fake TV. It models the state the app reads and changes (packages per user 0, settings,
//! Home, props) and supports injected failures, so whole workflows can be tested without hardware.

use crate::{DeviceEntry, DeviceState, MdnsService, PairingCode, Result, ShellCommand, ShellOutput, Transport, TransportError};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::Path;
use std::sync::Mutex;
use tvs_core::Serial;

#[derive(Debug, Clone, Default)]
pub struct MockDevice {
    pub state: Option<DeviceState>,
    pub props: BTreeMap<String, String>,
    /// package -> enabled for user 0
    pub packages: BTreeMap<String, bool>,
    /// Packages whose `pm disable-user` the firmware refuses (SecurityException).
    pub refuse_disable: BTreeSet<String>,
    /// Packages the firmware silently re-enables (command "succeeds", state doesn't change).
    pub sticky: BTreeSet<String>,
    pub settings_global: BTreeMap<String, String>,
    pub home: Option<String>,
    pub home_claimants: Vec<String>,
    pub files: BTreeMap<String, Vec<u8>>,
    /// Text returned by `cat <path>` (e.g. /proc/meminfo).
    pub text_files: BTreeMap<String, String>,
    /// Output of `df /data`.
    pub df_data: Option<String>,
    /// `dumpsys <args...>` output keyed by the joined args, e.g. "wifi" or "package com.x".
    pub dumpsys: BTreeMap<String, String>,
    /// `cmd package query-activities` answers keyed by the `-a` action (HOME uses `home_claimants`).
    pub activity_queries: BTreeMap<String, Vec<String>>,
    /// Packages that are system apps (`pm list packages -s`).
    pub system: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Injected {
    Error(TransportError),
}

#[derive(Default)]
struct Inner {
    devices: BTreeMap<String, MockDevice>,
    /// Endpoints that accept `adb connect`, and the device each one brings online.
    reachable: HashMap<String, MockDevice>,
    pair_codes: HashMap<String, String>,
    mdns: Vec<MdnsService>,
    /// Errors returned for the next calls whose shell string contains the key.
    failures: VecDeque<(String, Injected)>,
    log: Vec<String>,
}

#[derive(Default)]
pub struct MockTransport { inner: Mutex<Inner> }

impl MockTransport {
    pub fn new() -> Self { Self::default() }

    pub fn add_device(&self, serial: &str, dev: MockDevice) {
        self.inner.lock().unwrap().devices.insert(serial.into(), dev);
    }
    pub fn add_reachable(&self, endpoint: &str, dev: MockDevice) {
        self.inner.lock().unwrap().reachable.insert(endpoint.into(), dev);
    }
    pub fn set_pair_code(&self, endpoint: &str, code: &str) {
        self.inner.lock().unwrap().pair_codes.insert(endpoint.into(), code.into());
    }
    pub fn set_mdns(&self, services: Vec<MdnsService>) { self.inner.lock().unwrap().mdns = services; }
    /// The next shell call whose command contains `needle` fails with `err`.
    pub fn fail_next(&self, needle: &str, err: TransportError) {
        self.inner.lock().unwrap().failures.push_back((needle.into(), Injected::Error(err)));
    }
    pub fn device(&self, serial: &str) -> Option<MockDevice> { self.inner.lock().unwrap().devices.get(serial).cloned() }
    pub fn log(&self) -> Vec<String> { self.inner.lock().unwrap().log.clone() }
    /// Simulate a reboot / Wi-Fi drop: the device leaves the device list until reconnected.
    pub fn drop_device(&self, serial: &str) {
        let mut g = self.inner.lock().unwrap();
        if let Some(d) = g.devices.remove(serial) { g.reachable.insert(serial.into(), d); }
    }
}

fn ok(stdout: impl Into<String>) -> Result<ShellOutput> {
    Ok(ShellOutput { stdout: stdout.into(), stderr: String::new(), code: Some(0), elapsed_ms: 1 })
}
fn fail(stderr: impl Into<String>, code: i32) -> Result<ShellOutput> {
    Ok(ShellOutput { stdout: String::new(), stderr: stderr.into(), code: Some(code), elapsed_ms: 1 })
}

impl Transport for MockTransport {
    fn devices(&self) -> Result<Vec<DeviceEntry>> {
        let g = self.inner.lock().unwrap();
        Ok(g.devices.iter().map(|(s, d)| DeviceEntry {
            serial: Serial::new(s.clone()).expect("mock serial"),
            state: d.state.unwrap_or(DeviceState::Device),
            model: d.props.get("ro.product.model").cloned(),
            product: d.props.get("ro.product.name").cloned(),
        }).collect())
    }

    fn connect(&self, endpoint: &Serial) -> Result<()> {
        let mut g = self.inner.lock().unwrap();
        g.log.push(format!("connect {endpoint}"));
        if g.devices.contains_key(endpoint.as_str()) { return Ok(()); }
        match g.reachable.remove(endpoint.as_str()) {
            Some(d) => { g.devices.insert(endpoint.to_string(), d); Ok(()) }
            None => Err(TransportError::Refused(format!("cannot connect to {endpoint}"))),
        }
    }

    fn pair(&self, endpoint: &Serial, code: &PairingCode) -> Result<()> {
        let g = self.inner.lock().unwrap();
        match g.pair_codes.get(endpoint.as_str()) {
            Some(c) if c == code.as_str() => Ok(()),
            _ => Err(TransportError::PairingFailed("Failed: Wrong password or connection was dropped.".into())),
        }
    }

    fn disconnect(&self, endpoint: &Serial) -> Result<()> {
        self.drop_device(endpoint.as_str());
        Ok(())
    }

    fn mdns_services(&self) -> Result<Vec<MdnsService>> { Ok(self.inner.lock().unwrap().mdns.clone()) }

    fn shell(&self, serial: &Serial, cmd: &ShellCommand) -> Result<ShellOutput> {
        let line = cmd.to_shell_string();
        let mut g = self.inner.lock().unwrap();
        g.log.push(format!("{serial}: {line}"));
        if let Some(pos) = g.failures.iter().position(|(n, _)| line.contains(n.as_str())) {
            let (_, Injected::Error(e)) = g.failures.remove(pos).unwrap();
            return Err(e);
        }
        let dev = g.devices.get_mut(serial.as_str()).ok_or_else(|| TransportError::NoDevice(serial.to_string()))?;
        if dev.state == Some(DeviceState::Unauthorized) { return Err(TransportError::Unauthorized); }
        let t: Vec<&str> = cmd.tokens().iter().map(String::as_str).collect();
        match t.as_slice() {
            ["getprop"] => ok(dev.props.iter().map(|(k, v)| format!("[{k}]: [{v}]\n")).collect::<String>()),
            ["cat", path] => match dev.text_files.get(*path) {
                Some(t) => ok(t.clone()),
                None => fail(format!("cat: {path}: No such file or directory"), 1),
            },
            ["df", "/data"] => match &dev.df_data { Some(t) => ok(t.clone()), None => fail("df: /data: unsupported", 1) },
            ["getprop", key] => ok(format!("{}\n", dev.props.get(*key).cloned().unwrap_or_default())),
            ["pm", "list", "packages", rest @ ..] => {
                let want = |en: bool| if rest.contains(&"-d") { !en } else if rest.contains(&"-e") { en } else { true };
                let sys = rest.contains(&"-s");
                ok(dev.packages.iter().filter(|(p, &en)| want(en) && (!sys || dev.system.contains(*p))).map(|(p, _)| format!("package:{p}\n")).collect::<String>())
            }
            ["dumpsys", args @ ..] => match dev.dumpsys.get(&args.join(" ")) {
                Some(t) => ok(t.clone()),
                None => fail(format!("mock: no dumpsys {}", args.join(" ")), 1),
            },
            ["pm", "disable-user", "--user", "0", pkg] => {
                if !dev.packages.contains_key(*pkg) { return fail(format!("Unknown package: {pkg}"), 1); }
                if dev.refuse_disable.contains(*pkg) {
                    return fail(format!("java.lang.SecurityException: Shell cannot change component state for {pkg}"), 255);
                }
                if !dev.sticky.contains(*pkg) { dev.packages.insert((*pkg).into(), false); }
                ok(format!("Package {pkg} new state: disabled-user\n"))
            }
            ["pm", "enable", pkg] | ["pm", "enable", "--user", "0", pkg] => {
                if !dev.packages.contains_key(*pkg) { return fail(format!("Unknown package: {pkg}"), 1); }
                dev.packages.insert((*pkg).into(), true);
                ok(format!("Package {pkg} new state: enabled\n"))
            }
            ["settings", "get", "global", key] => ok(format!("{}\n", dev.settings_global.get(*key).cloned().unwrap_or_else(|| "null".into()))),
            ["settings", "put", "global", key, val] => { dev.settings_global.insert((*key).into(), (*val).into()); ok("") }
            ["settings", "delete", "global", key] => { dev.settings_global.remove(*key); ok("Deleted 1 rows\n") }
            ["cmd", "package", "resolve-activity", "--brief", ..] => {
                let home = dev.home.clone().unwrap_or_else(|| "android/com.android.internal.app.ResolverActivity".into());
                ok(format!("priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true\n{home}\n"))
            }
            ["cmd", "package", "query-activities", "--brief", rest @ ..] => {
                let action = rest.iter().position(|a| *a == "-a").and_then(|i| rest.get(i + 1)).copied().unwrap_or_default();
                let list = if rest.contains(&"android.intent.category.HOME") { Some(&dev.home_claimants) } else { dev.activity_queries.get(action) };
                match list {
                    Some(l) => ok(l.iter().map(|c| format!("{c}\n")).collect::<String>()),
                    None => ok("No activities found\n"),
                }
            }
            ["cmd", "package", "set-home-activity", "--user", "0", comp] => {
                let pkg = comp.split('/').next().unwrap_or_default();
                if dev.packages.get(pkg) == Some(&true) { dev.home = Some((*comp).into()); }
                ok("Success\n")
            }
            _ => fail(format!("mock: unsupported command: {line}"), 127),
        }
    }

    fn push(&self, serial: &Serial, local: &Path, remote: &str) -> Result<()> {
        let data = std::fs::read(local).map_err(|e| TransportError::Io(e.to_string()))?;
        let mut g = self.inner.lock().unwrap();
        let dev = g.devices.get_mut(serial.as_str()).ok_or_else(|| TransportError::NoDevice(serial.to_string()))?;
        dev.files.insert(remote.into(), data);
        Ok(())
    }

    fn pull(&self, serial: &Serial, remote: &str, local: &Path) -> Result<()> {
        let g = self.inner.lock().unwrap();
        let dev = g.devices.get(serial.as_str()).ok_or_else(|| TransportError::NoDevice(serial.to_string()))?;
        let data = dev.files.get(remote).ok_or_else(|| TransportError::Io(format!("remote object '{remote}' does not exist")))?;
        std::fs::write(local, data).map_err(|e| TransportError::Io(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvs_core::PackageName;

    fn tv() -> MockDevice {
        let mut d = MockDevice::default();
        d.props.insert("ro.product.model".into(), "Smart TV Pro".into());
        d.props.insert("ro.build.version.sdk".into(), "31".into());
        for p in ["com.tcl.logkit", "com.google.android.apps.tv.launcherx", "com.tcl.firm"] { d.packages.insert(p.into(), true); }
        d.refuse_disable.insert("com.tcl.firm".into());
        d.home = Some("com.google.android.apps.tv.launcherx/.home.HomeActivity".into());
        d
    }
    fn s(x: &str) -> Serial { Serial::new(x).unwrap() }
    fn disable(p: &str) -> ShellCommand {
        ShellCommand::new("pm").arg("disable-user").arg("--user").arg("0").package(&PackageName::new(p).unwrap())
    }

    #[test]
    fn connect_then_disable_and_readback() {
        let m = MockTransport::new();
        m.add_reachable("192.168.1.50:5555", tv());
        assert!(m.connect(&s("192.168.1.99:5555")).is_err());
        m.connect(&s("192.168.1.50:5555")).unwrap();
        let sn = s("192.168.1.50:5555");
        assert_eq!(m.devices().unwrap()[0].model.as_deref(), Some("Smart TV Pro"));
        m.shell(&sn, &disable("com.tcl.logkit")).unwrap();
        let d = m.shell(&sn, &ShellCommand::new("pm").arg("list").arg("packages").arg("-d")).unwrap();
        assert_eq!(d.stdout, "package:com.tcl.logkit\n");
    }

    #[test]
    fn refused_disable_is_reported_not_applied() {
        let m = MockTransport::new();
        m.add_device("tv:5555", tv());
        let out = m.shell(&s("tv:5555"), &disable("com.tcl.firm")).unwrap();
        assert_ne!(out.code, Some(0));
        assert!(m.device("tv:5555").unwrap().packages["com.tcl.firm"]);
    }

    #[test]
    fn injected_failure_and_reboot() {
        let m = MockTransport::new();
        m.add_device("tv:5555", tv());
        m.fail_next("disable-user", TransportError::Timeout(30000));
        assert_eq!(m.shell(&s("tv:5555"), &disable("com.tcl.logkit")).unwrap_err(), TransportError::Timeout(30000));
        m.drop_device("tv:5555");
        assert!(matches!(m.shell(&s("tv:5555"), &disable("com.tcl.logkit")), Err(TransportError::NoDevice(_))));
        m.connect(&s("tv:5555")).unwrap();
        assert!(m.shell(&s("tv:5555"), &disable("com.tcl.logkit")).is_ok());
    }

    #[test]
    fn unauthorized_device() {
        let m = MockTransport::new();
        let mut d = tv();
        d.state = Some(DeviceState::Unauthorized);
        m.add_device("tv:5555", d);
        assert_eq!(m.shell(&s("tv:5555"), &ShellCommand::new("getprop")).unwrap_err(), TransportError::Unauthorized);
    }

    #[test]
    fn set_home_success_does_not_imply_change() {
        let m = MockTransport::new();
        m.add_device("tv:5555", tv());
        let comp = tvs_core::Component::new("studio.tvcustomizer.home/.MainActivity").unwrap();
        let out = m.shell(&s("tv:5555"), &ShellCommand::new("cmd").arg("package").arg("set-home-activity").arg("--user").arg("0").component(&comp)).unwrap();
        assert!(out.stdout.contains("Success"));
        // Not installed, so Home did not move: callers must verify, never trust "Success".
        assert_ne!(m.device("tv:5555").unwrap().home.as_deref(), Some(comp.as_str()));
    }

    #[test]
    fn pairing() {
        let m = MockTransport::new();
        m.set_pair_code("192.168.1.50:37123", "482913");
        assert!(m.pair(&s("192.168.1.50:37123"), &PairingCode::new("000000").unwrap()).is_err());
        assert!(m.pair(&s("192.168.1.50:37123"), &PairingCode::new("482913").unwrap()).is_ok());
    }
}
