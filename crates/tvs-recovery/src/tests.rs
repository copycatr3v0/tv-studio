use super::*;
use tvs_device::inventory::list_packages;
use tvs_transport::mock::{MockDevice, MockTransport};

const SN: &str = "tv:5555";
fn s() -> Serial { Serial::new(SN).unwrap() }
fn p(x: &str) -> PackageName { PackageName::new(x).unwrap() }

fn tv() -> MockDevice {
    let mut d = MockDevice::default();
    for n in ["com.tcl.logkit", "com.tcl.ttvs", "com.tcl.waterfall", "com.tcl.firm", "com.tcl.tv", "com.google.android.apps.tv.launcherx", "com.netflix.ninja"] {
        d.packages.insert(n.into(), true);
    }
    d.packages.insert("com.tcl.already.off".into(), false);
    d.refuse_disable.insert("com.tcl.firm".into());
    d.home = Some("com.google.android.apps.tv.launcherx/.HomeActivity".into());
    d.settings_global.insert("window_animation_scale".into(), "0.5".into());
    d
}

fn mock() -> MockTransport { let m = MockTransport::new(); m.add_device(SN, tv()); m }

#[test]
fn capture_records_everything_undo_needs() {
    let m = mock();
    let snap = capture(&m, &s(), "id", "2026-10-26T10:00:00Z").unwrap();
    assert_eq!(snap.packages["com.tcl.logkit"], PkgState::Enabled);
    assert_eq!(snap.packages["com.tcl.already.off"], PkgState::Disabled);
    assert_eq!(snap.home.as_deref(), Some("com.google.android.apps.tv.launcherx/.HomeActivity"));
    assert_eq!(snap.settings["window_animation_scale"].as_deref(), Some("0.5"));
    assert_eq!(snap.settings["animator_duration_scale"], None);
}

#[test]
fn batches_are_verified_and_refusals_reported() {
    let m = mock();
    let sel = [p("com.tcl.logkit"), p("com.tcl.firm"), p("com.tcl.ttvs")];
    let mut events = vec![];
    let r = disable_in_batches(&m, &s(), &sel, 2, &mut |e| events.push(e));
    assert_eq!(r.changed, vec!["com.tcl.logkit", "com.tcl.ttvs"]);
    assert_eq!(r.refused.len(), 1);
    assert!(r.refused[0].1.contains("SecurityException"));
    assert!(r.stopped.is_none());
    assert_eq!(events.iter().filter(|e| matches!(e, Progress::Verified { .. })).count(), 2);
    // One pm command per shell call.
    assert!(m.log().iter().filter(|l| l.contains("pm disable-user")).all(|l| l.matches("pm ").count() == 1));
}

#[test]
fn silent_no_op_is_caught_by_readback() {
    let m = MockTransport::new();
    let mut d = tv();
    d.sticky.insert("com.tcl.waterfall".into());
    m.add_device(SN, d);
    let r = disable_in_batches(&m, &s(), &[p("com.tcl.waterfall")], 5, &mut |_| {});
    assert_eq!(r.unchanged, vec!["com.tcl.waterfall"]);
    assert!(r.changed.is_empty());
}

#[test]
fn firmware_that_refuses_everything_stops_after_first_batch() {
    let m = MockTransport::new();
    let mut d = tv();
    for n in ["com.tcl.logkit", "com.tcl.ttvs", "com.tcl.waterfall"] { d.refuse_disable.insert(n.into()); }
    m.add_device(SN, d);
    let r = disable_in_batches(&m, &s(), &[p("com.tcl.logkit"), p("com.tcl.ttvs"), p("com.tcl.waterfall")], 2, &mut |_| {});
    assert!(r.stopped.as_deref().unwrap().contains("does not allow"));
    assert_eq!(r.skipped, vec!["com.tcl.waterfall"]);
}

#[test]
fn lost_connection_stops_without_retry() {
    let m = mock();
    m.fail_next("com.tcl.ttvs", TransportError::Timeout(30_000));
    let r = disable_in_batches(&m, &s(), &[p("com.tcl.logkit"), p("com.tcl.ttvs"), p("com.tcl.waterfall")], 5, &mut |_| {});
    assert!(r.stopped.is_some());
    assert_eq!(m.log().iter().filter(|l| l.contains("disable-user --user 0 com.tcl.ttvs")).count(), 1);
    assert!(r.changed.contains(&"com.tcl.logkit".to_string()));
    assert_eq!(r.skipped, vec!["com.tcl.waterfall"]);
}

#[test]
fn round_trip_restores_exactly_what_was_there() {
    let m = mock();
    let snap = capture(&m, &s(), "id", "t0").unwrap();
    disable_in_batches(&m, &s(), &[p("com.tcl.logkit"), p("com.tcl.ttvs")], 5, &mut |_| {});
    // Something else changed afterwards too.
    m.shell(&s(), &ShellCommand::new("settings").arg("put").arg("global").arg("window_animation_scale").arg("1.0")).unwrap();
    m.shell(&s(), &ShellCommand::new("settings").arg("put").arg("global").arg("animator_duration_scale").arg("0.5")).unwrap();

    let current = list_packages(&m, &s()).unwrap();
    let cur_settings: BTreeMap<String, Option<String>> = [
        ("window_animation_scale".to_string(), Some("1.0".to_string())),
        ("transition_animation_scale".to_string(), None),
        ("animator_duration_scale".to_string(), Some("0.5".to_string())),
    ].into();
    let plan = restore_plan(&snap, &current, snap.home.as_deref(), &cur_settings, None);
    assert_eq!(plan.enable, vec!["com.tcl.logkit", "com.tcl.ttvs"]);
    assert_eq!(plan.left_disabled, vec!["com.tcl.already.off"], "already disabled before: stays disabled");
    assert_eq!(plan.set_home, None);
    let r = apply_restore(&m, &s(), &plan, 5, &mut |_| {});
    assert!(r.unchanged.is_empty(), "{r:?}");

    let after = capture(&m, &s(), "id", "t1").unwrap();
    assert_eq!(after.packages, snap.packages);
    assert_eq!(after.settings, snap.settings);
}

#[test]
fn scoped_restore_touches_only_selection() {
    let m = mock();
    let snap = capture(&m, &s(), "id", "t0").unwrap();
    disable_in_batches(&m, &s(), &[p("com.tcl.logkit"), p("com.tcl.ttvs")], 5, &mut |_| {});
    let current = list_packages(&m, &s()).unwrap();
    let plan = restore_plan(&snap, &current, None, &BTreeMap::new(), Some(&["com.tcl.ttvs".to_string()]));
    assert_eq!(plan.enable, vec!["com.tcl.ttvs"]);
    assert!(plan.set_home.is_none() && plan.settings.is_empty());
}

#[test]
fn new_packages_are_left_alone() {
    let m = mock();
    let snap = capture(&m, &s(), "id", "t0").unwrap();
    let mut current = list_packages(&m, &s()).unwrap();
    current.push(PackageInfo { name: "com.tcl.newpreload".into(), enabled: false, system: true });
    let plan = restore_plan(&snap, &current, None, &BTreeMap::new(), None);
    assert_eq!(plan.not_in_snapshot, vec!["com.tcl.newpreload"]);
}

#[test]
fn snapshot_serializes_round_trip() {
    let m = mock();
    let snap = capture(&m, &s(), "id", "t0").unwrap();
    let back: Snapshot = serde_json::from_value(serde_json::to_value(&snap).unwrap()).unwrap();
    assert_eq!(back, snap);
}
