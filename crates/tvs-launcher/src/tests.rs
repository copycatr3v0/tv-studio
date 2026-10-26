use crate::install::*;
use crate::theme::*;
use serde_json::json;
use std::time::Duration;
use tvs_core::{Component, Serial};
use tvs_transport::mock::{MockDevice, MockTransport};

const SN: &str = "tv:5555";
const STOCK: &str = "com.google.android.apps.tv.launcherx/.home.HomeActivity";
fn s() -> Serial { Serial::new(SN).unwrap() }
const NOW: Duration = Duration::from_millis(0);

fn tv() -> MockDevice {
    let mut d = MockDevice::default();
    d.packages.insert("com.google.android.apps.tv.launcherx".into(), true);
    d.home = Some(STOCK.into());
    d.home_claimants = vec![STOCK.into()];
    d.focused = Some(STOCK.into());
    d
}

fn apk() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("studio-home.apk");
    std::fs::write(&p, b"PK\x03\x04fake-apk").unwrap();
    (dir, p)
}

fn installed(m: &MockTransport) {
    let (_d, p) = apk();
    install(m, &s(), &p, 31).unwrap();
}

#[test]
fn install_verifies_and_cleans_staging() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let (_d, p) = apk();
    let st = install(&m, &s(), &p, 31).unwrap();
    assert!(st.installed && st.enabled);
    assert_eq!(st.version.as_deref(), Some("3.0-studio.1"));
    assert!(!st.is_resolved_home, "installing must not change Home");
    assert!(!m.device(SN).unwrap().files.contains_key("/data/local/tmp/tvs-studio-home.apk"));
}

#[test]
fn install_refusals_are_typed() {
    let m = MockTransport::new();
    let mut d = tv();
    d.install_failure = Some("Failure [INSTALL_FAILED_VERIFICATION_FAILURE]".into());
    m.add_device(SN, d);
    let (_d, p) = apk();
    assert!(matches!(install(&m, &s(), &p, 31), Err(LauncherError::VerificationRefused)));
    assert!(matches!(install(&m, &s(), &p, 25), Err(LauncherError::OlderSdk(25))));
}

#[test]
fn home_is_verified_on_screen() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    let h = make_home(&m, &s(), NOW).unwrap();
    assert!(h.verified, "{h:?}");
    assert_eq!(h.focused.as_deref(), Some(STUDIO_HOME));
}

#[test]
fn success_without_effect_is_not_verified() {
    let m = MockTransport::new();
    let mut d = tv();
    d.ignore_set_home = true;
    m.add_device(SN, d);
    installed(&m);
    let h = make_home(&m, &s(), NOW).unwrap();
    assert!(!h.verified);
    assert!(h.note.unwrap().contains("still shows"));
}

#[test]
fn hand_back_then_uninstall() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    assert!(make_home(&m, &s(), NOW).unwrap().verified);
    let back = hand_back_home(&m, &s(), Some(&Component::new(STOCK).unwrap()), NOW).unwrap();
    assert!(back.verified, "{back:?}");
    assert!(uninstall(&m, &s()).unwrap());
    assert!(!status(&m, &s()).unwrap().installed);
}

#[test]
fn hand_back_not_verified_when_tv_keeps_ours() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    let mut d = m.device(SN).unwrap();
    d.ignore_set_home = true;
    m.add_device(SN, d);
    let back = hand_back_home(&m, &s(), Some(&Component::new(STOCK).unwrap()), NOW).unwrap();
    assert!(!back.verified);
}

fn bundle() -> ThemeBundle {
    ThemeBundle {
        config: json!({"columns": 5, "background_color": "#070A14", "wallpaper": true, "dock_apps": ["com.netflix.ninja"]}),
        wallpapers: vec![("aurora-01.jpg".into(), vec![0xFF, 0xD8, 1, 2, 3])],
        captions: Some("aurora-01.jpg\tAurora\n".into()),
    }
}

#[test]
fn theme_push_via_provider_and_verified() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    let r = deploy(&m, &s(), &bundle(), "rev1").unwrap();
    assert!(r.config_verified);
    assert!(r.written.iter().all(|(_, w)| *w == WriteMethod::Provider));
    assert!(r.restarted, "Home was ours, so it restarts");
    assert_eq!(r.focused_after.as_deref(), Some(STUDIO_HOME));
    let files = m.device(SN).unwrap().files;
    let dir = "/sdcard/Android/data/studio.tvcustomizer.home/files";
    assert!(files.contains_key(&format!("{dir}/wallpapers/aurora-01.jpg")));
    assert_eq!(files[&format!("{dir}/theme_revision")], b"rev1");
    assert!(!files.keys().any(|k| k.starts_with("/data/local/tmp/tvs-")), "staging cleaned");
    // One command per call, and every redirect points into staging.
    assert!(m.log().iter().filter(|l| l.contains(" < ")).all(|l| l.contains("< /data/local/tmp/tvs-theme-")));
}

