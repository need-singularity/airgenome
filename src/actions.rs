//! Concrete executable remedies for each pair gate.
//!
//! Design rule: **airgenome never runs these commands itself**. It emits
//! the exact shell invocations so the user can audit and execute them
//! (often behind `sudo`). This is v2.0's contract: actionable, not
//! autonomous.

use crate::gate::PAIR_COUNT;

/// A single concrete command suggestion.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    /// Shell invocation (may require sudo).
    pub cmd: &'static str,
    /// Human-readable effect description.
    pub effect: &'static str,
    /// Whether the command typically needs root.
    pub needs_sudo: bool,
}

/// 1..3 concrete commands per pair gate.
pub type Actions = &'static [Command];

/// Per-pair executable remedies. Index = pair index (matches `PAIRS`).
pub const ACTIONS: [Actions; PAIR_COUNT] = [
    // 0: cpu × ram
    &[
        Command { cmd: "sudo purge", effect: "reclaim inactive memory (UBC flush)", needs_sudo: true },
        Command { cmd: "killall 'Google Chrome Helper'", effect: "drop renderer RSS", needs_sudo: false },
    ],
    // 1: cpu × gpu
    &[
        Command { cmd: "# consider Metal-backed alternative for current CPU hot path", effect: "offload to GPU", needs_sudo: false },
    ],
    // 2: cpu × npu
    &[
        Command { cmd: "# route ML via Core ML / ANE (MLX / CoreMLTools)", effect: "offload to ANE", needs_sudo: false },
    ],
    // 3: cpu × power
    &[
        Command { cmd: "sudo pmset -b lowpowermode 1", effect: "low-power mode on battery", needs_sudo: true },
        Command { cmd: "caffeinate -di sleep 0", effect: "inspect wake reasons first", needs_sudo: false },
    ],
    // 4: cpu × io
    &[
        Command { cmd: "sudo mdutil -i off /", effect: "pause Spotlight indexing (root volume)", needs_sudo: true },
        Command { cmd: "# reduce parallel jobs (e.g. cargo -j4 → -j2)", effect: "lower IO contention", needs_sudo: false },
    ],
    // 5: ram × gpu
    &[
        Command { cmd: "# lower texture / image cache in graphical apps", effect: "release GPU-side memory", needs_sudo: false },
    ],
    // 6: ram × npu
    &[
        Command { cmd: "# quantize model to 4-bit (llama.cpp Q4_K_M / MLX q4)", effect: "halve model RAM", needs_sudo: false },
    ],
    // 7: ram × power
    &[
        Command { cmd: "sudo purge", effect: "reclaim inactive memory", needs_sudo: true },
        Command { cmd: "# close background browser tabs + Electron apps", effect: "preserve battery", needs_sudo: false },
    ],
    // 8: ram × io
    &[
        Command { cmd: "sudo purge", effect: "flush UBC before thrashing", needs_sudo: true },
        Command { cmd: "# disable swap: sudo launchctl bootout system/com.apple.dynamic_pager (expert)", effect: "stop paging entirely", needs_sudo: true },
    ],
    // 9: gpu × npu
    &[
        Command { cmd: "# partition: graphics → Metal, ML → ANE", effect: "avoid GPU/NPU contention", needs_sudo: false },
    ],
    // 10: gpu × power
    &[
        Command { cmd: "# cap frame rate on game/video apps", effect: "lower GPU clock on battery", needs_sudo: false },
    ],
    // 11: gpu × io
    &[
        Command { cmd: "# enable mipmaps + stream textures from disk", effect: "hide IO stalls from GPU", needs_sudo: false },
    ],
    // 12: npu × power
    &[
        Command { cmd: "# batch inference (increase batch_size, reduce tok/s target)", effect: "reduce ANE duty cycle", needs_sudo: false },
    ],
    // 13: npu × io
    &[
        Command { cmd: "# mmap model weights; preload to RAM", effect: "eliminate model read spikes", needs_sudo: false },
    ],
    // 14: power × io
    &[
        Command { cmd: "sudo mdutil -i off /", effect: "pause Spotlight on battery", needs_sudo: true },
        Command { cmd: "sudo tmutil disable", effect: "pause Time Machine snapshots", needs_sudo: true },
    ],
];

pub fn commands_for(pair: usize) -> Option<Actions> {
    if pair >= PAIR_COUNT { return None; }
    Some(ACTIONS[pair])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pair_has_at_least_one_action() {
        for k in 0..PAIR_COUNT {
            let actions = commands_for(k).unwrap();
            assert!(!actions.is_empty(), "pair {} has no actions", k);
        }
    }

    #[test]
    fn all_commands_are_non_empty() {
        for actions in ACTIONS {
            for cmd in actions.iter() {
                assert!(!cmd.cmd.is_empty());
                assert!(!cmd.effect.is_empty());
            }
        }
    }

    #[test]
    fn sudo_flag_consistency() {
        // Commands starting with `sudo ` MUST set needs_sudo.
        // Commands starting with `#` are comments (no sudo).
        for actions in ACTIONS {
            for cmd in actions.iter() {
                if cmd.cmd.starts_with("sudo ") {
                    assert!(cmd.needs_sudo, "cmd '{}' starts with sudo but needs_sudo=false", cmd.cmd);
                }
                if cmd.cmd.starts_with('#') {
                    // comments are never sudo commands themselves
                    assert!(!cmd.cmd.starts_with("# sudo"), "comment line shouldn't advertise sudo: {}", cmd.cmd);
                }
            }
        }
    }

    #[test]
    fn out_of_range_returns_none() {
        assert!(commands_for(PAIR_COUNT).is_none());
        assert!(commands_for(999).is_none());
    }
}
