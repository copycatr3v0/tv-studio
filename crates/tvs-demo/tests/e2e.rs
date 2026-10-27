//! End-to-end scenarios on the simulated TCL Google TV (spec §15 testing list). Each test drives the same
//! library code the app's commands call, against a whole simulated TV, and checks the TV's state afterwards.
//! Simulated only: none of this proves behaviour on real firmware (see docs/TESTING.md).

use std::collections::BTreeMap;
use std::time::Duration;
use tvs_core::{PackageName, Serial};
use tvs_demo::*;
use tvs_deploy::*;
use tvs_launcher::theme::ThemeBundle;
use tvs_recovery::Snapshot;
use tvs_transport::mock::MockTransport;
use tvs_transport::{DeviceState, Transport, TransportError};

fn s() -> Serial { Serial::new(DEMO_SERIAL).unwrap() }
fn tv() -> MockTransport { let m = MockTransport::new(); m.add_device(DEMO_SERIAL, tcl_google_tv()); m }

type OnEvent = Box<dyn FnMut(&Event)>;

#[derive(Default)]
struct Hooks0 { snapshots: Vec<Snapshot>, journal: Vec<(String, String)>, cancel_after: Option<usize>, on_event: Option<OnEvent> }
impl Hooks for Hooks0 {
    fn event(&mut self, e: &Event) { if let Some(f) = self.on_event.as_mut() { f(e) } }
    fn journal(&mut self, step: &str, phase: &str, _d: Option<&str>) { self.journal.push((step.into(), phase.into())); }
    fn save_snapshot(&mut self, _id: &str, snap: &Snapshot) -> Result<(), String> { self.snapshots.push(snap.clone()); Ok(()) }
    fn pause(&mut self, _d: Duration) {}
    fn cancelled(&mut self) -> bool { self.cancel_after.is_some_and(|n| self.journal.iter().filter(|j| j.1 == "verified" || j.1 == "skipped").count() >= n) }
    fn now(&mut self) -> String { "2026-10-26T12:00:00Z".into() }
    fn reconnect_policy(&self) -> (u32, Duration) { (3, Duration::ZERO) }
}

fn apk() -> (tempfile::TempDir, std::path::PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("studio-home.apk");
    std::fs::write(&p, b"PK\x03\x04apk").unwrap();
    (d, p)
}

/// The tested G09 list minus setupwraith (the app refuses that one on this TV: it owns the Wi-Fi network).
fn g09_input(apk: Option<&std::path::Path>) -> DeployInput {
    let mut p = Profile::new("TCL living room");
    p.debloat.packages = TCL_BLOAT.iter().map(|s| s.to_string()).collect();
    // Talkback and the remote service are sensitive (accessibility / phone remote): accepted one by one.
    p.debloat.confirmed = vec!["com.google.android.marvin.talkback".into(), "com.google.android.tv.remote.service".into(), "com.tcl.hearaid".into()];
    p.settings.animation_scale = Some("0.5".into());
    p.settings.private_dns = PrivateDns::Hostname("dns.adguard-dns.com".into());
    if apk.is_some() {
        p.launcher = LauncherPlan { install: true, make_home: true, disable_stock: true };
        p.apply_theme = true;
    }
    DeployInput {
        profile: p,
        theme: apk.map(|_| ThemeBundle { config: serde_json::json!({"columns": 6, "background_color": "#0B1020"}), wallpapers: vec![], captions: None, ..Default::default() }),
        apk: apk.map(|a| a.to_path_buf()), batch_size: 8, settle: Duration::ZERO,
    }
}

fn blocked(p: &Plan) -> Vec<String> {
    p.steps.iter().filter_map(|x| match &x.preview { Preview::Blocked(m) => Some(m.clone()), _ => None }).collect()
}

fn changes(m: &MockTransport) -> usize {
    m.log().iter().filter(|l| l.contains("disable-user") || l.contains("pm enable") || l.contains("pm install") || l.contains("settings put")
        || l.contains("set-home-activity") || l.contains("content write")).count()
}

#[test]
fn the_simulated_tv_looks_like_a_tcl_google_tv() {
    let m = tv();
    let info = tvs_device::probe::probe(&m, &s());
    assert_eq!(info.model.as_deref(), Some("Smart TV Pro (simulated)"));
    assert_eq!(info.sdk, Some(31));
    assert!(info.identity().contains("demo-keys"), "the demo TV can never share a baseline with a real TCL set");
    let pk = tvs_device::inventory::list_packages(&m, &s()).unwrap();
    // "android" itself has no dot, so it never passes PackageName validation and is never listed (or touched).
    assert_eq!(pk.len() + 1, TCL_BLOAT.len() + TCL_KEEP.len() + GOOGLE.len() + CORE.len() + STREAMING.len());
    assert_eq!(tvs_device::inventory::current_home(&m, &s()).as_deref(), Some(STOCK_HOME));
}

