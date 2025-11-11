# Current State - Main Branch (Authorization Server)

**Last Updated**: 2025-11-12
**Branch**: `main`
**Architecture**: Authorization Server (ADR-004) with Dynamic Client Registration

---

## Implementation Status

### ✅ Completed Features

**OAuth2 Authorization Server (ADR-004)**:
- [x] Full OAuth2 Authorization Code Flow (RFC 6749)
- [x] PKCE implementation (RFC 7636) for security
- [x] Dynamic Client Registration (RFC 7591)
- [x] Authorization Server Metadata (RFC 8414)
- [x] Encrypted cookie-based state management (AES-256-GCM)
- [x] OAuth endpoints: `/oauth/authorize`, `/oauth/callback`, `/oauth/token`
- [x] Token refresh flow with Miro API

**Code Structure**:
- `src/oauth/proxy_provider.rs` - OAuth provider implementation
- `src/oauth/endpoints.rs` - HTTP endpoints for OAuth flow
- `src/oauth/cookie_manager.rs` - Encrypted cookie management
- `src/oauth/pkce.rs` - PKCE code challenge/verifier
- `src/oauth/dcr.rs` - Dynamic Client Registration
- `src/oauth/types.rs` - OAuth type definitions

**MCP Integration**:
- [x] MCP protocol support (HTTP/SSE transport)
- [x] Miro API client with all operations
- [x] Board operations (list, create, update, delete)
- [x] Item operations (sticky notes, shapes, text, frames, connectors)
- [x] Bulk operations (20 items per batch)
- [x] Bearer token authentication

**Deployment Infrastructure**:
- [x] Scaleway Containers deployment
- [x] GitHub Actions CI/CD pipeline
- [x] Automated deployment via git tags (v*.*.*)
- [x] Structured logging with correlation IDs
- [x] Health check endpoint

---

## 🚨 Remaining Work for Production

### P0 Blockers

1. **TEST3** - End-to-end OAuth validation with Claude.ai
   - Verify complete OAuth flow from Claude.ai web interface
   - Test MCP tool calls with obtained tokens
   - Document any issues or edge cases

2. **DEPLOY4** - Secure secret management
   - Move MIRO_CLIENT_SECRET to Scaleway Secret Manager
   - Move MIRO_ENCRYPTION_KEY to Scaleway Secret Manager
   - Update deployment scripts to inject secrets
   - Verify secrets never appear in logs

3. **TEST2** - OAuth metadata discovery validation
   - Verify Claude.ai discovers OAuth endpoints via metadata
   - Test Authorization Server Metadata endpoint (RFC 8414)
   - Confirm automatic OAuth flow initiation

### Medium Priority

- **TECH6** - Optimize build time (90s → 30s via feature flags)
- **DEPLOY3** - Automated deployment validation
- **DOCKER1-4** - Container hardening (non-root user, volumes, health checks)
- **DOC1-2** - Production documentation (architecture, deployment guide)

---

## Architecture Overview

**Pattern**: Proxy OAuth (Authorization Server)

```
Claude.ai Web
    ↓
    | (1) MCP connection
    ↓
Our Server (/oauth/authorize)
    ↓
    | (2) Redirect with PKCE
    ↓
Miro OAuth Server
    ↓
    | (3) Authorization code
    ↓
Our Server (/oauth/callback)
    ↓
    | (4) Exchange code for token
    ↓
Miro Token Endpoint
    ↓
    | (5) Access token (stored in encrypted cookie)
    ↓
Our Server (/oauth/token)
    ↓
    | (6) Return token to Claude.ai
    ↓
Claude.ai (stores token)
    ↓
    | (7) MCP tool calls with Bearer token
    ↓
Our Server → Miro API
```

**Key Components**:
- **Encrypted Cookies**: AES-256-GCM for OAuth state and tokens
- **PKCE**: Code challenge/verifier for authorization code flow
- **DCR**: Dynamic client registration for flexible OAuth config
- **Stateless**: No database required (cookies + in-memory cache)

---

## Deployment

**Platform**: Scaleway Managed Containers
**URL**: `https://miro-mcp.fly-agile.com`
**Container**: `flyagileapipx8njvei-miro-mcp` (namespace: fly-agile-api)

**Configuration**:
- Memory: 256Mi
- CPU: 0.25 vCPU
- Min/Max Scale: 1 (always-on)
- Port: 8080

**Environment Variables** (.env.production):
```bash
MIRO_CLIENT_ID=3458764647632208270
MIRO_REDIRECT_URI=https://miro-mcp.fly-agile.com/oauth/callback
BASE_URL=https://miro-mcp.fly-agile.com
MCP_SERVER_PORT=8080
```

**Secrets** (Scaleway Secret Manager - TODO):
- `MIRO_CLIENT_SECRET` (Miro app secret)
- `MIRO_ENCRYPTION_KEY` (AES-256-GCM key for cookie encryption)

---

## Alternative: Resource Server Pattern

**Status**: Explored in separate worktree (`feat/resource-server-pattern`)

**Documentation**: Archived in [planning/archive/](planning/archive/)
- ADR-005: Resource Server with Claude OAuth
- REFACTOR-BACKLOG.md: Implementation plan
- PIVOT-SUMMARY.md: Decision analysis

**Potential**: May become separate fork/project for simpler MCP OAuth servers

**Key Difference**:
- **This branch**: Full OAuth control (~1000 LOC, reusable infrastructure)
- **Resource Server**: Delegate to Claude (~150 LOC, simpler maintenance)

---

## Quick Commands

**Deploy to production**:
```bash
git tag v0.3.0 && git push origin v0.3.0
```

**Check deployment status**:
```bash
./scripts/check-logs.sh
```

**Test OAuth flow locally**:
```bash
cargo run --bin miro-mcp-http
# Open http://localhost:8080/oauth/authorize
```

**Run tests**:
```bash
cargo test
cargo clippy
```

---

## Next Steps

1. Complete TEST3 - validate OAuth flow with Claude.ai web
2. Complete DEPLOY4 - secure secret management
3. Complete TEST2 - verify metadata discovery
4. Production launch ✅

**Timeline**: 1-2 days remaining (secret management + validation)
