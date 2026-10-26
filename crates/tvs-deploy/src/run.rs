//! Execute a plan. Every step is check-then-act, so re-running a step whose effect already happened is
//! harmless: that is how a lost connection or a resumed run is handled, never by blind retries.
//! A step whose dependencies didn't succeed is skipped; a failed critical step stops the run.

use crate::plan::{get_setting, DeployInput, Plan, Preview, Step, StepKind};
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;
use tvs_core::{PackageName, Serial};
use tvs_device::inventory::{enabled_packages, gather_facts, list_packages};
use tvs_launcher::install::{self, STUDIO_PKG};
use tvs_recovery::{capture, disable_in_batches, Snapshot};
use tvs_safety::{evaluate_selection, Verdict};
use tvs_transport::{ShellCommand, Transport, TransportError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus { Running, Done, AlreadyDone, Failed, Skipped, Reconnecting }

#[derive(Debug, Clone, Serialize)]
pub struct Event { pub step: String, pub title: String, pub status: StepStatus, pub message: String, pub finished: usize, pub total: usize }

#[derive(Debug, Clone, Serialize)]
pub struct StepResult { pub id: String, pub title: String, pub status: StepStatus, pub message: String }

#[derive(Debug, Clone, Serialize)]
pub struct RunReport { pub results: Vec<StepResult>, pub completed: bool, pub cancelled: bool, pub stopped: Option<String> }

pub trait Hooks {
    fn event(&mut self, e: &Event);
    fn journal(&mut self, step: &str, phase: &str, detail: Option<&str>);
    /// Persist the pre-change snapshot (and the baseline if this TV has none). Err stops the run.
    fn save_snapshot(&mut self, identity: &str, snap: &Snapshot) -> Result<(), String>;
    fn pause(&mut self, d: Duration);
    fn cancelled(&mut self) -> bool { false }
    fn now(&mut self) -> String { String::new() }
    /// How many times, and how long apart, to try reconnecting after the TV drops off (reboot, Wi-Fi).
    fn reconnect_policy(&self) -> (u32, Duration) { (12, Duration::from_secs(15)) }
}

enum Outcome { Done(String), AlreadyDone(String) }
enum Fail { Transport(TransportError), Msg(String) }
impl From<TransportError> for Fail { fn from(e: TransportError) -> Self { Fail::Transport(e) } }
type R = Result<Outcome, Fail>;

fn is_connection(e: &TransportError) -> bool {
    matches!(e, TransportError::NoDevice(_) | TransportError::Timeout(_) | TransportError::Unreachable(_) | TransportError::Refused(_) | TransportError::Io(_))
}

pub fn run(t: &dyn Transport, s: &Serial, plan: &Plan, input: &DeployInput, hooks: &mut dyn Hooks) -> RunReport {
    let total = plan.steps.len();
    let mut results: Vec<StepResult> = vec![];
    let mut status: BTreeMap<String, StepStatus> = BTreeMap::new();
    let mut stopped: Option<String> = None;
    let mut cancelled = false;

    if !plan.runnable {
        return RunReport { results: vec![], completed: false, cancelled: false, stopped: Some("The plan has blocked steps; nothing was changed.".into()) };
    }

    for st in &plan.steps {
        let finished = results.len();
        let emit = |h: &mut dyn Hooks, status: StepStatus, message: &str| h.event(&Event {
            step: st.id.clone(), title: st.title.clone(), status, message: message.into(), finished, total });

        let skip_reason = if let Some(why) = &stopped { Some(format!("Not run: {why}")) }
            else if hooks.cancelled() { cancelled = true; stopped = Some("cancelled".into()); Some("Not run: cancelled".into()) }
            else { st.depends_on.iter().find(|d| !matches!(status.get(*d), Some(StepStatus::Done | StepStatus::AlreadyDone)))
                .map(|d| format!("Not run: it depends on \u{201c}{}\u{201d}, which did not succeed", plan.steps.iter().find(|x| &x.id == d).map(|x| x.title.as_str()).unwrap_or(d))) };
        if let Some(why) = skip_reason {
            hooks.journal(&st.id, "skipped", Some(&why));
            emit(hooks, StepStatus::Skipped, &why);
            status.insert(st.id.clone(), StepStatus::Skipped);
            results.push(StepResult { id: st.id.clone(), title: st.title.clone(), status: StepStatus::Skipped, message: why });
            continue;
        }
        if let Preview::AlreadyDone(m) = &st.preview {
            if !matches!(st.kind, StepKind::InstallLauncher | StepKind::MakeHome) {
                hooks.journal(&st.id, "skipped", Some(m));
                emit(hooks, StepStatus::AlreadyDone, m);
                status.insert(st.id.clone(), StepStatus::AlreadyDone);
                results.push(StepResult { id: st.id.clone(), title: st.title.clone(), status: StepStatus::AlreadyDone, message: m.clone() });
                continue;
            }
        }

        hooks.journal(&st.id, "begin", None);
        emit(hooks, StepStatus::Running, "");
        let mut res = execute(t, s, plan, st, input, hooks);
        if let Err(Fail::Transport(e)) = &res {
            if is_connection(e) {
                let first = e.to_string();
                res = match reconnect(t, s, &plan.identity, st, hooks, finished, total) {
                    // Check-then-act steps: running again only finishes what may have been cut off.
                    true => execute(t, s, plan, st, input, hooks),
                    false => Err(Fail::Msg(format!("Lost the connection to the TV ({first}) and could not reconnect.")))
                };
                if res.is_err() && matches!(res, Err(Fail::Transport(_))) { stopped = Some("the TV stopped responding".into()); }
            }
        }
        let (stat, msg) = match res {
            Ok(Outcome::Done(m)) => (StepStatus::Done, m),
            Ok(Outcome::AlreadyDone(m)) => (StepStatus::AlreadyDone, m),
            Err(Fail::Msg(m)) => (StepStatus::Failed, m),
            Err(Fail::Transport(e)) => (StepStatus::Failed, e.to_string()),
        };
        hooks.journal(&st.id, match stat { StepStatus::Failed => "failed", StepStatus::AlreadyDone => "skipped", _ => "verified" }, Some(&msg));
        emit(hooks, stat, &msg);
        if stat == StepStatus::Failed && (st.critical || msg.starts_with("Lost the connection")) {
            stopped.get_or_insert_with(|| format!("\u{201c}{}\u{201d} failed", st.title));
        }
        status.insert(st.id.clone(), stat);
        results.push(StepResult { id: st.id.clone(), title: st.title.clone(), status: stat, message: msg });
    }
    let completed = stopped.is_none() && results.iter().all(|r| matches!(r.status, StepStatus::Done | StepStatus::AlreadyDone));
    RunReport { results, completed, cancelled, stopped }
}

fn reconnect(t: &dyn Transport, s: &Serial, identity: &str, st: &Step, hooks: &mut dyn Hooks, finished: usize, total: usize) -> bool {
    let (tries, gap) = hooks.reconnect_policy();
    for i in 1..=tries {
        hooks.event(&Event { step: st.id.clone(), title: st.title.clone(), status: StepStatus::Reconnecting,
            message: format!("The TV stopped answering. Reconnecting (attempt {i} of {tries})…"), finished, total });
        hooks.pause(gap);
        if t.connect(s).is_ok() && tvs_device::probe::quick_identity(t, s).as_deref() == Some(identity) {
            hooks.journal(&st.id, "begin", Some("reconnected"));
            return true;
        }
    }
    false
}

fn press_home(t: &dyn Transport, s: &Serial, settle: Duration, hooks: &mut dyn Hooks) -> Option<String> {
    let _ = t.shell(s, &ShellCommand::new("input").arg("keyevent").arg("KEYCODE_HOME"));
    hooks.pause(settle);
    install::focused(t, s)
}

fn execute(t: &dyn Transport, s: &Serial, plan: &Plan, st: &Step, input: &DeployInput, hooks: &mut dyn Hooks) -> R {
    match &st.kind {
        StepKind::Identify => match tvs_device::probe::quick_identity(t, s) {
            Some(id) if id == plan.identity => Ok(Outcome::Done("Same TV as planned".into())),
            Some(id) => Err(Fail::Msg(format!("A different TV answered ({id}). Stopped before changing anything."))),
            None => Err(Fail::Transport(TransportError::NoDevice(s.to_string()))),
        },
        StepKind::Snapshot => {
            let snap = capture(t, s, &plan.identity, &hooks.now()).map_err(|e| Fail::Msg(format!("Could not read the TV's state: {e}")))?;
            hooks.save_snapshot(&plan.identity, &snap).map_err(Fail::Msg)?;
            Ok(Outcome::Done(format!("{} packages, Home and {} settings recorded", snap.packages.len(), snap.settings.len())))
        }
        StepKind::InstallLauncher => {
            if install::status(t, s)?.installed { return Ok(Outcome::AlreadyDone("Already installed".into())); }
            let apk = input.apk.as_ref().ok_or_else(|| Fail::Msg("No launcher APK in this build".into()))?;
            match install::install(t, s, apk, plan.sdk) {
                Ok(st) => Ok(Outcome::Done(format!("Installed, version {}", st.version.unwrap_or_default()))),
                Err(install::LauncherError::Transport(e)) => Err(Fail::Transport(e)),
                Err(e) => Err(Fail::Msg(e.to_string())),
            }
        }
        StepKind::MakeHome => {
            let h = install::make_home(t, s, input.settle)?;
            if h.verified { Ok(Outcome::Done("Pressed Home on the TV: the Studio launcher opened".into())) }
            else { Err(Fail::Msg(h.note.unwrap_or_else(|| "Home could not be confirmed".into()))) }
        }
        StepKind::DebloatBatch { packages } => {
            let enabled = enabled_packages(t, s).map_err(|e| Fail::Msg(e.to_string()))?;
            let todo: Vec<PackageName> = packages.iter().filter(|p| enabled.contains(*p)).filter_map(|p| PackageName::new(p.clone()).ok()).collect();
            if todo.is_empty() { return Ok(Outcome::AlreadyDone("Already disabled".into())); }
            let r = disable_in_batches(t, s, &todo, input.batch_size, &mut |_| {});
            if let Some(why) = r.stopped {
                if why.starts_with("Connection problem") { return Err(Fail::Transport(TransportError::Timeout(0))); }
                return Err(Fail::Msg(why));
            }
            if r.refused.is_empty() && r.unchanged.is_empty() { Ok(Outcome::Done(format!("Disabled and verified: {}", r.changed.join(", ")))) }
            else {
                let mut parts = vec![];
                if !r.changed.is_empty() { parts.push(format!("disabled {}", r.changed.join(", "))); }
                if !r.refused.is_empty() { parts.push(format!("refused by the TV: {}", r.refused.iter().map(|x| x.0.as_str()).collect::<Vec<_>>().join(", "))); }
                if !r.unchanged.is_empty() { parts.push(format!("reported success but unchanged: {}", r.unchanged.join(", "))); }
                Err(Fail::Msg(parts.join("; ")))
            }
        }
        StepKind::Settings { values } => {
            let mut changed = vec![];
            for (k, v) in values {
                if &get_setting(t, s, k) == v { continue; }
                let base = ShellCommand::new("settings");
                let cmd = match v { Some(val) => base.arg("put").arg("global").checked(k).and_then(|c| c.checked(val)), None => base.arg("delete").arg("global").checked(k) }
                    .map_err(|e| Fail::Msg(e.to_string()))?;
                t.shell(s, &cmd)?;
                if &get_setting(t, s, k) != v { return Err(Fail::Msg(format!("{k} did not take the new value"))); }
                changed.push(format!("{k} = {}", v.clone().unwrap_or_else(|| "default".into())));
            }
            if changed.is_empty() { Ok(Outcome::AlreadyDone("Already set".into())) } else { Ok(Outcome::Done(changed.join(", "))) }
        }
        StepKind::Theme => {
            let b = input.theme.as_ref().ok_or_else(|| Fail::Msg("No theme supplied".into()))?;
            let rev = format!("{}-{}", plan.identity.len(), hooks.now());
            match tvs_launcher::theme::deploy(t, s, b, &rev) {
                Ok(r) if r.config_verified => Ok(Outcome::Done(format!("{} files written and read back{}", r.written.len(), if r.restarted { "; Home restarted" } else { "" }))),
                Ok(_) => Err(Fail::Msg("Written, but reading it back did not match".into())),
                Err(tvs_launcher::theme::ThemeError::Transport(e)) => Err(Fail::Transport(e)),
                Err(e) => Err(Fail::Msg(e.to_string())),
            }
        }
        StepKind::DisableStockLauncher { package } => {
            let pkg = PackageName::new(package.clone()).map_err(|_| Fail::Msg("No stock launcher to disable".into()))?;
            let installed = list_packages(t, s).map_err(|e| Fail::Msg(e.to_string()))?;
            if !installed.iter().any(|p| p.name == package.as_str() && p.enabled) { return Ok(Outcome::AlreadyDone("Already disabled".into())); }
            // Re-check on the TV right now: our launcher must be the confirmed Home and the guards must allow it.
            if !press_home(t, s, input.settle, hooks).is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/"))) {
                return Err(Fail::Msg("The Studio launcher is not what opens on Home right now, so the stock launcher stays enabled.".into()));
            }
            let facts = gather_facts(t, s, &installed, std::slice::from_ref(&pkg));
            if let Some(Verdict::Blocked(why)) = evaluate_selection(std::slice::from_ref(&pkg), &facts).get(&pkg) {
                return Err(Fail::Msg(format!("Kept enabled: {why}")));
            }
            let r = disable_in_batches(t, s, std::slice::from_ref(&pkg), 1, &mut |_| {});
            if r.changed.is_empty() { return Err(Fail::Msg(format!("The TV did not disable {package}"))); }
            // Prove Home still works; if not, put the stock launcher straight back.
            if !press_home(t, s, input.settle, hooks).is_some_and(|f| f.starts_with(&format!("{STUDIO_PKG}/"))) {
                let _ = t.shell(s, &ShellCommand::new("pm").arg("enable").arg("--user").arg("0").package(&pkg));
                return Err(Fail::Msg(format!("Home stopped answering after disabling {package}, so it was re-enabled at once.")));
            }
            Ok(Outcome::Done(format!("{package} disabled; Home still opens the Studio launcher")))
        }
        StepKind::FinalVerify => {
            let focused = press_home(t, s, input.settle, hooks);
            let settings = t.shell(s, &ShellCommand::new("cmd").arg("package").arg("query-activities").arg("--brief").arg("-a").arg("android.settings.SETTINGS"))?;
            let settings_ok = settings.stdout.lines().any(|l| tvs_core::Component::new(l.trim()).is_ok());
            match (focused, settings_ok) {
                (Some(f), true) => Ok(Outcome::Done(format!("Home opens {f}; Settings can open"))),
                (None, _) => Err(Fail::Msg("Pressing Home did not show any app. Check the TV, or use Revert to original.".into())),
                (Some(_), false) => Err(Fail::Msg("No app answers Settings any more. Use Revert to original.".into())),
            }
        }
    }
}