#[test]
fn full_g09_deployment_then_exact_revert_to_original() {
    let m = tv();
    let (_d, a) = apk();
    let input = g09_input(Some(&a));
    let original = m.device(DEMO_SERIAL).unwrap();
    let baseline = tvs_recovery::capture(&m, &s(), "TCL|fp", "t0").unwrap();

    let plan = plan(&m, &s(), &input).unwrap();
    assert!(plan.runnable, "{:#?}", blocked(&plan));
    assert_eq!(changes(&m), 0, "planning changes nothing");
    let mut h = Hooks0::default();
    let r = run(&m, &s(), &plan, &input, &mut h);
    assert!(r.completed, "{:#?}", r.results);

    let d = m.device(DEMO_SERIAL).unwrap();
    for p in TCL_BLOAT { assert!(!d.packages[*p], "{p} should be disabled"); }
    for p in TCL_KEEP.iter().chain(CORE).chain(STREAMING) { assert!(d.packages[*p], "{p} must stay enabled"); }
    assert!(d.packages["com.google.android.tungsten.setupwraith"], "Wi-Fi owner untouched");
    assert!(!d.packages["com.google.android.apps.tv.launcherx"], "stock launcher disabled after Home was confirmed");
    assert_eq!(d.focused.as_deref(), Some(STUDIO_HOME));
    assert!(!launcher_files(&d).is_empty(), "theme delivered");
    assert!(m.log().iter().all(|l| !l.contains("pm uninstall") || l.contains("studio.tvcustomizer.home")), "never uninstalls system packages");

    // Revert to original, in the app's order: exact restore first (re-enables the stock launcher and gives
    // it Home back), then remove the Studio launcher once the stock Home is confirmed on screen.
    let current = tvs_device::inventory::list_packages(&m, &s()).unwrap();
    let settings = tvs_recovery::TRACKED_SETTINGS.iter().map(|k| (k.to_string(), m.device(DEMO_SERIAL).unwrap().settings_global.get(*k).cloned())).collect::<BTreeMap<_, _>>();
    let rp = tvs_recovery::restore_plan(&baseline, &current, tvs_device::inventory::current_home(&m, &s()).as_deref(), &settings, None);
    let rr = tvs_recovery::apply_restore(&m, &s(), &rp, 10, &mut |_| {});
    assert!(rr.stopped.is_none() && rr.refused.is_empty() && rr.unchanged.is_empty(), "{rr:?}");
    let (done, kept) = tvs_launcher::revert::remove_studio_launcher(&m, &s(), baseline.home.as_deref(), Duration::ZERO);
    assert!(kept.is_empty(), "{kept:?} / {done:?}");
    assert!(!m.device(DEMO_SERIAL).unwrap().packages.contains_key("studio.tvcustomizer.home"), "Studio launcher removed");

    let after = m.device(DEMO_SERIAL).unwrap();
    assert_eq!(after.packages, original.packages, "every package back to its original state");
    assert_eq!(after.home.as_deref(), Some(STOCK_HOME));
    for k in tvs_recovery::TRACKED_SETTINGS { assert_eq!(after.settings_global.get(k), original.settings_global.get(k), "{k}"); }
}

#[test]
fn multiple_devices_and_an_unauthorized_one() {
    let m = tv();
    m.add_device("R5CT90ABCDE", { let mut d = tcl_google_tv(); d.props.insert("ro.product.model".into(), "Bedroom TV".into()); d });
    m.add_device("192.168.1.77:5555", { let mut d = tcl_google_tv(); d.state = Some(DeviceState::Unauthorized); d });
    let list = m.devices().unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().any(|e| e.state == DeviceState::Unauthorized));
    // Every command names its TV: the bedroom set is not touched by a living-room run.
    let input = DeployInput { profile: { let mut p = Profile::new("x"); p.debloat.packages = vec!["com.tcl.logkit".into()]; p }, theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    let p = plan(&m, &s(), &input).unwrap();
    assert!(run(&m, &s(), &p, &input, &mut Hooks0::default()).completed);
    assert!(!m.device(DEMO_SERIAL).unwrap().packages["com.tcl.logkit"]);
    assert!(m.device("R5CT90ABCDE").unwrap().packages["com.tcl.logkit"]);
    // The unauthorized TV can't be read, and planning says so instead of guessing.
    let u = Serial::new("192.168.1.77:5555").unwrap();
    assert!(matches!(plan(&m, &u, &input), Err(PlanError::Unreachable(_))));
    assert!(m.log().iter().all(|l| !l.contains("192.168.1.77:5555") || !l.contains("disable-user")));
}

