# SkyScraper

Multi-tenant scraping API platform built in Rust. Orchestrates [Parallax](https://github.com/maulanasdqn/parallax) (proxy router) and [Cosmium](https://github.com/maulanasdqn/cosmium) (patched browser) to deliver resilient, anti-detection web scraping as a service.

## Architecture

```
Client → SkyScraper API (:3002)
           │
           ├─ Authenticate (API key)
           ├─ Rate limit (Redis)
           ├─ Enqueue job (Redis priority queue)
           │
      Worker picks up job
           │
           ├─ ProxySelector → Parallax (:3001) → pick carrier
           ├─ ProfileRotator → Cosmium (:3000) → pick fingerprint profile
           │
           └─ POST Cosmium /api/scrape
                 with proxy = socks5://{carrier}-{uuid}:{pass}@parallax:1080
                 with profile = selected fingerprint profile
                       │
                       └─ Patched Chromium → Parallax proxy → Target website
                                                    │
                                              Result → SkyScraper → DB → Client
```

### Ecosystem

| Project | Role | Port |
|---------|------|------|
| **SkyScraper** | Scraping API & orchestrator | 3002 |
| **Parallax** | SOCKS5/HTTP proxy router with carrier-based routing | 1080 (proxy), 3001 (API) |
| **Cosmium** | Anti-fingerprint patched Chromium browser automation | 3000 |

## Features

- **Synchronous & asynchronous scraping** — long-poll or fire-and-forget
- **Batch job submission** — up to 100 URLs per request
- **Redis job queue** — priority lanes (high/default/low), scheduled jobs, dead-letter queue
- **Intelligent proxy selection** — geo-aware carrier picking with failure tracking
- **Profile rotation** — round-robin fingerprint profiles with block avoidance
- **Multi-tenant** — API key auth, per-tenant rate limits and concurrency limits
- **Retry with diversification** — rotates carrier + profile on each retry attempt
- **Usage tracking** — per-tenant bandwidth and request accounting

## Prerequisites

- Rust 1.85+
- PostgreSQL 15+
- Redis 7+
- [Parallax](https://github.com/maulanasdqn/parallax) running on port 3001/1080
- [Cosmium](https://github.com/maulanasdqn/cosmium) running on port 3000

## Configuration

```env
# Required
DATABASE_URL=postgres://user:pass@localhost:5432/skyscraper
API_ADMIN_KEY=sk_admin_your_secret_key
PROXY_PASS=shared_secret_matching_parallax

# Optional (defaults shown)
API_LISTEN=0.0.0.0:3002
REDIS_URL=redis://localhost:6379
PARALLAX_URL=http://localhost:3001
PARALLAX_PROXY_HOST=localhost
PARALLAX_PROXY_PORT=1080
COSMIUM_URL=http://localhost:3000
WORKER_CONCURRENCY=4
```

## Getting Started

```bash
# Create database
createdb skyscraper

# Run API server (auto-runs migrations)
DATABASE_URL=postgres://localhost/skyscraper \
API_ADMIN_KEY=sk_admin_dev \
PROXY_PASS=dev_secret \
cargo run --bin skyscraper-api

# Run worker (separate terminal)
DATABASE_URL=postgres://localhost/skyscraper \
PROXY_PASS=dev_secret \
cargo run --bin skyscraper-worker
```

## API

### Public

```
GET  /api/v1/health
```

### Authenticated (X-Api-Key header)

```
POST /api/v1/scrape              — synchronous scrape (long-poll)
POST /api/v1/jobs                — create async job
POST /api/v1/jobs/batch          — batch create jobs
GET  /api/v1/jobs                — list jobs (?status=completed&page=1&per_page=20)
GET  /api/v1/jobs/{id}           — get job + result
DELETE /api/v1/jobs/{id}         — cancel pending job
GET  /api/v1/usage               — monthly usage summary
```

### Admin (X-Api-Key: admin key)

```
GET  /api/v1/admin/tenants
POST /api/v1/admin/tenants
PATCH /api/v1/admin/tenants/{id}
POST /api/v1/admin/tenants/{id}/regenerate-key
GET  /api/v1/admin/queue/stats
POST /api/v1/admin/queue/retry-dead
GET  /api/v1/admin/carriers
```

### Example: Synchronous Scrape

```bash
curl -X POST http://localhost:3002/api/v1/scrape \
  -H "X-Api-Key: sk_your_api_key" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com",
    "extract": [".title", ".content"],
    "screenshot": true,
    "geo": "US",
    "proxy_strategy": { "type": "auto" },
    "profile_strategy": { "type": "auto" }
  }'
```

### Example: Async Job

```bash
# Submit
curl -X POST http://localhost:3002/api/v1/jobs \
  -H "X-Api-Key: sk_your_api_key" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com", "extract": [".title"]}'

# Poll
curl http://localhost:3002/api/v1/jobs/{job_id} \
  -H "X-Api-Key: sk_your_api_key"
```

## Project Structure

```
skyscraper/
├── skyscraper-api/           API server + shared library
│   ├── migrations/           SQL migrations
│   └── src/
│       ├── domain/           Entities, ports, errors
│       ├── application/      Business logic (AppService)
│       ├── adapter/
│       │   ├── http/         Axum handlers, auth, routing
│       │   └── postgres/     Repository implementations
│       └── infrastructure/
│           ├── clients/      Parallax & Cosmium HTTP clients
│           ├── queue/        Redis job queue & worker
│           └── intelligence/ Proxy selector & profile rotator
└── skyscraper-worker/        Standalone worker binary
```

## License

MIT
