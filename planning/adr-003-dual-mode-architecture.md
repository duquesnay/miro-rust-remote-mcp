# ADR-003: Dual-Mode MCP Architecture (HTTP + stdio)

**Status:** Outdated (Superseded by ADR-004 for HTTP mode)
**Date:** 2025-11-11
**Context:** Miro MCP Server supporting both HTTP remote and stdio local deployment
**Decision Makers:** Solution Architect, Security Specialist

> **Critical Update (2025-11-11)**: This ADR assumed HTTP mode would use Resource Server pattern (ADR-002). Empirical testing proved this wrong - Claude.ai web requires Proxy OAuth pattern (ADR-001 via ADR-004). The stdio mode analysis remains valid, but HTTP mode architecture documented here is incorrect. See ADR-004 for actual HTTP mode implementation.

---

## Executive Summary

**What we're building**: Miro MCP server with TWO deployment modes
- **HTTP mode (Primary)**: Remote Proxy OAuth for Claude.ai web (**Updated** - was incorrectly described as Resource Server)
- **stdio mode (Optional)**: Local OAuth Client for Claude Desktop

**Key Decision**: Different architectures require different dependencies
- HTTP mode: ~~NO token storage → NO ring dependency~~ **CORRECTION**: Needs ring for encrypted cookies (Proxy OAuth pattern)
- stdio mode: Persistent tokens → NEEDS encryption (OS keychain OR ring)

**Critical Correction (2025-11-11)**: HTTP mode DOES need `ring` dependency for encrypted cookie state management. The Resource Server assumption (no persistent state) was wrong. HTTP mode uses Proxy OAuth (ADR-004), which requires encrypted cookies.

---

## The Two Architectures

### HTTP Mode (Primary - ~~ADR-002~~ ADR-004 Pattern)

**Use Case**: Claude.ai web interface accessing remote MCP server

```
User → Claude.ai Web
         ↓
Claude Platform (handles OAuth with Miro)
         ↓
Bearer token
         ↓
Your HTTP MCP Server (validates token, proxies to Miro)
         ↓ (GET https://api.miro.com/v1/oauth-token)
Miro API (validates token, returns user info)
         ↓
Cache result in-memory (LRU, 5min TTL)
         ↓
Use token to proxy Miro API calls
```

**Architecture** (**OUTDATED** - see ADR-004):
- ~~**Role**: OAuth Resource Server (RFC 9728)~~ **WRONG** - Actually Proxy OAuth
- ~~**Token source**: Received as Bearer header from Claude~~ **WRONG** - Server obtains tokens from Miro
- ~~**Token storage**: In-memory LRU cache ONLY (5-minute TTL)~~ **WRONG** - Encrypted cookies
- ~~**Encryption**: NOT needed~~ **WRONG** - Needs ring for cookie encryption

**Actual Architecture (ADR-004)**:
- **Role**: OAuth Proxy (Authorization Code Flow)
- **Token source**: Direct OAuth with Miro
- **Token storage**: Encrypted cookies (60-day TTL)
- **Encryption**: Required (ring for AES-GCM)

**Dependencies**:
```toml
tokio = "1.42"          # Async runtime
axum = "0.7"            # HTTP server
reqwest = "0.12"        # HTTP client for Miro API
lru = "0.12"            # In-memory cache
# NO ring needed - no persistent storage
```

**Binary**: `http-server-adr002` (no required-features)

---

### stdio Mode (Optional - Local Client)

**Use Case**: Claude Desktop running MCP server as local subprocess

```
User → Claude Desktop
         ↓
Spawns: miro-mcp-server (stdio)
         ↓
First run: OAuth flow with Miro
         ↓ (browser opens miro.com/oauth/authorize)
User authorizes
         ↓ (callback to localhost:3000/oauth/callback)
Server receives access + refresh tokens
         ↓
PERSIST tokens to disk (encrypted or OS keychain)
         ↓
Server dies when Claude Desktop closes
         ↓
Next run: Load tokens from storage
         ↓
Refresh if expired
         ↓
Use tokens for Miro API calls
```

**Architecture**:
- **Role**: OAuth Client (Authorization Code Flow)
- **Token source**: Direct OAuth with Miro
- **Token storage**: PERSISTENT (survives process restarts)
- **Token lifecycle**: Long-lived (refresh token valid 60 days)
- **Encryption**: REQUIRED (tokens at rest on disk)

