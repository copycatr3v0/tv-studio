//! Firmware families detected from installed packages. These are risk cases (CleanYourTV device notes),
//! not assumptions: each one only adds warnings and changes defaults, never unlocks anything.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OemFamily {
    GoogleTv,
    AndroidTvStock,
    Cvte,
    XiaomiPatchwall,
    Tcl,
    Sony,
    Philips,
    Hisense,
    SkyworthCoocaa,
    Nvidia,
    FireOs,
}

#[derive(Debug, Clone, Serialize)]
pub struct OemNote {
    pub family: OemFamily,
    pub label: &'static str,
    pub note: &'static str,
}

const RULES: &[(OemFamily, &str, &[&str], &str)] = &[
    (OemFamily::GoogleTv, "Google TV", &["com.google.android.apps.tv.launcherx", "com.google.android.tungsten.setupwraith"],
     "The setup wizard (setupwraith) can own the saved Wi-Fi network and also answers Home. set-home-activity may print Success without changing Home, so Home is verified on screen."),
    (OemFamily::AndroidTvStock, "Android TV (stock launcher)", &["com.google.android.tvlauncher", "com.google.android.leanbacklauncher"],
     "Stock Android TV home. Replacing it needs the Studio launcher (Android 8.0+)."),
    (OemFamily::Cvte, "CVTE board", &["com.cvte."],
     "The stock launcher stays enabled, the new Home is set and the TV rebooted once before anything else, and no remote-key helper is enabled in the same pass. Factory menus on these boards must never be opened."),
    (OemFamily::XiaomiPatchwall, "Xiaomi PatchWall", &["com.mitv.tvhome", "com.mitv.", "com.xiaomi.mitv."],
     "PatchWall launcher variants. Some China-market firmware rejects third-party Home apps or all sideloads."),
    (OemFamily::Tcl, "TCL", &["com.tcl."],
     "Some TCL Android 9 builds re-enable packages at every boot. On some Google TV 12 builds a key-filtering accessibility helper breaks held-button repeat (volume)."),
    (OemFamily::Sony, "Sony Bravia", &["com.sony.dtv."],
     "Some MediaTek Sony firmware restores a wrong display density after reboot; display overrides are recorded before changes."),
    (OemFamily::Philips, "Philips (TP Vision)", &["org.droidtv."],
     "Philips sets often return to the last input at power-on; keep the TV input and source apps."),
    (OemFamily::Hisense, "Hisense / VIDAA-Android", &["com.hisense.", "com.jamdeo."],
     "On some Hisense sets the stock launcher is the only handler of the Input button; the launcher can take it over."),
    (OemFamily::SkyworthCoocaa, "Skyworth / Coocaa", &["com.skyworth.", "com.coocaa."],
     "Coocaa OS lists sources through its own provider; ACR services (aisense) are common here."),
    (OemFamily::Nvidia, "NVIDIA Shield", &["com.nvidia."],
     "Shield firmware is well tested; keep NVIDIA's controller and remote services."),
    (OemFamily::FireOs, "Fire OS", &["com.amazon.tv.launcher"],
     "Fire OS is not supported: Home cannot be replaced and some builds refuse pm disable-user entirely. Read-only inspection only."),
];

/// Detect families from an installed-package list. A rule ending in '.' is a prefix match.
pub fn detect(packages: &[String]) -> Vec<OemNote> {
    RULES.iter()
        .filter(|(_, _, keys, _)| keys.iter().any(|k| packages.iter().any(|p| if k.ends_with('.') { p.starts_with(k) } else { p == k })))
        .map(|(family, label, _, note)| OemNote { family: *family, label, note })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pk(v: &[&str]) -> Vec<String> { v.iter().map(|s| s.to_string()).collect() }

    #[test]
    fn detects_google_tv_on_tcl() {
        let f: Vec<_> = detect(&pk(&["com.google.android.apps.tv.launcherx", "com.tcl.tv", "com.netflix.ninja"])).into_iter().map(|n| n.family).collect();
        assert_eq!(f, vec![OemFamily::GoogleTv, OemFamily::Tcl]);
    }

    #[test]
    fn prefix_and_exact() {
        assert!(detect(&pk(&["com.cvte.tv.launcher.x"])).iter().any(|n| n.family == OemFamily::Cvte));
        // "com.mitv.tvhome" exact and "com.mitv." prefix both map to one family entry.
        assert_eq!(detect(&pk(&["com.mitv.tvhome"])).len(), 1);
        assert!(detect(&pk(&["com.tclx.app"])).is_empty());
    }
}
