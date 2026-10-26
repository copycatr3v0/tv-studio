//! "Apply Everything": profile → previewed plan (dry run) → verified, journaled execution.

pub mod drift;
pub mod plan;
pub mod profile;
pub mod run;

pub use plan::{plan, DeployInput, Plan, PlanError, Preview, Step, StepKind};
pub use profile::{DebloatPlan, LauncherPlan, PrivateDns, Profile, SettingsPlan};
pub use run::{run, Event, Hooks, RunReport, StepResult, StepStatus};

#[cfg(test)]
mod tests;
