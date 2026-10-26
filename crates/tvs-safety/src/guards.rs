//! Guards that need facts read from the TV. Ported from CleanYourTV `tools/disable.ps1` / `check.py`
//! (MIT, (c) 2026 Gathin), with one deliberate difference: there is no environment-variable override.
//! A guard that could not run fails closed.

use crate::{can_disable, is_factory_package, Verdict, SETUPWRAITH};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tvs_core::PackageName;

pub const KNOWN_STOCK_LAUNCHERS: &[&str] = &[
    "com.google.android.tvlauncher",
    "com.google.android.apps.tv.launcherx",
    "com.google.android.leanbacklauncher",
    "com.mitv.tvhome",
];

/// Facts read from the device right before a selection is evaluated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceFacts {
    /// Package owning the default Home, if a single launcher owns it.
    pub home_package: Option<String>,
    /// True when the Home query answered (a chooser answer counts, with `home_package` None).
    pub home_known: bool,
    /// Enabled packages with a usable HOME activity (setup wizards, FallbackHome, factory tests excluded).
    pub usable_launchers: BTreeSet<String>,
    /// `None` when `dumpsys wifi` could not be read.
    pub setupwraith_owns_wifi: Option<bool>,
    /// Selected packages requesting android.permission.TV_INPUT_HARDWARE.
    pub input_hardware: BTreeSet<String>,
    pub input_hardware_checked: bool,
    /// Packages that open HDMI/AV channels (VIEW vnd.android.cursor.item/channel) and Settings.
    pub input_handlers: BTreeSet<String>,
    pub settings_handlers: BTreeSet<String>,
    pub handlers_checked: bool,
    /// Any com.cvte.* package installed.
    pub cvte: bool,
}

const ESSENTIAL: &[(&str, &str)] = &[
    ("com.android.vending", "Google Play Store: app installs and updates stop."),
    ("com.google.android.gms", "Google Play Services: sign-in, casting, many apps and some firmware updates depend on it."),
    ("com.google.android.gsf", "Google Services Framework: Play Services depends on it."),
    ("com.android.bluetooth", "Bluetooth: Bluetooth remotes, headphones and controllers stop working."),
    ("com.android.tv", "Live TV / TV inputs app."),
];
const ESSENTIAL_PATTERNS: &[(&str, &str)] = &[
    ("inputmethod", "On-screen keyboard: text entry (search, Wi-Fi passwords) may become impossible."),
    (".ime", "On-screen keyboard: text entry (search, Wi-Fi passwords) may become impossible."),
    ("remotepairing", "Remote pairing: a new or re-paired remote may not connect."),
    ("remotecontrol", "Remote control service: remote buttons may stop working."),
    ("tvinput", "TV input service: HDMI, AV or tuner inputs may stop working."),
    ("bluetooth", "Bluetooth-related service: remotes or audio devices may be affected."),
];

fn add(out: &mut BTreeMap<PackageName, Verdict>, p: &PackageName, v: Verdict) {
    let cur = out.remove(p).unwrap_or(Verdict::Allowed);
    let merged = match (cur, v) {
        (Verdict::Blocked(a), Verdict::Blocked(b)) => Verdict::Blocked(format!("{a} {b}")),
        (b @ Verdict::Blocked(_), _) | (_, b @ Verdict::Blocked(_)) => b,
        (Verdict::NeedsConfirm(a), Verdict::NeedsConfirm(b)) => Verdict::NeedsConfirm(format!("{a} {b}")),
        (c @ Verdict::NeedsConfirm(_), Verdict::Allowed) | (Verdict::Allowed, c @ Verdict::NeedsConfirm(_)) => c,
        (Verdict::Allowed, Verdict::Allowed) => Verdict::Allowed,
    };
    out.insert(p.clone(), merged);
}

