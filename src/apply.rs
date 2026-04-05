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
    /// No-op stub: advise the user to close browser tabs manually.
    /// Tier 1 cannot actually do this (requires scripting each browser).
    AdviseCloseTabs,
    /// No-op stub: advise the user to lower parallelism in a build tool.
    AdviseReduceParallelism { tool: String, from: u32, to: u32 },
}

/// For each firing pair, what Tier 1 UserAction (if any) would we suggest?
///
/// Returns `None` for pairs whose remedies are all sudo-only (no Tier 1
/// path yet). Returned actions are fully validated — call `.validate()`
/// is not required.
pub fn plan_for_pair(pair: usize) -> Option<UserAction> {
    match pair {
        // cpu×ram — suggest killing Chrome helpers (biggest RAM hogs)
        0 => Some(UserAction::KillProcess { pattern: "Google Chrome Helper (Renderer)".into() }),
        // cpu×io — suggest reducing build parallelism
        4 => Some(UserAction::AdviseReduceParallelism { tool: "cargo".into(), from: 8, to: 4 }),
        // ram×gpu — close tabs
        5 => Some(UserAction::AdviseCloseTabs),
        // ram×power — kill Slack/Electron helpers on battery
        7 => Some(UserAction::KillProcess { pattern: "Slack Helper".into() }),
        // ram×io — same tab guidance
        8 => Some(UserAction::AdviseCloseTabs),
        // power×io — same tab guidance
        14 => Some(UserAction::AdviseCloseTabs),
        _ => None,
    }
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
    /// Human-readable label.
    pub fn label(&self) -> String {
        match self {
            UserAction::KillProcess { pattern } => format!("kill '{}'", pattern),
            UserAction::AdviseCloseTabs => "close browser tabs (manual)".into(),
            UserAction::AdviseReduceParallelism { tool, from, to } =>
                format!("reduce {} parallelism {} → {}", tool, from, to),
        }
    }

    /// Validate the action; return `Ok(())` if safe to dry-run.
    pub fn validate(&self) -> Result<(), AbortReason> {
        match self {
            UserAction::AdviseCloseTabs => Ok(()),
            UserAction::AdviseReduceParallelism { .. } => Ok(()),
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
            UserAction::AdviseCloseTabs => {
                "# close background browser tabs (manual)".into()
            }
            UserAction::AdviseReduceParallelism { tool, from, to } => {
                format!("# invoke {} with -j{} instead of -j{}", tool, to, from)
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
    let kind = match action {
        UserAction::KillProcess { .. } => "kill",
        UserAction::AdviseCloseTabs => "advise-close-tabs",
        UserAction::AdviseReduceParallelism { .. } => "advise-parallelism",
    }.to_string();
    let target = match action {
        UserAction::KillProcess { pattern } => pattern.clone(),
        UserAction::AdviseCloseTabs => "browser".into(),
        UserAction::AdviseReduceParallelism { tool, .. } => tool.clone(),
    };
    Ok(PreSnapshot { ts, kind, target, observed: action.as_command() })
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

    #[test]
    fn advise_actions_always_valid() {
        assert!(UserAction::AdviseCloseTabs.validate().is_ok());
        assert!(UserAction::AdviseReduceParallelism {
            tool: "cargo".into(), from: 8, to: 4
        }.validate().is_ok());
    }

    #[test]
    fn plan_for_pair_covers_known_pairs() {
        // Known Tier 1 mappings.
        assert!(plan_for_pair(0).is_some());   // cpu×ram
        assert!(plan_for_pair(4).is_some());   // cpu×io
        assert!(plan_for_pair(5).is_some());   // ram×gpu
        assert!(plan_for_pair(7).is_some());   // ram×power
        assert!(plan_for_pair(8).is_some());   // ram×io
        assert!(plan_for_pair(14).is_some());  // power×io
        // Non-mapped pairs return None.
        assert!(plan_for_pair(9).is_none());   // gpu×npu (structural)
        assert!(plan_for_pair(99).is_none());  // out of range
    }

    #[test]
    fn plan_for_pair_actions_all_validate() {
        for k in 0..crate::gate::PAIR_COUNT {
            if let Some(action) = plan_for_pair(k) {
                assert!(action.validate().is_ok(),
                    "plan_for_pair({}) returned invalid action: {:?}", k, action);
            }
        }
    }

    #[test]
    fn labels_are_non_empty() {
        let actions = vec![
            kill("Slack"),
            UserAction::AdviseCloseTabs,
            UserAction::AdviseReduceParallelism { tool: "cargo".into(), from: 8, to: 4 },
        ];
        for a in actions { assert!(!a.label().is_empty()); }
    }
}
