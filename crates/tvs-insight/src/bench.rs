//! Home-screen benchmark. Method from CleanYourTV `tools/bench.py` (MIT, (c) 2026 Gathin):
//! - key response/movement from `dumpsys gfxinfo <pkg> framestats`, matching frames by completion time
//!   after the key (the uptime clock `gfxinfo` prints, not /proc/uptime); a key with no frame within
//!   150 ms caused no visual reaction and is left out;
//! - cold start from `am start -W` WaitTime after a force-stop with another app in front;
//! - return to Home from the input log to the launcher's resume event (same clock: epoch log column);
//! - idle CPU from /proc/stat deltas, launcher PSS from `dumpsys meminfo`.
//!
//! Nothing on the TV is changed except which app is in front and the gfxinfo counters.

use crate::stats::{compare, compare_single, stats, Stats, Verdict};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tvs_core::{Component, PackageName, Serial};
use tvs_transport::{ShellCommand, Transport, TransportError};

#[derive(Debug, Clone)]
pub struct BenchConfig {
    pub keys: u32,
    pub cold_starts: u32,
    pub home_returns: u32,
    pub idle_seconds: u32,
    /// Wait after each key before reading frames (CleanYourTV uses 1 s).
    pub key_wait: Duration,
    /// Wait between app switches.
    pub switch_wait: Duration,
}

impl Default for BenchConfig {
    fn default() -> Self { Self { keys: 24, cold_starts: 5, home_returns: 5, idle_seconds: 10, key_wait: Duration::from_millis(1000), switch_wait: Duration::from_millis(2500) } }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BenchRun {
    pub created: String,
    pub launcher: Option<String>,
    pub launcher_version: Option<String>,
    pub key_response_ms: Vec<f64>,
    pub key_movement_ms: Vec<f64>,
    pub keys_without_reaction: u32,
    pub frames: u64,
    pub janky_frames: u64,
    pub cold_start_ms: Vec<f64>,
    pub home_return_ms: Vec<f64>,
    pub idle_cpu_device_pct: Option<f64>,
    pub idle_cpu_launcher_pct: Option<f64>,
    pub launcher_pss_mb: Option<f64>,
    pub mem_available_mb: Option<f64>,
    pub notes: Vec<String>,
}

pub trait Pacer { fn pause(&mut self, d: Duration); fn progress(&mut self, _what: &str, _done: u32, _total: u32) {} fn cancelled(&mut self) -> bool { false } }

static UPTIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^Uptime:\s*(\d+)").unwrap());
static WAIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"WaitTime:\s*(\d+)").unwrap());
static PSS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*TOTAL(?: PSS)?:\s+(\d+)\b").unwrap());
static EVENT_TIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"eventTime=(\d+)").unwrap());
static EPOCH: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s*(\d+\.\d+)\s").unwrap());
static RESUMED: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(?:am|wm)_on_resume_called:\s*\[[^,]*,\s*([^,\]]+)").unwrap());

pub fn uptime_of(gfxinfo: &str) -> Option<u64> { UPTIME.captures(gfxinfo).and_then(|c| c[1].parse().ok()) }

/// PROFILEDATA rows as (flags, intended_vsync_ns, completed_ns); column positions come from the header.
pub fn frame_rows(framestats: &str) -> Vec<(i64, i64, i64)> {
    let mut rows = vec![];
    let mut inside = false;
    let mut cols: Option<[usize; 3]> = None;
    for line in framestats.lines() {
        if line.contains("---PROFILEDATA---") { inside = !inside; cols = None; continue; }
        if !inside { continue; }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        match cols {
            None => {
                let idx = |n: &str| parts.iter().position(|p| *p == n);
                if let (Some(a), Some(b), Some(c)) = (idx("Flags"), idx("IntendedVsync"), idx("FrameCompleted")) { cols = Some([a, b, c]); }
            }
            Some([a, b, c]) => {
                let get = |i: usize| parts.get(i).and_then(|v| v.parse::<i64>().ok());
                if let (Some(f), Some(iv), Some(fc)) = (get(a), get(b), get(c)) { rows.push((f, iv, fc)); }
            }
        }
    }
    rows
}

