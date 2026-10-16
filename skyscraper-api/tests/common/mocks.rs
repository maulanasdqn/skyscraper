use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use skyscraper::domain::entity::{Tenant, TenantPlan, UsageRecord};
use skyscraper::domain::error::DomainError;
use skyscraper::domain::port::{TenantRepository, UsageRepository};

pub struct MockTenantRepository {
    tenants: Mutex<Vec<Tenant>>,
}

impl MockTenantRepository {
    pub fn new() -> Self {
        Self {
            tenants: Mutex::new(Vec::new()),
        }
    }

    pub fn with_tenant(tenant: Tenant) -> Self {
        Self {
            tenants: Mutex::new(vec![tenant]),
        }
    }
}

#[async_trait]
impl TenantRepository for MockTenantRepository {
    async fn find_by_api_key(&self, key: &str) -> Result<Option<Tenant>, DomainError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.iter().find(|t| t.api_key == key).cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>, DomainError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants.iter().find(|t| t.id == id).cloned())
    }

    async fn create(&self, email: &str, plan: TenantPlan) -> Result<Tenant, DomainError> {
        let tenant = Tenant {
            id: Uuid::new_v4(),
            email: email.to_string(),
            api_key: format!("sk_{}", Uuid::new_v4()),
            plan,
            active: true,
            rate_limit_rpm: 60,
            concurrent_limit: 5,
            webhook_url: None,
            created_at: Utc::now(),
        };
        self.tenants.lock().unwrap().push(tenant.clone());
        Ok(tenant)
    }

    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Tenant>, DomainError> {
        let tenants = self.tenants.lock().unwrap();
        Ok(tenants
            .iter()
            .skip(offset as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn update_plan(&self, id: Uuid, plan: TenantPlan) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.plan = plan;
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn set_active(&self, id: Uuid, active: bool) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.active = active;
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn set_webhook_url(&self, id: Uuid, url: Option<&str>) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.webhook_url = url.map(String::from);
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn set_limits(&self, id: Uuid, rate_limit_rpm: i32, concurrent_limit: i32) -> Result<(), DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.rate_limit_rpm = rate_limit_rpm;
            t.concurrent_limit = concurrent_limit;
            Ok(())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }

    async fn regenerate_api_key(&self, id: Uuid) -> Result<String, DomainError> {
        let mut tenants = self.tenants.lock().unwrap();
        if let Some(t) = tenants.iter_mut().find(|t| t.id == id) {
            t.api_key = format!("sk_{}", Uuid::new_v4());
            Ok(t.api_key.clone())
        } else {
            Err(DomainError::NotFound("tenant".into()))
        }
    }
}

pub struct MockUsageRepository;

#[async_trait]
impl UsageRepository for MockUsageRepository {
    async fn record(&self, _: Uuid, _: Uuid, _: i64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn total_for_tenant(&self, _: Uuid, _: DateTime<Utc>) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn list_for_tenant(
        &self,
        _: Uuid,
        _: i64,
        _: i64,
    ) -> Result<Vec<UsageRecord>, DomainError> {
        Ok(vec![])
    }
}
