# ADR-005: Resource Server with Claude OAuth (vs Proxy OAuth)

**Date**: 2025-11-11
**Status**: Proposed (supersedes ADR-004)
**Context**: Scaleway deployment investigation, vault-server architecture comparison

---

## Decision

Switch from **Proxy OAuth pattern** (ADR-004) to **true MCP Resource Server pattern** where Claude.ai handles the OAuth flow directly with Miro, and our server only validates tokens.

## Context

### Current State (ADR-004)
We implemented a **Proxy OAuth pattern** where:
- Our MCP server implements OAuth endpoints (`/oauth/authorize`, `/oauth/callback`, `/oauth/token`)
- We handle authorization code exchange with Miro
- We store encrypted tokens in cookies
- Client secret stored securely in our server
- Redirect URI: `https://miro-mcp.fly-agile.com/oauth/callback`

**Complexity**: ~500-1000 LOC for OAuth implementation + encryption + token management

### Discovered Alternative (Resource Server Pattern)
**vault-server** uses Claude's Custom Connector pattern:
- Redirect URI: `https://claude.ai/api/mcp/auth_callback`
- Claude handles the entire OAuth flow with GitHub
- MCP server only validates tokens passed in Authorization headers
- No OAuth endpoints needed on MCP server
- No token storage needed

**Complexity**: ~50-100 LOC for metadata + token validation

### Key Findings

#### 1. Miro Supports External Redirect URIs ✅
- Miro OAuth2 accepts any HTTPS redirect URI (no domain restrictions)
- Claude's callback URL (`https://claude.ai/api/mcp/auth_callback`) can be registered in Miro Developer Portal
- Multiple developers confirm this pattern works successfully

#### 2. MCP OAuth 2.1 Specification Supports This ✅
From [MCP Authorization Spec](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization):

> "MCP servers MUST implement the OAuth 2.0 Protected Resource Metadata (RFC9728) specification to indicate the locations of authorization servers."

**Two patterns supported:**
1. **Authorization Server**: Server implements full OAuth flow (what we did in ADR-004)
2. **Resource Server**: Server delegates OAuth to external provider, validates tokens (what vault-server does)

#### 3. Discovery Flow
```
1. Client → MCP Server (no token)
2. MCP Server → 401 with WWW-Authenticate header pointing to metadata
3. Client → /.well-known/oauth-protected-resource
4. Client discovers Miro's authorization_endpoint and token_endpoint
5. Client (Claude) → Miro OAuth flow
6. Client → MCP Server (with access token)
7. MCP Server validates token → Miro API
```

## Consequences

### What We Eliminate

✅ **Remove these AUTH items from backlog:**
- AUTH10: OAuth proxy module (~200 LOC)
- AUTH11: OAuth HTTP endpoints (~150 LOC)
- AUTH12: Encrypted cookie state management (~100 LOC)
- AUTH13: PKCE implementation (~50 LOC)
- AUTH14: Metadata endpoint updates (already exists, just change URLs)

✅ **Remove infrastructure:**
- `MIRO_CLIENT_SECRET` from our server (Claude uses it)
- `MIRO_ENCRYPTION_KEY` (no token storage)
- Token storage encryption (AES-256-GCM)
- State management cookies

### What We Add (Simpler)

✅ **New backlog items:**
- **OAUTH1**: Implement Protected Resource Metadata endpoint (50 LOC)
- **OAUTH2**: Return 401 with WWW-Authenticate discovery hint (20 LOC)
- **OAUTH3**: Validate Bearer tokens from Claude (80 LOC)
- **OAUTH4**: Register Claude's callback URL in Miro portal (configuration only)

### Architectural Benefits

1. **90% less code** - Eliminate entire auth subsystem
2. **No secrets management** - Claude handles `MIRO_CLIENT_SECRET`
3. **No encryption complexity** - No token storage
4. **Simpler deployment** - HTTPS recommended but OAuth doesn't depend on it
5. **Standard MCP pattern** - Follows OAuth 2.1 Resource Server specification
6. **Better security** - Claude's infrastructure handles token lifecycle

### Trade-offs

**Accept:**
- Tight coupling to Claude's OAuth flow (acceptable for MCP server)
- Less control over token refresh (Claude handles it)
- Requires Claude.ai Pro/Team/Enterprise (user already has this)

**Gain:**
- Dramatic simplification
- Follows MCP best practices
- Proven pattern (vault-server, multiple others)
- Easier maintenance and debugging

## Implementation Impact

### Miro Developer Portal Configuration
**Change redirect URI from:**
```
https://miro-mcp.fly-agile.com/oauth/callback
```

**To:**
```
https://claude.ai/api/mcp/auth_callback
https://claude.com/api/mcp/auth_callback  (future-proof)
```

### Code Changes
**Remove:**
- `src/auth/oauth.rs` - Authorization endpoints
- `src/auth/token_store.rs` - Token storage
- Cookie encryption utilities
- PKCE implementation

**Add:**
- `src/auth/metadata.rs` - Protected Resource Metadata endpoint
- `src/auth/token_validation.rs` - JWT validation (verify audience claim)
- Update WWW-Authenticate headers in middleware

**Modify:**
- `.env.production` - Remove `MIRO_CLIENT_SECRET`, `MIRO_ENCRYPTION_KEY`, `MIRO_REDIRECT_URI`
- `src/config.rs` - Remove encryption key parsing

### Deployment Changes
**Scaleway Secret Manager:**
- Remove `MIRO_CLIENT_SECRET` (not needed on our server)
- Remove `MIRO_ENCRYPTION_KEY` (no token storage)

**Environment Variables:**
- Keep: `MIRO_CLIENT_ID` (for API calls)
- Keep: `BASE_URL` (for metadata endpoint)
- Remove: `MIRO_REDIRECT_URI`, secrets

## Timeline

**Estimated Effort**: 0.5-1 day (vs 2-3 days remaining for ADR-004 implementation)

**Approach**: Create feature branch `feat/resource-server-pattern` and implement cleanly, then merge to replace ADR-004 work.

## References

- [MCP Authorization Specification](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization)
- [RFC 9728 - OAuth 2.0 Protected Resource Metadata](https://datatracker.ietf.org/doc/html/rfc9728)
- [RFC 8414 - OAuth 2.0 Authorization Server Metadata](https://datatracker.ietf.org/doc/html/rfc8414)
- vault-server implementation example
- [Miro OAuth Documentation](https://developers.miro.com/docs/getting-started-with-oauth)

## Decision Rationale

**Why change now?**
1. Discovered simpler proven pattern during deployment investigation
2. ADR-004 not yet complete - can pivot with minimal sunk cost
3. 90% code reduction justifies refactor
4. Aligns with MCP specification best practices

**Why not earlier?**
- Lack of awareness of MCP Resource Server pattern
- No reference implementation (vault-server comparison revealed it)
- ADR-004 seemed like only path to Claude.ai integration

## Approval

- [ ] Architecture review
- [ ] Security review (simpler = fewer attack surfaces)
- [ ] Implementation plan approved
- [ ] Backlog updated
