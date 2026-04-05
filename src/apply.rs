//! Tier 1 apply — user-space actions, no sudo required.
//!
//! Design contract (from nexus6 scan `tier1-killprocess-safety`):
//!
//! 1. **UinverseLens** — every action records a `PreSnapshot` for audit.
//!    Some actions (like kill) aren't reversible; the snapshot is the best we have.
//! 2. **StabilityFilterLens** — action refuses to execute unless vitals stable.
//! 3. **UsurpriseLens** — action refuses if pre-state is anomalous.
//! 4. **CompletenessLens** — every Tier 1 action has a clear precondition.
//! 5. **RatioLens** — dosing tied to severity.
//!
//! **Safety**: Tier 1 MUST NOT exec anything outside its allowlist. System
//! processes (Finder, loginwindow, WindowServer, kernel_task, …) are hard-
//! banned from kill operations.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// A single pre-action snapshot — what we saw before executing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreSnapshot {
    pub ts: u64,
    pub kind: String,
    pub target: String,
    /// Opaque descriptor of what was observed (e.g. process name + pid list).
    pub observed: String,
}

/// Tier 1 user-space actions. No sudo, no system-level writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserAction {
    /// Kill all user-owned processes matching this pattern.
    /// Refuses system processes. Not reversible — snapshot only.
    KillProcess { pattern: String },
}

/// Why a Tier 1 apply was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum AbortReason {
    SystemProcessBanned(String),
    EmptyPattern,
    DangerousPattern(String),
}

/// Hard-coded block-list — these process names will NEVER be killed.
pub const BANNED_PROCESS_NAMES: &[&str] = &[
    "Finder", "loginwindow", "WindowServer", "kernel_task", "launchd",
    "SystemUIServer", "Dock", "ControlCenter", "NotificationCenter",
    "coreaudiod", "cfprefsd", "mDNSResponder", "syslogd", "sshd",
    "airgenome", "sudo", "su", "init", "systemd",
];

impl UserAction {
    /// Validate the action; return `Ok(())` if safe to dry-run.
    pub fn validate(&self) -> Result<(), AbortReason> {
        match self {
            UserAction::KillProcess { pattern } => {
                let pat = pattern.trim();
                if pat.is_empty() { return Err(AbortReason::EmptyPattern); }
                // Reject patterns that could match dangerous processes.
                for banned in BANNED_PROCESS_NAMES {
                    if pat.eq_ignore_ascii_case(banned) {
                        return Err(AbortReason::SystemProcessBanned((*banned).to_string()));
                    }
                    // Substring match guards broader patterns.
                    if pat.to_lowercase().contains(&banned.to_lowercase()) {
                        return Err(AbortReason::DangerousPattern(pat.to_string()));
                    }
                }
                // Reject wildcards that would match too much.
                if pat == "*" || pat == "." || pat == ".*" {
                    return Err(AbortReason::DangerousPattern(pat.to_string()));
                }
                Ok(())
            }
        }
    }

    /// Shell command this action would run if executed (dry-run surface).
    pub fn as_command(&self) -> String {
        match self {
            UserAction::KillProcess { pattern } => {
                format!("pkill -TERM -f '{}'", escape_shell(pattern))
            }
        }
    }
}

fn escape_shell(s: &str) -> String {
    s.replace('\'', r"'\''")
}

/// Build a PreSnapshot for later audit. Does NOT execute the action.
pub fn plan(action: &UserAction) -> Result<PreSnapshot, AbortReason> {
    action.validate()?;
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    Ok(PreSnapshot {
        ts,
        kind: match action { UserAction::KillProcess { .. } => "kill" }.to_string(),
        target: match action { UserAction::KillProcess { pattern } => pattern.clone() },
        observed: action.as_command(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kill(pat: &str) -> UserAction { UserAction::KillProcess { pattern: pat.to_string() } }

    #[test]
    fn rejects_empty_pattern() {
        assert_eq!(kill("").validate(), Err(AbortReason::EmptyPattern));
        assert_eq!(kill("   ").validate(), Err(AbortReason::EmptyPattern));
    }

    #[test]
    fn rejects_system_processes() {
        for banned in BANNED_PROCESS_NAMES {
            let res = kill(banned).validate();
            assert!(matches!(res, Err(AbortReason::SystemProcessBanned(_))),
                "expected {} to be banned, got {:?}", banned, res);
        }
    }

    #[test]
    fn rejects_case_insensitive_banned() {
        assert!(matches!(kill("FINDER").validate(),
            Err(AbortReason::SystemProcessBanned(_))));
        assert!(matches!(kill("kernel_TASK").validate(),
            Err(AbortReason::SystemProcessBanned(_))));
    }

    #[test]
    fn rejects_substring_patterns() {
        // "my-finder-helper" contains "finder" → banned
        assert!(matches!(kill("my-finder-helper").validate(),
            Err(AbortReason::DangerousPattern(_))));
        assert!(matches!(kill("xsudox").validate(),
            Err(AbortReason::DangerousPattern(_))));
    }

    #[test]
    fn rejects_wildcards() {
        assert!(matches!(kill("*").validate(), Err(AbortReason::DangerousPattern(_))));
        assert!(matches!(kill(".").validate(), Err(AbortReason::DangerousPattern(_))));
        assert!(matches!(kill(".*").validate(), Err(AbortReason::DangerousPattern(_))));
    }

    #[test]
    fn accepts_legitimate_patterns() {
        assert!(kill("Google Chrome Helper (Renderer)").validate().is_ok());
        assert!(kill("Slack").validate().is_ok());
        assert!(kill("node").validate().is_ok());
    }

    #[test]
    fn as_command_escapes_single_quotes() {
        let cmd = kill("it's me").as_command();
        assert!(cmd.contains(r"'it'\''s me'"));
    }

    #[test]
    fn plan_builds_snapshot_for_valid_action() {
        let snap = plan(&kill("Slack")).unwrap();
        assert_eq!(snap.kind, "kill");
        assert_eq!(snap.target, "Slack");
        assert!(snap.observed.contains("Slack"));
        assert!(snap.ts > 0);
    }

    #[test]
    fn plan_refuses_invalid_action() {
        assert!(plan(&kill("")).is_err());
        assert!(plan(&kill("Finder")).is_err());
    }
}
