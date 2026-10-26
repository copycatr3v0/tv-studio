//! Shared, validated identifiers. Anything that ends up inside a shell string on the TV is a newtype
//! here whose constructor rejects shell syntax, so the rest of the code cannot build an unsafe command
//! from raw user input or device output.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdError {
    #[error("not a valid Android package name: {0:?}")]
    Package(String),
    #[error("not a valid component name (pkg/Class): {0:?}")]
    Component(String),
    #[error("not a valid ADB serial: {0:?}")]
    Serial(String),
}

/// Same rule CleanYourTV applies before a name reaches the TV shell (`recovery.py PACKAGE_RE`).
static PACKAGE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$").unwrap());
static COMPONENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+/[A-Za-z0-9_.$]+$").unwrap());
/// host:port (IPv4 / hostname), [ipv6]:port, an mDNS service name, or a USB serial. No spaces or shell metacharacters.
static SERIAL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9._:\[\]\-]{1,128}$").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PackageName(String);

impl PackageName {
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        if PACKAGE_RE.is_match(&s) { Ok(Self(s)) } else { Err(IdError::Package(s)) }
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Component(String);

impl Component {
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        if COMPONENT_RE.is_match(&s) { Ok(Self(s)) } else { Err(IdError::Component(s)) }
    }
    pub fn package(&self) -> PackageName {
        PackageName(self.0.split('/').next().unwrap_or_default().to_string())
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Serial(String);

impl Serial {
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        if SERIAL_RE.is_match(&s) { Ok(Self(s)) } else { Err(IdError::Serial(s)) }
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

macro_rules! string_newtype {
    ($t:ty) => {
        impl TryFrom<String> for $t { type Error = IdError; fn try_from(s: String) -> Result<Self, IdError> { <$t>::new(s) } }
        impl From<$t> for String { fn from(v: $t) -> String { v.0 } }
        impl fmt::Display for $t { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) } }
    };
}
string_newtype!(PackageName);
string_newtype!(Component);
string_newtype!(Serial);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names() {
        for ok in ["com.google.android.tvlauncher", "a.b", "com.tcl.tv", "com.foo_bar.Baz1"] {
            assert!(PackageName::new(ok).is_ok(), "{ok}");
        }
        for bad in ["", "android", "com", "com.foo;reboot", "com.foo bar", "com.foo$(id)", ".com.x", "com..x", "1com.x", "com.x\n"] {
            assert!(PackageName::new(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn components() {
        let c = Component::new("com.google.android.apps.tv.launcherx/.MainActivity").unwrap();
        assert_eq!(c.package().as_str(), "com.google.android.apps.tv.launcherx");
        assert!(Component::new("com.x/Outer$Inner").is_ok());
        assert!(Component::new("com.x/.A;rm").is_err());
        assert!(Component::new("android").is_err());
    }

    #[test]
    fn serials() {
        for ok in ["192.168.1.50:5555", "[fe80::1]:5555", "adb-ABC123-xyz._adb-tls-connect._tcp", "R58M1234"] {
            assert!(Serial::new(ok).is_ok(), "{ok}");
        }
        for bad in ["", "1.2.3.4 5555", "x;reboot", "a|b", "$(x)"] {
            assert!(Serial::new(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn serde_rejects_invalid() {
        assert!(serde_json::from_str::<PackageName>("\"com.ok.app\"").is_ok());
        assert!(serde_json::from_str::<PackageName>("\"bad name\"").is_err());
    }
}
