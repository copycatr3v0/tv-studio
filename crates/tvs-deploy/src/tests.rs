use crate::*;
use std::time::Duration;
use tvs_core::Serial;
use tvs_launcher::theme::ThemeBundle;
use tvs_recovery::Snapshot;
use tvs_transport::mock::{MockDevice, MockTransport};
use tvs_transport::TransportError;

const SN: &str = "tv:5555";
const STOCK: &str = "com.google.android.apps.tv.launcherx/.home.HomeActivity";
fn s() -> Serial { Serial::new(SN).unwrap() }

fn tv() -> MockDevice {
    let mut d = MockDevice::default();
    d.props.insert("ro.build.fingerprint".into(), "TCL/G09/G09:12/STT2/AS24:user/release-keys".into());
    d.props.insert("ro.product.model".into(), "Smart TV Pro".into());
    d.props.insert("ro.build.version.sdk".into(), "31".into());
    for p in ["com.google.android.apps.tv.launcherx", "com.tcl.logkit", "com.tcl.ttvs", "com.tcl.waterfall", "com.google.android.katniss", "com.android.tv.settings", "com.tcl.tv"] {
        d.packages.insert(p.into(), true);
        d.dumpsys.insert(format!("package {p}"), "requested permissions:\n  android.permission.INTERNET\n".into());
    }
    d.home = Some(STOCK.into());
    d.home_claimants = vec![STOCK.into()];
    d.focused = Some(STOCK.into());
    d.activity_queries.insert("android.intent.action.VIEW".into(), vec!["com.tcl.tv/.Passthrough".into()]);
    d.activity_queries.insert("android.settings.SETTINGS".into(), vec!["com.android.tv.settings/.MainSettings".into()]);
    d
}

#[derive(Default)]
struct TestHooks { events: Vec<Event>, journal: Vec<(String, String)>, snapshots: Vec<Snapshot>, cancel_after: Option<usize>, gave_up_reconnect: bool }
impl Hooks for TestHooks {
    fn event(&mut self, e: &Event) { self.events.push(e.clone()); }
    fn journal(&mut self, step: &str, phase: &str, _d: Option<&str>) { self.journal.push((step.into(), phase.into())); }
    fn save_snapshot(&mut self, _id: &str, snap: &Snapshot) -> Result<(), String> { self.snapshots.push(snap.clone()); Ok(()) }
    fn pause(&mut self, _d: Duration) {}
    fn cancelled(&mut self) -> bool { self.cancel_after.is_some_and(|n| self.journal.iter().filter(|j| j.1 == "verified" || j.1 == "skipped").count() >= n) }
    fn now(&mut self) -> String { "2026-10-26T12:00:00Z".into() }
    fn reconnect_policy(&self) -> (u32, Duration) { (if self.gave_up_reconnect { 1 } else { 3 }, Duration::ZERO) }
}

fn apk() -> (tempfile::TempDir, std::path::PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("studio-home.apk");
    std::fs::write(&p, b"PK\x03\x04apk").unwrap();
    (d, p)
}

fn everything(apk: &std::path::Path) -> DeployInput {
    let mut p = Profile::new("Living room");
    p.launcher = LauncherPlan { install: true, make_home: true, disable_stock: true };
    p.debloat.packages = vec!["com.tcl.logkit".into(), "com.tcl.ttvs".into(), "com.tcl.waterfall".into()];
    p.settings.animation_scale = Some("0.5".into());
    p.settings.private_dns = PrivateDns::Hostname("dns.adguard-dns.com".into());
    p.apply_theme = true;
    DeployInput {
        profile: p,
        theme: Some(ThemeBundle { config: serde_json::json!({"columns": 5, "background_color": "#070A14"}), wallpapers: vec![], captions: None }),
        apk: Some(apk.to_path_buf()), batch_size: 2, settle: Duration::ZERO,
    }
}

