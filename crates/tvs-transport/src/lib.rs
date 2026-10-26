//! ADB transport. Every call names the device serial explicitly, every shell call carries exactly one
//! command (CleanYourTV: chained `pm` commands can silently stop after the first), and every call has a
//! timeout. `AdbExeTransport` drives the bundled adb.exe; `MockTransport` is a scriptable fake TV.

mod adb_exe;
mod command;
pub mod mock;

pub use adb_exe::{find_adb, AdbExeTransport};
pub use command::{is_staging_path, CommandError, ShellCommand, STAGING_PREFIX};

use serde::Serialize;
use std::path::Path;
use tvs_core::Serial;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    /// Exit code reported by adb. Never treated as proof that the device state changed.
    pub code: Option<i32>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    Device,
    Unauthorized,
    Offline,
    NoPermissions,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeviceEntry {
    pub serial: Serial,
    pub state: DeviceState,
    pub model: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum TransportError {
    #[error("adb executable not found")]
    AdbMissing,
    #[error("device {0} is not connected")]
    NoDevice(String),
    #[error("device has not authorized this computer; accept the prompt on the TV")]
    Unauthorized,
    #[error("TV unreachable: {0}")]
    Unreachable(String),
    #[error("connection refused: {0}")]
    Refused(String),
    #[error("pairing failed: {0}")]
    PairingFailed(String),
    #[error("timed out after {0} ms")]
    Timeout(u64),
    #[error("i/o error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TransportError>;

pub trait Transport: Send + Sync {
    fn devices(&self) -> Result<Vec<DeviceEntry>>;
    /// Legacy authorized TCP connect (`adb connect host:port`).
    fn connect(&self, endpoint: &Serial) -> Result<()>;
    /// Android 11+ wireless debugging pairing (`adb pair host:port code`).
    fn pair(&self, endpoint: &Serial, code: &PairingCode) -> Result<()>;
    fn disconnect(&self, endpoint: &Serial) -> Result<()>;
    /// Wireless-debugging services advertised on the LAN (`adb mdns services`).
    fn mdns_services(&self) -> Result<Vec<MdnsService>>;
    fn shell(&self, serial: &Serial, cmd: &ShellCommand) -> Result<ShellOutput>;
    fn push(&self, serial: &Serial, local: &Path, remote: &str) -> Result<()>;
    fn pull(&self, serial: &Serial, remote: &str, local: &Path) -> Result<()>;
}

/// Six-digit wireless-debugging pairing code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingCode(String);

impl PairingCode {
    pub fn new(s: &str) -> std::result::Result<Self, CommandError> {
        let t = s.trim();
        if t.len() == 6 && t.chars().all(|c| c.is_ascii_digit()) {
            Ok(Self(t.to_string()))
        } else {
            Err(CommandError::BadArgument("pairing code must be 6 digits".into()))
        }
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MdnsKind { Connect, Pairing, Legacy }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MdnsService {
    pub name: String,
    pub kind: MdnsKind,
    pub endpoint: Serial,
}

/// Parse `adb mdns services` output (tab- or space-separated: name, service type, host:port).
pub fn parse_mdns(out: &str) -> Vec<MdnsService> {
    out.lines()
        .filter(|l| !l.starts_with("List of discovered") && !l.trim().is_empty())
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            let [name, ty, ep] = cols.as_slice() else { return None };
            let kind = match ty.trim_end_matches('.') {
                "_adb-tls-connect._tcp" => MdnsKind::Connect,
                "_adb-tls-pairing._tcp" => MdnsKind::Pairing,
                "_adb._tcp" => MdnsKind::Legacy,
                _ => return None,
            };
            Some(MdnsService { name: (*name).to_string(), kind, endpoint: Serial::new(*ep).ok()? })
        })
        .collect()
}

/// Parse `adb devices -l` output.
pub fn parse_devices(out: &str) -> Vec<DeviceEntry> {
    out.lines()
        .filter(|l| !l.starts_with("List of devices") && !l.starts_with('*') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let serial = Serial::new(parts.next()?).ok()?;
            let state = match parts.next()? {
                "device" => DeviceState::Device,
                "unauthorized" => DeviceState::Unauthorized,
                "offline" => DeviceState::Offline,
                "no" => DeviceState::NoPermissions,
                _ => DeviceState::Other,
            };
            let mut model = None;
            let mut product = None;
            for kv in parts {
                if let Some(v) = kv.strip_prefix("model:") { model = Some(v.replace('_', " ")); }
                if let Some(v) = kv.strip_prefix("product:") { product = Some(v.to_string()); }
            }
            Some(DeviceEntry { serial, state, model, product })
        })
        .collect()
}

/// Map adb's stderr/stdout wording to a typed error.
pub fn classify_error(text: &str) -> Option<TransportError> {
    let t = text.to_ascii_lowercase();
    if t.contains("unauthorized") { return Some(TransportError::Unauthorized); }
    if t.contains("connection refused") { return Some(TransportError::Refused(text.trim().into())); }
    if t.contains("no route to host") || t.contains("timed out") || t.contains("unable to connect") || t.contains("failed to connect") {
        return Some(TransportError::Unreachable(text.trim().into()));
    }
    if t.contains("device '") && t.contains("not found") || t.contains("no devices/emulators found") {
        return Some(TransportError::NoDevice(text.trim().into()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_devices_list() {
        let out = "List of devices attached\n\
            192.168.1.50:5555      device product:G09_4K_GB model:Smart_TV_Pro device:G09 transport_id:3\n\
            192.168.1.51:5555      unauthorized transport_id:4\n\
            adb-XYZ._adb-tls-connect._tcp offline\n\n";
        let d = parse_devices(out);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].state, DeviceState::Device);
        assert_eq!(d[0].model.as_deref(), Some("Smart TV Pro"));
        assert_eq!(d[0].product.as_deref(), Some("G09_4K_GB"));
        assert_eq!(d[1].state, DeviceState::Unauthorized);
        assert_eq!(d[2].state, DeviceState::Offline);
    }

    #[test]
    fn classifies_errors() {
        assert_eq!(classify_error("error: device unauthorized."), Some(TransportError::Unauthorized));
        assert!(matches!(classify_error("cannot connect to 1.2.3.4:5555: Connection refused (10061)"), Some(TransportError::Refused(_))));
        assert!(matches!(classify_error("failed to connect to '1.2.3.4:5555': No route to host"), Some(TransportError::Unreachable(_))));
        assert!(matches!(classify_error("adb: device '1.2.3.4:5555' not found"), Some(TransportError::NoDevice(_))));
        assert_eq!(classify_error("Success"), None);
    }

    #[test]
    fn parses_mdns() {
        let out = "List of discovered mdns services\nadb-R58M12-AbC\t_adb-tls-connect._tcp\t192.168.1.50:37123\nadb-R58M12-AbC  _adb-tls-pairing._tcp.  192.168.1.50:41234\nweird line\n";
        let m = parse_mdns(out);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].kind, MdnsKind::Connect);
        assert_eq!(m[0].endpoint.as_str(), "192.168.1.50:37123");
        assert_eq!(m[1].kind, MdnsKind::Pairing);
    }

    #[test]
    fn pairing_code() {
        assert!(PairingCode::new(" 123456 ").is_ok());
        assert!(PairingCode::new("12345").is_err());
        assert!(PairingCode::new("12345a").is_err());
    }
}
