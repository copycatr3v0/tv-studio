use crate::bench::{run, BenchConfig, Pacer};
use crate::privacy::{audit, Category, MicHistory};
use std::time::Duration;
use tvs_core::Serial;
use tvs_transport::mock::{MockDevice, MockTransport};

const SN: &str = "tv:5555";
const HOME: &str = "com.google.android.apps.tv.launcherx/.home.HomeActivity";
fn s() -> Serial { Serial::new(SN).unwrap() }
struct NoWait;
impl Pacer for NoWait { fn pause(&mut self, _d: Duration) {} }

fn tv() -> MockDevice {
    let mut d = MockDevice::default();
    let pkg = "com.google.android.apps.tv.launcherx";
    for p in [pkg, "com.android.tv.settings", "com.google.android.katniss", "tv.inscape.tvclient.inscapeacr", "com.tcl.logkit", "com.vendor.voiceagent"] { d.packages.insert(p.into(), true); }
    d.home = Some(HOME.into());
    d.home_claimants = vec![HOME.into()];
    d.activity_queries.insert("android.settings.SETTINGS".into(), vec!["com.android.tv.settings/.MainSettings".into()]);
    d.dumpsys.insert(format!("gfxinfo {pkg} reset"), "Uptime: 5000 Realtime: 9000\n".into());
    d.dumpsys.insert(format!("gfxinfo {pkg} framestats"), "Uptime: 6000 Realtime: 10000\nTotal frames rendered: 3\nJanky frames: 0 (0.00%)\n---PROFILEDATA---\nFlags,IntendedVsync,FrameCompleted\n0,5004000000,5018000000\n0,5020000000,5034000000\n---PROFILEDATA---\n".into());
    d.dumpsys.insert(format!("meminfo {pkg}"), "** MEMINFO in pid 4321 **\n        TOTAL PSS:   153600            TOTAL RSS: 0\n".into());
    d.dumpsys.insert(format!("package {pkg}"), "versionName=1.2.3\n".into());
    d.pids.insert(pkg.into(), 4321);
    d.text_files.insert("/proc/stat".into(), "cpu  100 0 50 800 50 0 0 0 0 0\n".into());
    d.text_files.insert("/proc/4321/stat".into(), "4321 (launcherx) S 1 1 0 0 -1 0 0 0 0 0 10 5 0 0 20 0\n".into());
    d.text_files.insert("/proc/meminfo".into(), "MemTotal: 2000000 kB\nMemAvailable: 819200 kB\n".into());
    d.cold_start_ms = 640;
    d.logcat = "  1761480001.000 1000 1000 I Input: injectKeyEvent: KeyEvent { action=ACTION_DOWN, keyCode=KEYCODE_HOME }\n  1761480001.380 1000 1000 I wm_on_resume_called: [1,com.google.android.apps.tv.launcherx.home.HomeActivity,RESUME]\n".into();
    for p in ["com.google.android.katniss", "tv.inscape.tvclient.inscapeacr", "com.tcl.logkit", "com.vendor.voiceagent"] {
        d.dumpsys.insert(format!("package {p}"), "    userId=10123\n    requested permissions:\n      android.permission.INTERNET\n".into());
    }
    d.dumpsys.insert("package com.google.android.katniss".into(), "    userId=10050\n    requested permissions:\n      android.permission.RECORD_AUDIO\n    install permissions:\n".into());
    d.appops.insert("com.google.android.katniss".into(), "RECORD_AUDIO: allow; time=+2h3m0s12ms ago; duration=+1s".into());
    d.settings_secure.insert("voice_interaction_service".into(), "com.google.android.katniss/.VoiceService".into());
    d
}

#[test]
fn benchmark_measures_every_metric() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let cfg = BenchConfig { keys: 4, cold_starts: 2, home_returns: 2, idle_seconds: 10, ..Default::default() };
    let r = run(&m, &s(), &cfg, "2026-10-26T12:00:00Z", &mut NoWait).unwrap();
    assert_eq!(r.launcher.as_deref(), Some("com.google.android.apps.tv.launcherx"));
    assert_eq!(r.launcher_version.as_deref(), Some("1.2.3"));
    assert_eq!(r.key_response_ms, vec![18.0; 4], "timed from the reset uptime (no key log lines)");
    assert_eq!(r.cold_start_ms, vec![640.0, 640.0]);
    assert!(r.home_return_ms.is_empty(), "logcat -c before each HOME cleared the canned log");
    assert_eq!(r.launcher_pss_mb, Some(150.0));
    assert_eq!(r.mem_available_mb, Some(800.0));
    assert!(r.notes.iter().any(|n| n.contains("doesn't log injected keys")));
    // Nothing changed on the TV: no package, setting or Home change.
    assert!(!m.log().iter().any(|l| l.contains("disable-user") || l.contains("settings put") || l.contains("set-home-activity")));
}

#[test]
fn benchmark_without_a_default_home_measures_nothing() {
    let m = MockTransport::new();
    m.add_device(SN, { let mut d = tv(); d.home = None; d });
    let r = run(&m, &s(), &BenchConfig::default(), "t", &mut NoWait).unwrap();
    assert!(r.key_response_ms.is_empty() && r.launcher.is_none());
    assert!(r.notes[0].contains("No single default Home"));
}

#[test]
fn privacy_audit_grades_evidence() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let a = audit(&m, &s()).unwrap();
    let e = |p: &str| a.entries.iter().find(|x| x.package == p).unwrap_or_else(|| panic!("{p} missing"));
    let katniss = e("com.google.android.katniss");
    assert_eq!(katniss.category, Category::Voice);
    assert_eq!(katniss.mic_history, MicHistory::Recorded);
    assert!(katniss.evidence.iter().any(|x| x.contains("named by the TV")));
    assert!(katniss.evidence.iter().any(|x| x.contains("asking is not recording")));
    assert_eq!(e("tv.inscape.tvclient.inscapeacr").category, Category::Acr);
    let hinted = e("com.vendor.voiceagent");
    assert_eq!(hinted.category, Category::Unknown);
    assert!(hinted.note.contains("name only"));
    // Never "listening".
    assert!(!a.entries.iter().flat_map(|x| x.evidence.iter().chain(std::iter::once(&x.note))).any(|t| t.to_lowercase().contains("listening")));
}