#[test]
fn theme_push_falls_back_to_copy_on_old_android() {
    let m = MockTransport::new();
    let mut d = tv();
    d.no_content_write = true;
    m.add_device(SN, d);
    installed(&m);
    let r = deploy(&m, &s(), &bundle(), "rev1").unwrap();
    assert!(r.written.iter().all(|(_, w)| *w == WriteMethod::Copy));
    assert!(r.config_verified, "read back through cat");
    assert!(!r.restarted, "not Home: nothing brought to the front");
}

#[test]
fn theme_push_fails_loudly_when_both_paths_refused() {
    let m = MockTransport::new();
    let mut d = tv();
    d.no_content_write = true;
    d.no_cp_into_app_data = true;
    m.add_device(SN, d);
    installed(&m);
    assert!(matches!(deploy(&m, &s(), &bundle(), "r"), Err(ThemeError::WriteFailed(f)) if f == "config.json"));
}

#[test]
fn new_theme_replaces_old_wallpapers() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    deploy(&m, &s(), &bundle(), "r1").unwrap();
    let mut b = bundle();
    b.wallpapers = vec![("dunes.jpg".into(), vec![0xFF, 0xD8])];
    b.captions = None;
    deploy(&m, &s(), &b, "r2").unwrap();
    let files = m.device(SN).unwrap().files;
    assert!(!files.keys().any(|k| k.ends_with("aurora-01.jpg")));
    assert!(!files.keys().any(|k| k.ends_with("captions.txt")));
    assert!(files.keys().any(|k| k.ends_with("dunes.jpg")));
}

#[test]
fn invalid_theme_or_missing_launcher_touches_nothing() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    let mut b = bundle();
    b.config = json!({"columns": 99});
    assert!(matches!(deploy(&m, &s(), &b, "r"), Err(ThemeError::Invalid(_))));
    assert!(matches!(deploy(&m, &s(), &bundle(), "r"), Err(ThemeError::NoLauncher)));
    assert!(m.log().iter().all(|l| !l.contains("content write")));
}

#[test]
fn reset_removes_theme_files() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    deploy(&m, &s(), &bundle(), "r1").unwrap();
    let r = reset(&m, &s(), "r2").unwrap();
    assert!(r.config_verified);
    let files = m.device(SN).unwrap().files;
    assert!(!files.keys().any(|k| k.ends_with("config.json") || k.contains("/wallpapers/")));
}

#[test]
fn full_revert_round_trip() {
    use std::collections::BTreeMap;
    use tvs_core::PackageName;
    use tvs_recovery::{apply_restore, capture, disable_in_batches, restore_plan};
    let m = MockTransport::new();
    let mut d = tv();
    for p in ["com.tcl.logkit", "com.tcl.ttvs"] { d.packages.insert(p.into(), true); }
    m.add_device(SN, d);
    let baseline = capture(&m, &s(), "id", "t0").unwrap();

    // Everything the app can do today.
    disable_in_batches(&m, &s(), &[PackageName::new("com.tcl.logkit").unwrap()], 5, &mut |_| {});
    installed(&m);
    assert!(make_home(&m, &s(), NOW).unwrap().verified);
    deploy(&m, &s(), &bundle(), "r1").unwrap();

    // Revert.
    let current = tvs_device::inventory::list_packages(&m, &s()).unwrap();
    let home = tvs_device::inventory::current_home(&m, &s());
    let plan = restore_plan(&baseline, &current, home.as_deref(), &BTreeMap::new(), None);
    assert_eq!(plan.set_home.as_deref(), Some(STOCK));
    let r = apply_restore(&m, &s(), &plan, 5, &mut |_| {});
    assert!(r.unchanged.is_empty(), "{r:?}");
    let (changed, kept) = crate::revert::remove_studio_launcher(&m, &s(), baseline.home.as_deref(), NOW);
    assert!(kept.is_empty(), "{kept:?}");
    assert_eq!(changed.len(), 1);

    let after = capture(&m, &s(), "id", "t1").unwrap();
    assert_eq!(after.packages, baseline.packages, "same packages, same states, launcher gone");
    assert_eq!(after.home, baseline.home);
    assert_eq!(m.device(SN).unwrap().focused.as_deref(), Some(STOCK), "the TV shows the stock Home");
    assert!(!m.device(SN).unwrap().files.keys().any(|k| k.contains("studio.tvcustomizer.home")), "theme files gone");
}

#[test]
fn revert_keeps_launcher_when_home_cannot_be_confirmed() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    let mut d = m.device(SN).unwrap();
    d.ignore_set_home = true; // the TV refuses to move Home back
    m.add_device(SN, d);
    let (changed, kept) = crate::revert::remove_studio_launcher(&m, &s(), Some(STOCK), NOW);
    assert!(changed.is_empty());
    assert_eq!(kept.len(), 1);
    assert!(status(&m, &s()).unwrap().installed, "never uninstall the launcher that is still Home");
}
