//! First-run adb setup. Google's prebuilt platform-tools are covered by the Android SDK licence, which
//! doesn't allow redistributing them inside another product (§3.4), so the installer doesn't ship adb.
//! Instead, when the user clicks "Get adb from Google", this downloads Google's official zip over HTTPS
//! from Google's own download host and extracts only the three files adb needs. Nothing runs during the
//! download; the extracted adb.exe is only run later, exactly as a manually installed one would be.

use crate::{blocking, AppState, CmdError, CmdResult};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Manager;

pub const PLATFORM_TOOLS_URL: &str = "https://dl.google.com/android/repository/platform-tools-latest-windows.zip";
/// The only files taken from the zip, written under fixed names (no path from the archive is ever used).
pub const FILES: [&str; 3] = ["adb.exe", "AdbWinApi.dll", "AdbWinUsbApi.dll"];
const MAX_DOWNLOAD: u64 = 64 * 1024 * 1024;
const MAX_FILE: u64 = 32 * 1024 * 1024;

/// Where the downloaded copy lives (per user, next to the database).
pub fn adb_dir(data_dir: &Path) -> PathBuf { data_dir.join("platform-tools") }
pub fn downloaded_adb(data_dir: &Path) -> Option<PathBuf> {
    let p = adb_dir(data_dir).join("adb.exe");
    p.is_file().then_some(p)
}

/// Google serves the zip from dl.google.com, sometimes through its gvt1.com download CDN.
pub fn allowed_host(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").to_ascii_lowercase();
    let host = host.rsplit_once('@').map_or(host.as_str(), |(_, h)| h).split(':').next().unwrap_or("").to_string();
    host == "dl.google.com" || host == "dl-ssl.google.com" || host.ends_with(".gvt1.com")
}

/// Extract the three adb files from `zip` into `dest`, replacing an older copy only once all three are written.
pub fn extract(zip: &[u8], dest: &Path) -> Result<(), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip)).map_err(|e| format!("Not a valid zip from Google: {e}"))?;
    let staging = dest.with_extension("partial");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    for name in FILES {
        let mut entry = archive.by_name(&format!("platform-tools/{name}")).map_err(|_| format!("The download is missing {name}"))?;
        if entry.size() > MAX_FILE { return Err(format!("{name} is unexpectedly large")); }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        (&mut entry).take(MAX_FILE + 1).read_to_end(&mut buf).map_err(|e| format!("{name}: {e}"))?;
        if buf.len() as u64 > MAX_FILE { return Err(format!("{name} is unexpectedly large")); }
        if !buf.starts_with(b"MZ") { return Err(format!("{name} is not a Windows program")); }
        std::fs::File::create(staging.join(name)).and_then(|mut f| f.write_all(&buf)).map_err(|e| e.to_string())?;
    }
    // Keep Google's licence notice next to the binaries when the zip has one.
    if let Ok(mut n) = archive.by_name("platform-tools/NOTICE.txt") {
        let mut buf = vec![];
        if (&mut n).take(MAX_FILE).read_to_end(&mut buf).is_ok() { let _ = std::fs::write(staging.join("NOTICE.txt"), buf); }
    }
    let _ = std::fs::remove_dir_all(dest);
    std::fs::rename(&staging, dest).map_err(|e| format!("Could not finish installing adb: {e}"))
}

const TERMS_URL: &str = "https://developer.android.com/studio/releases/platform-tools";

/// Open Google's platform-tools page in the default browser. The URL is fixed; nothing from the UI is passed on.
#[tauri::command]
pub fn open_platform_tools_terms() -> CmdResult<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", TERMS_URL]).creation_flags(0x0800_0000)
            .spawn().map(|_| ()).map_err(|e| CmdError::Invalid(e.to_string()))
    }
    #[cfg(not(windows))]
    { Err(CmdError::Invalid(format!("Open {TERMS_URL} in your browser."))) }
}

#[derive(Debug, Serialize)]
pub struct AdbSetup { path: String, version: Option<String>, signature: String }

/// Windows' Authenticode verdict for the extracted adb.exe ("Valid", "NotSigned", ...). A broken signature
/// (signed, then modified) is refused; anything else is reported so the user can see it.
#[cfg(windows)]
fn signature_status(p: &Path) -> String {
    use std::os::windows::process::CommandExt;
    let script = format!("$s = Get-AuthenticodeSignature -LiteralPath '{}'; \"$($s.Status)|$($s.SignerCertificate.Subject)\"", p.display().to_string().replace('\'', "''"));
    std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x0800_0000).output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_else(|| "Unknown|".into())
}
#[cfg(not(windows))]
fn signature_status(_p: &Path) -> String { "Unknown|".into() }

