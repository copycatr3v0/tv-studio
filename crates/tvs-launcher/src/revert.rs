//! The launcher part of "Revert to original". Home goes back to the recorded stock launcher and is
//! confirmed on screen *before* the Studio launcher is uninstalled, so the TV is never left without Home.

use crate::install::{hand_back_home, status, uninstall, STUDIO_PKG};
use std::time::Duration;
use tvs_core::{Component, Serial};
use tvs_transport::Transport;

/// Returns (verified changes, things kept with the reason). Does nothing if the launcher is not installed.
pub fn remove_studio_launcher(t: &dyn Transport, serial: &Serial, baseline_home: Option<&str>, settle: Duration) -> (Vec<String>, Vec<String>) {
    let (mut changed, mut kept) = (vec![], vec![]);
    if !status(t, serial).map(|s| s.installed).unwrap_or(false) { return (changed, kept); }
    let stock = baseline_home.and_then(|h| Component::new(h.to_string()).ok()).filter(|c| c.package().as_str() != STUDIO_PKG);
    let other_launcher = tvs_device::inventory::list_packages(t, serial).ok().is_some_and(|pk| {
        tvs_device::inventory::gather_facts(t, serial, &pk, &[]).usable_launchers.iter().any(|l| l != STUDIO_PKG)
    });
    let back = match hand_back_home(t, serial, stock.as_ref(), settle) {
        Ok(b) => b,
        Err(e) => { kept.push(format!("Studio launcher kept: {e}")); return (changed, kept); }
    };
    let safe = if stock.is_some() { back.verified } else { other_launcher && back.verified };
    if !safe {
        kept.push(back.note.unwrap_or_else(|| "Studio launcher kept: no other launcher is enabled to take over Home.".into()));
        return (changed, kept);
    }
    // The canvas screensaver lives in the launcher: give the TV its own one back before removing it.
    if crate::theme::restore_screensaver(t, serial) { changed.push("Screensaver → the TV's previous one".into()); }
    match uninstall(t, serial) {
        Ok(true) => changed.push(format!("Home → {} (confirmed on screen); Studio launcher and its theme files removed",
            back.focused.unwrap_or_else(|| "stock launcher".into()))),
        Ok(false) => kept.push("The TV did not remove the Studio launcher.".into()),
        Err(e) => kept.push(format!("Studio launcher removal failed: {e}")),
    }
    (changed, kept)
}