**Dependencies**:
```toml
rmcp = "0.7"            # MCP protocol (stdio transport)
oauth2 = "4.4"          # OAuth client implementation
base64 = "0.22"         # Encoding

# TOKEN STORAGE (choose one):
# Option A (preferred):
keyring = "2.0"         # OS keychain (macOS/Windows/Linux)

# Option B (if keyring unavailable):
ring = "0.17"           # Encryption for file storage
```

**Binary**: `miro-mcp-server` (requires stdio-mcp feature)

---

## Why Different Dependencies?

### HTTP Mode: No ring needed

**Question**: "Why doesn't HTTP mode need ring?"

**Answer**: No persistent storage = no encryption needed

```rust
// HTTP server state
pub struct AppState {
    validator: Arc<MiroTokenValidator>,  // Has in-memory LRU cache
    miro_client: Arc<MiroClient>,
}

// Cache is just a HashMap in memory
struct TokenValidator {
    cache: Mutex<LruCache<String, CachedUserInfo>>,  // Lost on restart - OK!
}
```

**On server restart**:
- Cache cleared ✓
- Next request cache miss → validates with Miro ✓
- Cache rebuilds ✓
- **No problem**: Claude resends Bearer token every request

---

### stdio Mode: Encryption required

**Question**: "Why does stdio mode need ring/keyring?"

**Answer**: Persistent storage = encryption required

```rust
// stdio server needs to survive restarts
struct TokenStore {
    // MUST persist across process restarts
    access_token: String,   // Valid 1 hour
    refresh_token: String,  // Valid 60 days
    expires_at: DateTime,
}

// Problem: Claude Desktop kills/restarts server process
// Solution: Store tokens on disk
// Security: MUST encrypt tokens at rest

// Option A: OS Keychain (preferred)
fn save_tokens(store: &TokenStore) -> Result<()> {
    let keyring = Entry::new("miro-mcp", "oauth-tokens")?;
    keyring.set_password(&serde_json::to_string(store)?)?;
    Ok(())
}

// Option B: Encrypted file
fn save_tokens_encrypted(store: &TokenStore) -> Result<()> {
    let key = derive_key(); // From PASSWORD or env var
    let encrypted = encrypt_with_ring(key, &serde_json::to_string(store)?)?;
    fs::write("~/.config/miro-mcp/tokens.enc", encrypted)?;
    Ok(())
}
```

**On server restart**:
- Process dies ✗
- Tokens lost from memory ✗
- Must reload from storage ✓
- **Requires**: Persistent encrypted storage

---

## Architectural Comparison Table

| Aspect | HTTP Mode (Primary) | stdio Mode (Optional) |
|--------|-------------------|---------------------|
| **Deployment** | Remote (Scaleway Containers) | Local (Claude Desktop subprocess) |
| **MCP Transport** | HTTP | stdio |
| **OAuth Role** | Resource Server | OAuth Client |
| **OAuth Handler** | Claude Platform | MCP Server itself |
| **Token Source** | Bearer header (per request) | Direct OAuth with Miro |
| **Token Storage** | In-memory cache (5min) | Persistent (OS keychain/file) |
| **Encryption** | Not needed | Required (ring or keyring) |
| **Server Lifecycle** | Long-lived (days/weeks) | Short-lived (Claude Desktop session) |
| **ring Dependency** | ✗ No | ✓ Yes (or keyring) |
| **Compilation Time** | Fast (~30s) | Slower (~2min if using ring) |
| **Complexity** | Simple (150 LOC) | Complex (500 LOC OAuth flow) |
| **Security** | Stateless validation | Encrypted token storage |

---

## Implementation Status

### What We've Built (HTTP Mode)

**Files**:
- ✅ `src/bin/http-server-adr002.rs` - HTTP Resource Server
- ✅ `src/auth/token_validator.rs` - Bearer token validation + LRU cache
- ✅ `src/auth/types.rs` - Auth types (UserInfo, AuthError)
- ✅ `src/miro/client.rs` - Miro API client (boards, items)
- ✅ `tests/integration/test_local_http.rs` - Integration tests

