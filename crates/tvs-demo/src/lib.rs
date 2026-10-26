//! A realistic simulated Google TV, modelled on CleanYourTV's tested TCL Smart TV Pro (G09, Google TV 12)
//! list (examples/tcl-g09-google-tv, MIT, (c) 2026 Gathin). Used by demo mode and the end-to-end tests.
//! Everything here is fake device state: nothing reaches a real TV.

use std::collections::BTreeMap;
use tvs_transport::mock::{MockDevice, MockTransport};
use tvs_transport::{MdnsKind, MdnsService};

pub const DEMO_HOST: &str = "192.168.1.50";
pub const DEMO_SERIAL: &str = "192.168.1.50:5555";
pub const DEMO_PAIR: &str = "192.168.1.50:41234";
pub const DEMO_CONNECT: &str = "192.168.1.50:37123";
pub const DEMO_PAIR_CODE: &str = "123456";
pub const STOCK_HOME: &str = "com.google.android.apps.tv.launcherx/com.google.android.apps.tv.launcherx.home.HomeActivity";
pub const STUDIO_HOME: &str = "studio.tvcustomizer.home/com.gathin.tvhome.MainActivity";

/// The 36 packages CleanYourTV's TCL G09 list disables.
pub const TCL_BLOAT: &[&str] = &[
    "com.tcl.ttvs", "com.tcl.logkit", "com.tcl.interactive", "com.google.android.feedback", "com.graymatrix.did",
    "com.mirakulo.astrotv.service", "com.tcl.channelplus", "com.tcl.waterfall.overseas", "com.tcl.tv.tclhome_passive",
    "com.tcl.partnercustomizer", "com.tcl.messagebox", "android.autoinstalls.config.google.gtvpai", "com.tcl.smartlink.core",
    "com.tcl.magiconnectfree", "com.tcl.guard", "com.tcl.smartalexa", "com.tcl.useragreement", "com.tcl.initsetup",
    "com.tcl.usercenter", "com.tcl.exhibit", "com.tcl.esticker", "com.tcl.hotelmenu", "com.google.android.tv.remote.service",
    "com.tcl.ocean.instructions", "com.tcl.repairguide", "com.tcl.gamebar", "com.tcl.UpdatePeripheral", "com.tcl.hearaid",
    "com.google.android.marvin.talkback", "com.google.android.play.games", "com.google.android.videos",
    "com.google.android.youtube.tvmusic", "com.google.android.apps.tv.dreamx", "com.google.android.apps.mediashell",
    "com.tcl.ui_mediaCenter",
];
/// What the TV needs to work (kept on the tested set).
pub const TCL_KEEP: &[&str] = &[
    "com.tcl.tv", "com.tcl.tvinput", "com.tcl.suspension", "com.tcl.dashboard", "com.tcl.autopair", "com.tcl.systemserver",
    "com.tcl.providers.config",
];
pub const GOOGLE: &[&str] = &[
    "com.google.android.apps.tv.launcherx", "com.google.android.tungsten.setupwraith", "com.google.android.gms",
    "com.google.android.gsf", "com.android.vending", "com.google.android.katniss", "com.google.android.leanback.ime",
    "com.google.android.youtube.tv",
];
pub const CORE: &[&str] = &[
    "android", "com.android.systemui", "com.android.shell", "com.android.settings", "com.android.tv.settings",
    "com.android.permissioncontroller", "com.android.providers.settings", "com.android.providers.tv", "com.android.bluetooth",
];
pub const STREAMING: &[&str] = &["com.netflix.ninja", "com.amazon.amazonvideo.livingroom", "com.globo.globotv", "com.disney.disneyplus"];

const LAUNCHABLE: &[&str] = &[
    "com.google.android.youtube.tv", "com.netflix.ninja", "com.amazon.amazonvideo.livingroom", "com.globo.globotv",
    "com.disney.disneyplus", "com.google.android.youtube.tvmusic", "com.android.vending", "com.google.android.videos",
    "com.google.android.play.games", "com.tcl.tv", "com.android.tv.settings",
];

fn framestats(response_ms: u64) -> String {
    // Two frames after the key (uptime 5000 ms): the first completes `response_ms` later.
    let first = 5000 * 1_000_000 + response_ms * 1_000_000;
    format!("Uptime: 6000 Realtime: 10000\nTotal frames rendered: 3\nJanky frames: 0 (0.00%)\n---PROFILEDATA---\nFlags,IntendedVsync,FrameCompleted\n0,{},{first}\n0,{},{}\n---PROFILEDATA---\n",
        first - 10_000_000, first + 6_000_000, first + 16_000_000)
}

