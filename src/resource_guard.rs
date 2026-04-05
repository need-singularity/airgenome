//! airgenome resource_guard — OS-level hard limits + adaptive soft throttle.
//!
//! Hard limits (A): setrlimit + nice — OS가 강제하는 천장
//! Soft limits (B): 자가 모니터링 → 적응적 배치/깊이 축소
//!
//! 사용법:
//!   resource_guard::init_hard_limits(HardLimits::default());
//!   let throttle = AdaptiveThrottle::new(SoftLimits::default());
//!   // 매 사이클:
//!   throttle.check_and_adapt();

use std::sync::atomic::{AtomicBool, Ordering};

static HARD_LIMITS_APPLIED: AtomicBool = AtomicBool::new(false);

// ═══════════════════════════════════════════════════════════
// ─── A. OS HARD LIMITS ───
// ═══════════════════��═══════════════════════════════════════

/// OS-level hard limit 설정값.
#[derive(Debug, Clone)]
pub struct HardLimits {
    /// 프로세스 RSS 상한 (MB). 0 = 제한 없음.
    pub max_rss_mb: u64,
    /// 프로세스 DATA 세그먼트 상한 (MB). 0 = 제한 없음.
    pub max_data_mb: u64,
    /// nice 값 (0=보통, 10=낮은 우선순위, 19=최저). -1 = 변경 안함.
    pub nice_level: i32,
    /// CPU time soft limit (초). 0 = 제한 없음.
    pub max_cpu_seconds: u64,
}

impl Default for HardLimits {
    fn default() -> Self {
        Self {
            max_rss_mb: 512,
            max_data_mb: 1024,
            nice_level: 10,
            max_cpu_seconds: 0,
        }
    }
}

/// OS-level 하드 제한 적용. 프로세스 시작 시 1회 호출.
/// 이미 적용되었으면 무시.
pub fn init_hard_limits(limits: HardLimits) -> Result<(), String> {
    if HARD_LIMITS_APPLIED.swap(true, Ordering::SeqCst) {
        return Ok(()); // 이미 적용됨
    }

    #[cfg(unix)]
    {
        use std::io;

        // ── setrlimit: RSS ──
        if limits.max_rss_mb > 0 {
            let bytes = limits.max_rss_mb * 1024 * 1024;
            set_rlimit(libc_rlimit::RLIMIT_RSS, bytes)
                .map_err(|e| format!("rlimit RSS failed: {}", e))?;
        }

        // ── setrlimit: DATA ��─
        if limits.max_data_mb > 0 {
            let bytes = limits.max_data_mb * 1024 * 1024;
            set_rlimit(libc_rlimit::RLIMIT_DATA, bytes)
                .map_err(|e| format!("rlimit DATA failed: {}", e))?;
        }

        // ── setrlimit: CPU time ─��
        if limits.max_cpu_seconds > 0 {
            set_rlimit(libc_rlimit::RLIMIT_CPU, limits.max_cpu_seconds)
                .map_err(|e| format!("rlimit CPU failed: {}", e))?;
        }

        // ── nice (setpriority) ──
        if limits.nice_level >= 0 {
            set_nice(limits.nice_level)
                .map_err(|e| format!("nice({}) failed: {}", limits.nice_level, e))?;
        }

        let _ = io::stderr(); // suppress unused warning
    }

    #[cfg(not(unix))]
    {
        let _ = limits;
    }

    Ok(())
}

/// 현재 하드 제한이 적용되었는지 확인.
pub fn hard_limits_active() -> bool {
    HARD_LIMITS_APPLIED.load(Ordering::Relaxed)
}

// ── Unix syscall wrappers ──

#[cfg(unix)]
mod libc_rlimit {
    // macOS rlimit 상수 (libc 크레이트 없이 직접 정의)
    #[cfg(target_os = "macos")]
    pub const RLIMIT_RSS: i32 = 5;
    #[cfg(target_os = "macos")]
    pub const RLIMIT_DATA: i32 = 2;
    #[cfg(target_os = "macos")]
    pub const RLIMIT_CPU: i32 = 0;

    #[cfg(target_os = "linux")]
    pub const RLIMIT_RSS: i32 = 5;
    #[cfg(target_os = "linux")]
    pub const RLIMIT_DATA: i32 = 2;
    #[cfg(target_os = "linux")]
    pub const RLIMIT_CPU: i32 = 0;
}

#[cfg(unix)]
#[repr(C)]
struct Rlimit {
    rlim_cur: u64, // soft limit
    rlim_max: u64, // hard limit
}

#[cfg(unix)]
extern "C" {
    fn setrlimit(resource: i32, rlim: *const Rlimit) -> i32;
    fn setpriority(which: i32, who: u32, prio: i32) -> i32;
}

