//! Push a theme to the Studio launcher without rebuilding it, the way CleanYourTV's theme.ps1 does:
//! stage under /data/local/tmp, write through the launcher's ThemeProvider (files end up owned by the
//! launcher, which matters on Android 14), fall back to `cp` on firmware without `content write`, then
//! read the config back and restart Home only if it is Home.

use crate::install::{focused, status, STUDIO_HOME, STUDIO_PKG};
use crate::schema::{validate, Issue};
use crate::assets::valid_name;
use serde::Serialize;
use serde_json::{json, Value};
use tvs_core::{Component, PackageName, Serial};
use tvs_transport::{ShellCommand, Transport, TransportError};

#[derive(Debug, Clone, Default)]
pub struct ThemeBundle {
    pub config: Value,
    /// Already normalized JPEGs (see `assets::normalize`), keyed by safe file name.
    pub wallpapers: Vec<(String, Vec<u8>)>,
    /// `wallpapers/captions.txt` content, when the theme names its pictures.
    pub captions: Option<String>,
    /// The free-canvas layout (`layout.json`), already validated by `layout::validate` before it is sent.
    pub layout: Option<Value>,
    /// Background videos for the layout, keyed by plain file name (`rain.mp4`). At most `MAX_VIDEO_BYTES` each.
    pub videos: Vec<(String, Vec<u8>)>,
    /// Font files the layout names as `file:<name>`, keyed by plain file name. At most `MAX_FONTS`, `MAX_FONT_BYTES` each.
    pub fonts: Vec<(String, Vec<u8>)>,
    /// Pictures the layout gives individual apps (`apps.<pkg>.image`), already normalized to JPEG, under `tiles/`.
    pub tiles: Vec<(String, Vec<u8>)>,
    /// Other profiles' layouts, keyed by the ids the main layout lists (`profiles/<id>.json`).
    pub profiles: Vec<(String, Value)>,
}

pub const MAX_FONTS: usize = 6;
pub const MAX_FONT_BYTES: usize = 4 * 1024 * 1024;

pub const MAX_VIDEO_BYTES: usize = 60 * 1024 * 1024;
pub const MAX_TILES: usize = 60;
pub const MAX_TILE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_VIDEOS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteMethod { Provider, Copy }

#[derive(Debug, Clone, Serialize)]
pub struct ThemeReport {
    pub written: Vec<(String, WriteMethod)>,
    pub config_verified: bool,
    pub restarted: bool,
    /// Some(true) when the canvas screensaver was set and read back; None when the layout does not ask for it.
    #[serde(default)]
    pub screensaver: Option<bool>,
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
    let helper_on = crate::install::home_key_enabled(t, s);
    let pkg = PackageName::new(STUDIO_PKG).unwrap();
    let _ = t.shell(s, &ShellCommand::new("am").arg("force-stop").package(&pkg));
    // A force stop switches the launcher's Home-button helper off; Android binds it again only when the list changes.
    if helper_on { let _ = crate::install::enable_home_key(t, s); }
    if was_front {
        let _ = t.shell(s, &ShellCommand::new("am").arg("start").arg("-n").component(&Component::new(STUDIO_HOME).unwrap()));
    }
    (was_front, focused(t, s))
}

// ---------------------------------------------------------------- now playing

/// The launcher's media listener, as `cmd notification` names it.
pub const MEDIA_LISTENER: &str = "studio.tvcustomizer.home/com.gathin.tvhome.MediaListener";

/// Allow or disallow the launcher's notification listener (only ours; other apps' listeners are untouched).
pub fn set_media_listener(t: &dyn Transport, s: &Serial, on: bool) {
    let cmd = ShellCommand::new("cmd").arg("notification").arg(if on { "allow_listener" } else { "disallow_listener" }).arg(MEDIA_LISTENER);
    let _ = t.shell(s, &cmd);
}

// ---------------------------------------------------------------- screensaver

/// The canvas screensaver, as Android names it in `screensaver_components`.
pub const CANVAS_DREAM: &str = "studio.tvcustomizer.home/com.gathin.tvhome.CanvasDream";
/// The TV's screensaver before Studio changed it, kept beside the theme until Revert puts it back.
const SCREENSAVER_PREV: &str = "screensaver_prev";

