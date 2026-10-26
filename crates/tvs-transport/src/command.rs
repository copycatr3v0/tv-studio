//! A single shell command built only from fixed tokens and validated identifiers.

use tvs_core::{Component, PackageName};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("unsafe token in shell command: {0:?}")]
    UnsafeToken(String),
    #[error("{0}")]
    BadArgument(String),
}

/// Characters that would let a token chain, pipe, substitute or redirect in the TV's shell.
const FORBIDDEN: &[char] = &[';', '&', '|', '`', '$', '>', '<', '\n', '\r', '(', ')', '\'', '"', '\\', '*', '?', '{', '}', '#', '~', '!'];

/// Staged files this app writes on the TV live here and nowhere else.
pub const STAGING_PREFIX: &str = "/data/local/tmp/tvs-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    tokens: Vec<String>,
    /// Optional `< file` redirect, only ever a validated staging path.
    stdin: Option<String>,
}

impl ShellCommand {
    /// Program name, e.g. `pm`, `settings`, `dumpsys`. Must be a plain word.
    pub fn new(program: &'static str) -> Self {
        assert!(Self::safe(program) && !program.is_empty(), "unsafe program name {program:?}");
        Self { tokens: vec![program.to_string()], stdin: None }
    }

    /// A fixed token written in source code. Panics on shell syntax (a programming error).
    pub fn arg(mut self, token: &'static str) -> Self {
        assert!(Self::safe(token), "unsafe static token {token:?}");
        self.tokens.push(token.to_string());
        self
    }

    pub fn package(mut self, p: &PackageName) -> Self {
        self.tokens.push(p.as_str().to_string());
        self
    }

    pub fn component(mut self, c: &Component) -> Self {
        self.tokens.push(c.as_str().to_string());
        self
    }

    pub fn int(mut self, n: i64) -> Self {
        self.tokens.push(n.to_string());
        self
    }

    /// A dynamic token (settings key, path, value) checked against the forbidden set at runtime.
    pub fn checked(mut self, token: &str) -> Result<Self, CommandError> {
        if token.is_empty() || !Self::safe(token) || token.contains(' ') {
            return Err(CommandError::UnsafeToken(token.to_string()));
        }
        self.tokens.push(token.to_string());
        Ok(self)
    }

    /// Feed a staged file on the TV to the command's stdin (`content write` needs this). Only paths under
    /// `/data/local/tmp/tvs-` made of `[A-Za-z0-9_.-]` are accepted.
    pub fn stdin_from(mut self, staged: &str) -> Result<Self, CommandError> {
        if !is_staging_path(staged) { return Err(CommandError::UnsafeToken(staged.to_string())); }
        self.stdin = Some(staged.to_string());
        Ok(self)
    }

    pub fn stdin(&self) -> Option<&str> { self.stdin.as_deref() }

    fn safe(t: &str) -> bool { !t.chars().any(|c| FORBIDDEN.contains(&c)) }

    pub fn tokens(&self) -> &[String] { &self.tokens }

    /// The command line as sent to `adb shell`.
    pub fn to_shell_string(&self) -> String {
        match &self.stdin { Some(f) => format!("{} < {f}", self.tokens.join(" ")), None => self.tokens.join(" ") }
    }

    /// Arguments for `adb shell` (the redirect is interpreted by the TV's shell).
    pub fn adb_args(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.tokens.iter().map(String::as_str).collect();
        if let Some(f) = &self.stdin { v.push("<"); v.push(f); }
        v
    }

    pub fn program(&self) -> &str { &self.tokens[0] }
}

pub fn is_staging_path(p: &str) -> bool {
    p.strip_prefix(STAGING_PREFIX).is_some_and(|rest| {
        !rest.is_empty() && !rest.contains("..") && rest.chars().all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_disable() {
        let p = PackageName::new("com.tcl.logkit").unwrap();
        let c = ShellCommand::new("pm").arg("disable-user").arg("--user").arg("0").package(&p);
        assert_eq!(c.to_shell_string(), "pm disable-user --user 0 com.tcl.logkit");
    }

    #[test]
    #[should_panic]
    fn rejects_chaining_in_static() {
        let _ = ShellCommand::new("pm").arg("list; reboot");
    }

    #[test]
    fn stdin_only_from_staging() {
        let c = ShellCommand::new("content").arg("write").stdin_from("/data/local/tmp/tvs-theme-config.json").unwrap();
        assert_eq!(c.to_shell_string(), "content write < /data/local/tmp/tvs-theme-config.json");
        assert_eq!(c.adb_args(), vec!["content", "write", "<", "/data/local/tmp/tvs-theme-config.json"]);
        for bad in ["/sdcard/x", "/data/local/tmp/other", "/data/local/tmp/tvs-a/../b", "/data/local/tmp/tvs-a;b", "/data/local/tmp/tvs-", "/data/local/tmp/tvs-a b"] {
            assert!(ShellCommand::new("content").stdin_from(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn checked_rejects_injection() {
        for bad in ["a;b", "$(id)", "x|y", "a b", "", "a&&b", "`x`", "a>b"] {
            assert!(ShellCommand::new("settings").checked(bad).is_err(), "{bad:?}");
        }
        assert!(ShellCommand::new("settings").checked("window_animation_scale").is_ok());
        assert!(ShellCommand::new("settings").checked("0.5").is_ok());
        assert!(ShellCommand::new("ls").checked("/data/local/tmp/tvs-theme").is_ok());
    }
}
