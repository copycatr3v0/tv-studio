//! What tested TVs did with each package. Built by `tools/build_kb.py` from CleanYourTV's
//! `examples/*/debloat.txt` and privacy catalogue (MIT, (c) 2026 Gathin).
//!
//! This is *evidence* for the recommender and the UI. It never makes a package safe: the hard rules in
//! `tvs-safety` and the live device guards always decide.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Telemetry,
    AdsContent,
    RetailDemo,
    Preinstalled,
    Streaming,
    Voice,
    Casting,
    Updater,
    Launcher,
    Factory,
    Privacy,
    Other,
}

impl Category {
    /// Features a household may rely on: never pre-selected, always confirmed per item.
    pub fn is_sensitive(self) -> bool {
        matches!(self, Category::Streaming | Category::Voice | Category::Casting | Category::Updater | Category::Launcher)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyNote { pub category: String, pub note: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbEntry {
    pub disabled_in: u32,
    pub kept_in: u32,
    pub category: Category,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy: Option<PrivacyNote>,
}

impl KbEntry {
    /// Share of tested TVs that disabled it among those that mention it.
    pub fn disable_ratio(&self) -> f32 {
        let n = self.disabled_in + self.kept_in;
        if n == 0 { 0.0 } else { self.disabled_in as f32 / n as f32 }
    }
}

#[derive(Debug, Deserialize)]
struct KbFile { source: String, lists: u32, packages: BTreeMap<String, KbEntry> }

static KB: Lazy<KbFile> = Lazy::new(|| serde_json::from_str(include_str!("../data/kb.json")).expect("bundled kb.json is valid"));

pub fn get(package: &str) -> Option<&'static KbEntry> { KB.packages.get(package) }
pub fn list_count() -> u32 { KB.lists }
pub fn source() -> &'static str { &KB.source }
pub fn len() -> usize { KB.packages.len() }
pub fn is_empty() -> bool { KB.packages.is_empty() }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Safe, Balanced, Advanced }

/// Whether a package is pre-selected in a mode, from evidence alone (before device guards).
/// Advanced pre-selects the same as Balanced; it only *allows* more, with confirmation.
pub fn preselect(entry: Option<&KbEntry>, mode: Mode) -> bool {
    let Some(e) = entry else { return false };
    if e.category.is_sensitive() || e.category == Category::Factory || e.category == Category::Other {
        return false;
    }
    match mode {
        Mode::Safe => matches!(e.category, Category::Telemetry | Category::AdsContent | Category::RetailDemo | Category::Privacy)
            && e.disabled_in >= 2 && e.disable_ratio() >= 0.9,
        Mode::Balanced | Mode::Advanced => matches!(e.category, Category::Telemetry | Category::AdsContent | Category::RetailDemo | Category::Privacy | Category::Preinstalled)
            && e.disabled_in >= 1 && e.disable_ratio() >= 0.75,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_bundled_data() {
        assert!(list_count() >= 300);
        assert!(len() > 1000);
        assert!(source().contains("MIT"));
    }

    #[test]
    fn known_entries() {
        let logkit = get("com.tcl.logkit").unwrap();
        assert_eq!(logkit.category, Category::Telemetry);
        assert!(logkit.disabled_in > 10);
        assert_eq!(get("com.google.android.katniss").unwrap().category, Category::Voice);
        assert!(get("com.tcl.tv").unwrap().kept_in > 10);
    }

    #[test]
    fn sensitive_never_preselected() {
        for p in ["com.google.android.katniss", "com.netflix.ninja", "com.google.android.apps.tv.launcherx", "com.google.android.tungsten.setupwraith"] {
            for m in [Mode::Safe, Mode::Balanced, Mode::Advanced] {
                assert!(!preselect(get(p), m), "{p} {m:?}");
            }
        }
    }

    #[test]
    fn unknown_never_preselected() {
        assert!(!preselect(None, Mode::Advanced));
    }

    #[test]
    fn safe_is_subset_of_balanced() {
        for e in KB.packages.values() {
            if preselect(Some(e), Mode::Safe) { assert!(preselect(Some(e), Mode::Balanced)); }
        }
    }

    #[test]
    fn contested_packages_are_not_safe() {
        let e = KbEntry { disabled_in: 5, kept_in: 5, category: Category::Telemetry, notes: vec![], devices: vec![], privacy: None };
        assert!(!preselect(Some(&e), Mode::Safe));
        assert!(!preselect(Some(&e), Mode::Balanced));
    }
}
