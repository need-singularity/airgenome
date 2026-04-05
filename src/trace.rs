//! Trace — analyse a `vitals.jsonl` log written by `airgenome daemon`.
//!
//! Pure std — no JSON dependency. We parse only the fields we own (flat
//! numeric object with known keys). Lines that don't match are skipped.

use crate::gate::PAIR_COUNT;

/// One parsed vitals record from the daemon log.
#[derive(Debug, Clone, Copy)]
pub struct TraceRecord {
    pub ts: u64,
    pub cpu: f64,
    pub ram: f64,
    pub gpu: f64,
    pub npu: f64,
    pub power: f64,
    pub io: f64,
    pub firing: usize,
}

/// Aggregated statistics over a trace.
#[derive(Debug, Clone)]
pub struct TraceStats {
    pub count: usize,
    pub span_secs: u64,
    pub firing_mean: f64,
    pub firing_max: usize,
    pub cpu_mean: f64,
    pub ram_mean: f64,
    pub io_mean: f64,
    pub on_battery_frac: f64,
    /// Work fraction estimate = 1 − firing_mean / PAIR_COUNT.
    pub work_fraction: f64,
}

/// Parse a single JSONL line into a [`TraceRecord`].
///
/// Expects flat numeric keys: `ts`, `cpu`, `ram`, `gpu`, `npu`, `power`,
/// `io`, `firing`. Extra whitespace and key order tolerated. Returns
/// `None` on any parsing mishap.
pub fn parse_line(line: &str) -> Option<TraceRecord> {
    let mut ts: Option<u64> = None;
    let mut cpu: Option<f64> = None;
    let mut ram: Option<f64> = None;
    let mut gpu: Option<f64> = None;
    let mut npu: Option<f64> = None;
    let mut power: Option<f64> = None;
    let mut io: Option<f64> = None;
    let mut firing: Option<usize> = None;

    // strip outer braces
    let s = line.trim();
    let s = s.strip_prefix('{')?;
    let s = s.strip_suffix('}')?;

    for field in s.split(',') {
        let mut it = field.splitn(2, ':');
        let raw_key = it.next()?.trim();
        let raw_val = it.next()?.trim();
        let key = raw_key.trim_matches('"');
        match key {
            "ts" => ts = raw_val.parse().ok(),
            "cpu" => cpu = raw_val.parse().ok(),
            "ram" => ram = raw_val.parse().ok(),
            "gpu" => gpu = raw_val.parse().ok(),
            "npu" => npu = raw_val.parse().ok(),
            "power" => power = raw_val.parse().ok(),
            "io" => io = raw_val.parse().ok(),
            "firing" => firing = raw_val.parse().ok(),
            _ => {}
        }
    }

    Some(TraceRecord {
        ts: ts?,
        cpu: cpu?,
        ram: ram?,
        gpu: gpu?,
        npu: npu?,
        power: power?,
        io: io?,
        firing: firing?,
    })
}

/// Parse a whole `vitals.jsonl` body into records. Invalid lines are skipped.
pub fn parse_log(body: &str) -> Vec<TraceRecord> {
    body.lines().filter_map(parse_line).collect()
}

/// Aggregate a trace into summary statistics.
pub fn summarize(records: &[TraceRecord]) -> TraceStats {
    let count = records.len();
    if count == 0 {
        return TraceStats {
            count: 0, span_secs: 0,
            firing_mean: 0.0, firing_max: 0,
            cpu_mean: 0.0, ram_mean: 0.0, io_mean: 0.0,
            on_battery_frac: 0.0, work_fraction: 0.0,
        };
    }
    let n = count as f64;
    let first = records.first().unwrap();
    let last = records.last().unwrap();
    let span_secs = last.ts.saturating_sub(first.ts);

    let firing_sum: usize = records.iter().map(|r| r.firing).sum();
    let firing_max = records.iter().map(|r| r.firing).max().unwrap_or(0);
    let cpu_sum: f64 = records.iter().map(|r| r.cpu).sum();
    let ram_sum: f64 = records.iter().map(|r| r.ram).sum();
    let io_sum: f64 = records.iter().map(|r| r.io).sum();
    let battery_sum: usize = records.iter().filter(|r| r.power < 0.5).count();

    let firing_mean = firing_sum as f64 / n;
    let work_fraction = 1.0 - firing_mean / PAIR_COUNT as f64;

    TraceStats {
        count,
        span_secs,
        firing_mean,
        firing_max,
        cpu_mean: cpu_sum / n,
        ram_mean: ram_sum / n,
        io_mean: io_sum / n,
        on_battery_frac: battery_sum as f64 / n,
        work_fraction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"ts":1775351857,"cpu":3.1,"ram":0.9,"gpu":8,"npu":8,"power":1,"io":1.435602,"firing":7}"#;
    const BAD: &str = "not json";

    #[test]
    fn parse_one_line() {
        let r = parse_line(SAMPLE).expect("should parse");
        assert_eq!(r.ts, 1775351857);
        assert_eq!(r.firing, 7);
        assert!((r.cpu - 3.1).abs() < 1e-9);
        assert!((r.io - 1.435602).abs() < 1e-9);
    }

    #[test]
    fn parse_line_handles_garbage() {
        assert!(parse_line(BAD).is_none());
        assert!(parse_line("").is_none());
        assert!(parse_line("{}").is_none()); // missing required fields
    }

    #[test]
    fn parse_log_skips_bad_lines() {
        let body = format!("{}\n{}\n{}\n", SAMPLE, BAD, SAMPLE);
        let recs = parse_log(&body);
        assert_eq!(recs.len(), 2);
    }

    #[test]
    fn summarize_empty_is_zero() {
        let s = summarize(&[]);
        assert_eq!(s.count, 0);
        assert_eq!(s.firing_mean, 0.0);
        assert_eq!(s.work_fraction, 0.0);
    }

    #[test]
    fn summarize_computes_span_and_means() {
        let recs = vec![
            TraceRecord { ts: 100, cpu: 2.0, ram: 0.5, gpu: 8.0, npu: 8.0, power: 1.0, io: 1.0, firing: 5 },
            TraceRecord { ts: 160, cpu: 4.0, ram: 0.9, gpu: 8.0, npu: 8.0, power: 0.0, io: 2.0, firing: 10 },
        ];
        let s = summarize(&recs);
        assert_eq!(s.count, 2);
        assert_eq!(s.span_secs, 60);
        assert_eq!(s.firing_mean, 7.5);
        assert_eq!(s.firing_max, 10);
        assert!((s.cpu_mean - 3.0).abs() < 1e-9);
        assert!((s.ram_mean - 0.7).abs() < 1e-9);
        assert!((s.on_battery_frac - 0.5).abs() < 1e-9);
        // work_fraction = 1 - 7.5/15 = 0.5
        assert!((s.work_fraction - 0.5).abs() < 1e-9);
    }

    #[test]
    fn work_fraction_hits_two_thirds_when_five_firing() {
        let recs = vec![
            TraceRecord { ts: 0, cpu: 0.0, ram: 0.0, gpu: 0.0, npu: 0.0, power: 1.0, io: 0.0, firing: 5 }
        ];
        let s = summarize(&recs);
        // 1 - 5/15 = 10/15 = 2/3
        assert!((s.work_fraction - 2.0/3.0).abs() < 1e-9);
    }
}