/// (response_ms, movement_ms) for one key that went down at uptime `t0_ms`, or None if it caused no frame.
pub fn key_latency(rows: &[(i64, i64, i64)], t0_ms: u64) -> Option<(f64, f64)> {
    let threshold = t0_ms as i64 * 1_000_000;
    let (mut first, mut last, mut prev) = (None::<i64>, 0i64, 0i64);
    for &(flags, intended, completed) in rows {
        if flags != 0 || completed < threshold { continue; }
        match first {
            None => { first = Some(completed); last = completed; prev = intended; }
            Some(_) => {
                if (intended - prev) as f64 / 1e6 > 50.0 { break; }
                last = completed; prev = intended;
            }
        }
    }
    let first = first?;
    let response = (first - threshold) as f64 / 1e6;
    if response > 150.0 { return None; }
    Some((response, (last - threshold) as f64 / 1e6))
}

fn counter(gfx: &str, label: &str) -> Option<u64> {
    gfx.lines().find_map(|l| l.trim().strip_prefix(label).and_then(|r| r.split_whitespace().next()).and_then(|n| n.parse().ok()))
}

/// The ACTION_DOWN uptime of the newest injected key named `name` in the log.
pub fn key_down_time(log: &str, name: &str) -> Option<u64> {
    log.lines().rev()
        .find(|l| l.contains("injectKeyEvent") && l.contains("action=ACTION_DOWN") && l.contains(name))
        .and_then(|l| EVENT_TIME.captures(l)).and_then(|c| c[1].parse().ok())
}

/// Return-to-Home time (ms) from a `logcat -v epoch` dump: HOME key → last resume of `class` after it.
pub fn home_return_from_log(log: &str, class: &str) -> Option<f64> {
    let mut key_at = None;
    let mut resumed_at = None;
    for line in log.lines() {
        let stamp = EPOCH.captures(line).and_then(|c| c[1].parse::<f64>().ok());
        if line.contains("injectKeyEvent") && line.contains("KEYCODE_HOME") && key_at.is_none() { key_at = stamp; }
        if let (Some(k), Some(cap), Some(t)) = (key_at, RESUMED.captures(line), stamp) {
            if cap[1].trim() == class && t >= k { resumed_at = Some(t); }
        }
    }
    Some(((resumed_at? - key_at?) * 1000.0).round())
}

/// (total ticks, idle ticks) from the `cpu ` line of /proc/stat.
pub fn cpu_line(stat: &str) -> Option<(u64, u64)> {
    let l = stat.lines().find(|l| l.starts_with("cpu "))?;
    let f: Vec<u64> = l.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
    (f.len() >= 5).then(|| (f.iter().sum(), f[3] + f[4]))
}

/// utime + stime of a process from its /proc/<pid>/stat line.
pub fn proc_ticks(stat: &str) -> Option<u64> {
    let rest = &stat[stat.rfind(')')? + 2..];
    let p: Vec<&str> = rest.split_whitespace().collect();
    Some(p.get(11)?.parse::<u64>().ok()? + p.get(12)?.parse::<u64>().ok()?)
}

fn sh(t: &dyn Transport, s: &Serial, c: &ShellCommand) -> Result<String, TransportError> { Ok(t.shell(s, c)?.stdout) }

