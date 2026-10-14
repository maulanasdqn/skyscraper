use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    name: String,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    failure_threshold: u32,
    success_threshold: u32,
    recovery_timeout: Duration,
    last_failure_epoch: AtomicU64,
}

impl CircuitBreaker {
    pub fn new(name: &str, failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            name: name.to_string(),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            failure_threshold,
            success_threshold: 2,
            recovery_timeout,
            last_failure_epoch: AtomicU64::new(0),
        }
    }

    pub fn state(&self) -> CircuitState {
        let failures = self.failure_count.load(Ordering::Relaxed);
        if failures < self.failure_threshold {
            return CircuitState::Closed;
        }

        let last_failure = self.last_failure_epoch.load(Ordering::Relaxed);
        let now = epoch_secs();
        let elapsed = now.saturating_sub(last_failure);

        if elapsed >= self.recovery_timeout.as_secs() {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    pub fn is_call_permitted(&self) -> bool {
        match self.state() {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        }
    }

    pub fn record_success(&self) {
        match self.state() {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.success_threshold {
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    tracing::info!("circuit breaker '{}' closed (recovered)", self.name);
                }
            }
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&self) {
        let was_half_open = self.state() == CircuitState::HalfOpen;
        self.success_count.store(0, Ordering::Relaxed);
        self.last_failure_epoch
            .store(epoch_secs(), Ordering::Relaxed);

        if was_half_open {
            self.failure_count
                .store(self.failure_threshold, Ordering::Relaxed);
            tracing::warn!(
                "circuit breaker '{}' reopened from half-open after failure",
                self.name
            );
        } else {
            let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count >= self.failure_threshold {
                tracing::warn!(
                    "circuit breaker '{}' opened after {count} failures",
                    self.name
                );
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_closed() {
        let cb = CircuitBreaker::new("test", 3, Duration::from_secs(30));
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_call_permitted());
    }

    #[test]
    fn opens_after_threshold_failures() {
        let cb = CircuitBreaker::new("test", 3, Duration::from_secs(30));
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_call_permitted());
    }

    #[test]
    fn success_resets_failure_count_when_closed() {
        let cb = CircuitBreaker::new("test", 3, Duration::from_secs(30));
        cb.record_failure();
        cb.record_failure();
        cb.record_success();

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn half_open_after_recovery_timeout() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_secs(0));
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.is_call_permitted());
    }

    #[test]
    fn half_open_closes_after_success_threshold() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_secs(0));
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_success();
        cb.record_success();

        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn half_open_reopens_on_failure() {
        let cb = CircuitBreaker::new("test", 2, Duration::from_secs(0));
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::HalfOpen);
        cb.record_failure();

        // After failure in half-open, last_failure_epoch is refreshed,
        // but with recovery_timeout=0 it immediately returns to half-open.
        // Use a longer timeout to test the reopen behavior.
        let cb2 = CircuitBreaker::new("test2", 2, Duration::from_mins(1));
        cb2.record_failure();
        cb2.record_failure();
        assert_eq!(cb2.state(), CircuitState::Open);

        // Simulate recovery timeout passing by setting last_failure to the past.
        cb2.last_failure_epoch
            .store(epoch_secs() - 61, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(cb2.state(), CircuitState::HalfOpen);

        cb2.record_failure();
        assert_eq!(cb2.state(), CircuitState::Open);
    }
}