fn secure_get(t: &dyn Transport, s: &Serial, key: &'static str) -> Option<String> {
    let o = t.shell(s, &ShellCommand::new("settings").arg("get").arg("secure").arg(key)).ok()?;
    let v = o.stdout.trim();
    (!v.is_empty() && v != "null").then(|| v.to_string())
}

fn secure_set(t: &dyn Transport, s: &Serial, key: &'static str, v: Option<&str>) {
    let base = ShellCommand::new("settings");
    let cmd = match v { Some(val) => base.arg("put").arg("secure").arg(key).checked(val), None => Ok(base.arg("delete").arg("secure").arg(key)) };
    if let Ok(c) = cmd { let _ = t.shell(s, &c); }
}

/// Make the canvas the TV's screensaver, keeping the earlier choice (first record wins) for Revert.
fn set_screensaver(t: &dyn Transport, s: &Serial, tmp: &tempfile::TempDir) -> Result<bool, ThemeError> {
    if secure_get(t, s, "screensaver_components").as_deref() != Some(CANVAS_DREAM) {
        if read_back(t, s, SCREENSAVER_PREV).is_none_or(|p| p.trim().is_empty()) {
            let prev = json!({"components": secure_get(t, s, "screensaver_components"), "enabled": secure_get(t, s, "screensaver_enabled")});
            put(t, s, SCREENSAVER_PREV, prev.to_string().as_bytes(), tmp)?;
        }
        secure_set(t, s, "screensaver_components", Some(CANVAS_DREAM));
    }
    secure_set(t, s, "screensaver_enabled", Some("1"));
    Ok(secure_get(t, s, "screensaver_components").as_deref() == Some(CANVAS_DREAM))
}

/// Put back the screensaver the TV had before Studio set the canvas one. True when something changed.
pub fn restore_screensaver(t: &dyn Transport, s: &Serial) -> bool {
    let Some(prev) = read_back(t, s, SCREENSAVER_PREV).filter(|p| !p.trim().is_empty()) else { return false };
    let v: Value = serde_json::from_str(prev.trim()).unwrap_or(Value::Null);
    let get = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    secure_set(t, s, "screensaver_components", get("components").as_deref());
    secure_set(t, s, "screensaver_enabled", get("enabled").as_deref());
    drop_file(t, s, SCREENSAVER_PREV);
    true
}

/// Problems with the canvas layout or its videos; empty when there is none or it is fine.
fn canvas_issues(theme: &ThemeBundle) -> Vec<Issue> {
    use crate::schema::IssueLevel::Error;
    let mut out: Vec<Issue> = vec![];
    if let Some(l) = &theme.layout {
        out.extend(crate::layout::validate(l).into_iter().map(|i| Issue { level: Error, key: format!("layout.{}", i.path), message: i.message }));
        let wants = l.pointer("/background/kind").and_then(Value::as_str) == Some("video");
        if let (true, Some(v)) = (wants, l.pointer("/background/video").and_then(Value::as_str)) {
            let file = if crate::layout::valid_video_name(v) { v.to_string() } else { format!("{v}.mp4") };
            if !theme.videos.iter().any(|(n, _)| *n == file) {
                out.push(Issue { level: Error, key: "layout.background.video".into(), message: format!("The video {file} was not included") });
            }
        }
        let wanted: std::collections::BTreeSet<&str> = l.get("items").and_then(Value::as_array).into_iter().flatten()
            .filter_map(|i| i.pointer("/style/font").and_then(Value::as_str)).filter_map(crate::layout::font_file).collect();
        for f in wanted {
            if !theme.fonts.iter().any(|(n, _)| n == f) {
                out.push(Issue { level: Error, key: "layout.style.font".into(), message: format!("The font file {f} was not included") });
            }
        }
    } else if !theme.videos.is_empty() || !theme.fonts.is_empty() || !theme.tiles.is_empty() {
        out.push(Issue { level: Error, key: "videos".into(), message: "Videos and fonts need a layout that uses them".into() });
    }
    if let Some(l) = &theme.layout {
        let listed = crate::layout::profile_ids(l);
        for (id, p) in &theme.profiles {
            if !listed.contains(id) { out.push(Issue { level: Error, key: format!("profiles/{id}"), message: "A profile the main layout does not list".into() }); continue; }
            out.extend(crate::layout::validate(p).into_iter().map(|i| Issue { level: Error, key: format!("profiles/{id}.{}", i.path), message: i.message }));
        }
    } else if !theme.profiles.is_empty() {
        out.push(Issue { level: Error, key: "profiles".into(), message: "Profiles need a main layout".into() });
    }
    if theme.tiles.len() > MAX_TILES {
        out.push(Issue { level: Error, key: "tiles".into(), message: format!("At most {MAX_TILES} app pictures") });
    }
    for (n, b) in &theme.tiles {
        if !crate::layout::valid_tile_image(n) || b.is_empty() || b.len() > MAX_TILE_BYTES {
            out.push(Issue { level: Error, key: format!("tiles/{n}"), message: "An app picture must be a JPG, PNG or WebP of at most 2 MB".into() });
        }
    }
    if theme.fonts.len() > MAX_FONTS {
        out.push(Issue { level: Error, key: "fonts".into(), message: format!("At most {MAX_FONTS} font files") });
    }
    for (n, b) in &theme.fonts {
        if !crate::layout::valid_font_name(n) {
            out.push(Issue { level: Error, key: format!("fonts/{n}"), message: "Invalid font file name".into() });
        } else if b.len() < 12 || b.len() > MAX_FONT_BYTES {
            out.push(Issue { level: Error, key: format!("fonts/{n}"), message: format!("A font file must be at most {} MB", MAX_FONT_BYTES / 1024 / 1024) });
        }
    }
    if theme.videos.len() > MAX_VIDEOS {
        out.push(Issue { level: Error, key: "videos".into(), message: format!("At most {MAX_VIDEOS} videos") });
    }
    for (n, b) in &theme.videos {
        if !crate::layout::valid_video_name(n) {
            out.push(Issue { level: Error, key: format!("video/{n}"), message: "Invalid video file name".into() });
        } else if b.is_empty() || b.len() > MAX_VIDEO_BYTES {
            out.push(Issue { level: Error, key: format!("video/{n}"), message: format!("A video must be 1 byte to {} MB", MAX_VIDEO_BYTES / 1024 / 1024) });
        }
    }
    out
}

