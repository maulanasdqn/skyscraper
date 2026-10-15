#!/bin/bash
set -e

GREEN='\033[32m'
CYAN='\033[36m'
YELLOW='\033[33m'
RED='\033[31m'
BOLD='\033[1m'
NC='\033[0m'

LOG_DIR="/tmp/skyscraper-logs"
mkdir -p "$LOG_DIR"

cleanup() {
  echo -e "\n${YELLOW}Shutting down all services...${NC}"
  kill $COSMIUM_PID $PARALLAX_PID $SKY_API_PID $SKY_WORKER_PID $UI_PID 2>/dev/null
  wait 2>/dev/null
  echo -e "${GREEN}All services stopped.${NC}"
}
trap cleanup EXIT INT TERM

echo -e "${BOLD}${CYAN}"
echo "  ╔═══════════════════════════════════════════╗"
echo "  ║         SKYSCRAPER PLATFORM               ║"
echo "  ║     Starting all services...               ║"
echo "  ╚═══════════════════════════════════════════╝"
echo -e "${NC}"

# ── Check infrastructure ────────────────────────────
echo -e "${YELLOW}Checking infrastructure...${NC}"
pg_isready -h localhost -q 2>/dev/null || { echo -e "${RED}PostgreSQL not running!${NC}"; exit 1; }
redis-cli ping -q 2>/dev/null || { echo -e "${RED}Redis not running!${NC}"; exit 1; }
echo -e "${GREEN}  PostgreSQL ✓  Redis ✓${NC}"

# ── Env defaults ────────────────────────────────────
export DATABASE_URL="${DATABASE_URL:-postgres://skyscraper:skyscraper@localhost:5432/skyscraper}"
export PARALLAX_DB="${PARALLAX_DB:-postgres://skyscraper:skyscraper@localhost:5432/parallax}"
export API_ADMIN_KEY="${API_ADMIN_KEY:-sk_admin_test}"
export PROXY_PASS="${PROXY_PASS:-test_secret}"
export REDIS_URL="${REDIS_URL:-redis://localhost:6379}"
export COSMIUM_BINARY="${COSMIUM_BINARY:-$(dirname "$0")/../cosmium/out/cosmium/chrome}"
export RUST_LOG="${RUST_LOG:-info}"

DIR="$(cd "$(dirname "$0")" && pwd)"
COSMIUM_DIR="$(cd "$DIR/../cosmium" 2>/dev/null && pwd || echo "")"
PARALLAX_DIR="$(cd "$DIR/../parallax" 2>/dev/null && pwd || echo "")"

# ── 1. Cosmium (:3000) ─────────────────────────────
echo -e "${CYAN}[1/5] Cosmium :3000${NC}"
if [ -x "$COSMIUM_DIR/target/release/cosmium" ]; then
  cd "$COSMIUM_DIR"
  COSMIUM_PORT=3000 \
  COSMIUM_BINARY="$COSMIUM_DIR/out/cosmium/chrome" \
  COSMIUM_API_KEY="${COSMIUM_API_KEY:-dev-key}" \
  RUST_LOG="cosmium=$RUST_LOG" \
  ./target/release/cosmium serve > "$LOG_DIR/cosmium.log" 2>&1 &
  COSMIUM_PID=$!
  echo -e "  ${GREEN}PID $COSMIUM_PID${NC}"
else
  echo -e "  ${YELLOW}Binary not found, skipping${NC}"
  COSMIUM_PID=""
fi

# ── 2. Parallax (:3001) ────────────────────────────
echo -e "${CYAN}[2/5] Parallax :3001${NC}"
if [ -x "$PARALLAX_DIR/target/release/parallax-api" ]; then
  cd "$PARALLAX_DIR"
  DATABASE_URL="$PARALLAX_DB" \
  API_LISTEN="0.0.0.0:3001" \
  RUST_LOG="parallax=$RUST_LOG" \
  ./target/release/parallax-api > "$LOG_DIR/parallax.log" 2>&1 &
  PARALLAX_PID=$!
  echo -e "  ${GREEN}PID $PARALLAX_PID${NC}"