**Architecture**: Resource Server with in-memory caching (ADR-002)

**Dependencies**: No ring (not needed for in-memory cache)

---

### What We Haven't Built (stdio Mode)

**Files** (would need to create):
- ⬜ `src/bin/miro-mcp-server.rs` - stdio MCP server
- ⬜ `src/auth/oauth_client.rs` - OAuth Authorization Code Flow
- ⬜ `src/auth/token_storage.rs` - Persistent token storage (encrypted)
- ⬜ OAuth callback handler (localhost HTTP server)

**Architecture**: OAuth Client with persistent tokens (ADR-001 pattern)

**Dependencies**: Would need ring OR keyring for encryption

**Status**: Optional feature, not currently prioritized

---

## Cargo.toml Configuration

```toml
[dependencies]
# Core (always needed)
tokio = { version = "1.42", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
anyhow = "1.0"

# HTTP mode (primary)
axum = "0.7"
lru = "0.12"            # In-memory cache only

# stdio mode (optional feature)
rmcp = { version = "0.7", optional = true }
oauth2 = { version = "4.4", optional = true }
ring = { version = "0.17", optional = true }    # For encrypted file storage
# OR
keyring = { version = "2.0", optional = true }  # For OS keychain (better)

[features]
stdio-mcp = ["rmcp", "oauth2", "ring"]  # Or ["rmcp", "oauth2", "keyring"]
default = []  # HTTP mode by default (no stdio)

[[bin]]
name = "http-server-adr002"
path = "src/bin/http-server-adr002.rs"
# No required-features - builds with core deps only

[[bin]]
name = "miro-mcp-server"
path = "src/bin/miro-mcp-server.rs"
required-features = ["stdio-mcp"]  # Only builds when feature enabled
```

**Current choice**: Default to HTTP mode (no stdio feature), which means **no ring dependency** for primary use case.

---

## Security Considerations

### HTTP Mode (Stateless)

**Threat Model**:
- ✅ Token theft in transit → Mitigated by HTTPS
- ✅ Invalid token → Validated with Miro API every request (with caching)
- ✅ Revoked token → Detected within 5 minutes (cache TTL)
- ✅ Server compromise → No tokens persist (in-memory only)

**Security Properties**:
- No persistent secrets (cache cleared on restart)
- Token validation with Miro API (authoritative)
- Audit trail (logs user_id from validated token)

---

### stdio Mode (Stateful)

**Threat Model**:
- ✅ Token theft from disk → Mitigated by encryption (ring/keyring)
- ✅ Token theft from memory → Process isolation (OS responsibility)
- ✅ Refresh token leak → 60-day validity window (Miro limit)
- ⚠️ Encryption key compromise → Tokens exposed

**Security Properties**:
- Tokens encrypted at rest (ring or OS keychain)
- OAuth PKCE (prevents code interception)
- Refresh token rotation (new refresh on each use)
- Secure storage location (`~/.config/miro-mcp/` with 0600 permissions)

**Why keyring > ring**:
- OS keychain uses hardware security (Secure Enclave on macOS)
- No custom crypto (less attack surface)
- Platform-managed key derivation
- Faster compilation

---

## Decision Summary

### HTTP Mode (Primary)

**Status**: ✅ Implemented and tested

**Rationale**:
- Primary use case: Claude.ai web interface
- Simpler architecture (Resource Server pattern)
- No token storage = no encryption needed
- Fast compilation (no ring dependency)
- Stateless = scales horizontally

**Trade-offs**:
- Requires HTTPS deployment (Scaleway Containers)
- Depends on Claude Platform OAuth

---

### stdio Mode (Optional)

**Status**: ⬜ Not implemented (optional feature)

**Rationale**:
- Secondary use case: Claude Desktop users
- Would enable local MCP server
- Requires OAuth client implementation
- Needs persistent token storage
- Would add ring/keyring dependency

**Trade-offs**:
- Slower compilation (ring is heavy)
- More complex (OAuth flow + token storage)
- Higher maintenance burden
- Not needed for primary use case

**Decision**: Defer implementation until needed

---

## Why This Confusion Happened

**Root Cause**: Two separate ADRs with different patterns created confusion

