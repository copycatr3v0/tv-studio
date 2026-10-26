//! Hard safety rules. Nothing in the UI, a profile or a config file can override a `Blocked` verdict.
//!
//! Ported from CleanYourTV (MIT, © 2026 Gathin): `tools/recovery.py` CORE_RE/CORE_OK, the factory
//! never-start rule in PROMPT.md, and the not-a-launcher list in `tools/check.py`. Device-state guards
//! (Home owner, setupwraith Wi-Fi, TV_INPUT_HARDWARE, intent handlers) need live device queries and
//! arrive with the inventory module (M5); they will return the same `Verdict` type.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use tvs_core::{Component, PackageName};

pub mod guards;
pub use guards::{evaluate_selection, DeviceFacts, KNOWN_STOCK_LAUNCHERS};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "reason")]
pub enum Verdict {
    Allowed,
    /// Allowed only after the user confirms this specific item.
    NeedsConfirm(String),
    /// Never allowed. No override.
    Blocked(String),
}

static CORE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(android|com\.android\.systemui|com\.android\.shell|com\.android\.phone|com\.android\.settings|com\.android\.tv\.settings|com\.(google\.)?android\.(permissioncontroller|packageinstaller|ext\.services|ext\.shared|modulemetadata)|com\.google\.android\.overlay\.modules.*|com\.android\.providers\..*)$").unwrap()
});
static CORE_OK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^com\.android\.providers\.(calendar|contacts|userdictionary|blockednumber)$").unwrap()
});
/// Factory / service-menu packages: never started, not even to look (CVTE/Haier factory flag).
static FACTORY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(factory|hkcfac|fac\.menu|\.fac\.|factorymenu|factorytest|factory_test|factorymode|^com\.at\.factory\.)").unwrap()
});
static FALLBACK_HOME_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"/\S*FallbackHome$").unwrap());

pub const NOT_LAUNCHERS: &[&str] = &[
    "com.google.android.tungsten.setupwraith",
    "com.android.tv.settings",
    "com.android.settings",
    "com.android.provision",
];
pub const SETUPWRAITH: &str = "com.google.android.tungsten.setupwraith";

/// Core Android packages: with one disabled, Android may not boot (CleanYourTV #884).
pub fn is_core_package(p: &PackageName) -> bool {
    CORE_RE.is_match(p.as_str()) && !CORE_OK.is_match(p.as_str())
}

pub fn is_factory_package(p: &PackageName) -> bool {
    FACTORY_RE.is_match(p.as_str())
}

/// Whether a HOME claimant is a usable launcher (not a setup wizard, settings app, FallbackHome or factory test).
pub fn is_usable_launcher(c: &Component) -> bool {
    let pkg = c.package();
    !NOT_LAUNCHERS.contains(&pkg.as_str())
        && !FALLBACK_HOME_RE.is_match(c.as_str())
        && !is_factory_package(&pkg)
}

/// Static verdict for disabling a package, before any device-state guard runs.
pub fn can_disable(p: &PackageName) -> Verdict {
    if is_core_package(p) {
        return Verdict::Blocked(format!(
            "{p} is a core Android package; without it the TV may not boot again."
        ));
    }
    if p.as_str() == SETUPWRAITH {
        return Verdict::NeedsConfirm(
            "The setup wizard may own the saved Wi-Fi network; the Wi-Fi ownership check must pass first.".into(),
        );
    }
    Verdict::Allowed
}

/// Whether the app may ever *start* (launch, open an activity of) this package.
pub fn can_start(p: &PackageName) -> Verdict {
    if is_factory_package(p) {
        Verdict::Blocked(format!(
            "{p} looks like a factory/service package; opening it can set a factory flag that cannot be undone."
        ))
    } else {
        Verdict::Allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(s: &str) -> PackageName { PackageName::new(s).unwrap() }
    fn c(s: &str) -> Component { Component::new(s).unwrap() }

    // Cases mirror CleanYourTV tests/test_core_packages.py.
    #[test]
    fn core_blocked() {
        for name in [
            "com.android.ext.services", "com.android.ext.shared", "com.android.modulemetadata",
            "com.android.packageinstaller", "com.android.permissioncontroller", "com.android.phone",
            "com.android.providers.downloads", "com.android.providers.media", "com.android.providers.settings",
            "com.android.providers.tv", "com.android.settings", "com.android.shell", "com.android.systemui",
            "com.android.tv.settings", "com.google.android.ext.services", "com.google.android.ext.shared",
            "com.google.android.modulemetadata", "com.google.android.overlay.modules.ext.services",
            "com.google.android.overlay.modules.permissioncontroller", "com.google.android.packageinstaller",
            "com.google.android.permissioncontroller",
        ] {
            assert!(is_core_package(&p(name)), "{name} should be core");
            assert!(matches!(can_disable(&p(name)), Verdict::Blocked(_)), "{name}");
        }
    }

    #[test]
    fn core_exceptions_and_lookalikes_allowed() {
        for name in [
            "com.android.providers.calendar", "com.android.providers.contacts",
            "com.android.providers.userdictionary", "com.android.providers.blockednumber",
            "com.android.phone.extra", "com.android.settings.intelligence", "com.android.shell2",
            "com.android.tv.settings.overlay", "com.example.android.ext.services", "com.example.ads",
            "com.google.android.apps.tv.launcherx",
        ] {
            assert!(!is_core_package(&p(name)), "{name} should not be core");
        }
    }

    #[test]
    fn verdict_json_shape_matches_ui_schema() {
        assert_eq!(serde_json::to_string(&Verdict::Allowed).unwrap(), r#"{"kind":"Allowed"}"#);
        assert_eq!(serde_json::to_string(&Verdict::Blocked("x".into())).unwrap(), r#"{"kind":"Blocked","reason":"x"}"#);
        assert_eq!(serde_json::to_string(&Verdict::NeedsConfirm("y".into())).unwrap(), r#"{"kind":"NeedsConfirm","reason":"y"}"#);
    }

    #[test]
    fn setupwraith_needs_confirmation() {
        assert!(matches!(can_disable(&p(SETUPWRAITH)), Verdict::NeedsConfirm(_)));
    }

    #[test]
    fn factory_never_started() {
        for name in ["com.cvte.tv.hkcfac", "com.cvte.tv.fac.menu", "com.at.factory.test", "mediatek.factorymenu.ui", "com.duokan.factorytest"] {
            assert!(matches!(can_start(&p(name)), Verdict::Blocked(_)), "{name}");
        }
        assert_eq!(can_start(&p("com.netflix.ninja")), Verdict::Allowed);
    }

    #[test]
    fn launcher_usability() {
        assert!(is_usable_launcher(&c("com.google.android.apps.tv.launcherx/.home.HomeActivity")));
        assert!(!is_usable_launcher(&c("com.google.android.tungsten.setupwraith/.RecoveryActivity")));
        assert!(!is_usable_launcher(&c("com.tcl.keycustomfunctionservice/.FallbackHome")));
        assert!(!is_usable_launcher(&c("com.android.tv.settings/.system.FallbackHome")));
        assert!(!is_usable_launcher(&c("com.duokan.factorytest/.FactoryLauncher")));
    }
}
