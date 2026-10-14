pub mod entity;
pub mod error;
pub mod job_status;
pub mod port;

#[cfg(test)]
mod tests {
    use super::error::DomainError;
    use super::job_status::JobStatus;

    mod job_status_transitions {
        use super::*;

        #[test]
        fn valid_transitions() {
            assert!(JobStatus::Pending.can_transition_to(JobStatus::Running));
            assert!(JobStatus::Running.can_transition_to(JobStatus::Completed));
            assert!(JobStatus::Running.can_transition_to(JobStatus::Failed));
            assert!(JobStatus::Failed.can_transition_to(JobStatus::Dead));
            assert!(JobStatus::Failed.can_transition_to(JobStatus::Pending));
            assert!(JobStatus::Dead.can_transition_to(JobStatus::Pending));
            assert!(JobStatus::Scheduled.can_transition_to(JobStatus::Pending));
            assert!(JobStatus::Running.can_transition_to(JobStatus::Pending));
        }

        #[test]
        fn invalid_transitions() {
            assert!(!JobStatus::Pending.can_transition_to(JobStatus::Completed));
            assert!(!JobStatus::Completed.can_transition_to(JobStatus::Pending));
            assert!(!JobStatus::Pending.can_transition_to(JobStatus::Dead));
            assert!(!JobStatus::Completed.can_transition_to(JobStatus::Running));
            assert!(!JobStatus::Dead.can_transition_to(JobStatus::Failed));
            assert!(!JobStatus::Pending.can_transition_to(JobStatus::Failed));
        }

        #[test]
        fn is_terminal() {
            assert!(JobStatus::Completed.is_terminal());
            assert!(JobStatus::Dead.is_terminal());
            assert!(!JobStatus::Pending.is_terminal());
            assert!(!JobStatus::Scheduled.is_terminal());
            assert!(!JobStatus::Running.is_terminal());
            assert!(!JobStatus::Failed.is_terminal());
        }
    }

    mod scrape_job_request_serde {
        use super::super::entity::{
            ProfileStrategy, ProxyStrategy, ScrapeJobRequest, WorkflowStep,
        };

        #[test]
        fn minimal_json_uses_defaults() {
            let json = r#"{"url": "https://example.com"}"#;
            let req: ScrapeJobRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.url, "https://example.com");
            assert_eq!(req.wait_ms, 3000);
            assert!(!req.screenshot);
            assert!(!req.include_html);
            assert!(!req.headful);
            assert!(req.extract.is_empty());
            assert!(req.script.is_none());
            assert!(req.workflow.is_empty());
            assert!(req.geo.is_none());
            assert!(matches!(req.proxy_strategy, ProxyStrategy::Auto));
            assert!(matches!(req.profile_strategy, ProfileStrategy::Auto));
        }

