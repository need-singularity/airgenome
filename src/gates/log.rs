//! JSONL append logger for gate genomes, writing to `~/.airgenome/gates.jsonl`.

use crate::gates::{GateGenome, GateId};
use std::path::PathBuf;

/// Resolve the log file path, creating parent dirs on demand.
pub fn log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let dir = PathBuf::from(home).join(".airgenome");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("gates.jsonl")
}

/// Format one genome as a single JSON line.
pub fn format_line(gid: GateId, g: &GateGenome) -> String {
    format!(
        "{{\"ts\":{},\"gate\":\"{}\",\"cpu\":{:.4},\"ram\":{:.4},\"gpu\":{:.4},\"npu\":{:.4},\"power\":{:.4},\"io\":{:.4},\"firing\":{},\"procs\":{},\"rss_mb\":{:.1},\"cpu_pct\":{:.2}}}",
        g.ts, gid.name(),
        g.axes[0], g.axes[1], g.axes[2], g.axes[3], g.axes[4], g.axes[5],
        g.firing_bits,
        g.counters[0] as u32, g.counters[1], g.counters[2]
    )
}

/// Append all 5 gate genomes as one batch to the log file.
pub fn append_batch(genomes: &[GateGenome; 5]) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(log_path())?;
    for (i, g) in genomes.iter().enumerate() {
        let gid = GateId::ALL[i];
        writeln!(f, "{}", format_line(gid, g))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_line_contains_all_fields() {
        let mut g = GateGenome::zeroed();
        g.ts = 1775397756;
        g.axes = [0.5, 0.25, 0.0, 0.0, 1.0, 0.125];
        g.firing_bits = 0b1010;
        g.counters = [3.0, 1024.5, 85.0];
        let s = format_line(GateId::Safari, &g);
        assert!(s.contains("\"gate\":\"safari\""));
        assert!(s.contains("\"ts\":1775397756"));
        assert!(s.contains("\"firing\":10"));
        assert!(s.contains("\"procs\":3"));
        assert!(s.contains("\"rss_mb\":1024.5"));
        assert!(s.contains("\"cpu\":0.5000"));
    }

    #[test]
    fn log_path_ends_with_gates_jsonl() {
        let p = log_path();
        assert!(p.to_string_lossy().ends_with(".airgenome/gates.jsonl"));
    }
}
