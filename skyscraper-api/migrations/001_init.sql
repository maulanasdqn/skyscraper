CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE tenant_plan AS ENUM ('free', 'starter', 'pro', 'business');
CREATE TYPE job_status AS ENUM ('pending', 'scheduled', 'running', 'completed', 'failed', 'dead');

CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT NOT NULL UNIQUE,
    api_key TEXT NOT NULL UNIQUE,
    plan tenant_plan NOT NULL DEFAULT 'free',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    rate_limit_rpm INTEGER NOT NULL DEFAULT 10,
    concurrent_limit INTEGER NOT NULL DEFAULT 2,
    webhook_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    status job_status NOT NULL DEFAULT 'pending',
    priority SMALLINT NOT NULL DEFAULT 5,
    request JSONB NOT NULL,
    result JSONB,
    carrier_id UUID,
    profile_name TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 2,
    scheduled_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE usage_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    bytes_consumed BIGINT NOT NULL DEFAULT 0,
    billed BOOLEAN NOT NULL DEFAULT FALSE,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenants_api_key ON tenants(api_key);
CREATE INDEX idx_jobs_tenant_id ON jobs(tenant_id);
CREATE INDEX idx_jobs_status ON jobs(status);
CREATE INDEX idx_jobs_tenant_status ON jobs(tenant_id, status);
CREATE INDEX idx_jobs_scheduled ON jobs(scheduled_at) WHERE status = 'scheduled';
CREATE INDEX idx_usage_tenant ON usage_records(tenant_id, recorded_at);