#[test]
fn protected_and_tv_specific_packages_are_refused() {
    let m = tv();
    let mut input = DeployInput { profile: Profile::new("bad"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    for p in ["com.android.systemui", "com.tcl.tvinput", "com.google.android.tungsten.setupwraith", "com.android.tv.settings", "com.android.providers.settings"] {
        input.profile.debloat.packages = vec![p.into()];
        let pl = plan(&m, &s(), &input).unwrap();
        assert!(!pl.runnable, "{p} must be refused");
        assert!(!blocked(&pl).is_empty(), "{p}");
        assert!(run(&m, &s(), &pl, &input, &mut Hooks0::default()).results.is_empty());
    }
    // The Wi-Fi reason is specific, not a generic refusal.
    input.profile.debloat.packages = vec!["com.google.android.tungsten.setupwraith".into()];
    let msg = blocked(&plan(&m, &s(), &input).unwrap()).join(" ");
    assert!(msg.to_lowercase().contains("wi-fi"), "{msg}");
    assert_eq!(changes(&m), 0);
}

#[test]
fn the_tv_changes_between_plan_and_run() {
    let m = tv();
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 2, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.tcl.logkit".into(), "com.tcl.ttvs".into(), "com.tcl.gamebar".into(), "com.tcl.esticker".into()];
    let p = plan(&m, &s(), &input).unwrap();
    // Meanwhile: someone disabled one by hand and a firmware update removed another.
    let mut d = m.device(DEMO_SERIAL).unwrap();
    d.packages.insert("com.tcl.ttvs".into(), false);
    d.packages.remove("com.tcl.gamebar");
    m.add_device(DEMO_SERIAL, d);
    let r = run(&m, &s(), &p, &input, &mut Hooks0::default());
    let d = m.device(DEMO_SERIAL).unwrap();
    assert!(!d.packages["com.tcl.logkit"] && !d.packages["com.tcl.esticker"]);
    assert!(!m.log().iter().any(|l| l.ends_with("disable-user --user 0 com.tcl.ttvs")), "already disabled: not repeated");
    assert!(!m.log().iter().any(|l| l.ends_with("disable-user --user 0 com.tcl.gamebar")), "gone: not attempted");
    assert!(r.results.iter().all(|x| x.status != StepStatus::Failed), "{:#?}", r.results);
}

#[test]
fn invalid_configuration_is_rejected_before_anything_runs() {
    // Unknown fields in a profile file are refused, not silently dropped.
    let bad = r#"{"version":1,"name":"x","debloat":{"packages":[],"confirmed":[]},"disable_everything":true}"#;
    assert!(serde_json::from_str::<Profile>(bad).is_err());
    // A malformed package name never becomes a shell command.
    assert!(PackageName::new("com.tcl.logkit; reboot").is_err());
    let m = tv();
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.tcl.logkit;reboot".into()];
    let p = plan(&m, &s(), &input).unwrap();
    assert!(!p.runnable);
    input.profile.debloat.packages = vec![];
    input.profile.settings.private_dns = PrivateDns::Hostname("not a host!".into());
    assert!(!plan(&m, &s(), &input).unwrap().runnable);
    // Theme values outside the schema are reported.
    let issues = tvs_launcher::schema::validate(&serde_json::json!({"columns": 99, "background_color": "blue"}));
    assert!(issues.len() >= 2, "{issues:?}");
    assert_eq!(changes(&m), 0);
}

#[test]
fn firmware_that_refuses_every_disable() {
    let m = MockTransport::new();
    m.add_device(DEMO_SERIAL, { let mut d = tcl_google_tv(); d.refuse_disable = TCL_BLOAT.iter().map(|s| s.to_string()).collect(); d });
    let input = g09_input(None);
    let p = plan(&m, &s(), &input).unwrap();
    let r = run(&m, &s(), &p, &input, &mut Hooks0::default());
    assert!(!r.completed);
    assert!(r.results.iter().any(|x| x.status == StepStatus::Failed));
    let d = m.device(DEMO_SERIAL).unwrap();
    for p in TCL_BLOAT { assert!(d.packages[*p], "{p}"); }
    // It stopped asking after the firmware refused the first batch, instead of hammering every batch.
    assert!(m.log().iter().filter(|l| l.contains("disable-user")).count() <= input.batch_size, "{}", m.log().iter().filter(|l| l.contains("disable-user")).count());
}

#[test]
fn sticky_packages_are_reported_not_claimed() {
    let m = MockTransport::new();
    m.add_device(DEMO_SERIAL, { let mut d = tcl_google_tv(); d.sticky.insert("com.tcl.guard".into()); d });
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
    input.profile.debloat.packages = vec!["com.tcl.logkit".into(), "com.tcl.guard".into()];
    let p = plan(&m, &s(), &input).unwrap();
    let r = run(&m, &s(), &p, &input, &mut Hooks0::default());
    let batch = r.results.iter().find(|x| x.id.starts_with("debloat")).unwrap();
    assert!(batch.message.contains("com.tcl.guard"), "{batch:?}");
    assert!(m.device(DEMO_SERIAL).unwrap().packages["com.tcl.guard"]);
    assert!(!m.device(DEMO_SERIAL).unwrap().packages["com.tcl.logkit"]);
}

#[test]
fn tv_reboots_mid_run_and_the_run_finishes_once() {
    let m = std::rc::Rc::new(tv());
    let (_d, a) = apk();
    let input = g09_input(Some(&a));
    let p = plan(&*m, &s(), &input).unwrap();
    // The TV drops off the network the first time a debloat batch is about to run.
    m.fail_next("disable-user", TransportError::NoDevice(DEMO_SERIAL.into()));
    let m2 = m.clone();
    let mut dropped = false;
    let mut h = Hooks0 { on_event: Some(Box::new(move |e: &Event| {
        if e.status == StepStatus::Reconnecting && !dropped { dropped = true; m2.drop_device(DEMO_SERIAL); }
    })), ..Default::default() };
    let r = run(&*m, &s(), &p, &input, &mut h);
    assert!(r.completed, "{:#?}", r.results);
    let d = m.device(DEMO_SERIAL).unwrap();
    for p in TCL_BLOAT { assert!(!d.packages[*p], "{p}"); }
    assert_eq!(h.snapshots.len(), 1, "one snapshot, taken before the first change");
}

#[test]
fn interrupted_run_resumes_from_the_tvs_real_state() {
    let m = tv();
    let input = g09_input(None);
    let p1 = plan(&m, &s(), &input).unwrap();
    let mut h = Hooks0 { cancel_after: Some(4), ..Default::default() };
    let r1 = run(&m, &s(), &p1, &input, &mut h);
    assert!(r1.cancelled);
    let done_before: usize = m.device(DEMO_SERIAL).unwrap().packages.iter().filter(|(_, e)| !**e).count();
    assert!(done_before > 0 && done_before < TCL_BLOAT.len());
    let p2 = plan(&m, &s(), &input).unwrap();
    let r2 = run(&m, &s(), &p2, &input, &mut Hooks0::default());
    assert!(r2.completed);
    for p in TCL_BLOAT {
        assert_eq!(m.log().iter().filter(|l| l.ends_with(&format!("disable-user --user 0 {p}"))).count(), 1, "{p} disabled exactly once");
    }
}

#[test]
fn benchmark_is_reproducible_and_the_verdict_is_cautious() {
    struct NoWait;
    impl tvs_insight::bench::Pacer for NoWait { fn pause(&mut self, _d: Duration) {} }
    let cfg = tvs_insight::bench::BenchConfig { keys: 6, cold_starts: 5, home_returns: 5, idle_seconds: 10, ..Default::default() };
    let m = tv();
    let a = tvs_insight::bench::run(&m, &s(), &cfg, "t1", &mut NoWait).unwrap();
    let b = tvs_insight::bench::run(&m, &s(), &cfg, "t2", &mut NoWait).unwrap();
    assert_eq!(a.key_response_ms, b.key_response_ms, "same TV, same numbers");
    assert_eq!(a.home_return_ms.len(), 5, "{:?}", a.notes);
    // Same launcher twice: no clear change, never "better".
    use tvs_insight::stats::Verdict;
    assert!(tvs_insight::bench::comparison(&a, &b).iter().all(|x| x.verdict != Verdict::Better && x.verdict != Verdict::Worse));
    // After switching to the Studio launcher, the simulated numbers are lower and the verdict says better.
    let (_d, apk) = apk();
    let mut input = DeployInput { profile: Profile::new("x"), theme: None, apk: Some(apk), batch_size: 5, settle: Duration::ZERO };
    input.profile.launcher = LauncherPlan { install: true, make_home: true, disable_stock: false };
    let p = plan(&m, &s(), &input).unwrap();
    assert!(run(&m, &s(), &p, &input, &mut Hooks0::default()).completed);
    let c = tvs_insight::bench::run(&m, &s(), &cfg, "t3", &mut NoWait).unwrap();
    assert_eq!(c.launcher.as_deref(), Some("studio.tvcustomizer.home"));
    let key = tvs_insight::bench::comparison(&a, &c).into_iter().find(|x| x.key == "key_response").unwrap();
    assert_eq!(key.verdict, Verdict::Better);
    assert_eq!(changes(&m), m.log().iter().filter(|l| l.contains("pm install") || l.contains("set-home-activity")).count(), "benchmarks change nothing");
}

#[test]
fn privacy_audit_on_the_simulated_tv() {
    let m = tv();
    let a = tvs_insight::privacy::audit(&m, &s()).unwrap();
    let k = a.entries.iter().find(|e| e.package == "com.google.android.katniss").unwrap();
    assert_eq!(k.mic_history, tvs_insight::privacy::MicHistory::Recorded);
    assert!(a.entries.iter().all(|e| !e.note.to_lowercase().contains("listening")));
    assert_eq!(changes(&m), 0);
}

#[test]
fn the_demo_world_connects_and_pairs() {
    let m = demo_transport();
    assert!(m.devices().unwrap().is_empty(), "nothing connected until the user connects");
    assert_eq!(m.mdns_services().unwrap().len(), 2);
    let pair = Serial::new(DEMO_PAIR).unwrap();
    assert!(m.pair(&pair, &tvs_transport::PairingCode::new("000000").unwrap()).is_err());
    m.pair(&pair, &tvs_transport::PairingCode::new(DEMO_PAIR_CODE).unwrap()).unwrap();
    m.connect(&Serial::new(DEMO_CONNECT).unwrap()).unwrap();
    m.connect(&s()).unwrap();
    assert_eq!(m.devices().unwrap().len(), 2);
}

/// What the Guided setup does: evidence-based preselection, then the TV's live checks, then plan and run.
fn guided_selection(m: &MockTransport, mode: tvs_kb::Mode) -> Vec<String> {
    use tvs_safety::Verdict;
    let installed = tvs_device::inventory::list_packages(m, &s()).unwrap();
    let wanted: Vec<PackageName> = installed.iter()
        .filter(|p| p.enabled && !matches!(PackageName::new(p.name.clone()).map(|n| tvs_safety::can_disable(&n)), Ok(Verdict::Blocked(_)) | Err(_)))
        .filter(|p| tvs_kb::preselect(tvs_kb::get(&p.name), mode))
        .filter_map(|p| PackageName::new(p.name.clone()).ok()).collect();
    let facts = tvs_device::inventory::gather_facts(m, &s(), &installed, &wanted);
    let verdicts = tvs_safety::guards::evaluate_selection(&wanted, &facts);
    wanted.iter().filter(|n| matches!(verdicts.get(*n), Some(Verdict::Allowed))).map(|n| n.to_string()).collect()
}

#[test]
fn guided_setup_on_the_g09_is_safe_in_both_clean_up_modes() {
    for mode in [tvs_kb::Mode::Safe, tvs_kb::Mode::Balanced] {
        let m = tv();
        let picked = guided_selection(&m, mode);
        assert!(!picked.is_empty(), "{mode:?} should find something to turn off on a G09");
        for never in ["com.google.android.tungsten.setupwraith", "com.tcl.tvinput", "com.google.android.apps.tv.launcherx", "com.android.tv.settings", "com.tcl.tv", "com.google.android.gms"] {
            assert!(!picked.iter().any(|p| p == never), "{mode:?} must not pick {never}");
        }
        let mut input = DeployInput { profile: Profile::new("guided"), theme: None, apk: None, batch_size: 5, settle: Duration::ZERO };
        input.profile.debloat.packages = picked.clone();
        input.profile.settings.animation_scale = Some("0.5".into());
        let p = plan(&m, &s(), &input).unwrap();
        assert!(p.runnable, "{mode:?}: {:#?}", blocked(&p));
        let r = run(&m, &s(), &p, &input, &mut Hooks0::default());
        assert!(r.completed, "{mode:?}: {:#?}", r.results);
        let d = m.device(DEMO_SERIAL).unwrap();
        for n in &picked { assert!(!d.packages[n], "{n}"); }
        assert!(d.packages["com.google.android.apps.tv.launcherx"] && d.home.as_deref() == Some(STOCK_HOME), "the guide never touches Home");
    }
}
