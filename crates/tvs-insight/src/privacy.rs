//! Privacy audit with graded evidence (CleanYourTV `tools/privacy.py`, MIT, (c) 2026 Gathin).
//! It reports what is known about each package and how it is known. It never claims a package is
//! listening: on firmware where many packages share the system UID, "has RECORD_AUDIO" means nothing.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeMap;
use tvs_core::{PackageName, Serial};
use tvs_transport::{ShellCommand, Transport, TransportError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category { Voice, Acr, Telemetry, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MicHistory { Recorded, NoneReturned, Unknown }

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub package: String,
    pub enabled: bool,
    pub category: Category,
    pub note: String,
    pub evidence: Vec<String>,
    pub mic_history: MicHistory,
}

#[derive(Debug, Clone, Serialize)]
pub struct Audit { pub entries: Vec<Entry>, pub skipped: usize, pub installed: usize }

static NAME_HINT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)(?:^|\.)(acr|analytics|assistant|alexa|hotword|inscape|listen|microphone|privacy|speech|telemetry|tracker|tracking|voice)").unwrap());
static UID: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\s*(?:userId|appId)=(\d+)").unwrap());
static RECENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"; time=(?:0|[+-](?:\d+[dhms])*\d+ms) ago(?:;| \(running\)|$)").unwrap());
const VOICE_SETTINGS: [&str; 3] = ["voice_interaction_service", "voice_recognition_service", "assistant"];
const SYSTEM_UID: u32 = 1000;
pub const MAX_PROBES: usize = 25;

/// True: recorded access in the returned history. False: "No operations." None: unrecognised output.
pub fn mic_history(appops: &str) -> MicHistory {
    let low = appops.to_lowercase();
    if appops.trim().is_empty() || low.contains("error") || low.contains("exception") { return MicHistory::Unknown; }
    for l in appops.lines().map(str::trim) {
        if l.starts_with("RECORD_AUDIO: ") && (l.ends_with(" (running)") || RECENT.is_match(l)) { return MicHistory::Recorded; }
    }
    if appops.lines().any(|l| l.trim() == "No operations.") { MicHistory::NoneReturned } else { MicHistory::Unknown }
}

pub fn declares_record_audio(dumpsys: &str) -> bool {
    let own = dumpsys.split("\nShared users:").next().unwrap_or_default();
    let mut inside = false;
    for l in own.lines().map(str::trim) {
        if l.starts_with("requested permissions:") { inside = true; continue; }
        if inside && (l.starts_with("install permissions:") || l.starts_with("runtime permissions:") || l.starts_with("User ")) { break; }
        if inside && l.starts_with("android.permission.RECORD_AUDIO") { return true; }
    }
    false
}

fn kb_category(pkg: &str) -> Option<(Category, String)> {
    let e = tvs_kb::get(pkg)?;
    if let Some(p) = &e.privacy {
        let c = match p.category.as_str() { "voice" => Category::Voice, "acr" => Category::Acr, "telemetry" => Category::Telemetry, _ => Category::Unknown };
        return Some((c, p.note.clone()));
    }
    match e.category {
        tvs_kb::Category::Voice => Some((Category::Voice, e.notes.first().cloned().unwrap_or_else(|| "voice assistant (from tested TVs)".into()))),
        tvs_kb::Category::Telemetry => Some((Category::Telemetry, e.notes.first().cloned().unwrap_or_else(|| "telemetry (from tested TVs)".into()))),
        _ => None,
    }
}

pub fn audit(t: &dyn Transport, s: &Serial) -> Result<Audit, TransportError> {
    let pkgs = tvs_device::inventory::list_packages(t, s).map_err(|e| TransportError::Other(e.to_string()))?;
    let mut owners: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for key in VOICE_SETTINGS {
        let v = t.shell(s, &ShellCommand::new("settings").arg("get").arg("secure").arg(key))?.stdout.trim().to_string();
        if let Some(pkg) = v.split('/').next().filter(|_| v.contains('/')) {
            if PackageName::new(pkg).is_ok() { owners.entry(pkg.to_string()).or_default().push(key); }
        }
    }
    let mut candidates: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).filter(|n| kb_category(n).is_some() || owners.contains_key(*n)).collect();
    let hinted: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).filter(|n| !candidates.contains(n) && NAME_HINT.is_match(n)).collect();
    candidates.extend(hinted);
    let skipped = candidates.len().saturating_sub(MAX_PROBES);
    let mut entries = vec![];
    for name in candidates.into_iter().take(MAX_PROBES) {
        let pn = PackageName::new(name).expect("from pm list");
        let (mut category, mut note) = kb_category(name).unwrap_or((Category::Unknown, "matched by name only; nothing else is known".into()));
        let mut evidence = vec![];
        if let Some(keys) = owners.get(name) {
            if category == Category::Unknown { category = Category::Voice; note = "the TV's own speech service".into(); }
            evidence.push(format!("named by the TV's settings: {}", keys.join(", ")));
        }
        if let Some(e) = tvs_kb::get(name) {
            if e.disabled_in + e.kept_in > 0 { evidence.push(format!("tested TVs: disabled on {}, kept on {}", e.disabled_in, e.kept_in)); }
        }
        let dump = t.shell(s, &ShellCommand::new("dumpsys").arg("package").package(&pn))?.stdout;
        let uid = UID.captures(&dump).and_then(|c| c[1].parse::<u32>().ok());
        if uid == Some(SYSTEM_UID) {
            evidence.push("shares the system UID: the permissions it shows are the system's, not evidence that it records".into());
        } else if declares_record_audio(&dump) {
            evidence.push("declares RECORD_AUDIO (asking is not recording)".into());
        }
        let ops = t.shell(s, &ShellCommand::new("appops").arg("get").package(&pn).arg("RECORD_AUDIO"))?;
        let mic = mic_history(&format!("{}{}", ops.stdout, ops.stderr));
        evidence.push(match mic {
            MicHistory::Recorded => "microphone access appears in the TV's returned history".into(),
            MicHistory::NoneReturned => "no microphone access in the returned history".into(),
            MicHistory::Unknown => "microphone history could not be read on this TV".into(),
        });
        let enabled = pkgs.iter().any(|p| p.name == name && p.enabled);
        entries.push(Entry { package: name.into(), enabled, category, note, evidence, mic_history: mic });
    }
    Ok(Audit { entries, skipped, installed: pkgs.len() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mic_history_parsing() {
        assert_eq!(mic_history("RECORD_AUDIO: allow; time=+3h12m5s123ms ago; duration=+2s"), MicHistory::Recorded);
        assert_eq!(mic_history("RECORD_AUDIO: allow (running)"), MicHistory::Recorded);
        assert_eq!(mic_history("No operations."), MicHistory::NoneReturned);
        assert_eq!(mic_history("Error: Unknown package"), MicHistory::Unknown);
        assert_eq!(mic_history("RECORD_AUDIO: allow"), MicHistory::Unknown);
    }

    #[test]
    fn requested_only() {
        assert!(declares_record_audio("    requested permissions:\n      android.permission.RECORD_AUDIO\n    install permissions:\n"));
        assert!(!declares_record_audio("    requested permissions:\n      android.permission.INTERNET\n    install permissions:\n      android.permission.RECORD_AUDIO: granted=true\n"));
        assert!(!declares_record_audio("requested permissions:\n  x\nShared users:\n  requested permissions:\n    android.permission.RECORD_AUDIO\n"));
    }
}
