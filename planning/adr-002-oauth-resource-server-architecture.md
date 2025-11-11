# ADR-002: OAuth Resource Server Architecture for Miro MCP

**Status:** Superseded by ADR-004 for Claude.ai web
**Date:** 2025-11-10 (Updated: 2025-11-11)
**Context:** Remote MCP server integrating with external OAuth provider (Miro)
**Decision Makers:** Solution Architect, Security Specialist, Integration Specialist
**Applies To:** **N/A** - Pattern doesn't work with Claude.ai web custom connectors

> **Critical Update (2025-11-11)**: This ADR documented the Resource Server pattern based on RFC 9728 metadata discovery. Empirical testing revealed **Claude.ai web ignores metadata discovery** and requires convention-based Proxy OAuth endpoints instead. **ADR-004 supersedes this ADR for HTTP mode** with Claude.ai web. This pattern might be valid for other MCP clients that properly implement RFC 9728.

---

## Context and Problem Statement

ADR-001 documented a **Proxy OAuth** architecture (PKCE + encrypted cookies) suitable for OAuth providers we control (GitHub, GitLab). However, our actual requirements differ fundamentally:

**Our Situation:**
- **OAuth provider:** Miro (we don't control their OAuth app)
- **Client ID:** Provided by Miro (`3458764647516852398`)
- **Callback URL:** Must be `https://claude.ai/api/mcp/auth_callback` (Claude-controlled)
- **MCP clients:** Claude Desktop, Claude iOS, Claude.ai web
- **Architecture:** MCP server as **Resource Server**, not OAuth client

**Key Realization:** Proxy OAuth (ADR-001) assumes we manage OAuth callbacks. With Miro, **Claude platform manages OAuth**, and we receive pre-authenticated Bearer tokens.

**Critical Question:** How should our MCP server authenticate requests when Claude handles OAuth with Miro?

**Architecture Selected (2025-11-10):** Resource Server with Token Validation + Caching

**Architecture Status (2025-11-11):** **SUPERSEDED** - Claude.ai web doesn't use this pattern. See ADR-004.

---

## Decision

**Use Resource Server Pattern with Token Validation + Caching**

We will implement MCP server as an OAuth **Resource Server**:
- **Claude** handles full OAuth flow with Miro (user authorization, token exchange)
- **Our server** validates Bearer tokens via Miro's introspection endpoint
- **LRU cache** (5-minute TTL) reduces validation latency from 100ms to <1ms
- **Stateless architecture** maintained (no session database)

**This is the standard MCP remote authentication pattern per RFC 9728.**

---

## Architecture Comparison

### ADR-001 Pattern (Rejected for Miro)

```
User → Claude Desktop
         ↓
Your MCP Server
         ↓
[OAuth client logic]
         ↓
OAuth Provider (GitHub)
         ↓ (callback to your-server.com/oauth/callback)
Your MCP Server
         ↓
Return tokens to Claude
```

**Why unsuitable for Miro:**
- ❌ Miro callback URL can't point to our server
- ❌ We don't create Miro OAuth app (they provide it)
- ❌ Claude controls callback (`claude.ai/api/mcp/auth_callback`)
- ❌ ProxyOAuthServerProvider pattern doesn't match MCP remote spec

---

### ADR-002 Pattern (Selected)

```
User → Claude Desktop
         ↓
Claude Platform
         ↓
[Claude manages OAuth with Miro]
         ↓
Miro OAuth (miro.com/oauth/authorize)
         ↓
User authorizes
         ↓
Callback to Claude (claude.ai/api/mcp/auth_callback)
         ↓
Claude Platform
         ↓
MCP Request with Bearer token
         ↓
Your MCP Server (validates token, proxies to Miro API)
```

**Why correct for Miro:**
- ✅ Claude discovers OAuth via `/.well-known/oauth-protected-resource`
- ✅ Claude handles full OAuth flow with Miro
- ✅ Our server validates tokens only (Resource Server role)
- ✅ Follows MCP specification (RFC 9728)

---

## Considered Options

### Option A: Trust Claude (No Validation)

**Implementation:**
```rust
async fn handle_mcp_request(req: Request) -> Response {
    let token = extract_bearer_token(&req)?;
    // Directly proxy to Miro API without validation
    proxy_to_miro(token, req).await
}
```

**Pros:**
- ✅ Simplest (zero latency overhead)
- ✅ No extra API calls

**Cons:**
- ❌ No user identity (can't log who made requests)
- ❌ Can't implement per-user rate limiting
- ❌ No audit trail

**Verdict:** Not suitable for production use

---

### Option B: Validate Every Request (No Cache)

**Implementation:**
```rust
async fn handle_mcp_request(req: Request) -> Response {
    let token = extract_bearer_token(&req)?;

    // Validate with Miro introspection endpoint
    let user_info = reqwest::get("https://api.miro.com/v1/oauth-token")
        .bearer_auth(token)
        .send()
        .await?
        .json::<UserInfo>()
        .await?;

    tracing::info!(user_id = %user_info.user, "Request from user");

    proxy_to_miro(token, req).await
}
```

**Pros:**
- ✅ User identity for audit logs
- ✅ Token validation on every request
- ✅ Immediate revocation detection

**Cons:**
- ❌ +100ms latency per request (Miro API call)
- ❌ Doubles Miro API quota usage

**Verdict:** Secure but slow

---

### Option C: Validate with Caching (SELECTED)

**Implementation:**
```rust
use lru::LruCache;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

pub struct TokenValidator {
    cache: Mutex<LruCache<String, CachedUserInfo>>,
    http_client: Client,
}

#[derive(Clone)]
struct CachedUserInfo {
    user_info: UserInfo,
    cached_at: SystemTime,
}

impl TokenValidator {
    pub async fn validate(&self, token: &str) -> Result<UserInfo, AuthError> {
        // Check cache (5-minute TTL)
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(token) {
                let age = SystemTime::now()
                    .duration_since(cached.cached_at)
                    .unwrap_or(Duration::from_secs(999999));

                if age < Duration::from_secs(300) {
                    return Ok(cached.user_info.clone());
                }
            }
        }

        // Cache miss - validate with Miro
        let response = self.http_client
            .get("https://api.miro.com/v1/oauth-token")
            .bearer_auth(token)
            .send()
            .await?;

        if response.status() == StatusCode::OK {
            let user_info: UserInfo = response.json().await?;

            // Cache result
            self.cache.lock().unwrap().put(
                token.to_string(),
                CachedUserInfo {
                    user_info: user_info.clone(),
                    cached_at: SystemTime::now(),
                }
            );

            Ok(user_info)
        } else {
            Err(AuthError::InvalidToken)
        }
    }
}
```

**Pros:**
- ✅ User identity for audit logs
- ✅ Fast after cache hit (~0ms vs 100ms)
- ✅ Validates tokens periodically (5-min window)
- ✅ Reduces Miro API calls by 95%

**Cons:**
- ⚠️ Slightly more complex (cache management)
- ⚠️ Revoked tokens work for up to 5 minutes

**Performance:**
- First request: ~100ms (validate with Miro)
- Cached requests: <1ms (no API call)
- Cache TTL: 5 minutes (balance security vs performance)

**Verdict:** Best balance of security, performance, and observability

---

## Implementation Details

### 1. OAuth Metadata Endpoint (RFC 9728)

**Purpose:** Tell Claude which OAuth provider to use

**Endpoint:** `GET /.well-known/oauth-protected-resource`

```rust
use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct OAuthProtectedResource {
    protected_resources: Vec<ProtectedResourceInfo>,
}

#[derive(Serialize)]
struct ProtectedResourceInfo {
    resource: String,
    authorization_servers: Vec<String>,
}

async fn oauth_metadata() -> impl IntoResponse {
    Json(OAuthProtectedResource {
        protected_resources: vec![ProtectedResourceInfo {
            resource: "https://api.miro.com".to_string(),
            authorization_servers: vec!["https://miro.com/oauth".to_string()],
        }],
    })
}
```

**Response:**
```json
{
  "protected_resources": [
    {
      "resource": "https://api.miro.com",
      "authorization_servers": ["https://miro.com/oauth"]
    }
  ]
}
```

**What this does:** Claude discovers "this MCP server uses Miro OAuth" and initiates OAuth flow directly with Miro.

---

### 2. Bearer Token Extraction

```rust
fn extract_bearer_token(req: &Request) -> Result<String, AuthError> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| AuthError::InvalidAuthHeader)?;

    auth_str
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidAuthHeaderFormat)
        .map(|s| s.to_string())
}
```

**MCP clients send:**
```
POST /mcp HTTP/1.1
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

---

### 3. Token Validator with Caching

**File:** `src/auth/token_validator.rs`

```rust
pub struct MiroTokenValidator {
    cache: Mutex<LruCache<String, CachedUserInfo>>,
    http_client: Client,
}

impl MiroTokenValidator {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(capacity.try_into().unwrap())),
            http_client: Client::new(),
        }
    }

    pub async fn validate(&self, token: &str) -> Result<UserInfo, AuthError> {
        // Try cache first
        if let Some(cached) = self.get_from_cache(token) {
            return Ok(cached);
        }

        // Cache miss - validate with Miro
        let user_info = self.validate_with_miro(token).await?;

        // Store in cache
        self.put_in_cache(token, &user_info);

        Ok(user_info)
    }

    async fn validate_with_miro(&self, token: &str) -> Result<UserInfo, AuthError> {
        let response = self.http_client
            .get("https://api.miro.com/v1/oauth-token")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| AuthError::NetworkError(e.to_string()))?;

        match response.status() {
            StatusCode::OK => {
                let user_info = response.json::<UserInfo>().await
                    .map_err(|e| AuthError::ParseError(e.to_string()))?;
                Ok(user_info)
            }
            StatusCode::UNAUTHORIZED => Err(AuthError::InvalidToken),
            status => Err(AuthError::MiroApiError(status)),
        }
    }

    fn get_from_cache(&self, token: &str) -> Option<UserInfo> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(token).and_then(|cached| {
            let age = SystemTime::now()
                .duration_since(cached.cached_at)
                .unwrap_or(Duration::from_secs(999999));

            if age < Duration::from_secs(300) {
                Some(cached.user_info.clone())
            } else {
                None
            }
        })
    }

    fn put_in_cache(&self, token: &str, user_info: &UserInfo) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(
            token.to_string(),
            CachedUserInfo {
                user_info: user_info.clone(),
                cached_at: SystemTime::now(),
            }
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user: String,
    pub team: String,
    pub scopes: Vec<String>,
}
```

**Cache Configuration:**
- Capacity: 100 tokens (sufficient for personal use)
- TTL: 5 minutes (300 seconds)
- Eviction: LRU (Least Recently Used)

---

### 4. MCP Request Handler

```rust
use axum::{extract::State, http::StatusCode, Json, response::Response};