fn settings_component(t: &dyn Transport, s: &Serial) -> Option<Component> {
    let out = sh(t, s, &ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief").arg("-a").arg("android.settings.SETTINGS")).ok()?;
    out.lines().find_map(|l| Component::new(l.trim()).ok())
}

const KEYS: [(&str, &str); 2] = [("KEYCODE_DPAD_RIGHT", "22"), ("KEYCODE_DPAD_LEFT", "21")];

/// Run the benchmark against whatever launcher currently answers Home.
pub fn run(t: &dyn Transport, s: &Serial, cfg: &BenchConfig, now: &str, pacer: &mut dyn Pacer) -> Result<BenchRun, TransportError> {
    let mut r = BenchRun { created: now.to_string(), ..Default::default() };
    let home = tvs_device::inventory::current_home(t, s);
    let Some(home) = home.and_then(|h| Component::new(h).ok()) else {
        r.notes.push("No single default Home: the launcher could not be identified, so nothing was measured.".into());
        return Ok(r);
    };
    let pkg: PackageName = home.package();
    r.launcher = Some(pkg.to_string());
    r.launcher_version = sh(t, s, &ShellCommand::new("dumpsys").arg("package").package(&pkg)).ok()
        .and_then(|o| o.lines().find_map(|l| l.trim().strip_prefix("versionName=").map(str::to_string)));

    // Start from Home.
    t.shell(s, &ShellCommand::new("input").arg("keyevent").arg("KEYCODE_HOME"))?;
    pacer.pause(cfg.switch_wait);

    // Keys.
    let mut log_seen = false;
    let mut log_misses = 0;
    for i in 0..cfg.keys {
        if pacer.cancelled() { r.notes.push("Stopped early.".into()); break; }
        pacer.progress("Key response", i, cfg.keys);
        let (name, _code) = KEYS[(i as usize / 4) % 2];
        let reset = sh(t, s, &ShellCommand::new("dumpsys").arg("gfxinfo").package(&pkg).arg("reset"))?;
        let t_before = uptime_of(&reset);
        t.shell(s, &ShellCommand::new("input").arg("keyevent").checked(name).expect("static key name"))?;
        pacer.pause(cfg.key_wait);
        let fs = sh(t, s, &ShellCommand::new("dumpsys").arg("gfxinfo").package(&pkg).arg("framestats"))?;
        let t_after = uptime_of(&fs);
        // The log line is only trusted inside the [reset, dump] window; otherwise time from the reset.
        let mut t0 = None;
        if log_seen || log_misses < 3 {
            let log = sh(t, s, &ShellCommand::new("logcat").arg("-d").arg("-b").arg("main").arg("-t").arg("2000")).unwrap_or_default();
            match key_down_time(&log, name) { Some(v) => { log_seen = true; t0 = Some(v); } None => log_misses += 1 }
        }
        if t0.is_none_or(|v| t_before.is_some_and(|b| v < b) || t_after.is_some_and(|a| v > a)) { t0 = t_before; }
        r.frames += counter(&fs, "Total frames rendered:").unwrap_or(0);
        r.janky_frames += counter(&fs, "Janky frames:").unwrap_or(0);
        match t0.and_then(|t0| key_latency(&frame_rows(&fs), t0)) {
            Some((resp, mov)) => { r.key_response_ms.push(resp); r.key_movement_ms.push(mov); }
            None => r.keys_without_reaction += 1,
        }
    }
    if !log_seen { r.notes.push("This TV doesn't log injected keys, so key timing starts just before each key (slightly pessimistic).".into()); }

    // Cold start and return to Home need another app in front.
    match settings_component(t, s) {
        None => r.notes.push("No Settings app found to switch to: cold start and return to Home were not measured.".into()),
        Some(other) => {
            for i in 0..cfg.cold_starts {
                if pacer.cancelled() { break; }
                pacer.progress("Cold start", i, cfg.cold_starts);
                t.shell(s, &ShellCommand::new("am").arg("start").arg("-W").arg("--activity-reorder-to-front").arg("-n").component(&other))?;
                pacer.pause(cfg.switch_wait);
                t.shell(s, &ShellCommand::new("am").arg("force-stop").package(&pkg))?;
                pacer.pause(cfg.switch_wait);
                let out = sh(t, s, &ShellCommand::new("am").arg("start").arg("-W").arg("-a").arg("android.intent.action.MAIN")
                    .arg("-c").arg("android.intent.category.HOME").arg("-n").component(&home))?;
                if let Some(c) = WAIT.captures(&out) { r.cold_start_ms.push(c[1].parse().unwrap_or(0.0)); }
                pacer.pause(cfg.switch_wait);
            }
            let class = {
                let c = home.as_str().split('/').nth(1).unwrap_or_default();
                if let Some(rel) = c.strip_prefix('.') { format!("{pkg}.{rel}") } else { c.to_string() }
            };
            for i in 0..cfg.home_returns {
                if pacer.cancelled() { break; }
                pacer.progress("Return to Home", i, cfg.home_returns);
                t.shell(s, &ShellCommand::new("am").arg("start").arg("-W").arg("--activity-reorder-to-front").arg("-n").component(&other))?;
                pacer.pause(cfg.switch_wait);
                t.shell(s, &ShellCommand::new("logcat").arg("-c").arg("-b").arg("main"))?;
                t.shell(s, &ShellCommand::new("input").arg("keyevent").arg("KEYCODE_HOME"))?;
                pacer.pause(cfg.switch_wait);
                let log = sh(t, s, &ShellCommand::new("logcat").arg("-d").arg("-b").arg("main").arg("-b").arg("events").arg("-v").arg("epoch"))?;
                if let Some(v) = home_return_from_log(&log, &class) { r.home_return_ms.push(v); }
            }
        }
    }

    // Idle CPU and memory, with Home in front.
    t.shell(s, &ShellCommand::new("input").arg("keyevent").arg("KEYCODE_HOME"))?;
    pacer.pause(cfg.switch_wait);
    let pid = sh(t, s, &ShellCommand::new("pidof").package(&pkg)).ok().and_then(|o| o.split_whitespace().next().and_then(|p| p.parse::<u32>().ok()));
    let proc_stat = |pid: u32| sh(t, s, &ShellCommand::new("cat").checked(&format!("/proc/{pid}/stat")).expect("numeric path")).ok().and_then(|o| proc_ticks(&o));
    let cpu0 = sh(t, s, &ShellCommand::new("cat").arg("/proc/stat")).ok().and_then(|o| cpu_line(&o));
    let p0 = pid.and_then(proc_stat);
    pacer.progress("Idle CPU", 0, 1);
    pacer.pause(Duration::from_secs(cfg.idle_seconds as u64));
    let cpu1 = sh(t, s, &ShellCommand::new("cat").arg("/proc/stat")).ok().and_then(|o| cpu_line(&o));
    let p1 = pid.and_then(proc_stat);
    if let (Some((t0, i0)), Some((t1, i1))) = (cpu0, cpu1) {
        let total = t1.saturating_sub(t0);
        if total > 0 { r.idle_cpu_device_pct = Some(100.0 * (1.0 - (i1.saturating_sub(i0)) as f64 / total as f64)); }
    }
    if let (Some(a), Some(b)) = (p0, p1) {
        if cfg.idle_seconds > 0 { r.idle_cpu_launcher_pct = Some(100.0 * b.saturating_sub(a) as f64 / (100.0 * cfg.idle_seconds as f64)); }
    }
    r.launcher_pss_mb = sh(t, s, &ShellCommand::new("dumpsys").arg("meminfo").package(&pkg)).ok()
        .and_then(|o| PSS.captures(&o).and_then(|c| c[1].parse::<f64>().ok())).map(|kb| (kb / 1024.0 * 10.0).round() / 10.0);
    r.mem_available_mb = sh(t, s, &ShellCommand::new("cat").arg("/proc/meminfo")).ok()
        .and_then(|o| tvs_device::probe::parse_meminfo(&o).1).map(|kb| (kb as f64 / 1024.0).round());
    Ok(r)
}

#[derive(Debug, Clone, Serialize)]
pub struct Metric { pub key: &'static str, pub label: &'static str, pub unit: &'static str, pub before: Option<Stats>, pub after: Option<Stats>, pub verdict: Verdict, pub note: &'static str }

/// Before/after table. Every metric is "lower is better".
pub fn comparison(before: &BenchRun, after: &BenchRun) -> Vec<Metric> {
    let m = |key, label, unit, b: &[f64], a: &[f64], note| {
        let (sb, sa) = (stats(b), stats(a));
        Metric { key, label, unit, verdict: compare(sb.as_ref(), sa.as_ref()), before: sb, after: sa, note }
    };
    let single = |key, label, unit, b: Option<f64>, a: Option<f64>, noise, note| Metric {
        key, label, unit, before: b.and_then(|v| stats(&[v])), after: a.and_then(|v| stats(&[v])), verdict: compare_single(b, a, noise), note };
    let jank = |r: &BenchRun| (r.frames > 0).then(|| 100.0 * r.janky_frames as f64 / r.frames as f64);
    vec![
        m("key_response", "Remote key response", "ms", &before.key_response_ms, &after.key_response_ms, "Time from key press to the first frame that reacts."),
        m("key_movement", "Focus movement finished", "ms", &before.key_movement_ms, &after.key_movement_ms, "Includes animation length: shorter animations improve this without the TV being faster."),
        m("cold_start", "Launcher cold start", "ms", &before.cold_start_ms, &after.cold_start_ms, "am start -W after a force-stop."),
        m("home_return", "Return to Home", "ms", &before.home_return_ms, &after.home_return_ms, "HOME key to the launcher resuming."),
        single("jank", "Janky frames", "%", jank(before), jank(after), 2.0, "One aggregate per run."),
        single("idle_cpu_launcher", "Launcher idle CPU", "% of one core", before.idle_cpu_launcher_pct, after.idle_cpu_launcher_pct, 0.5, "One reading per run."),
        single("idle_cpu_device", "Whole-TV idle CPU", "%", before.idle_cpu_device_pct, after.idle_cpu_device_pct, 3.0, "One reading per run; background apps vary a lot."),
        single("pss", "Launcher memory (PSS)", "MB", before.launcher_pss_mb, after.launcher_pss_mb, 5.0, "One reading per run."),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: &str = "Uptime: 5000 Realtime: 9000\n\nTotal frames rendered: 12\nJanky frames: 1 (8.3%)\n---PROFILEDATA---\nFlags,IntendedVsync,Vsync,OldestInputEvent,FrameCompleted,DequeueBufferDuration\n\
0,4990000000,4990000000,0,4995000000,0\n\
0,5004000000,5004000000,0,5020000000,0\n\
0,5020000000,5020000000,0,5036000000,0\n\
1,5036000000,5036000000,0,5052000000,0\n\
0,5150000000,5150000000,0,5160000000,0\n\
---PROFILEDATA---\n";

    #[test]
    fn frames_after_the_key() {
        let rows = frame_rows(FS);
        assert_eq!(rows.len(), 5);
        let (resp, mov) = key_latency(&rows, 5000).unwrap();
        assert_eq!(resp, 20.0);
        assert_eq!(mov, 36.0, "stops at the >50 ms gap; flagged frames ignored");
        assert_eq!(uptime_of(FS), Some(5000));
        assert_eq!(counter(FS, "Janky frames:"), Some(1));
    }

    #[test]
    fn no_reaction_is_dropped() {
        assert!(key_latency(&frame_rows(FS), 5100).is_none() || key_latency(&frame_rows(FS), 5100).unwrap().0 <= 150.0);
        assert!(key_latency(&frame_rows(FS), 4000).is_none(), "first frame 995 ms later is not this key's answer");
    }

    #[test]
    fn log_parsers() {
        let log = "10-26 12:00:00.1 I Input: injectKeyEvent: KeyEvent { action=ACTION_DOWN, keyCode=KEYCODE_DPAD_RIGHT, eventTime=4999 }\n\
                   10-26 12:00:00.2 I Input: injectKeyEvent: KeyEvent { action=ACTION_UP, keyCode=KEYCODE_DPAD_RIGHT, eventTime=5050 }";
        assert_eq!(key_down_time(log, "KEYCODE_DPAD_RIGHT"), Some(4999));
        assert_eq!(key_down_time(log, "KEYCODE_DPAD_LEFT"), None);
        let ev = "  1761480000.100  1000  1000 I wm_on_resume_called: [123,com.google.android.apps.tv.launcherx.home.HomeActivity,RESUME]\n\
                  1761480001.000  1000  1000 I Input: injectKeyEvent: KeyEvent { action=ACTION_DOWN, keyCode=KEYCODE_HOME }\n\
                  1761480001.420  1000  1000 I wm_on_resume_called: [124,com.google.android.apps.tv.launcherx.home.HomeActivity,RESUME]";
        assert_eq!(home_return_from_log(ev, "com.google.android.apps.tv.launcherx.home.HomeActivity"), Some(420.0));
    }

    #[test]
    fn proc_parsers() {
        assert_eq!(cpu_line("cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 1 2 3 4 5"), Some((1000, 850)));
        let stat = "1234 (com.x (odd) name) S 1 1234 0 0 -1 4194560 100 0 0 0 70 30 0 0 20 0";
        assert_eq!(proc_ticks(stat), Some(100));
    }

    #[test]
    fn comparison_is_cautious() {
        let before = BenchRun { key_response_ms: vec![40.0, 42.0, 41.0, 39.0, 43.0, 40.5], frames: 100, janky_frames: 10, launcher_pss_mb: Some(180.0), ..Default::default() };
        let after = BenchRun { key_response_ms: vec![20.0, 21.0, 19.0, 22.0, 20.5, 21.5], frames: 100, janky_frames: 9, launcher_pss_mb: Some(60.0), ..Default::default() };
        let c = comparison(&before, &after);
        let v = |k: &str| c.iter().find(|m| m.key == k).unwrap().verdict;
        assert_eq!(v("key_response"), Verdict::Better);
        assert_eq!(v("jank"), Verdict::NoClearChange);
        assert_eq!(v("pss"), Verdict::Better);
        assert_eq!(v("cold_start"), Verdict::NotEnoughData);
    }
}