else
  echo -e "  ${YELLOW}Binary not found, skipping${NC}"
  PARALLAX_PID=""
fi

# ── 3. SkyScraper API (:3002) ──────────────────────
echo -e "${CYAN}[3/5] SkyScraper API :3002${NC}"
cd "$DIR"
DATABASE_URL="$DATABASE_URL" \
API_ADMIN_KEY="$API_ADMIN_KEY" \
PROXY_PASS="$PROXY_PASS" \
REDIS_URL="$REDIS_URL" \
PARALLAX_URL="http://localhost:3001" \
PARALLAX_PROXY_HOST="localhost" \
PARALLAX_PROXY_PORT="1080" \
COSMIUM_URL="http://localhost:3000" \
API_LISTEN="0.0.0.0:3002" \
RUST_LOG="skyscraper=$RUST_LOG" \
LOG_FORMAT="text" \
CACHE_TTL_SECS="${CACHE_TTL_SECS:-300}" \
./target/release/skyscraper-api > "$LOG_DIR/skyscraper-api.log" 2>&1 &
SKY_API_PID=$!
echo -e "  ${GREEN}PID $SKY_API_PID${NC}"

# ── 4. SkyScraper Worker ───────────────────────────
echo -e "${CYAN}[4/5] SkyScraper Worker${NC}"
DATABASE_URL="$DATABASE_URL" \
PROXY_PASS="$PROXY_PASS" \
REDIS_URL="$REDIS_URL" \
PARALLAX_URL="http://localhost:3001" \
PARALLAX_PROXY_HOST="localhost" \
PARALLAX_PROXY_PORT="1080" \
COSMIUM_URL="http://localhost:3000" \
WORKER_CONCURRENCY="${WORKER_CONCURRENCY:-4}" \
RUST_LOG="skyscraper=$RUST_LOG" \
LOG_FORMAT="text" \
CACHE_TTL_SECS="${CACHE_TTL_SECS:-300}" \
DOMAIN_MAX_RPM="${DOMAIN_MAX_RPM:-120}" \
./target/release/skyscraper-worker > "$LOG_DIR/skyscraper-worker.log" 2>&1 &
SKY_WORKER_PID=$!
echo -e "  ${GREEN}PID $SKY_WORKER_PID${NC}"

# ── 5. Dashboard UI (:5173) ───────────────────────
echo -e "${CYAN}[5/5] Dashboard UI :5173${NC}"
cd "$DIR/skyscraper-ui"
npx vite --host > "$LOG_DIR/ui.log" 2>&1 &
UI_PID=$!
echo -e "  ${GREEN}PID $UI_PID${NC}"

# ── Wait & verify ──────────────────────────────────
sleep 3

echo ""
echo -e "${BOLD}${CYAN}═══ Service Status ═══${NC}"

check() {
  local name=$1 url=$2
  local code=$(curl -s -o /dev/null -w '%{http_code}' "$url" 2>/dev/null || echo "000")
  if [ "$code" = "200" ]; then
    echo -e "  ${GREEN}✓ $name${NC}"
  else
    echo -e "  ${RED}✗ $name (HTTP $code)${NC}"
  fi
}

check "Cosmium      :3000" "http://localhost:3000/api/health"
check "Parallax     :3001" "http://localhost:3001/api/carriers"
check "SkyScraper   :3002" "http://localhost:3002/api/v1/health"
check "Dashboard    :5173" "http://localhost:5173"

echo ""
echo -e "${BOLD}${GREEN}  ▸ Dashboard:  http://localhost:5173${NC}"
echo -e "${BOLD}${GREEN}  ▸ API:        http://localhost:3002/api/v1${NC}"
echo -e "${BOLD}${GREEN}  ▸ Admin Key:  $API_ADMIN_KEY${NC}"
echo ""
echo -e "${YELLOW}Logs: $LOG_DIR/${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo ""

wait
