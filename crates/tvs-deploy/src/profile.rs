//! A deployment profile: everything chosen before "Apply Everything", versioned and serializable.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub const PROFILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LauncherPlan {
    pub install: bool,
    pub make_home: bool,
    /// Disable the stock launcher once the Studio launcher is confirmed as Home. Guarded at run time.
    pub disable_stock: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebloatPlan {
    pub packages: Vec<String>,
    /// Packages whose "needs confirmation" warning the user accepted, item by item.
    pub confirmed: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "hostname", rename_all = "snake_case")]
pub enum PrivateDns {
    #[default]
    Keep,
    /// Android's default ("opportunistic").
    Automatic,
    Off,
    Hostname(String),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SettingsPlan {
    /// "1.0", "0.5" or "0"; None keeps the TV's value.
    pub animation_scale: Option<String>,
    pub private_dns: PrivateDns,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default = "v1")]
    pub version: u32,
    pub name: String,
    /// When set, the profile only runs on this TV (build identity, not IP address).
    #[serde(default)]
    pub device_identity: Option<String>,
    #[serde(default)]
    pub launcher: LauncherPlan,
    #[serde(default)]
    pub debloat: DebloatPlan,
    #[serde(default)]
    pub settings: SettingsPlan,
    /// Whether the theme supplied with the run is applied.
    #[serde(default)]
    pub apply_theme: bool,
}

fn v1() -> u32 { PROFILE_VERSION }

impl Profile {
    pub fn new(name: &str) -> Self {
        Self { version: PROFILE_VERSION, name: name.into(), device_identity: None, launcher: LauncherPlan::default(),
               debloat: DebloatPlan::default(), settings: SettingsPlan::default(), apply_theme: false }
    }
}

static HOST: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\.)+[A-Za-z]{2,63}$").unwrap());
pub const ANIMATION_VALUES: [&str; 4] = ["1.0", "0.5", "0.25", "0"];

pub fn valid_dns_hostname(h: &str) -> bool { h.len() <= 253 && HOST.is_match(h) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_round_trips_and_defaults() {
        let p: Profile = serde_json::from_str(r#"{"name":"Living room"}"#).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.settings.private_dns, PrivateDns::Keep);
        let mut q = Profile::new("x");
        q.settings.private_dns = PrivateDns::Hostname("dns.adguard-dns.com".into());
        let back: Profile = serde_json::from_value(serde_json::to_value(&q).unwrap()).unwrap();
        assert_eq!(back, q);
    }

    #[test]
    fn accepts_the_ui_shapes() {
        // emptyProfile() and a filled profile exactly as src/lib/ipc.ts sends them.
        let empty = r#"{"version":1,"name":"My TV setup","device_identity":null,"launcher":{"install":false,"make_home":false,"disable_stock":false},
            "debloat":{"packages":[],"confirmed":[]},"settings":{"animation_scale":null,"private_dns":{"mode":"keep"}},"apply_theme":false}"#;
        assert_eq!(serde_json::from_str::<Profile>(empty).unwrap(), Profile::new("My TV setup"));
        let full = r#"{"version":1,"name":"x","device_identity":"m|fp","launcher":{"install":true,"make_home":true,"disable_stock":true},
            "debloat":{"packages":["com.tcl.logkit"],"confirmed":[]},"settings":{"animation_scale":"0.5","private_dns":{"mode":"hostname","hostname":"dns.adguard-dns.com"}},"apply_theme":true}"#;
        let p: Profile = serde_json::from_str(full).unwrap();
        assert_eq!(p.settings.private_dns, PrivateDns::Hostname("dns.adguard-dns.com".into()));
        assert_eq!(serde_json::to_value(&p).unwrap(), serde_json::from_str::<serde_json::Value>(full).unwrap());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(serde_json::from_str::<Profile>(r#"{"name":"x","launcher":{"instal":true}}"#).is_err());
        assert!(serde_json::from_str::<Profile>(r#"{"name":"x","debloat_packages":[]}"#).is_err());
    }

    #[test]
    fn dns_hostnames() {
        assert!(valid_dns_hostname("dns.adguard-dns.com"));
        assert!(valid_dns_hostname("one.one.one.one"));
        for bad in ["", "localhost", "a;b.com", "dns .com", "-x.com", "x..com", "1.1.1.1"] {
            assert!(!valid_dns_hostname(bad), "{bad}");
        }
    }
}
