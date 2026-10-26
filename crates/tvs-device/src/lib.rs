//! Device discovery, connection diagnostics and read-only capability detection.

pub mod endpoint;
pub mod oem;
pub mod inventory;
pub mod probe;

pub use endpoint::{advice, probe_port, Endpoint, EndpointError, PortState, DEFAULT_PORT};
pub use oem::{OemFamily, OemNote};
pub use probe::{probe, Capabilities, DeviceInfo, Finding, HomeCandidate, HomeState, Severity};

#[cfg(test)]
mod tests;