- **ADR-001**: Documented Proxy OAuth (stdio pattern) - Valid for stdio mode
- **ADR-002**: Documented Resource Server (HTTP pattern) - Implemented for HTTP mode
- **Cargo.toml**: Had optional `stdio-mcp` feature with ring
- **Confusion**: Unclear which pattern applies to which deployment mode

**Resolution**: This ADR clarifies:
1. **Two modes exist** with different architectures (both valid)
2. **HTTP mode** (primary) uses ADR-002 - no ring needed
3. **stdio mode** (optional) would use ADR-001 - needs ring/keyring
4. **Current focus** is HTTP mode only (ADR-002 implemented)
5. **ADR-001 and ADR-002** restored with clarified scope

---

## Files Updated

**Restored and clarified**:
- ✅ `planning/adr-001-oauth2-stateless-architecture.md` → Scope: stdio mode only (not implemented)
- ✅ `planning/adr-002-oauth-resource-server-architecture.md` → Scope: HTTP mode (implemented)

**Created**:
- ✅ This file: `planning/adr-003-dual-mode-architecture.md` → Comparison and clarification

**Status**:
- ADR-001: Valid for stdio mode, deferred implementation
- ADR-002: Valid for HTTP mode, currently implemented
- ADR-003: Clarifies relationship between the two

---

## Consequences

### Positive

- ✅ Clear architectural separation (HTTP vs stdio)
- ✅ Dependency choice explained (ring only for stdio)
- ✅ Primary use case (HTTP) has fast compilation
- ✅ Optional feature (stdio) documented but not mandatory
- ✅ No confusion about when ring is needed

### Negative

- ⚠️ stdio mode not implemented (acceptable - not primary use case)
- ⚠️ Two architectures to maintain (if stdio added later)

### Neutral

- Optional stdio-mcp feature remains in Cargo.toml (harmless)
- Can be removed if never implementing stdio mode

---

## References

**Standards**:
- [RFC 9728 - OAuth 2.0 Protected Resource Metadata](https://datatracker.ietf.org/doc/html/rfc9728)
- [MCP Specification - stdio transport](https://modelcontextprotocol.io/specification/basic/transports#stdio)
- [MCP Specification - HTTP transport](https://modelcontextprotocol.io/specification/basic/transports#http-with-sse)

**API Documentation**:
- [Miro REST API](https://developers.miro.com/reference/api-reference)
- [Miro OAuth](https://developers.miro.com/docs/getting-started-with-oauth)

**Related ADRs**:
- **ADR-001** (Correct pattern): OAuth2 Stateless Architecture - Proxy OAuth pattern (implemented via ADR-004)
- **ADR-002** (Superseded): OAuth Resource Server Architecture - Doesn't work with Claude.ai web
- **ADR-003** (This document - OUTDATED): Dual-mode architecture based on incorrect assumptions
- **ADR-004** (Actual implementation): Proxy OAuth for Claude.ai Web - Corrects this ADR's HTTP mode analysis

---

## Quick Reference

**"Why doesn't HTTP mode need ring?"**
→ No persistent storage (in-memory cache only)

**"Why does stdio mode need ring?"**
→ Persistent token storage requires encryption

**"Which mode are we building?"**
→ HTTP mode (primary), stdio mode (optional, deferred)

**"Should I remove the stdio-mcp feature?"**
→ Optional: harmless if left, can remove if never using

**"What's the compile time difference?"**
→ HTTP mode: ~30s | stdio mode: ~2min (ring is slow)

---

**Relationship to Other ADRs**:
- ~~**Clarifies** ADR-001 (applies to stdio mode only)~~ **WRONG** - ADR-001 applies to HTTP mode too (via ADR-004)
- ~~**Clarifies** ADR-002 (applies to HTTP mode only)~~ **WRONG** - ADR-002 doesn't work for HTTP mode
- **Superseded by** ADR-004 for HTTP mode architecture

**Key Learning (2025-11-11)**:
- This ADR assumed Resource Server pattern for HTTP mode - WRONG
- Claude.ai web requires Proxy OAuth pattern (ADR-001 via ADR-004)
- stdio mode analysis remains valid
- HTTP mode dependency analysis was wrong (does need ring)

**Next Review**: Mark as historical reference only - ADR-004 is the authoritative HTTP mode documentation
