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
    d.no_key_helper = true;
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
        ..Default::default()
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

fn canvas_bundle(video: &str) -> ThemeBundle {
    let mut b = bundle();
    b.layout = Some(json!({"version":1,"enabled":true,
        "background":{"kind":"video","color":"#0b1020","color2":"#27325c","video":video,"dim":0.3,"blur":0},
        "items":[{"id":"clock-1","kind":"clock","enabled":true,"x":72,"y":5,"w":24,"h":12,
            "style":{"font":"sans-serif-light","size":72,"color":"#ffffff","opacity":1,"radius":0,"fill":null,"align":"end"},
            "motion":{"type":"float","speed":1,"amount":20}}]}));
    b.videos = vec![("rain.mp4".into(), vec![0, 0, 0, 24, b'f', b't', b'y', b'p'])];
    b
}

#[test]
fn canvas_layout_and_video_are_pushed_and_reset_removes_them() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    let r = deploy(&m, &s(), &canvas_bundle("rain"), "rev1").unwrap();
    assert!(r.written.iter().any(|(n, _)| n == "layout.json") && r.written.iter().any(|(n, _)| n == "video/rain.mp4"));
    let dir = "/sdcard/Android/data/studio.tvcustomizer.home/files";
    let files = m.device(SN).unwrap().files;
    assert!(files.contains_key(&format!("{dir}/layout.json")) && files.contains_key(&format!("{dir}/video/rain.mp4")));
    // A theme without a canvas removes the old one.
    deploy(&m, &s(), &bundle(), "rev2").unwrap();
    let files = m.device(SN).unwrap().files;
    assert!(!files.contains_key(&format!("{dir}/layout.json")) && !files.contains_key(&format!("{dir}/video/rain.mp4")));
    deploy(&m, &s(), &canvas_bundle("rain"), "rev3").unwrap();
    reset(&m, &s(), "rev4").unwrap();
    let files = m.device(SN).unwrap().files;
    assert!(!files.contains_key(&format!("{dir}/layout.json")) && !files.contains_key(&format!("{dir}/video/rain.mp4")));
}

#[test]
fn a_bad_canvas_sends_nothing() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    let missing = deploy(&m, &s(), &canvas_bundle("rainy-window"), "r").unwrap_err();
    assert!(matches!(missing, ThemeError::Invalid(ref v) if v.iter().any(|i| i.key == "layout.background.video")));
    let mut b = canvas_bundle("rain");
    b.videos[0].0 = "../evil.mp4".into();
    assert!(matches!(deploy(&m, &s(), &b, "r"), Err(ThemeError::Invalid(_))));
    let mut b = canvas_bundle("rain");
    b.layout.as_mut().unwrap()["items"][0]["style"]["font"] = json!("Papyrus");
    assert!(matches!(deploy(&m, &s(), &b, "r"), Err(ThemeError::Invalid(_))));
    let mut b = canvas_bundle("rain");
    b.videos[0].1 = vec![0; MAX_VIDEO_BYTES + 1];
    assert!(matches!(deploy(&m, &s(), &b, "r"), Err(ThemeError::Invalid(_))));
    let mut b = bundle();
    b.videos = vec![("rain.mp4".into(), vec![1])];
    assert!(matches!(deploy(&m, &s(), &b, "r"), Err(ThemeError::Invalid(_))), "videos without a layout");
    assert!(m.device(SN).unwrap().files.is_empty(), "nothing reached the TV");
}

#[test]
fn custom_fonts_are_pushed_checked_and_removed() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    let ttf = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut b = canvas_bundle("rain");
    b.layout.as_mut().unwrap()["items"][0]["style"]["font"] = json!("file:Mont.ttf");
    // The layout names a font that is not included.
    assert!(matches!(deploy(&m, &s(), &b, "r"), Err(ThemeError::Invalid(ref v)) if v.iter().any(|i| i.key == "layout.style.font")));
    b.fonts = vec![("Mont.ttf".into(), ttf.clone())];
    deploy(&m, &s(), &b, "r1").unwrap();
    let dir = "/sdcard/Android/data/studio.tvcustomizer.home/files";
    assert!(m.device(SN).unwrap().files.contains_key(&format!("{dir}/fonts/Mont.ttf")));
    // Bad names and sizes are refused; fonts without a layout are refused.
    let mut bad = b.clone(); bad.fonts[0].0 = "../Mont.ttf".into();
    assert!(matches!(deploy(&m, &s(), &bad, "r"), Err(ThemeError::Invalid(_))));
    let mut big = b.clone(); big.fonts[0].1 = vec![1; MAX_FONT_BYTES + 1];
    assert!(matches!(deploy(&m, &s(), &big, "r"), Err(ThemeError::Invalid(_))));
    let mut nolayout = bundle(); nolayout.fonts = vec![("Mont.ttf".into(), ttf)];
    assert!(matches!(deploy(&m, &s(), &nolayout, "r"), Err(ThemeError::Invalid(_))));
    reset(&m, &s(), "r2").unwrap();
    assert!(!m.device(SN).unwrap().files.contains_key(&format!("{dir}/fonts/Mont.ttf")));
}