/// Validate, push, verify. `revision` is written last so the launcher sees a complete theme.
pub fn deploy(t: &dyn Transport, s: &Serial, theme: &ThemeBundle, revision: &str) -> Result<ThemeReport, ThemeError> {
    let issues = validate(&theme.config);
    let bad_names: Vec<Issue> = theme.wallpapers.iter().filter(|(n, _)| !valid_name(n)).map(|(n, _)| Issue {
        level: crate::schema::IssueLevel::Error, key: format!("wallpapers/{n}"), message: "Invalid wallpaper file name".into(),
    }).collect();
    let extra = canvas_issues(theme);
    if !issues.is_empty() || !bad_names.is_empty() || !extra.is_empty() {
        return Err(ThemeError::Invalid(issues.into_iter().chain(bad_names).chain(extra).collect()));
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
    // The canvas layout and its videos: replaced as a set, so a video from an earlier theme never lingers.
    let _ = run(t, s, ShellCommand::new("mkdir").arg("-p").checked(&format!("{}/video", files_dir())));
    drop_file(t, s, "video");
    if let Ok(o) = run(t, s, ShellCommand::new("ls").checked(&format!("{}/video", files_dir()))) {
        for name in o.stdout.split_whitespace().filter(|n| crate::layout::valid_video_name(n)) { drop_file(t, s, &format!("video/{name}")); }
    }
    for (name, bytes) in &theme.videos {
        let rel = format!("video/{name}");
        written.push((rel.clone(), put(t, s, &rel, bytes, &tmp)?));
    }
    let _ = run(t, s, ShellCommand::new("mkdir").arg("-p").checked(&format!("{}/fonts", files_dir())));
    drop_file(t, s, "fonts");
    if let Ok(o) = run(t, s, ShellCommand::new("ls").checked(&format!("{}/fonts", files_dir()))) {
        for name in o.stdout.split_whitespace().filter(|n| crate::layout::valid_font_name(n)) { drop_file(t, s, &format!("fonts/{name}")); }
    }
    for (name, bytes) in &theme.fonts {
        let rel = format!("fonts/{name}");
        written.push((rel.clone(), put(t, s, &rel, bytes, &tmp)?));
    }
    let _ = run(t, s, ShellCommand::new("mkdir").arg("-p").checked(&format!("{}/profiles", files_dir())));
    drop_file(t, s, "profiles");
    if let Ok(o) = run(t, s, ShellCommand::new("ls").checked(&format!("{}/profiles", files_dir()))) {
        for name in o.stdout.split_whitespace().filter(|n| n.strip_suffix(".json").is_some_and(crate::layout::valid_profile_id)) { drop_file(t, s, &format!("profiles/{name}")); }
    }
    for (id, p) in &theme.profiles {
        let rel = format!("profiles/{id}.json");
        let bytes = serde_json::to_vec_pretty(p).expect("valid JSON value");
        written.push((rel.clone(), put(t, s, &rel, &bytes, &tmp)?));
    }
    let _ = run(t, s, ShellCommand::new("mkdir").arg("-p").checked(&format!("{}/tiles", files_dir())));
    drop_file(t, s, "tiles");
    if let Ok(o) = run(t, s, ShellCommand::new("ls").checked(&format!("{}/tiles", files_dir()))) {
        for name in o.stdout.split_whitespace().filter(|n| crate::layout::valid_tile_image(n)) { drop_file(t, s, &format!("tiles/{name}")); }
    }
    for (name, bytes) in &theme.tiles {
        let rel = format!("tiles/{name}");
        written.push((rel.clone(), put(t, s, &rel, bytes, &tmp)?));
    }
    match &theme.layout {
        Some(l) => {
            let bytes = serde_json::to_vec_pretty(l).expect("valid JSON value");
            written.push(("layout.json".into(), put(t, s, "layout.json", &bytes, &tmp)?));
        }
        None => drop_file(t, s, "layout.json"),
    }
    written.push(("theme_revision".into(), put(t, s, "theme_revision", revision.as_bytes(), &tmp)?));

    let config_verified = read_back(t, s, "config.json")
        .and_then(|txt| serde_json::from_str::<Value>(&txt).ok())
        .is_some_and(|v| v == theme.config);
    // Now Playing reads media sessions through the launcher's listener: allowed only while a layout shows it.
    let wants_media = theme.layout.as_ref().is_some_and(crate::layout::uses_now_playing) || theme.profiles.iter().any(|(_, p)| crate::layout::uses_now_playing(p));
    set_media_listener(t, s, wants_media);
    // The canvas screensaver follows the layout's switch; turning it off puts the TV's own one back.
    let wants_dream = theme.layout.as_ref().and_then(|l| l.get("screensaver")).and_then(Value::as_bool).unwrap_or(false);
    let screensaver = if wants_dream { Some(set_screensaver(t, s, &tmp)?) } else { restore_screensaver(t, s); None };
    let (restarted, focused_after) = restart_if_home(t, s);
    Ok(ThemeReport { written, config_verified, restarted, focused_after, screensaver })
}

/// Ask the TV to forget its PIN: the launcher deletes the PIN (and this file) the next time Home loads.
pub fn reset_pin(t: &dyn Transport, s: &Serial) -> Result<WriteMethod, ThemeError> {
    if !status(t, s)?.installed { return Err(ThemeError::NoLauncher); }
    let tmp = tempfile::tempdir().map_err(|e| ThemeError::WriteFailed(e.to_string()))?;
    let m = put(t, s, "pin_reset", b"1", &tmp)?;
    restart_if_home(t, s);
    Ok(m)
}

/// Remove every theme file (back to the launcher's built-in look).
pub fn reset(t: &dyn Transport, s: &Serial, revision: &str) -> Result<ThemeReport, ThemeError> {
    if !status(t, s)?.installed { return Err(ThemeError::NoLauncher); }
    let tmp = tempfile::tempdir().map_err(|e| ThemeError::WriteFailed(e.to_string()))?;
    restore_screensaver(t, s);
    set_media_listener(t, s, false);
    for rel in ["config.json", "layout.json", "wallpaper.jpg", "wallpapers/captions.txt", "wallpapers", "video", "fonts", "tiles", "profiles"] { drop_file(t, s, rel); }
    let written = vec![("theme_revision".to_string(), put(t, s, "theme_revision", revision.as_bytes(), &tmp)?)];
    let config_verified = read_back(t, s, "config.json").is_none_or(|c| c.trim().is_empty());
    let (restarted, focused_after) = restart_if_home(t, s);
    Ok(ThemeReport { written, config_verified, restarted, focused_after, screensaver: None })
}