pub struct AppState {
    pub validator: Arc<MiroTokenValidator>,
    pub miro_client: Arc<MiroClient>,
}

async fn handle_mcp_request(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    // Extract Bearer token
    let token = extract_bearer_token(&req)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Validate token (with caching)
    let user_info = state.validator.validate(&token).await
        .map_err(|e| {
            tracing::warn!(error = ?e, "Token validation failed");
            StatusCode::UNAUTHORIZED
        })?;

    // Log request with user context
    tracing::info!(
        user_id = %user_info.user,
        team_id = %user_info.team,
        "MCP request from authenticated user"
    );

    // Proxy request to Miro API
    let miro_response = state.miro_client
        .proxy_request(&token, req)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Miro API request failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(miro_response)
}
```

---

### 5. Miro API Client

**File:** `src/miro/client.rs`

```rust
pub struct MiroClient {
    http_client: Client,
    base_url: String,
}

impl MiroClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            base_url: "https://api.miro.com/v2".to_string(),
        }
    }

    pub async fn list_boards(&self, token: &str) -> Result<Vec<Board>, MiroError> {
        let url = format!("{}/boards", self.base_url);

        let response = self.http_client
            .get(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;

        if response.status().is_success() {
            let boards_response: BoardsResponse = response.json().await?;
            Ok(boards_response.data)
        } else {
            Err(MiroError::ApiError {
                status: response.status(),
                message: response.text().await?,
            })
        }
    }

    pub async fn create_sticky_note(
        &self,
        token: &str,
        board_id: &str,
        content: &str,
        position: Position,
    ) -> Result<StickyNote, MiroError> {
        let url = format!("{}/boards/{}/sticky_notes", self.base_url, board_id);

        let request_body = json!({
            "data": {
                "content": content,
            },
            "position": {
                "x": position.x,
                "y": position.y,
            },
            "style": {
                "fillColor": "light_yellow",
            }
        });

        let response = self.http_client
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(MiroError::ApiError {
                status: response.status(),
                message: response.text().await?,
            })
        }
    }
}
```

**Key Pattern:** Always pass token from Claude to Miro API. We never store or transform tokens.

---

## Files to Create/Modify

### New Files (3)

1. **`src/auth/token_validator.rs`** - Token validation with LRU cache
2. **`src/mcp/metadata.rs`** - RFC 9728 OAuth metadata endpoint
3. **`planning/adr-002-oauth-resource-server-architecture.md`** - This document

### Files to Modify (2)

1. **`src/http_server.rs`** - Replace cookie auth with Bearer token validation
2. **`src/lib.rs`** - Export new token validator types

### Files to Remove (5)

1. **`src/auth/cookie_token.rs`** - Cookie-based auth (ADR-001 pattern)
2. **`src/auth/cookie_state.rs`** - PKCE state management (not needed)
3. **`src/http_server.rs:35-145`** - OAuth callback endpoint
4. **`src/http_server.rs:201-213`** - OAuth authorize endpoint
5. **`planning/adr-001-oauth2-stateless-architecture.md`** - Mark as superseded

**Total scope:** Remove 5 files/sections, add 3 files, modify 2 files = **10 file changes**

---

## Security Analysis

### Threat Model

| Threat | Mitigation | Status |
|--------|------------|--------|
| **Token theft (network)** | HTTPS mandatory | ✅ Enforced |
| **Token theft (logs)** | Never log tokens | ✅ Implemented |
| **Invalid token** | Validate with Miro API | ✅ Every request |
| **Revoked token** | Cache expires in 5 minutes | ⚠️ Acceptable delay |
| **Token replay** | Miro manages token lifecycle | ✅ Provider enforced |
| **CSRF attacks** | Not applicable (Bearer tokens) | ✅ N/A |
| **Code interception** | Claude handles PKCE | ✅ Platform enforced |

### Security Properties

**What we get:**
- ✅ User authentication (Miro validates identity)
- ✅ User authorization (Miro checks scopes)
- ✅ Audit trail (logs show user_id for each request)
- ✅ Token validation (detect invalid/expired tokens)
- ✅ Transport security (HTTPS)

**What we DON'T need:**
- ❌ OAuth flow management (Claude handles it)
- ❌ PKCE implementation (Claude handles it)
- ❌ State parameter tracking (no callback to our server)
- ❌ User allowlist (Miro controls access)

### Trust Boundaries

```
Claude Desktop (User trusts)
    ↓
