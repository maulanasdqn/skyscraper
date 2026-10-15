use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

#[derive(Serialize)]
pub struct Metrics {
    pub jobs_processed: u64,
    pub jobs_failed: u64,
    pub jobs_timed_out: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub scrape_total_ms: u64,
    pub webhook_deliveries: u64,
    pub webhook_failures: u64,
    pub dedup_coalesced: u64,
    pub domain_throttled: u64,
}

pub struct MetricsCollector {
    pub jobs_processed: AtomicU64,
    pub jobs_failed: AtomicU64,
    pub jobs_timed_out: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub scrape_total_ms: AtomicU64,
    pub webhook_deliveries: AtomicU64,
    pub webhook_failures: AtomicU64,
    pub dedup_coalesced: AtomicU64,
    pub domain_throttled: AtomicU64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            jobs_processed: AtomicU64::new(0),
            jobs_failed: AtomicU64::new(0),
            jobs_timed_out: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            scrape_total_ms: AtomicU64::new(0),
            webhook_deliveries: AtomicU64::new(0),
            webhook_failures: AtomicU64::new(0),
            dedup_coalesced: AtomicU64::new(0),
            domain_throttled: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> Metrics {
        Metrics {
            jobs_processed: self.jobs_processed.load(Ordering::Relaxed),
            jobs_failed: self.jobs_failed.load(Ordering::Relaxed),
            jobs_timed_out: self.jobs_timed_out.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            scrape_total_ms: self.scrape_total_ms.load(Ordering::Relaxed),
            webhook_deliveries: self.webhook_deliveries.load(Ordering::Relaxed),
            webhook_failures: self.webhook_failures.load(Ordering::Relaxed),
            dedup_coalesced: self.dedup_coalesced.load(Ordering::Relaxed),
            domain_throttled: self.domain_throttled.load(Ordering::Relaxed),
        }
    }

    pub fn record_job_completed(&self, elapsed_ms: u64) {
        self.jobs_processed.fetch_add(1, Ordering::Relaxed);
        self.scrape_total_ms
            .fetch_add(elapsed_ms, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_updates() {
        let m = MetricsCollector::new();
        m.jobs_processed.fetch_add(5, Ordering::Relaxed);
        m.cache_hits.fetch_add(3, Ordering::Relaxed);
        m.cache_misses.fetch_add(2, Ordering::Relaxed);
        m.record_job_completed(150);

        let snap = m.snapshot();
        assert_eq!(snap.jobs_processed, 6);
        assert_eq!(snap.cache_hits, 3);
        assert_eq!(snap.cache_misses, 2);
        assert_eq!(snap.scrape_total_ms, 150);
    }

    #[test]
    fn metrics_serializes() {
        let m = MetricsCollector::new();
        m.jobs_processed.fetch_add(10, Ordering::Relaxed);
        let snap = m.snapshot();
        let json = serde_json::to_value(&snap).unwrap();
        assert_eq!(json["jobs_processed"], 10);
        assert_eq!(json["cache_hits"], 0);
    }
}