/// Download and install adb. Only runs when the user asks for it.
#[tauri::command]
pub async fn install_adb(app: tauri::AppHandle) -> CmdResult<AdbSetup> {
    let _busy = crate::begin_change(&app.state::<AppState>(), "adb setup")?;
    let data_dir = app.state::<AppState>().data_dir.clone();
    blocking(move || {
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(120)).try_proxy_from_env(true).build();
        let resp = agent.get(PLATFORM_TOOLS_URL).call().map_err(|e| CmdError::Invalid(format!("Could not download adb from Google: {e}")))?;
        if !allowed_host(resp.get_url()) {
            return Err(CmdError::Invalid(format!("The download was redirected to an unexpected address ({}); nothing was installed.", resp.get_url())));
        }
        let mut zip = vec![];
        resp.into_reader().take(MAX_DOWNLOAD + 1).read_to_end(&mut zip).map_err(|e| CmdError::Invalid(format!("Download interrupted: {e}")))?;
        if zip.len() as u64 > MAX_DOWNLOAD { return Err(CmdError::Invalid("The download is unexpectedly large; nothing was installed.".into())); }
        let dest = adb_dir(&data_dir);
        // Windows locks a running adb.exe: stop our own adb server before replacing an older copy.
        if let Some(old) = downloaded_adb(&data_dir) { let _ = tvs_transport::AdbExeTransport::new(old).kill_server(); }
        extract(&zip, &dest).map_err(CmdError::Invalid)?;
        let adb = dest.join("adb.exe");
        let sig = signature_status(&adb);
        if sig.starts_with("HashMismatch") || sig.starts_with("NotTrusted") {
            let _ = std::fs::remove_dir_all(&dest);
            return Err(CmdError::Invalid(format!("adb.exe failed Windows' signature check ({sig}); it was removed.")));
        }
        let version = tvs_transport::AdbExeTransport::new(adb.clone()).version().ok();
        Ok(AdbSetup { path: adb.display().to_string(), version, signature: sig })
    }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (n, b) in entries {
            w.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(b).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn hosts() {
        assert!(allowed_host(PLATFORM_TOOLS_URL));
        assert!(allowed_host("https://r3---sn-abc.gvt1.com/edgedl/android/repository/x.zip"));
        assert!(!allowed_host("http://dl.google.com/x.zip"), "plain HTTP");
        assert!(!allowed_host("https://dl.google.com.evil.example/x.zip"));
        assert!(!allowed_host("https://evil.example/dl.google.com/x.zip"));
        assert!(!allowed_host("https://dl.google.com@evil.example/x.zip"));
    }

    #[test]
    fn extracts_only_the_three_files() {
        let z = zip_with(&[
            ("platform-tools/adb.exe", b"MZadb"), ("platform-tools/AdbWinApi.dll", b"MZapi"), ("platform-tools/AdbWinUsbApi.dll", b"MZusb"),
            ("platform-tools/fastboot.exe", b"MZfb"), ("../../evil.exe", b"MZx"), ("platform-tools/NOTICE.txt", b"licence"),
        ]);
        let d = tempfile::tempdir().unwrap();
        let dest = adb_dir(d.path());
        extract(&z, &dest).unwrap();
        let mut names: Vec<_> = std::fs::read_dir(&dest).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
        names.sort();
        assert_eq!(names, vec!["AdbWinApi.dll", "AdbWinUsbApi.dll", "NOTICE.txt", "adb.exe"]);
        assert!(!d.path().join("evil.exe").exists() && !d.path().parent().unwrap().join("evil.exe").exists());
        assert_eq!(downloaded_adb(d.path()), Some(dest.join("adb.exe")));
    }

    #[test]
    fn refuses_incomplete_or_wrong_files_and_keeps_the_old_copy() {
        let d = tempfile::tempdir().unwrap();
        let dest = adb_dir(d.path());
        let good = zip_with(&[("platform-tools/adb.exe", b"MZ1"), ("platform-tools/AdbWinApi.dll", b"MZ"), ("platform-tools/AdbWinUsbApi.dll", b"MZ")]);
        extract(&good, &dest).unwrap();
        let missing = zip_with(&[("platform-tools/adb.exe", b"MZ2")]);
        assert!(extract(&missing, &dest).unwrap_err().contains("missing"));
        let script = zip_with(&[("platform-tools/adb.exe", b"#!/bin/sh"), ("platform-tools/AdbWinApi.dll", b"MZ"), ("platform-tools/AdbWinUsbApi.dll", b"MZ")]);
        assert!(extract(&script, &dest).unwrap_err().contains("not a Windows program"));
        assert!(extract(b"not a zip", &dest).is_err());
        assert_eq!(std::fs::read(dest.join("adb.exe")).unwrap(), b"MZ1", "a failed update leaves the working copy alone");
    }
}