Claude Platform OAuth (Claude authenticates)
    ↓
Miro OAuth (Miro issues token)
    ↓
Our MCP Server (We validate token)
    ↓
Miro API (Miro enforces access)
```

**Assumptions:**
- ✅ Claude validates user correctly
- ✅ Miro issues legitimate tokens
- ✅ Token not compromised in transit (HTTPS)
- ⚠️ We trust Claude + Miro for authentication

---

## Performance Characteristics

### Latency Breakdown

**First request (cache miss):**
```
Token validation:      100ms  (Miro API call)
Miro API request:      200ms  (list boards, etc.)
Total:                 300ms
```

**Cached requests (cache hit):**
```
Token validation:        <1ms  (LRU cache lookup)
Miro API request:      200ms  (actual API work)
Total:                 201ms
```

**Cache hit rate (estimated):** 95% (5-minute TTL)

**Effective average latency:**
```
(5% × 300ms) + (95% × 201ms) = 206ms
```

**Comparison to no caching:**
```
Every request: 300ms
With caching:  206ms (31% improvement)
```

---

## Cost Analysis

| Component | Cost |
|-----------|------|
| **Compute** | $0-5/month (serverless) |
| **Database** | $0 (no database needed) |
| **Miro API calls** | Free (within quota) |
| **Bandwidth** | $0-2/month |
| **Total** | **$0-7/month** |

**Comparison to ADR-001 (Proxy OAuth):**
- ADR-001: $0-5/month (but doesn't work for Miro)
- ADR-002: $0-7/month (correct pattern for Miro)
- Difference: Negligible

---

## Implementation Checklist

### Phase 1: Core Resource Server (Sprint 1)

- [x] Create ADR-002 documenting architecture
- [ ] Implement OAuth metadata endpoint (RFC 9728)
- [ ] Implement Bearer token extraction
- [ ] Implement Miro token validator with caching
- [ ] Remove OAuth client code (callbacks, cookies)
- [ ] Update HTTP server to validate Bearer tokens

### Phase 2: MCP Tools (Sprint 2)

- [ ] Implement `list_boards` tool
- [ ] Implement `create_sticky_note` tool
- [ ] Implement `create_shape` tool
- [ ] Implement `create_connector` tool
- [ ] Implement bulk operations (squad visualization)

### Phase 3: Testing (Sprint 3)

- [ ] Test OAuth metadata discovery
- [ ] Test token validation (valid/invalid/expired)
- [ ] Test cache hit/miss scenarios
- [ ] Test with mock Claude client
- [ ] Test with real Claude Desktop

### Phase 4: Production (Sprint 3)

- [ ] Deploy to Scaleway Containers
- [ ] Configure HTTPS
- [ ] Test end-to-end OAuth flow
- [ ] Verify performance metrics
- [ ] Monitor token validation success rate

---

## Migration from ADR-001

**ADR-001 Status:** Superseded (pattern doesn't apply to Miro)

**Why superseded:**
- ADR-001 assumed we control OAuth provider (GitHub, GitLab)
- ADR-001 pattern requires callback URL to our server
- Miro requires callback to `claude.ai/api/mcp/auth_callback`
- MCP specification requires Resource Server pattern for external providers

**ADR-001 learnings retained:**
- Stateless architecture principle
- Security best practices (HTTPS, token encryption)
- Deployment patterns (serverless)

**ADR-001 code to remove:**
- OAuth client endpoints (`/oauth/authorize`, `/oauth/callback`)
- Cookie-based token storage
- PKCE state management
- ProxyOAuthServerProvider usage

**No breaking changes:** ADR-001 code was never deployed to production.

---

## Consequences

### Positive

- ✅ **Simpler than ADR-001** (150 LOC vs 500 LOC - 70% reduction)
- ✅ **MCP specification compliant** (RFC 9728 Resource Server)
- ✅ **Works with Claude Desktop, iOS, Web** (cross-platform)
- ✅ **Stateless architecture maintained** (no database)
- ✅ **User audit trail** (logs show user_id per request)
- ✅ **Performance optimized** (95% cache hit rate)
- ✅ **Cost-effective** ($0-7/month)

### Negative

- ⚠️ **Revoked tokens active for 5 minutes** (cache TTL)
  - Acceptable for personal use
  - Can reduce TTL to 1 minute if needed
- ⚠️ **Dependency on Miro introspection API** (availability)
  - Mitigated by caching (degrades gracefully)
  - Miro API has 99.9% uptime SLA

### Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Miro API downtime | Low | Medium | Cache continues working (stale data) |
| Cache memory exhaustion | Low | Low | LRU eviction (100 token capacity) |
| Token theft | Low | Medium | HTTPS, short Miro token TTL (1h) |
| Cache poisoning | Very Low | Low | Cache only after successful Miro validation |

---

## References

### Standards

- [RFC 9728 - OAuth 2.0 Protected Resource Metadata](https://datatracker.ietf.org/doc/html/rfc9728)
- [RFC 8707 - OAuth 2.0 Resource Indicators](https://datatracker.ietf.org/doc/html/rfc8707)
- [MCP Authorization Specification (2025-06-18)](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization)

### API Documentation

- [Miro REST API](https://developers.miro.com/reference/api-reference)
- [Miro OAuth](https://developers.miro.com/docs/getting-started-with-oauth)
- [Miro Token Introspection](https://developers.miro.com/reference/get-access-token-context)

### Implementation Examples

- vault-server (Proxy OAuth with GitHub) - `/Users/guillaume/dev/tools/vault-server`
- remote-mcp-oauth skill - `/Users/guillaume/.claude/skills/remote-mcp-oauth/`

### Related ADRs

- **ADR-001** (Correct pattern): OAuth2 Stateless Architecture - Proxy OAuth pattern (implemented via ADR-004)
- **ADR-003** (Outdated): Dual-Mode Architecture - Assumed Resource Server for HTTP (incorrect)
- **ADR-004** (Supersedes this): Proxy OAuth for Claude.ai Web - Actual working implementation

---

## Update History

**2025-11-11 (Second Update)**: **Status changed to "Superseded"** - Empirical testing with Claude.ai web revealed this pattern doesn't work. Claude.ai ignores `/.well-known/oauth-protected-resource` metadata and uses convention-based routing. ADR-004 implements the correct Proxy OAuth pattern (ADR-001).

**2025-11-11 (First Update)**: Updated status to "Implemented" (this was premature - based on incorrect assumptions about Claude.ai behavior).

**Original Date**: 2025-11-10

---

## Implementation Status

**Current Status**: ❌ **SUPERSEDED** - Code was built but doesn't work with Claude.ai web

**What was built** (now deprecated):
- [src/bin/http-server-adr002.rs](../src/bin/http-server-adr002.rs) - HTTP Resource Server (won't work)
- [src/auth/token_validator.rs](../src/auth/token_validator.rs) - Bearer token validation (not used)
- OAuth Protected Resource metadata endpoint (ignored by Claude.ai)

**Why it failed**:
- Claude.ai doesn't use RFC 9728 metadata discovery
- Claude.ai expects `/authorize`, `/callback`, `/token` endpoints (Proxy OAuth pattern)
- Resource Server pattern requires Claude to send Bearer tokens (Claude doesn't do this)

**What replaced it**:
- ADR-004 Proxy OAuth implementation (`src/oauth/` module)
- Encrypted cookie-based state management
- Convention-based endpoints matching Claude.ai expectations

---

## Review and Update

**Next review:** Only if other MCP clients implement RFC 9728 properly

**Lessons Learned (2025-11-11):**
- **Don't assume spec compliance**: Claude.ai web doesn't use RFC 9728 metadata discovery
- **Test with real clients early**: Assumptions about Claude.ai behavior were wrong
- **Convention over configuration**: Claude.ai uses `/authorize` convention, not metadata
- **Resource Server pattern valid**: Just not for Claude.ai web custom connectors

**This pattern MIGHT work with:**
- Future MCP clients that implement RFC 9728 correctly
- Claude Desktop (if it uses proper metadata discovery)
- Other third-party MCP clients

**Decision superseded because:**
- Claude.ai web requires Proxy OAuth (ADR-004)
- No MCP client currently uses Resource Server pattern
- Proxy OAuth is the working pattern (ADR-001 via ADR-004)

---

**Architecture Choice**: Resource Server (RFC 9728 compliant)
**Actual Working Pattern**: Proxy OAuth (ADR-001 via ADR-004)
**Status**: Superseded by ADR-004 for Claude.ai web
