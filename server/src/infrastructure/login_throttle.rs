//! In-memory per-IP login attempt throttling.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Consecutive failures that trigger a lockout.
const MAX_FAILURES: u32 = 3;
/// Lockout length once triggered.
const LOCKOUT_DURATION: Duration = Duration::from_mins(5);

#[derive(Debug, Default)]
struct Attempts {
    failures: u32,
    locked_until: Option<Instant>,
}

/// Per-IP login throttle with process-local state.
///
/// After `MAX_FAILURES` consecutive wrong passwords an IP is locked out for
/// `LOCKOUT_DURATION`; every attempt during the lockout (even with the
/// correct password) is refused without extending the timer. State is
/// cleared on restart, by design.
#[derive(Debug, Clone)]
pub struct LoginThrottle {
    inner: Arc<Mutex<HashMap<IpAddr, Attempts>>>,
    max_failures: u32,
    lockout: Duration,
}

impl LoginThrottle {
    /// Create a throttle with the default limits (3 failures, 5 minutes).
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_failures: MAX_FAILURES,
            lockout: LOCKOUT_DURATION,
        }
    }

    /// Remaining lockout seconds for `ip`, or `None` when login is allowed.
    /// An expired lockout is cleared so a fresh cycle begins.
    #[must_use]
    pub fn locked_secs(&self, ip: IpAddr) -> Option<u64> {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let attempts = guard.get(&ip)?;
        match attempts.locked_until {
            Some(until) if until > Instant::now() => {
                Some(until.duration_since(Instant::now()).as_secs().max(1))
            }
            Some(_) => {
                guard.remove(&ip);
                None
            }
            None => None,
        }
    }

    /// Record a failed attempt; once the limit is hit the lockout starts.
    /// Calls during an active lockout do not extend it.
    pub fn record_failure(&self, ip: IpAddr) {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let attempts = guard.entry(ip).or_default();
        if attempts
            .locked_until
            .is_some_and(|until| until > Instant::now())
        {
            return;
        }
        attempts.failures += 1;
        if attempts.failures >= self.max_failures {
            attempts.locked_until = Some(Instant::now() + self.lockout);
            attempts.failures = 0;
        }
        drop(guard);
    }

    /// Clear all failure state for `ip` after a successful login.
    pub fn record_success(&self, ip: IpAddr) {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard.remove(&ip);
    }

    /// Throttle with custom limits, for tests.
    #[cfg(test)]
    fn with_limits(max_failures: u32, lockout: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_failures,
            lockout,
        }
    }
}

impl Default for LoginThrottle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn ip(last: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, last))
    }

    #[test]
    fn allows_login_before_the_limit() {
        let throttle = LoginThrottle::new();
        throttle.record_failure(ip(1));
        throttle.record_failure(ip(1));
        assert_eq!(throttle.locked_secs(ip(1)), None);
    }

    #[test]
    fn locks_out_after_three_consecutive_failures() {
        let throttle = LoginThrottle::new();
        for _ in 0..3 {
            throttle.record_failure(ip(1));
        }
        let secs = throttle.locked_secs(ip(1));
        assert!(secs.is_some_and(|s| s > 290 && s <= 300));
    }

    #[test]
    fn tracks_ips_independently() {
        let throttle = LoginThrottle::new();
        for _ in 0..3 {
            throttle.record_failure(ip(1));
        }
        assert_eq!(throttle.locked_secs(ip(2)), None);
    }

    #[test]
    fn successful_login_clears_the_failure_count() {
        let throttle = LoginThrottle::new();
        throttle.record_failure(ip(1));
        throttle.record_failure(ip(1));
        throttle.record_success(ip(1));
        throttle.record_failure(ip(1));
        throttle.record_failure(ip(1));
        assert_eq!(throttle.locked_secs(ip(1)), None);
    }

    #[test]
    fn expired_lockout_allows_a_fresh_cycle() {
        let throttle = LoginThrottle::with_limits(3, Duration::from_millis(10));
        for _ in 0..3 {
            throttle.record_failure(ip(1));
        }
        assert!(throttle.locked_secs(ip(1)).is_some());
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(throttle.locked_secs(ip(1)), None);
        throttle.record_failure(ip(1));
        assert_eq!(throttle.locked_secs(ip(1)), None);
    }

    #[test]
    fn failures_during_lockout_do_not_extend_it() {
        let throttle = LoginThrottle::with_limits(3, Duration::from_millis(50));
        for _ in 0..3 {
            throttle.record_failure(ip(1));
        }
        std::thread::sleep(Duration::from_millis(30));
        throttle.record_failure(ip(1));
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(throttle.locked_secs(ip(1)), None);
    }
}