#[test]
fn dry_run_previews_every_step_and_changes_nothing() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let (_d, a) = apk();
    let plan = plan(&m, &s(), &everything(&a)).unwrap();
    assert!(plan.runnable, "{:#?}", plan.steps);
    let ids: Vec<_> = plan.steps.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, vec!["identify", "snapshot", "launcher-install", "launcher-home", "debloat-1", "debloat-2", "settings-animation", "settings-dns", "theme", "launcher-disable-stock", "verify"]);
    assert!(plan.notes.iter().any(|n| n.contains("router")));
    assert!(!m.log().iter().any(|l| l.contains("disable-user") || l.contains("pm install") || l.contains("settings put")));
}

#[test]
fn full_run_applies_and_verifies_everything_snapshot_first() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let (_d, a) = apk();
    let input = everything(&a);
    let plan = plan(&m, &s(), &input).unwrap();
    let mut h = TestHooks::default();
    let r = run(&m, &s(), &plan, &input, &mut h);
    assert!(r.completed, "{:#?}", r.results);
    assert_eq!(h.snapshots.len(), 1);
    let d = m.device(SN).unwrap();
    for p in ["com.tcl.logkit", "com.tcl.ttvs", "com.tcl.waterfall", "com.google.android.apps.tv.launcherx"] { assert!(!d.packages[p], "{p}"); }
    assert_eq!(d.settings_global["window_animation_scale"], "0.5");
    assert_eq!(d.settings_global["private_dns_mode"], "hostname");
    assert!(d.focused.as_deref().unwrap().starts_with("studio.tvcustomizer.home/"));
    // The snapshot was taken before the first change reached the TV.
    let log = m.log();
    let first_change = log.iter().position(|l| l.contains("pm install") || l.contains("disable-user") || l.contains("settings put")).unwrap();
    let snapshot_read = log.iter().position(|l| l.contains("settings get global private_dns_specifier")).unwrap();
    assert!(snapshot_read < first_change);
    // Progress events are derived from finished steps.
    assert_eq!(h.events.last().unwrap().total, plan.steps.len());
}

#[test]
fn launcher_in_app_list_blocks_the_plan_and_nothing_runs() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let (_d, a) = apk();
    let mut input = everything(&a);
    input.profile.launcher.disable_stock = false;
    input.profile.debloat.packages.push("com.google.android.apps.tv.launcherx".into());
    let plan = plan(&m, &s(), &input).unwrap();
    assert!(!plan.runnable);
    assert!(plan.steps.iter().any(|x| matches!(&x.preview, Preview::Blocked(m) if m.contains("is a launcher"))));
    let r = run(&m, &s(), &plan, &input, &mut TestHooks::default());
    assert!(r.stopped.is_some() && r.results.is_empty());
    assert!(!m.log().iter().any(|l| l.contains("disable-user") || l.contains("pm install")));
}

#[test]
fn sensitive_apps_need_their_confirmation() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.google.android.katniss".into()];
    // katniss is not "essential", so it's allowed; Play-store-like essentials need confirmation:
    m.add_device(SN, { let mut d = tv(); d.packages.insert("com.android.vending".into(), true); d.dumpsys.insert("package com.android.vending".into(), "requested permissions:\n".into()); d });
    input.profile.debloat.packages.push("com.android.vending".into());
    assert!(!plan(&m, &s(), &input).unwrap().runnable);
    input.profile.debloat.confirmed = vec!["com.android.vending".into()];
    assert!(plan(&m, &s(), &input).unwrap().runnable);
}

#[test]
fn unconfirmed_home_skips_stock_disable_but_independent_steps_finish() {
    let m = MockTransport::new();
    m.add_device(SN, { let mut d = tv(); d.ignore_set_home = true; d });
    let (_d, a) = apk();
    let input = everything(&a);
    let plan = plan(&m, &s(), &input).unwrap();
    let r = run(&m, &s(), &plan, &input, &mut TestHooks::default());
    let st = |id: &str| r.results.iter().find(|x| x.id == id).unwrap().status;
    assert_eq!(st("launcher-home"), StepStatus::Failed);
    assert_eq!(st("launcher-disable-stock"), StepStatus::Skipped);
    assert_eq!(st("debloat-2"), StepStatus::Done);
    assert_eq!(st("settings-dns"), StepStatus::Done);
    assert!(!r.completed);
    assert!(m.device(SN).unwrap().packages["com.google.android.apps.tv.launcherx"], "stock launcher left enabled");
}