/// A TCL Google TV 12 set, freshly set up: everything enabled, Google TV home, setup wizard owning the Wi-Fi.
pub fn tcl_google_tv() -> MockDevice {
    let mut d = MockDevice::default();
    for (k, v) in [
        ("ro.product.brand", "TCL"), ("ro.product.manufacturer", "TCL"), ("ro.product.model", "Smart TV Pro (simulated)"),
        ("ro.product.device", "G09"), ("ro.product.name", "G09_4K_GB"), ("ro.build.version.release", "12"),
        ("ro.build.version.sdk", "31"), ("ro.build.id", "STT2.230203.001"), ("ro.build.version.incremental", "AS24"),
        ("ro.build.fingerprint", "TCL/G09_4K_GB/G09:12/STT2.230203.001/AS24:user/demo-keys"),
        ("ro.board.platform", "t5m"), ("ro.adb.secure", "1"),
    ] { d.props.insert(k.into(), v.into()); }
    let all: Vec<&str> = TCL_BLOAT.iter().chain(TCL_KEEP).chain(GOOGLE).chain(CORE).chain(STREAMING).copied().collect();
    for p in &all {
        d.packages.insert((*p).into(), true);
        if !STREAMING.contains(p) { d.system.insert((*p).into()); }
        let perms = if *p == "com.tcl.tvinput" { "      android.permission.TV_INPUT_HARDWARE\n" }
            else if *p == "com.google.android.katniss" { "      android.permission.RECORD_AUDIO\n" } else { "      android.permission.INTERNET\n" };
        let uid = if p.starts_with("com.tcl.") { 1000 } else { 10_000 + all.iter().position(|x| x == p).unwrap_or(0) };
        d.dumpsys.insert(format!("package {p}"), format!("Packages:\n  Package [{p}]:\n    userId={uid}\n    versionName=1.0\n    requested permissions:\n{perms}    install permissions:\n"));
    }
    d.home = Some(STOCK_HOME.into());
    d.focused = Some(STOCK_HOME.into());
    d.home_claimants = vec![STOCK_HOME.into(), "com.google.android.tungsten.setupwraith/.RecoveryActivity".into(), "com.android.tv.settings/.system.FallbackHome".into()];
    d.activity_queries.insert("android.intent.action.VIEW".into(), vec!["com.tcl.tv/.PassthroughActivity".into()]);
    d.activity_queries.insert("android.settings.SETTINGS".into(), vec!["com.android.tv.settings/.MainSettings".into()]);
    d.activity_queries.insert("android.intent.action.MAIN".into(), LAUNCHABLE.iter().map(|p| format!("{p}/.Main")).collect());
    d.dumpsys.insert("wifi".into(), "WifiConfigManager:\n  ID: 0 SSID: \"HomeWiFi\" cname=com.google.android.tungsten.setupwraith uid=1000\n".into());
    d.text_files.insert("/proc/meminfo".into(), "MemTotal:        1956324 kB\nMemFree:          112340 kB\nMemAvailable:     803112 kB\n".into());
    d.text_files.insert("/proc/stat".into(), "cpu  1200 0 600 9000 200 0 0 0 0 0\n".into());
    d.df_data = Some("Filesystem     1K-blocks    Used Available Use% Mounted on\n/dev/block/dm-5  10335916 6201544   4134372  60% /data\n".into());
    // Benchmark data: the stock launcher is slower and heavier than the Studio launcher.
    for (pkg, pid, resp, pss_kb, ticks) in [("com.google.android.apps.tv.launcherx", 2101, 46, 217_088, 40), ("studio.tvcustomizer.home", 3102, 21, 59_392, 2)] {
        d.dumpsys.insert(format!("gfxinfo {pkg} reset"), "Uptime: 5000 Realtime: 9000\n".into());
        d.dumpsys.insert(format!("gfxinfo {pkg} framestats"), framestats(resp));
        d.dumpsys.insert(format!("meminfo {pkg}"), format!("** MEMINFO in pid {pid} [{pkg}] **\n        TOTAL PSS:   {pss_kb}            TOTAL RSS: 0\n"));
        d.pids.insert(pkg.into(), pid);
        d.text_files.insert(format!("/proc/{pid}/stat"), format!("{pid} (launcher) S 1 1 0 0 -1 0 0 0 0 0 {ticks} 0 0 0 20 0\n"));
    }
    d.cold_start_ms = 1850;
    d.home_return_ms = 420;
    d.settings_secure.insert("voice_interaction_service".into(), "com.google.android.katniss/.VoiceInteractionService".into());
    d.appops.insert("com.google.android.katniss".into(), "Uid mode: RECORD_AUDIO: allow\nRECORD_AUDIO: allow; time=+2h13m4s120ms ago; duration=+3s200ms\n".into());
    d.apk_identity = Some(("studio.tvcustomizer.home".into(), "com.gathin.tvhome.MainActivity".into(), "3.0-studio.1".into()));
    d
}

/// The demo world: the TV is on the network and answers both the legacy port and Wireless debugging.
pub fn demo_transport() -> MockTransport {
    let m = MockTransport::new();
    m.add_reachable(DEMO_SERIAL, tcl_google_tv());
    m.add_reachable(DEMO_CONNECT, tcl_google_tv());
    m.set_pair_code(DEMO_PAIR, DEMO_PAIR_CODE);
    m.set_mdns(vec![
        MdnsService { name: "adb-G09DEMO-Ab12Cd".into(), kind: MdnsKind::Pairing, endpoint: serial(DEMO_PAIR) },
        MdnsService { name: "adb-G09DEMO-Ab12Cd".into(), kind: MdnsKind::Connect, endpoint: serial(DEMO_CONNECT) },
    ]);
    m
}

fn serial(s: &str) -> tvs_core::Serial { tvs_core::Serial::new(s).expect("valid demo serial") }

/// Make the simulated studio launcher's theme files visible: path → bytes, for assertions.
pub fn launcher_files(d: &MockDevice) -> BTreeMap<String, Vec<u8>> {
    d.files.iter().filter(|(k, _)| k.starts_with("/sdcard/Android/data/studio.tvcustomizer.home/")).map(|(k, v)| (k.clone(), v.clone())).collect()
}