#[test]
fn google_tv_home_goes_through_the_key_helper_and_back() {
    let m = MockTransport::new();
    let mut d = tv();
    d.ignore_set_home = true;
    d.settings_secure.insert("enabled_accessibility_services".into(), "com.other/.Svc".into());
    m.add_device(SN, d);
    installed(&m);
    let h = make_home(&m, &s(), NOW).unwrap();
    assert!(h.verified, "{h:?}");
    let list = m.device(SN).unwrap().settings_secure["enabled_accessibility_services"].clone();
    assert_eq!(list, format!("com.other/.Svc:{}", crate::install::HOME_KEY_SERVICE));
    hand_back_home(&m, &s(), Some(&Component::new(STOCK).unwrap()), NOW).unwrap();
    assert_eq!(m.device(SN).unwrap().settings_secure["enabled_accessibility_services"], "com.other/.Svc", "other services kept");
}

#[test]
fn applying_a_theme_keeps_the_home_key_helper_working() {
    let m = MockTransport::new();
    let mut d = tv();
    d.ignore_set_home = true;
    m.add_device(SN, d);
    installed(&m);
    assert!(make_home(&m, &s(), NOW).unwrap().verified);
    crate::theme::deploy(&m, &s(), &bundle(), "r1").unwrap();
    assert!(!m.device(SN).unwrap().unbound_helpers, "helper bound again after the restart");
    assert!(make_home(&m, &s(), NOW).unwrap().verified);
}

#[test]
fn canvas_screensaver_is_set_then_the_tvs_own_one_comes_back() {
    let m = MockTransport::new();
    let mut d = tv();
    d.settings_secure.insert("screensaver_components".into(), "com.google.android.backdrop/.Backdrop".into());
    d.settings_secure.insert("screensaver_enabled".into(), "1".into());
    m.add_device(SN, d);
    installed(&m);
    let mut b = canvas_bundle("rain");
    b.layout.as_mut().unwrap()["screensaver"] = json!(true);
    let r = crate::theme::deploy(&m, &s(), &b, "r1").unwrap();
    assert_eq!(r.screensaver, Some(true));
    assert_eq!(m.device(SN).unwrap().settings_secure["screensaver_components"], crate::theme::CANVAS_DREAM);
    // Applying again keeps the first record of the TV's own screensaver.
    crate::theme::deploy(&m, &s(), &b, "r2").unwrap();
    // Switching it off puts the TV's own one back.
    b.layout.as_mut().unwrap()["screensaver"] = json!(false);
    let r = crate::theme::deploy(&m, &s(), &b, "r3").unwrap();
    assert_eq!(r.screensaver, None);
    assert_eq!(m.device(SN).unwrap().settings_secure["screensaver_components"], "com.google.android.backdrop/.Backdrop");
}

#[test]
fn reset_restores_a_screensaver_that_was_unset() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    let mut b = canvas_bundle("rain");
    b.layout.as_mut().unwrap()["screensaver"] = json!(true);
    crate::theme::deploy(&m, &s(), &b, "r1").unwrap();
    crate::theme::reset(&m, &s(), "r2").unwrap();
    assert!(!m.device(SN).unwrap().settings_secure.contains_key("screensaver_components"), "deleted, as it was before");
}