#[cfg(unix)]
fn set_rlimit(resource: i32, value: u64) -> Result<(), std::io::Error> {
    let rlim = Rlimit {
        rlim_cur: value,
        rlim_max: value,
    };
    let ret = unsafe { setrlimit(resource, &rlim) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn set_nice(level: i32) -> Result<(), std::io::Error> {
    // PRIO_PROCESS = 0, who = 0 (self)
    let ret = unsafe { setpriority(0, 0, level) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

// ═══════════════════════════════════════════════════════════
// ─── B. ADAPTIVE SOFT THROTTLE ───
// ═══��═══════════════════════════════════════════════════════

/// 소프트 제한 설정값.
#[derive(Debug, Clone)]
pub struct SoftLimits {
    /// RSS 소프트 경고 임계값 (MB). 이 이상이면 적응 시작.
    pub warn_rss_mb: u64,
    /// RSS 하드 임계값 (MB). 이 이상이면 최소 모드.
    pub critical_rss_mb: u64,
    /// CPU 사용률 소프트 경고 (%). macOS top 기준.
    pub warn_cpu_pct: f64,
    /// 적응 시 sleep 삽입 (ms).
    pub throttle_sleep_ms: u64,
    /// 적응 시 배치 크기 축소 비율 (0.0–1.0).
    pub batch_scale_factor: f64,
}

impl Default for SoftLimits {
    fn default() -> Self {
        Self {
            warn_rss_mb: 384,     // HardLimits 512의 75%
            critical_rss_mb: 480, // HardLimits 512의 ~94%
            warn_cpu_pct: 80.0,
            throttle_sleep_ms: 100,
            batch_scale_factor: 0.5,
        }
    }
}

/// 적응 상태 — 매 사이클 check_and_adapt() 호출.
#[derive(Debug)]
pub struct AdaptiveThrottle {
    pub limits: SoftLimits,
    pub current_level: ThrottleLevel,
    pub checks: u64,
    pub throttled_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleLevel {
    /// 정상 — 제한 없음.
    Normal,
    /// 경고 — 배치 축소 + 짧은 sleep.
    Warn,
    /// 위험 — 최소 배치 + 긴 sleep.
    Critical,
}

impl AdaptiveThrottle {
    pub fn new(limits: SoftLimits) -> Self {
        Self {
            limits,
            current_level: ThrottleLevel::Normal,
            checks: 0,
            throttled_count: 0,
        }
    }

    /// 현재 프로세스 RSS(MB) 조회.
    pub fn current_rss_mb() -> u64 {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("ps")
                .args(["-o", "rss=", "-p", &std::process::id().to_string()])
                .output()
                .ok()
                .and_then(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .parse::<u64>()
                        .ok()
                })
                .map(|kb| kb / 1024)
                .unwrap_or(0)
        }
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/self/statm")
                .ok()
                .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
                .map(|pages| pages * 4 / 1024) // pages → MB
                .unwrap_or(0)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        { 0 }
    }

    /// 매 사이클 호출 — 현재 리소스 확인 후 적응 레벨 조정.
    /// 반환: (level, batch_scale, sleep_ms)
    pub fn check_and_adapt(&mut self) -> (ThrottleLevel, f64, u64) {
        self.checks += 1;
        let rss = Self::current_rss_mb();

        let level = if rss >= self.limits.critical_rss_mb {
            ThrottleLevel::Critical
        } else if rss >= self.limits.warn_rss_mb {
            ThrottleLevel::Warn
        } else {
            ThrottleLevel::Normal
        };

        self.current_level = level;

        match level {
            ThrottleLevel::Normal => (level, 1.0, 0),
            ThrottleLevel::Warn => {
                self.throttled_count += 1;
                (level, self.limits.batch_scale_factor, self.limits.throttle_sleep_ms)
            }
            ThrottleLevel::Critical => {
                self.throttled_count += 1;
                (
                    level,
                    self.limits.batch_scale_factor * 0.5, // 25% 배치
                    self.limits.throttle_sleep_ms * 3,     // 300ms sleep
                )
            }
        }
    }

    /// sleep 삽입 (check_and_adapt 결과의 sleep_ms 사용).
    pub fn maybe_sleep(&self, sleep_ms: u64) {
        if sleep_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        }
    }

    /// 상태 요약 문자열.
    pub fn status(&self) -> String {
        let rss = Self::current_rss_mb();
        format!(
            "rss={}MB level={:?} checks={} throttled={}",
            rss, self.current_level, self.checks, self.throttled_count
        )
    }
}

// ═══════════════════════════════════════════════════════════
// ─── C. COMBINED GUARD ───
// ═══════��════════════════════════════════════════���══════════

/// 통합 가드: init 시 하드 제한 적용 + AdaptiveThrottle 반환.
pub fn init_guard(hard: HardLimits, soft: SoftLimits) -> Result<AdaptiveThrottle, String> {
    init_hard_limits(hard)?;
    Ok(AdaptiveThrottle::new(soft))
}

/// 기본값으로 통합 가드 초기화.
pub fn init_default_guard() -> Result<AdaptiveThrottle, String> {
    init_guard(HardLimits::default(), SoftLimits::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hard_limits_are_sane() {
        let h = HardLimits::default();
        assert_eq!(h.max_rss_mb, 512);
        assert_eq!(h.nice_level, 10);
    }

    #[test]
    fn default_soft_limits_below_hard() {
        let h = HardLimits::default();
        let s = SoftLimits::default();
        assert!(s.warn_rss_mb < h.max_rss_mb);
        assert!(s.critical_rss_mb < h.max_rss_mb);
        assert!(s.critical_rss_mb > s.warn_rss_mb);
    }

    #[test]
    fn throttle_levels_progression() {
        let soft = SoftLimits {
            warn_rss_mb: 100,
            critical_rss_mb: 200,
            ..SoftLimits::default()
        };
        let mut t = AdaptiveThrottle::new(soft);
        // 초기 상태
        assert_eq!(t.current_level, ThrottleLevel::Normal);
        // check 호출 (실제 RSS에 따라 레벨 결정)
        let (level, scale, _) = t.check_and_adapt();
        assert!(scale > 0.0 && scale <= 1.0);
        assert!(t.checks == 1);
        let _ = level;
    }

    #[test]
    fn status_string_contains_rss() {
        let t = AdaptiveThrottle::new(SoftLimits::default());
        let s = t.status();
        assert!(s.contains("rss="));
        assert!(s.contains("level="));
    }

    #[test]
    fn rss_query_does_not_panic() {
        let rss = AdaptiveThrottle::current_rss_mb();
        // 0 이상 (정상 환경에서)
        let _ = rss;
    }
}