#[test]
fn dropped_connection_reconnects_and_finishes_the_step_once() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.tcl.logkit".into(), "com.tcl.ttvs".into()];
    let plan = plan(&m, &s(), &input).unwrap();
    m.fail_next("disable-user --user 0 com.tcl.ttvs", TransportError::Timeout(30_000));
    let mut h = TestHooks::default();
    let r = run(&m, &s(), &plan, &input, &mut h);
    assert!(r.completed, "{:#?}", r.results);
    assert!(h.events.iter().any(|e| e.status == StepStatus::Reconnecting));
    let d = m.device(SN).unwrap();
    assert!(!d.packages["com.tcl.logkit"] && !d.packages["com.tcl.ttvs"]);
    // logkit was already disabled when the step re-ran: no second command for it.
    assert_eq!(m.log().iter().filter(|l| l.ends_with("disable-user --user 0 com.tcl.logkit")).count(), 1);
}

#[test]
fn tv_gone_for_good_stops_the_run() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 1, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.tcl.logkit".into(), "com.tcl.ttvs".into()];
    input.profile.settings.animation_scale = Some("0.5".into());
    let plan = plan(&m, &s(), &input).unwrap();
    m.fail_next("disable-user --user 0 com.tcl.logkit", TransportError::Timeout(30_000));
    m.unplug(SN);
    m.add_device(SN, tv()); // still answers the first command...
    struct Unplug<'a> { inner: TestHooks, m: &'a MockTransport }
    impl Hooks for Unplug<'_> {
        fn event(&mut self, e: &Event) { if e.status == StepStatus::Reconnecting { self.m.unplug(SN); } self.inner.event(e) }
        fn journal(&mut self, a: &str, b: &str, c: Option<&str>) { self.inner.journal(a, b, c) }
        fn save_snapshot(&mut self, i: &str, sn: &Snapshot) -> Result<(), String> { self.inner.save_snapshot(i, sn) }
        fn pause(&mut self, _d: Duration) {}
        fn reconnect_policy(&self) -> (u32, Duration) { (2, Duration::ZERO) }
    }
    let mut h = Unplug { inner: TestHooks::default(), m: &m };
    let r = run(&m, &s(), &plan, &input, &mut h);
    assert!(r.stopped.is_some());
    let st = |id: &str| r.results.iter().find(|x| x.id == id).unwrap();
    assert_eq!(st("debloat-1").status, StepStatus::Failed);
    assert!(st("debloat-1").message.contains("could not reconnect"));
    assert_eq!(st("settings-animation").status, StepStatus::Skipped);
}

#[test]
fn profile_bound_to_another_tv_refuses_to_plan() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    input.profile.device_identity = Some("Other TV|fp".into());
    assert!(matches!(plan(&m, &s(), &input), Err(PlanError::WrongDevice { .. })));
}

#[test]
fn private_dns_needs_android_9() {
    let m = MockTransport::new();
    m.add_device(SN, { let mut d = tv(); d.props.insert("ro.build.version.sdk".into(), "27".into()); d });
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    input.profile.settings.private_dns = PrivateDns::Automatic;
    let p = plan(&m, &s(), &input).unwrap();
    assert!(!p.runnable);
}

#[test]
fn resuming_after_an_interruption_skips_what_is_already_done() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 1, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.tcl.logkit".into(), "com.tcl.ttvs".into(), "com.tcl.waterfall".into()];
    let p1 = plan(&m, &s(), &input).unwrap();
    let mut h = TestHooks { cancel_after: Some(3), ..Default::default() }; // identify, snapshot, batch 1
    let r1 = run(&m, &s(), &p1, &input, &mut h);
    assert!(r1.cancelled && !r1.completed);
    // Resume = plan again from the TV's real state and run.
    let p2 = plan(&m, &s(), &input).unwrap();
    assert_eq!(p2.steps.iter().filter(|x| matches!(x.kind, StepKind::DebloatBatch { .. })).count(), 2, "logkit is done, two batches left");
    let r2 = run(&m, &s(), &p2, &input, &mut TestHooks::default());
    assert!(r2.completed);
    assert_eq!(m.log().iter().filter(|l| l.ends_with("disable-user --user 0 com.tcl.logkit")).count(), 1);
}