#[test]
fn profiles_are_sent_as_files_and_reset_removes_them() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    let mut b = canvas_bundle("rain");
    b.layout.as_mut().unwrap()["profiles"] = json!([{"id": "kids", "name": "Kids"}]);
    let mut kids = b.layout.clone().unwrap();
    kids.as_object_mut().unwrap().remove("profiles");
    b.profiles = vec![("kids".into(), kids)];
    crate::theme::deploy(&m, &s(), &b, "r1").unwrap();
    assert!(m.device(SN).unwrap().files.keys().any(|k| k.ends_with("/profiles/kids.json")));
    // A profile the main layout does not list is refused before anything is written.
    let mut bad = b.clone();
    bad.profiles = vec![("guest".into(), b.profiles[0].1.clone())];
    assert!(crate::theme::deploy(&m, &s(), &bad, "r2").is_err());
    crate::theme::reset(&m, &s(), "r3").unwrap();
    assert!(!m.device(SN).unwrap().files.keys().any(|k| k.contains("/profiles/")));
    crate::theme::reset_pin(&m, &s()).unwrap();
    assert!(m.device(SN).unwrap().files.keys().any(|k| k.ends_with("/pin_reset")));
}

#[test]
fn now_playing_listener_follows_the_layout_and_reset() {
    let m = MockTransport::new();
    m.add_device(SN, tv());
    installed(&m);
    let mut b = canvas_bundle("rain");
    b.layout.as_mut().unwrap()["items"][0]["kind"] = json!("nowplaying");
    b.layout.as_mut().unwrap()["items"][0].as_object_mut().unwrap().remove("options");
    crate::theme::deploy(&m, &s(), &b, "r1").unwrap();
    assert_eq!(m.device(SN).unwrap().settings_secure.get("enabled_notification_listeners").map(String::as_str), Some(crate::theme::MEDIA_LISTENER));
    crate::theme::reset(&m, &s(), "r2").unwrap();
    assert!(!m.device(SN).unwrap().settings_secure.contains_key("enabled_notification_listeners"));
}

fn google_tv() -> MockDevice {
    let mut d = tv();
    d.ignore_set_home = true;   // launcherx outranks every installed Home while it is on
    d
}

#[test]
fn google_tv_can_be_switched_off_and_back_on() {
    let m = MockTransport::new();
    m.add_device(SN, google_tv());
    installed(&m);
    assert!(make_home(&m, &s(), NOW).unwrap().verified);
    let off = google_home_turn_off(&m, &s(), NOW).unwrap();
    assert!(off.verified, "{off:?}");
    assert_eq!(off.focused.as_deref(), Some(STUDIO_HOME));
    assert_eq!(google_home_off(&m, &s()), vec!["com.google.android.apps.tv.launcherx".to_string()]);
    assert!(status(&m, &s()).unwrap().is_resolved_home, "the Studio launcher is now the TV's real Home");
    // Asking again changes nothing.
    assert!(google_home_turn_off(&m, &s(), NOW).unwrap().verified);
    assert_eq!(google_home_turn_on(&m, &s()).unwrap(), vec!["com.google.android.apps.tv.launcherx".to_string()]);
    assert!(google_home_off(&m, &s()).is_empty());
}

#[test]
fn google_tv_stays_on_unless_studio_home_already_works() {
    let m = MockTransport::new();
    let mut d = google_tv();
    d.no_key_helper = true;
    m.add_device(SN, d);
    let none = google_home_turn_off(&m, &s(), NOW).unwrap();
    assert!(!none.verified && none.note.unwrap().contains("Install"));
    installed(&m);
    let h = google_home_turn_off(&m, &s(), NOW).unwrap();
    assert!(!h.verified);
    assert!(h.note.unwrap().contains("Make the Studio launcher Home first"));
    assert!(google_home_off(&m, &s()).is_empty(), "nothing was switched off");
}

#[test]
fn google_tv_comes_straight_back_when_the_switch_does_not_hold() {
    let m = MockTransport::new();
    let mut d = google_tv();
    d.sticky.insert("com.google.android.apps.tv.launcherx".into());   // firmware turns it back on by itself
    m.add_device(SN, d);
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    let h = google_home_turn_off(&m, &s(), NOW).unwrap();
    assert!(!h.verified);
    assert!(h.note.unwrap().contains("switched back on"));
    assert!(google_home_off(&m, &s()).is_empty());
}

#[test]
fn hand_back_switches_google_tv_on_first() {
    let m = MockTransport::new();
    m.add_device(SN, google_tv());
    installed(&m);
    make_home(&m, &s(), NOW).unwrap();
    assert!(google_home_turn_off(&m, &s(), NOW).unwrap().verified);
    let back = hand_back_home(&m, &s(), Some(&Component::new(STOCK).unwrap()), NOW).unwrap();
    assert!(google_home_off(&m, &s()).is_empty());
    assert!(back.verified, "{back:?}");
    assert_eq!(back.focused.as_deref(), Some(STOCK));
}

