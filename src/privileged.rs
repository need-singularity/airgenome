//! Tier 2 privileged helper — protocol types + whitelist.
//!
//! This module defines the IPC protocol between a **user-space airgenome**
//! client and a **root-owned helper daemon**. The helper is a separate
//! LaunchDaemon that holds privileges; the user process holds the
//! decision logic. They talk over a Unix domain socket.
//!
//! **This module is protocol-only in v3.5.** No real helper ships yet —
//! we simply define the wire format and the sysctl whitelist so future
//! stages can add the LaunchDaemon + socket server.

use serde::{Deserialize, Serialize};

/// Socket path the helper listens on (inside root's $TMPDIR).
pub const SOCKET_PATH: &str = "/var/run/airgenome.sock";

/// Maximum allowed magnitude for any numeric sysctl change, per-key.
/// Prevents a hijacked client from setting absurd values.
pub const MAX_DELTA_RATIO: f64 = 2.0;

/// Whitelisted sysctl keys the helper will consider writing.
///
/// Anything outside this list is rejected unconditionally. Keys are
/// chosen so that even a worst-case mis-set is recoverable without
/// reboot.
pub const SYSCTL_WHITELIST: &[&str] = &[
    "vm.compressor_mode",          // memory compressor tier (0..4)
    "vm.page_free_target",         // low-water page count
    "kern.vm_swapsubdir",          // swap subdir gating
    "kern.timer.longterm.qlen",    // long-term timer queue length
];

/// Recommended Tier 2 sysctl tune for a given pair index.
///
/// Returns `(key, value)` for pairs whose critical firing has a
/// well-understood sysctl remedy. Pairs return `None` if no safe
/// automatic remedy is established.
pub fn plan_tier2_for_pair(pair: usize) -> Option<(&'static str, &'static str)> {
    match pair {
        // cpu×ram critical: aggressive memory compressor.
        0 => Some(("vm.compressor_mode", "4")),
        // ram×io thrashing: raise free-page water mark.
        8 => Some(("vm.page_free_target", "4000")),
        // ram×power: aggressive compression on battery too.
        7 => Some(("vm.compressor_mode", "4")),
        _ => None,
    }
}

/// One request from user-space airgenome → privileged helper.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    /// Liveness check. No side effects.
    Ping,
    /// Read a whitelisted sysctl key.
    SysctlGet { key: String },
    /// Propose a sysctl write. Helper may still refuse.
    SysctlSet { key: String, value: String },
    /// Request a `sudo purge`-equivalent memory reclaim.
    Purge,
}

/// Response from helper → user-space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok { detail: String },
    Refused { reason: String },
    Error { message: String },
}

/// Reasons the helper refuses a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    KeyNotWhitelisted,
    ValueOutOfRange,
    PeerNotAuthenticated,
    HelperNotInstalled,
    UnsupportedOp,
}

impl RefuseReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            RefuseReason::KeyNotWhitelisted => "sysctl key not in whitelist",
            RefuseReason::ValueOutOfRange => "value exceeds MAX_DELTA_RATIO",
            RefuseReason::PeerNotAuthenticated => "peer identity not verified",
            RefuseReason::HelperNotInstalled => "helper daemon not installed",
            RefuseReason::UnsupportedOp => "operation not supported by this helper version",
        }
    }
}

/// Check whether a sysctl key is on the whitelist.
pub fn is_whitelisted(key: &str) -> bool {
    SYSCTL_WHITELIST.contains(&key)
}

/// Stub client: always returns `Refused { HelperNotInstalled }` in v3.5.
/// Future stages will dial SOCKET_PATH and actually exchange messages.
pub fn send(_req: &Request) -> Response {
    Response::Refused {
        reason: RefuseReason::HelperNotInstalled.as_str().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_non_empty() {
        assert!(!SYSCTL_WHITELIST.is_empty());
        assert!(is_whitelisted("vm.compressor_mode"));
    }

    #[test]
    fn arbitrary_keys_not_whitelisted() {
        assert!(!is_whitelisted("kern.hostname"));
        assert!(!is_whitelisted("random.key"));
        assert!(!is_whitelisted(""));
    }

    #[test]
    fn stub_send_always_refuses() {
        let r = send(&Request::Ping);
        match r {
            Response::Refused { reason } => {
                assert!(reason.contains("not installed"));
            }
            _ => panic!("expected Refused, got {:?}", r),
        }
    }

    #[test]
    fn refuse_reasons_have_messages() {
        for r in &[
            RefuseReason::KeyNotWhitelisted,
            RefuseReason::ValueOutOfRange,
            RefuseReason::PeerNotAuthenticated,
            RefuseReason::HelperNotInstalled,
            RefuseReason::UnsupportedOp,
        ] {
            assert!(!r.as_str().is_empty());
        }
    }

    #[test]
    fn max_delta_ratio_is_bounded() {
        assert!(MAX_DELTA_RATIO > 1.0);
        assert!(MAX_DELTA_RATIO <= 10.0);
    }

    #[test]
    fn tier2_plan_keys_are_whitelisted() {
        for k in 0..crate::gate::PAIR_COUNT {
            if let Some((key, _val)) = plan_tier2_for_pair(k) {
                assert!(is_whitelisted(key),
                    "pair {} tier2 plan key '{}' not whitelisted", k, key);
            }
        }
    }

    #[test]
    fn tier2_plan_values_are_integer() {
        for k in 0..crate::gate::PAIR_COUNT {
            if let Some((_k, v)) = plan_tier2_for_pair(k) {
                assert!(v.parse::<i64>().is_ok(), "value '{}' not integer", v);
            }
        }
    }

    #[test]
    fn tier2_plan_returns_none_for_unmapped() {
        assert!(plan_tier2_for_pair(9).is_none());
        assert!(plan_tier2_for_pair(99).is_none());
    }
}
