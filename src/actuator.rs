//! Actuator — proposed system parameter changes with rollback snapshots.
//!
//! **Safety:** every `Action` is dry-run by default. `apply()` only records
//! the intended change and returns a `Snapshot` for rollback. Actual system
//! mutation requires explicit opt-in via `apply_live()` (not implemented in
//! the skeleton — left as the implant's privileged surface).

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::gate::{Axis, PAIRS, PAIR_COUNT};

/// A single proposed actuation — an axis pair targeting some knob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// Pair index in `PAIRS` (0..15).
    pub pair: usize,
    /// Human-readable knob identifier (e.g. "vm_compressor").
    pub knob: String,
    /// Proposed new value (as string — knob semantics decide parsing).
    pub new_value: String,
    /// Previous value before the proposal (for rollback).
    pub prev_value: String,
}

impl Action {
    pub fn new(pair: usize, knob: &str, prev: &str, new: &str) -> Self {
        Self {
            pair,
            knob: knob.to_string(),
            new_value: new.to_string(),
            prev_value: prev.to_string(),
        }
    }

    /// Return the pair of axes this action targets, if `pair` is in range.
    pub fn axes(&self) -> Option<(Axis, Axis)> {
        PAIRS.get(self.pair).copied()
    }
}

/// Outcome of a dry-run `apply()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub ts: u64,
    pub actions: Vec<Action>,
    /// Dry run marker: `false` means nothing was actually written.
    pub live: bool,
}

impl Snapshot {
    pub fn dry() -> Self {
        Self { ts: now(), actions: vec![], live: false }
    }

    pub fn push(&mut self, action: Action) {
        self.actions.push(action);
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Dry-run actuator: records actions and never touches the system.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Actuator {
    pub history: Vec<Snapshot>,
}

impl Actuator {
    pub fn new() -> Self { Self::default() }

    /// Record a batch of proposed actions without touching the system.
    pub fn apply(&mut self, actions: Vec<Action>) -> Snapshot {
        let snap = Snapshot { ts: now(), actions, live: false };
        self.history.push(snap.clone());
        snap
    }

    /// Return the inverse of a snapshot: swap `prev` and `new` on each
    /// action so applying it again restores the original state.
    pub fn invert(snap: &Snapshot) -> Snapshot {
        let actions = snap.actions.iter().map(|a| Action {
            pair: a.pair,
            knob: a.knob.clone(),
            prev_value: a.new_value.clone(),
            new_value: a.prev_value.clone(),
        }).collect();
        Snapshot { ts: now(), actions, live: snap.live }
    }

    /// Number of actions ever recorded (all snapshots).
    pub fn total_actions(&self) -> usize {
        self.history.iter().map(|s| s.actions.len()).sum()
    }
}

/// Validate that a list of actions targets only legal pair indices.
pub fn validate(actions: &[Action]) -> Result<(), String> {
    for a in actions {
        if a.pair >= PAIR_COUNT {
            return Err(format!(
                "action targets invalid pair {} (must be 0..{})",
                a.pair, PAIR_COUNT
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_action(p: usize) -> Action {
        Action::new(p, "vm_compressor", "4", "2")
    }

    #[test]
    fn apply_is_dry_run_by_default() {
        let mut act = Actuator::new();
        let snap = act.apply(vec![sample_action(0)]);
        assert!(!snap.live);
        assert_eq!(snap.actions.len(), 1);
        assert_eq!(act.total_actions(), 1);
    }

    #[test]
    fn invert_swaps_prev_and_new() {
        let snap = Snapshot {
            ts: 100,
            actions: vec![sample_action(3)],
            live: false,
        };
        let inv = Actuator::invert(&snap);
        assert_eq!(inv.actions[0].prev_value, "2");
        assert_eq!(inv.actions[0].new_value, "4");
    }

    #[test]
    fn double_invert_is_identity_on_values() {
        let a = sample_action(7);
        let snap = Snapshot { ts: 0, actions: vec![a.clone()], live: false };
        let reinv = Actuator::invert(&Actuator::invert(&snap));
        assert_eq!(reinv.actions[0].prev_value, a.prev_value);
        assert_eq!(reinv.actions[0].new_value, a.new_value);
    }

    #[test]
    fn validate_rejects_out_of_range_pair() {
        let bad = Action::new(99, "x", "1", "2");
        assert!(validate(&[bad]).is_err());
    }

    #[test]
    fn validate_accepts_all_legal_pairs() {
        let good: Vec<Action> = (0..PAIR_COUNT)
            .map(|p| Action::new(p, "k", "a", "b"))
            .collect();
        assert!(validate(&good).is_ok());
    }

    #[test]
    fn action_axes_matches_canonical_pairs() {
        let a = Action::new(0, "k", "a", "b");
        let (x, y) = a.axes().unwrap();
        assert_eq!((x, y), PAIRS[0]);
    }

    #[test]
    fn dry_snapshot_has_no_actions() {
        let s = Snapshot::dry();
        assert!(s.actions.is_empty());
        assert!(!s.live);
    }
}
