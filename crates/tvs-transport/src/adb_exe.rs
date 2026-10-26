use crate::{classify_error, parse_devices, parse_mdns, MdnsService, DeviceEntry, PairingCode, Result, ShellCommand, ShellOutput, Transport, TransportError};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tvs_core::Serial;
use wait_timeout::ChildExt;

#[cfg(windows)]
const ADB_NAME: &str = "adb.exe";
#[cfg(not(windows))]
const ADB_NAME: &str = "adb";

/// Locate adb: bundled resources first, then `TVS_ADB`, the Android SDK, then PATH.
pub fn find_adb(bundled_dir: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(d) = bundled_dir {
        candidates.push(d.join("platform-tools").join(ADB_NAME));
        candidates.push(d.join(ADB_NAME));
    }
    if let Some(p) = std::env::var_os("TVS_ADB") { candidates.push(PathBuf::from(p)); }
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(p) = std::env::var_os(var) { candidates.push(PathBuf::from(p).join("platform-tools").join(ADB_NAME)); }
    }
    if let Some(p) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(PathBuf::from(p).join("Android").join("Sdk").join("platform-tools").join(ADB_NAME));
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) { candidates.push(dir.join(ADB_NAME)); }
    }
    candidates.into_iter().find(|p| p.is_file())
}

pub struct AdbExeTransport {
    adb: PathBuf,
    timeout: Duration,
}

impl AdbExeTransport {
    pub fn new(adb: PathBuf) -> Self { Self { adb, timeout: Duration::from_secs(30) } }
    pub fn with_timeout(mut self, t: Duration) -> Self { self.timeout = t; self }
    pub fn adb_path(&self) -> &Path { &self.adb }

    pub fn version(&self) -> Result<String> {
        let out = self.run(&["version"], Duration::from_secs(10))?;
        Ok(out.stdout.lines().next().unwrap_or_default().to_string())
    }

    /// Stop the adb server started from this adb.exe (needed before replacing the file on Windows).
    pub fn kill_server(&self) -> Result<()> { self.run(&["kill-server"], Duration::from_secs(10)).map(|_| ()) }

    fn run(&self, args: &[&str], timeout: Duration) -> Result<ShellOutput> {
        let start = Instant::now();
        let mut cmd = Command::new(&self.adb);
        cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound { TransportError::AdbMissing } else { TransportError::Io(e.to_string()) }
        })?;
        // Drain pipes on threads so a large dumpsys cannot block the child on a full pipe.
        let mut so = child.stdout.take().unwrap();
        let mut se = child.stderr.take().unwrap();
        let t_out = std::thread::spawn(move || { let mut s = Vec::new(); let _ = so.read_to_end(&mut s); s });
        let t_err = std::thread::spawn(move || { let mut s = Vec::new(); let _ = se.read_to_end(&mut s); s });
        let status = match child.wait_timeout(timeout).map_err(|e| TransportError::Io(e.to_string()))? {
            Some(s) => s,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TransportError::Timeout(timeout.as_millis() as u64));
            }
        };
        let stdout = String::from_utf8_lossy(&t_out.join().unwrap_or_default()).replace("\r\n", "\n");
        let stderr = String::from_utf8_lossy(&t_err.join().unwrap_or_default()).replace("\r\n", "\n");
        Ok(ShellOutput { stdout, stderr, code: status.code(), elapsed_ms: start.elapsed().as_millis() as u64 })
    }

    fn check(out: ShellOutput) -> Result<ShellOutput> {
        if out.code != Some(0) {
            if let Some(e) = classify_error(&format!("{}\n{}", out.stderr, out.stdout)) { return Err(e); }
        }
        Ok(out)
    }
}

impl Transport for AdbExeTransport {
    fn devices(&self) -> Result<Vec<DeviceEntry>> {
        let out = self.run(&["devices", "-l"], Duration::from_secs(10))?;
        Ok(parse_devices(&out.stdout))
    }

    fn connect(&self, endpoint: &Serial) -> Result<()> {
        let out = self.run(&["connect", endpoint.as_str()], Duration::from_secs(15))?;
        // adb connect exits 0 even on failure; judge by its text.
        let text = format!("{}{}", out.stdout, out.stderr);
        if text.contains("connected to") { return Ok(()); }
        Err(classify_error(&text).unwrap_or_else(|| TransportError::Other(text.trim().into())))
    }

    fn pair(&self, endpoint: &Serial, code: &PairingCode) -> Result<()> {
        let out = self.run(&["pair", endpoint.as_str(), code.as_str()], Duration::from_secs(30))?;
        let text = format!("{}{}", out.stdout, out.stderr);
        if text.contains("Successfully paired") { Ok(()) } else { Err(TransportError::PairingFailed(text.trim().into())) }
    }

    fn disconnect(&self, endpoint: &Serial) -> Result<()> {
        self.run(&["disconnect", endpoint.as_str()], Duration::from_secs(10)).map(|_| ())
    }

    fn mdns_services(&self) -> Result<Vec<MdnsService>> {
        let out = self.run(&["mdns", "services"], Duration::from_secs(10))?;
        Ok(parse_mdns(&out.stdout))
    }

    fn shell(&self, serial: &Serial, cmd: &ShellCommand) -> Result<ShellOutput> {
        let mut args: Vec<&str> = vec!["-s", serial.as_str(), "shell"];
        args.extend(cmd.adb_args());
        Self::check(self.run(&args, self.timeout)?)
    }

    fn push(&self, serial: &Serial, local: &Path, remote: &str) -> Result<()> {
        // The app only ever writes its own staging files on the TV; anything else is a bug, refused here too.
        if !crate::is_staging_path(remote) { return Err(TransportError::Io(format!("refusing to push outside the staging area: {remote}"))); }
        let local = local.to_str().ok_or_else(|| TransportError::Io("non-UTF-8 path".into()))?;
        Self::check(self.run(&["-s", serial.as_str(), "push", local, remote], Duration::from_secs(300))?).map(|_| ())
    }

    fn pull(&self, serial: &Serial, remote: &str, local: &Path) -> Result<()> {
        let local = local.to_str().ok_or_else(|| TransportError::Io("non-UTF-8 path".into()))?;
        Self::check(self.run(&["-s", serial.as_str(), "pull", remote, local], Duration::from_secs(300))?).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary_is_typed() {
        let t = AdbExeTransport::new(PathBuf::from("/definitely/not/here/adb"));
        assert_eq!(t.devices().unwrap_err(), TransportError::AdbMissing);
    }

    #[test]
    fn find_adb_prefers_bundled() {
        let dir = std::env::temp_dir().join(format!("tvs-adb-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("platform-tools")).unwrap();
        let exe = dir.join("platform-tools").join(ADB_NAME);
        std::fs::write(&exe, b"").unwrap();
        assert_eq!(find_adb(Some(&dir)), Some(exe));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