        #[test]
        fn full_json_round_trip() {
            let json = r#"{
                "url": "https://example.com",
                "extract": [".title", ".body"],
                "script": "return document.title",
                "workflow": [
                    {"type": "click", "selector": ".btn"},
                    {"type": "delay", "duration_ms": 500}
                ],
                "screenshot": true,
                "include_html": true,
                "wait_ms": 5000,
                "headful": true,
                "geo": "US",
                "proxy_strategy": {"type": "carrier", "name": "tmobile"},
                "profile_strategy": {"type": "named", "name": "win11_rtx"}
            }"#;
            let req: ScrapeJobRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.url, "https://example.com");
            assert_eq!(req.extract, vec![".title", ".body"]);
            assert_eq!(req.script.as_deref(), Some("return document.title"));
            assert_eq!(req.workflow.len(), 2);
            assert!(req.screenshot);
            assert!(req.include_html);
            assert_eq!(req.wait_ms, 5000);
            assert!(req.headful);
            assert_eq!(req.geo.as_deref(), Some("US"));
            assert!(matches!(req.proxy_strategy, ProxyStrategy::Carrier { ref name } if name == "tmobile"));
            assert!(matches!(req.profile_strategy, ProfileStrategy::Named { ref name } if name == "win11_rtx"));

            let serialized = serde_json::to_value(&req).unwrap();
            let deserialized: ScrapeJobRequest =
                serde_json::from_value(serialized).unwrap();
            assert_eq!(deserialized.url, req.url);
            assert_eq!(deserialized.wait_ms, req.wait_ms);
        }

        #[test]
        fn proxy_strategy_variants() {
            let auto: ProxyStrategy = serde_json::from_str(r#"{"type":"auto"}"#).unwrap();
            assert!(matches!(auto, ProxyStrategy::Auto));

            let carrier: ProxyStrategy =
                serde_json::from_str(r#"{"type":"carrier","name":"tmobile"}"#).unwrap();
            assert!(matches!(carrier, ProxyStrategy::Carrier { ref name } if name == "tmobile"));

            let none: ProxyStrategy = serde_json::from_str(r#"{"type":"none"}"#).unwrap();
            assert!(matches!(none, ProxyStrategy::None));
        }

        #[test]
        fn profile_strategy_variants() {
            let auto: ProfileStrategy = serde_json::from_str(r#"{"type":"auto"}"#).unwrap();
            assert!(matches!(auto, ProfileStrategy::Auto));

            let named: ProfileStrategy =
                serde_json::from_str(r#"{"type":"named","name":"chrome_mac"}"#).unwrap();
            assert!(matches!(named, ProfileStrategy::Named { ref name } if name == "chrome_mac"));

            let random: ProfileStrategy =
                serde_json::from_str(r#"{"type":"random"}"#).unwrap();
            assert!(matches!(random, ProfileStrategy::Random));
        }

        #[test]
        fn workflow_step_variants() {
            let click: WorkflowStep =
                serde_json::from_str(r#"{"type":"click","selector":".btn"}"#).unwrap();
            assert!(matches!(click, WorkflowStep::Click { ref selector } if selector == ".btn"));

            let delay: WorkflowStep =
                serde_json::from_str(r#"{"type":"delay","duration_ms":1000}"#).unwrap();
            assert!(matches!(delay, WorkflowStep::Delay { duration_ms: 1000 }));

            let extract: WorkflowStep = serde_json::from_str(
                r#"{"type":"extract","name":"items","selector":".item","limit":50}"#,
            )
            .unwrap();
            assert!(
                matches!(extract, WorkflowStep::Extract { ref name, ref selector, limit } if name == "items" && selector == ".item" && limit == Some(50))
            );

            let script: WorkflowStep =
                serde_json::from_str(r#"{"type":"script","code":"return 1"}"#).unwrap();
            assert!(matches!(script, WorkflowStep::Script { ref code } if code == "return 1"));
        }
    }

    mod domain_error_display {
        use super::*;

        #[test]
        fn error_messages() {
            assert_eq!(
                DomainError::NotFound("user".into()).to_string(),
                "not found: user"
            );
            assert_eq!(
                DomainError::AlreadyExists("email".into()).to_string(),
                "already exists: email"
            );
            assert_eq!(DomainError::Unauthorized.to_string(), "unauthorized");
            assert_eq!(
                DomainError::RateLimitExceeded.to_string(),
                "rate limit exceeded"
            );
            assert_eq!(
                DomainError::ConcurrencyLimitExceeded.to_string(),
                "concurrency limit exceeded"
            );
            assert_eq!(DomainError::TenantInactive.to_string(), "tenant inactive");
            assert_eq!(
                DomainError::InvalidInput("bad url".into()).to_string(),
                "invalid input: bad url"
            );
            assert_eq!(
                DomainError::ScrapeBlocked.to_string(),
                "scrape blocked by target site"
            );
            assert_eq!(
                DomainError::Upstream("timeout".into()).to_string(),
                "upstream error: timeout"
            );
        }
    }
}
