use crate::probe::*;
use crate::OemFamily;
use tvs_core::Serial;
use tvs_transport::mock::{MockDevice, MockTransport};
use tvs_transport::TransportError;

const MEMINFO: &str = "MemTotal:        1956324 kB\nMemFree:          112340 kB\nMemAvailable:     803112 kB\nBuffers:            9876 kB\n";
const DF_TOYBOX: &str = "Filesystem     1K-blocks    Used Available Use% Mounted on\n/dev/block/dm-5  10335916 6201544   4134372  60% /data\n";
const DF_TOOLBOX: &str = "Filesystem               Size     Used     Free   Blksize\n/data                    4.4G     2.1G     2.3G   4096\n";

fn google_tv() -> MockDevice {
    let mut d = MockDevice::default();
    for (k, v) in [
        ("ro.product.brand", "TCL"), ("ro.product.manufacturer", "TCL"), ("ro.product.model", "Smart TV Pro"),
        ("ro.product.device", "G09"), ("ro.product.name", "G09_4K_GB"), ("ro.build.version.release", "12"),
        ("ro.build.version.sdk", "31"), ("ro.build.fingerprint", "TCL/G09_4K_GB/G09:12/STT2.230203.001/AS24:user/release-keys"),
        ("ro.board.platform", "t5m"), ("ro.adb.secure", "1"),
    ] { d.props.insert(k.into(), v.into()); }
    for p in ["com.google.android.apps.tv.launcherx", "com.google.android.tungsten.setupwraith", "com.tcl.tv", "com.tcl.logkit"] {
        d.packages.insert(p.into(), true);
    }
    d.packages.insert("com.tcl.ttvs".into(), false);
    d.text_files.insert("/proc/meminfo".into(), MEMINFO.into());
    d.df_data = Some(DF_TOYBOX.into());
    d.home = Some("com.google.android.apps.tv.launcherx/com.google.android.apps.tv.launcherx.home.HomeActivity".into());
    d.home_claimants = vec![
        "com.google.android.apps.tv.launcherx/com.google.android.apps.tv.launcherx.home.HomeActivity".into(),
        "com.google.android.tungsten.setupwraith/.RecoveryActivity".into(),
    ];
    d
}

fn s(x: &str) -> Serial { Serial::new(x).unwrap() }

#[test]
fn full_probe_google_tv() {
    let m = MockTransport::new();
    m.add_device("192.168.1.50:5555", google_tv());
    let i = probe(&m, &s("192.168.1.50:5555"));
    assert!(i.probe_errors.is_empty(), "{:?}", i.probe_errors);
    assert_eq!(i.model.as_deref(), Some("Smart TV Pro"));
    assert_eq!(i.sdk, Some(31));
    assert_eq!(i.ram_total_kb, Some(1_956_324));
    assert_eq!(i.ram_available_kb, Some(803_112));
    assert_eq!(i.data_total_kb, Some(10_335_916));
    assert_eq!(i.data_free_kb, Some(4_134_372));
    assert_eq!(i.package_count, Some(5));
    assert_eq!(i.disabled_count, Some(1));
    assert!(matches!(&i.home, HomeState::Component(c) if c.starts_with("com.google.android.apps.tv.launcherx/")));
    assert_eq!(i.home_candidates.len(), 2);
    assert!(i.home_candidates[0].usable);
    assert!(!i.home_candidates[1].usable, "setupwraith is not a launcher");
    let fams: Vec<_> = i.oem.iter().map(|n| n.family).collect();
    assert_eq!(fams, vec![OemFamily::GoogleTv, OemFamily::Tcl]);
    assert!(i.capabilities.studio_launcher && i.capabilities.wireless_pairing && i.capabilities.content_write);
    assert!(!i.capabilities.install_via_pm);
    assert_eq!(i.adb_secure, Some(true));
    assert_eq!(i.identity(), "Smart TV Pro|TCL/G09_4K_GB/G09:12/STT2.230203.001/AS24:user/release-keys");
}

#[test]
fn failed_queries_are_reported_not_guessed() {
    let m = MockTransport::new();
    let mut d = google_tv();
    d.text_files.clear();
    d.df_data = None;
    m.add_device("tv:5555", d);
    m.fail_next("query-activities", TransportError::Timeout(30_000));
    let i = probe(&m, &s("tv:5555"));
    assert_eq!(i.ram_total_kb, None);
    assert_eq!(i.data_total_kb, None);
    assert!(i.home_candidates.is_empty());
    assert_eq!(i.probe_errors.len(), 3, "{:?}", i.probe_errors);
    assert!(i.probe_errors.iter().any(|e| e.starts_with("home candidates")));
}

#[test]
fn old_and_insecure_firmware_findings() {
    let m = MockTransport::new();
    let mut d = google_tv();
    d.props.insert("ro.build.version.sdk".into(), "25".into());
    d.props.insert("ro.adb.secure".into(), "0".into());
    m.add_device("tv:5555", d);
    let i = probe(&m, &s("tv:5555"));
    assert!(!i.capabilities.studio_launcher);
    assert!(i.capabilities.cmd_package);
    let text: Vec<_> = i.findings.iter().map(|f| f.message.as_str()).collect();
    assert!(text.iter().any(|t| t.contains("API 25")));
    assert!(text.iter().any(|t| t.contains("ro.adb.secure=0")));
}

#[test]
fn chooser_and_fire_os() {
    let m = MockTransport::new();
    let mut d = google_tv();
    d.home = None;
    d.packages.insert("com.amazon.tv.launcher".into(), true);
    m.add_device("tv:5555", d);
    let i = probe(&m, &s("tv:5555"));
    assert_eq!(i.home, HomeState::Chooser);
    assert!(i.findings.iter().any(|f| matches!(f.severity, Severity::Blocker)));
}

#[test]
fn unauthorized_device_probe_reports_errors() {
    let m = MockTransport::new();
    let mut d = google_tv();
    d.state = Some(tvs_transport::DeviceState::Unauthorized);
    m.add_device("tv:5555", d);
    let i = probe(&m, &s("tv:5555"));
    assert_eq!(i.sdk, None);
    assert!(!i.probe_errors.is_empty());
    assert_eq!(i.identity(), "serial:tv:5555");
}

#[test]
fn parsers() {
    assert_eq!(parse_df(DF_TOOLBOX).0, Some((4.4 * 1024.0 * 1024.0) as u64));
    assert_eq!(parse_df("garbage"), (None, None));
    assert_eq!(parse_resolve_home("No activity found\n"), HomeState::Unknown);
    assert_eq!(
        parse_home_process("  mHomeProcess: ProcessRecord{1a2b3c 1234:com.google.android.tvlauncher/u0a56}\n"),
        Some("com.google.android.tvlauncher".into())
    );
    let props = parse_getprop("[ro.a]: [x]\n[ro.empty]: []\nnoise\n");
    assert_eq!(props.get("ro.a").map(String::as_str), Some("x"));
    assert_eq!(props.get("ro.empty").map(String::as_str), Some(""));
}
