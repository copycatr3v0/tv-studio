//! Push a theme to the Studio launcher without rebuilding it, the way CleanYourTV's theme.ps1 does:
//! stage under /data/local/tmp, write through the launcher's ThemeProvider (files end up owned by the
//! launcher, which matters on Android 14), fall back to `cp` on firmware without `content write`, then
//! read the config back and restart Home only if it is Home.

use crate::install::{focused, status, STUDIO_HOME, STUDIO_PKG};
use crate::schema::{validate, Issue};
use crate::assets::valid_name;
use serde::Serialize;
use serde_json::Value;
use tvs_core::{Component, PackageName, Serial};
use tvs_transport::{ShellCommand, Transport, TransportError};

#[derive(Debug, Clone, Default)]
pub struct ThemeBundle {
    pub config: Value,
    /// Already normalized JPEGs (see `assets::normalize`), keyed by safe file name.
    pub wallpapers: Vec<(String, Vec<u8>)>,
    /// `wallpapers/captions.txt` content, when the theme names its pictures.
    pub captions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteMethod { Provider, Copy }

#[derive(Debug, Clone, Serialize)]
pub struct ThemeReport {
    pub written: Vec<(String, WriteMethod)>,
    pub config_verified: bool,
    pub restarted: bool,
    pub focused_after: Option<String>,
}

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ThemeError {
    #[error("The theme has problems; nothing was sent to the TV.")]
    Invalid(Vec<Issue>),
    #[error("The Studio launcher is not installed on this TV. Install it first (Launcher Builder).")]
    NoLauncher,
    #[error("Could not write {0} to the launcher: both the theme provider and the copy fallback were refused.")]
    WriteFailed(String),
    #[error("{0}")]
    Transport(#[from] TransportError),
}

fn files_dir() -> String { format!("/sdcard/Android/data/{STUDIO_PKG}/files") }
fn uri(rel: &str) -> String { format!("content://{STUDIO_PKG}.theme/{rel}") }
fn staged(rel: &str) -> String { format!("/data/local/tmp/tvs-theme-{}", rel.replace('/', "-")) }

fn output_failed(o: &tvs_transport::ShellOutput) -> bool {
    let text = format!("{}{}", o.stdout, o.stderr);
    o.code != Some(0) || text.contains("[ERROR]") || text.contains("Unsupported operation") || text.contains("Exception")
}

fn run(t: &dyn Transport, s: &Serial, c: Result<ShellCommand, tvs_transport::CommandError>) -> Result<tvs_transport::ShellOutput, ThemeError> {
    let c = c.map_err(|e| ThemeError::WriteFailed(e.to_string()))?;
    Ok(t.shell(s, &c)?)
}

/// Write one file: provider first, `cp` fallback. The staged copy is always removed.
fn put(t: &dyn Transport, s: &Serial, rel: &str, bytes: &[u8], tmp: &tempfile::TempDir) -> Result<WriteMethod, ThemeError> {
    let local = tmp.path().join(rel.replace('/', "-"));
    std::fs::write(&local, bytes).map_err(|e| ThemeError::WriteFailed(format!("{rel}: {e}")))?;
    let st = staged(rel);
    t.push(s, &local, &st)?;
    let dest = format!("{}/{rel}", files_dir());
    let _ = run(t, s, ShellCommand::new("rm").arg("-f").checked(&dest));
    let via_provider = run(t, s, ShellCommand::new("content").arg("write").arg("--uri").checked(&uri(rel)).and_then(|c| c.stdin_from(&st)))?;
    let method = if !output_failed(&via_provider) { Some(WriteMethod::Provider) } else {
        let cp = run(t, s, ShellCommand::new("cp").checked(&st).and_then(|c| c.checked(&dest)))?;
        (!output_failed(&cp)).then_some(WriteMethod::Copy)
    };
    let _ = run(t, s, ShellCommand::new("rm").arg("-f").checked(&st));
    method.ok_or_else(|| ThemeError::WriteFailed(rel.to_string()))
}

fn drop_file(t: &dyn Transport, s: &Serial, rel: &str) {
    let _ = run(t, s, ShellCommand::new("rm").arg("-f").checked(&format!("{}/{rel}", files_dir())));
    let _ = run(t, s, ShellCommand::new("content").arg("delete").arg("--uri").checked(&uri(rel)));
}

fn read_back(t: &dyn Transport, s: &Serial, rel: &str) -> Option<String> {
    if let Ok(o) = run(t, s, ShellCommand::new("content").arg("read").arg("--uri").checked(&uri(rel))) {
        if !output_failed(&o) { return Some(o.stdout); }
    }
    run(t, s, ShellCommand::new("cat").checked(&format!("{}/{rel}", files_dir()))).ok().filter(|o| o.code == Some(0)).map(|o| o.stdout)
}

fn restart_if_home(t: &dyn Transport, s: &Serial) -> (bool, Option<String>) {
    let was_front = focused(t, s).is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/")));
    let pkg = PackageName::new(STUDIO_PKG).unwrap();
    let _ = t.shell(s, &ShellCommand::new("am").arg("force-stop").package(&pkg));
    if was_front {
        let _ = t.shell(s, &ShellCommand::new("am").arg("start").arg("-n").component(&Component::new(STUDIO_HOME).unwrap()));
    }
    (was_front, focused(t, s))
}

/// Validate, push, verify. `revision` is written last so the launcher sees a complete theme.
pub fn deploy(t: &dyn Transport, s: &Serial, theme: &ThemeBundle, revision: &str) -> Result<ThemeReport, ThemeError> {
    let issues = validate(&theme.config);
    let bad_names: Vec<Issue> = theme.wallpapers.iter().filter(|(n, _)| !valid_name(n)).map(|(n, _)| Issue {
        level: crate::schema::IssueLevel::Error, key: format!("wallpapers/{n}"), message: "Invalid wallpaper file name".into(),
    }).collect();
    if !issues.is_empty() || !bad_names.is_empty() {
        return Err(ThemeError::Invalid(issues.into_iter().chain(bad_names).collect()));
    }
    if !status(t, s)?.installed { return Err(ThemeError::NoLauncher); }

    let tmp = tempfile::tempdir().map_err(|e| ThemeError::WriteFailed(e.to_string()))?;
    let _ = run(t, s, ShellCommand::new("mkdir").arg("-p").checked(&format!("{}/wallpapers", files_dir())));
    let mut written = vec![];
    let config = serde_json::to_vec_pretty(&theme.config).expect("valid JSON value");
    written.push(("config.json".to_string(), put(t, s, "config.json", &config, &tmp)?));

    // Replace the whole collection: a stale picture from an earlier theme would otherwise keep showing.
    drop_file(t, s, "wallpapers");
    if let Ok(o) = run(t, s, ShellCommand::new("ls").checked(&format!("{}/wallpapers", files_dir()))) {
        for name in o.stdout.split_whitespace().filter(|n| valid_name(n)) { drop_file(t, s, &format!("wallpapers/{name}")); }
    }
    drop_file(t, s, "wallpaper.jpg");
    for (name, bytes) in &theme.wallpapers {
        let rel = format!("wallpapers/{name}");
        written.push((rel.clone(), put(t, s, &rel, bytes, &tmp)?));
    }
    match &theme.captions {
        Some(c) => written.push(("wallpapers/captions.txt".into(), put(t, s, "wallpapers/captions.txt", c.as_bytes(), &tmp)?)),
        None => drop_file(t, s, "wallpapers/captions.txt"),
    }
    written.push(("theme_revision".into(), put(t, s, "theme_revision", revision.as_bytes(), &tmp)?));

    let config_verified = read_back(t, s, "config.json")
        .and_then(|txt| serde_json::from_str::<Value>(&txt).ok())
        .is_some_and(|v| v == theme.config);
    let (restarted, focused_after) = restart_if_home(t, s);
    Ok(ThemeReport { written, config_verified, restarted, focused_after })
}

/// Remove every theme file (back to the launcher's built-in look).
pub fn reset(t: &dyn Transport, s: &Serial, revision: &str) -> Result<ThemeReport, ThemeError> {
    if !status(t, s)?.installed { return Err(ThemeError::NoLauncher); }
    let tmp = tempfile::tempdir().map_err(|e| ThemeError::WriteFailed(e.to_string()))?;
    for rel in ["config.json", "wallpaper.jpg", "wallpapers/captions.txt", "wallpapers"] { drop_file(t, s, rel); }
    let written = vec![("theme_revision".to_string(), put(t, s, "theme_revision", revision.as_bytes(), &tmp)?)];
    let config_verified = read_back(t, s, "config.json").is_none_or(|c| c.trim().is_empty());
    let (restarted, focused_after) = restart_if_home(t, s);
    Ok(ThemeReport { written, config_verified, restarted, focused_after })
}