/// Verdict for disabling each package in `selection`, taking the whole selection into account
/// (a launcher may be disabled only if another usable launcher stays enabled).
pub fn evaluate_selection(selection: &[PackageName], f: &DeviceFacts) -> BTreeMap<PackageName, Verdict> {
    let mut out = BTreeMap::new();
    let selected: BTreeSet<&str> = selection.iter().map(PackageName::as_str).collect();
    let remaining_launchers = f.usable_launchers.iter().filter(|l| !selected.contains(l.as_str())).count();

    for p in selection {
        let name = p.as_str();
        add(&mut out, p, can_disable(p));

        if !f.input_hardware_checked {
            add(&mut out, p, Verdict::Blocked("The HDMI/tuner driver check did not run; inspect the TV again before disabling anything.".into()));
        } else if f.input_hardware.contains(name) {
            add(&mut out, p, Verdict::Blocked("It drives the HDMI/AV/tuner inputs (TV_INPUT_HARDWARE). Disabling it leaves inputs on \u{201c}No Signal\u{201d} until a full restore and a cold boot.".into()));
        }
        if !f.handlers_checked {
            add(&mut out, p, Verdict::Blocked("The input/Settings handler check did not run; inspect the TV again.".into()));
        } else {
            if f.input_handlers.contains(name) {
                add(&mut out, p, Verdict::Blocked("The TV opens its HDMI/AV inputs with it.".into()));
            }
            if f.settings_handlers.contains(name) {
                add(&mut out, p, Verdict::Blocked("The TV opens Settings with it; without it there is no way back to Developer options.".into()));
            }
        }

        if name == SETUPWRAITH {
            match f.setupwraith_owns_wifi {
                Some(true) => add(&mut out, p, Verdict::Blocked("The setup wizard owns the saved Wi-Fi network; disabling it deletes that network and the TV drops off Wi-Fi and ADB. Forget and re-join the network in Settings first.".into())),
                None => add(&mut out, p, Verdict::Blocked("Could not check which app owns the saved Wi-Fi network, so the setup wizard stays enabled.".into())),
                Some(false) => {}
            }
        }

        let is_launcher = f.usable_launchers.contains(name) || KNOWN_STOCK_LAUNCHERS.contains(&name);
        if is_launcher {
            if !f.home_known {
                add(&mut out, p, Verdict::Blocked("Could not tell which app owns Home, so no launcher is disabled.".into()));
            } else if remaining_launchers == 0 {
                add(&mut out, p, Verdict::Blocked("No other launcher would be left to answer Home. Install and verify the Studio launcher first.".into()));
            } else if f.home_package.as_deref() == Some(name) {
                add(&mut out, p, Verdict::NeedsConfirm("It is the current Home screen. Another launcher must be verified as Home first; the deployment does that step before disabling it.".into()));
            } else {
                add(&mut out, p, Verdict::NeedsConfirm("It is a launcher.".into()));
            }
            if f.cvte && name.starts_with("com.cvte.tv.launcher") {
                add(&mut out, p, Verdict::Blocked("On CVTE boards the stock launcher stays enabled.".into()));
            }
        }
        if f.cvte && name.starts_with("com.cvte.tv.launcher") && !is_launcher {
            add(&mut out, p, Verdict::Blocked("On CVTE boards the stock launcher stays enabled.".into()));
        }

        if let Some((_, why)) = ESSENTIAL.iter().find(|(n, _)| *n == name) {
            add(&mut out, p, Verdict::NeedsConfirm((*why).into()));
        } else if let Some((_, why)) = ESSENTIAL_PATTERNS.iter().find(|(pat, _)| name.contains(pat)) {
            add(&mut out, p, Verdict::NeedsConfirm((*why).into()));
        }
        if is_factory_package(p) {
            add(&mut out, p, Verdict::NeedsConfirm("Factory/service package. It is only disabled, never opened.".into()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PackageName { PackageName::new(s).unwrap() }
    fn facts() -> DeviceFacts {
        DeviceFacts {
            home_package: Some("com.google.android.apps.tv.launcherx".into()),
            home_known: true,
            usable_launchers: ["com.google.android.apps.tv.launcherx".to_string()].into(),
            setupwraith_owns_wifi: Some(false),
            input_hardware: ["com.tcl.tvinput".to_string()].into(),
            input_hardware_checked: true,
            input_handlers: ["com.tcl.tv".to_string()].into(),
            settings_handlers: ["com.android.tv.settings".to_string()].into(),
            handlers_checked: true,
            cvte: false,
        }
    }
    fn blocked(v: &Verdict) -> bool { matches!(v, Verdict::Blocked(_)) }

    #[test]
    fn plain_telemetry_allowed() {
        let r = evaluate_selection(&[p("com.tcl.logkit")], &facts());
        assert_eq!(r[&p("com.tcl.logkit")], Verdict::Allowed);
    }

    #[test]
    fn only_launcher_blocked_until_replacement_exists() {
        let sel = [p("com.google.android.apps.tv.launcherx")];
        assert!(blocked(&evaluate_selection(&sel, &facts())[&sel[0]]));
        let mut f = facts();
        f.usable_launchers.insert("studio.tvcustomizer.home".into());
        assert!(matches!(evaluate_selection(&sel, &f)[&sel[0]], Verdict::NeedsConfirm(_)));
        // Disabling both launchers in one selection leaves none.
        let both = [p("com.google.android.apps.tv.launcherx"), p("studio.tvcustomizer.home")];
        let r = evaluate_selection(&both, &f);
        assert!(r.values().all(blocked));
    }

    #[test]
    fn unknown_home_fails_closed_for_stock_launchers() {
        let mut f = facts();
        f.home_known = false;
        f.usable_launchers.clear();
        assert!(blocked(&evaluate_selection(&[p("com.google.android.tvlauncher")], &f)[&p("com.google.android.tvlauncher")]));
    }

    #[test]
    fn setupwraith_wifi() {
        let sw = p(SETUPWRAITH);
        let mut f = facts();
        f.setupwraith_owns_wifi = Some(true);
        assert!(blocked(&evaluate_selection(std::slice::from_ref(&sw), &f)[&sw]));
        f.setupwraith_owns_wifi = None;
        assert!(blocked(&evaluate_selection(std::slice::from_ref(&sw), &f)[&sw]));
        f.setupwraith_owns_wifi = Some(false);
        assert!(matches!(evaluate_selection(std::slice::from_ref(&sw), &f)[&sw], Verdict::NeedsConfirm(_)));
    }

    #[test]
    fn inputs_and_handlers_blocked() {
        let r = evaluate_selection(&[p("com.tcl.tvinput"), p("com.tcl.tv"), p("com.android.tv.settings")], &facts());
        assert!(r.values().all(blocked));
    }

    #[test]
    fn checks_that_did_not_run_fail_closed() {
        let mut f = facts();
        f.input_hardware_checked = false;
        assert!(blocked(&evaluate_selection(&[p("com.tcl.logkit")], &f)[&p("com.tcl.logkit")]));
        let mut f = facts();
        f.handlers_checked = false;
        assert!(blocked(&evaluate_selection(&[p("com.tcl.logkit")], &f)[&p("com.tcl.logkit")]));
    }

    #[test]
    fn core_stays_blocked_and_essentials_need_confirmation() {
        let r = evaluate_selection(&[p("com.android.permissioncontroller"), p("com.android.vending"), p("com.google.android.leanback.ime")], &facts());
        assert!(blocked(&r[&p("com.android.permissioncontroller")]));
        assert!(matches!(r[&p("com.android.vending")], Verdict::NeedsConfirm(_)));
        assert!(matches!(r[&p("com.google.android.leanback.ime")], Verdict::NeedsConfirm(_)));
    }

    #[test]
    fn cvte_launcher_kept() {
        let mut f = facts();
        f.cvte = true;
        f.usable_launchers.insert("com.cvte.tv.launcher.atv".into());
        f.usable_launchers.insert("studio.tvcustomizer.home".into());
        assert!(blocked(&evaluate_selection(&[p("com.cvte.tv.launcher.atv")], &f)[&p("com.cvte.tv.launcher.atv")]));
    }
}
