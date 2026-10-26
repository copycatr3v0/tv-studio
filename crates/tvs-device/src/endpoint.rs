//! User-typed TV addresses and plain TCP reachability checks (no adb involved).

use serde::Serialize;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use tvs_core::Serial;

pub const DEFAULT_PORT: u16 = 5555;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EndpointError {
    #[error("Enter the TV's IP address, for example 192.168.1.50")]
    Empty,
    #[error("\"{0}\" is not an IP address or host name")]
    BadHost(String),
    #[error("\"{0}\" is not a port number (1-65535)")]
    BadPort(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    /// Accepts `192.168.1.50`, `192.168.1.50:5555`, `tv.local:5555`, `[fe80::1]:5555`. Missing port = `default_port`.
    pub fn parse(input: &str, default_port: u16) -> Result<Self, EndpointError> {
        let s = input.trim();
        if s.is_empty() { return Err(EndpointError::Empty); }
        let (host, port) = if let Some(rest) = s.strip_prefix('[') {
            let (h, after) = rest.split_once(']').ok_or_else(|| EndpointError::BadHost(s.into()))?;
            (h.to_string(), after.strip_prefix(':').map(str::to_string))
        } else if s.matches(':').count() == 1 {
            let (h, p) = s.split_once(':').unwrap();
            (h.to_string(), Some(p.to_string()))
        } else {
            (s.to_string(), None)
        };
        let host_ok = !host.is_empty()
            && host.len() <= 253
            && (host.parse::<std::net::IpAddr>().is_ok()
                || host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'));
        if !host_ok { return Err(EndpointError::BadHost(host)); }
        let port = match port {
            None => default_port,
            Some(p) => p.parse::<u16>().ok().filter(|n| *n > 0).ok_or(EndpointError::BadPort(p))?,
        };
        Ok(Self { host, port })
    }

    pub fn serial(&self) -> Serial {
        let s = if self.host.contains(':') { format!("[{}]:{}", self.host, self.port) } else { format!("{}:{}", self.host, self.port) };
        Serial::new(s).expect("validated endpoint is a valid serial")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", content = "detail", rename_all = "snake_case")]
pub enum PortState {
    /// Something accepts TCP connections.
    Open,
    /// The host answered and refused: reachable, nothing listening. The only proof a port is closed.
    Refused,
    /// No answer in time: wrong IP, TV asleep, firewall or client isolation.
    Timeout,
    /// Name did not resolve or the network is unreachable.
    Unreachable(String),
}

pub fn probe_port(ep: &Endpoint, timeout: Duration) -> PortState {
    let addrs: Vec<SocketAddr> = match (ep.host.as_str(), ep.port).to_socket_addrs() {
        Ok(a) => a.collect(),
        Err(e) => return PortState::Unreachable(e.to_string()),
    };
    let Some(addr) = addrs.first() else { return PortState::Unreachable("no address".into()) };
    match TcpStream::connect_timeout(addr, timeout) {
        Ok(_) => PortState::Open,
        Err(e) => match e.kind() {
            std::io::ErrorKind::ConnectionRefused => PortState::Refused,
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => PortState::Timeout,
            _ => PortState::Unreachable(e.to_string()),
        },
    }
}

/// What the user should do next for a given port state on the legacy ADB port.
pub fn advice(state: &PortState, port: u16) -> &'static str {
    match state {
        PortState::Open => "The TV is listening. Connect, then accept the \u{201c}Allow USB debugging?\u{201d} prompt on the TV.",
        PortState::Refused if port == DEFAULT_PORT => "The TV is reachable but network debugging is off. Turn on USB debugging and Network/ADB debugging in Developer options. Google TV sets often only offer Wireless debugging: use the pairing option instead.",
        PortState::Refused => "The TV is reachable but nothing listens on this port. Wireless debugging ports change every time it is switched on: read the current port on the TV.",
        PortState::Timeout => "No answer. Check the IP under Settings > Network on the TV, that the TV is on (not in standby), and that this PC is on the same network (guest Wi-Fi often blocks devices from seeing each other).",
        PortState::Unreachable(_) => "This PC has no route to that address. Check the IP and that both devices are on the same network.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn parses_addresses() {
        assert_eq!(Endpoint::parse(" 192.168.1.50 ", 5555).unwrap(), Endpoint { host: "192.168.1.50".into(), port: 5555 });
        assert_eq!(Endpoint::parse("192.168.1.50:37123", 5555).unwrap().port, 37123);
        assert_eq!(Endpoint::parse("living-room.local", 5555).unwrap().serial().as_str(), "living-room.local:5555");
        assert_eq!(Endpoint::parse("[fe80::1]:5555", 5555).unwrap().serial().as_str(), "[fe80::1]:5555");
        assert_eq!(Endpoint::parse("", 5555), Err(EndpointError::Empty));
        assert!(matches!(Endpoint::parse("192.168.1.50:0", 5555), Err(EndpointError::BadPort(_))));
        assert!(matches!(Endpoint::parse("192.168.1.50:99999", 5555), Err(EndpointError::BadPort(_))));
        assert!(matches!(Endpoint::parse("tv;reboot", 5555), Err(EndpointError::BadHost(_))));
        assert!(matches!(Endpoint::parse("a b", 5555), Err(EndpointError::BadHost(_))));
    }

    #[test]
    fn probes_open_and_refused() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let ep = Endpoint { host: "127.0.0.1".into(), port };
        assert_eq!(probe_port(&ep, Duration::from_secs(2)), PortState::Open);
        drop(l);
        assert_eq!(probe_port(&ep, Duration::from_secs(2)), PortState::Refused);
    }

    #[test]
    fn advice_differs_for_legacy_port() {
        assert_ne!(advice(&PortState::Refused, 5555), advice(&PortState::Refused, 37123));
    }
}
