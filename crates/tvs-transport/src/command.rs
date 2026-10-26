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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    tokens: Vec<String>,
}

impl ShellCommand {
    /// Program name, e.g. `pm`, `settings`, `dumpsys`. Must be a plain word.
    pub fn new(program: &'static str) -> Self {
        assert!(Self::safe(program) && !program.is_empty(), "unsafe program name {program:?}");
        Self { tokens: vec![program.to_string()] }
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

    fn safe(t: &str) -> bool { !t.chars().any(|c| FORBIDDEN.contains(&c)) }

    pub fn tokens(&self) -> &[String] { &self.tokens }

    /// The command line as sent to `adb shell`.
    pub fn to_shell_string(&self) -> String { self.tokens.join(" ") }

    pub fn program(&self) -> &str { &self.tokens[0] }
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
    fn checked_rejects_injection() {
        for bad in ["a;b", "$(id)", "x|y", "a b", "", "a&&b", "`x`", "a>b"] {
            assert!(ShellCommand::new("settings").checked(bad).is_err(), "{bad:?}");
        }
        assert!(ShellCommand::new("settings").checked("window_animation_scale").is_ok());
        assert!(ShellCommand::new("settings").checked("0.5").is_ok());
        assert!(ShellCommand::new("ls").checked("/data/local/tmp/tvs-theme").is_ok());
    }
}
