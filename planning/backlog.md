# Miro MCP Server - Product Backlog

## Completed
- [x] AUTH1: User authenticates with Miro securely via OAuth2 ✅ 2025-11-10
- [x] AUTH2: System refreshes tokens automatically (vs manual re-auth) ✅ 2025-11-10
- [x] BOARD1: User lists accessible Miro boards programmatically ✅ 2025-11-10
- [x] BOARD2: User creates new boards via Claude prompts ✅ 2025-11-10
- [x] VIS1: User creates sticky notes with custom content and styling ✅ 2025-11-10
- [x] VIS2: User creates shapes for organizational structure ✅ 2025-11-10
- [x] VIS3: User creates text elements on boards ✅ 2025-11-10
- [x] VIS4: User creates frames for grouping related content ✅ 2025-11-10
- [x] ITEM1: User lists board items filtered by type ✅ 2025-11-10
- [x] ITEM2: User updates item properties dynamically ✅ 2025-11-10
- [x] ITEM3: User removes items from boards ✅ 2025-11-10
- [x] REL1: User connects items with styled arrows/lines ✅ 2025-11-10
- [x] REL2: User adds captions to connectors ✅ 2025-11-10
- [x] BULK1: User creates multiple items efficiently (vs individual API calls) ✅ 2025-11-10
- [x] TECH1: MCP server responds to protocol requests (vs crashing on tools/list) ✅ 2025-11-10
- [x] LAYER2: User understands item stacking order when reading/creating items ✅ 2025-11-10
- [x] FRAME1: User creates items directly in frames (vs manual move after creation) ✅ 2025-11-10
- [x] FRAME2: User moves items between frames for reorganization ✅ 2025-11-10
- [x] FRAME3: User filters items by containing frame ✅ 2025-11-10
- [x] FRAME4: User removes items from frames to board root ✅ 2025-11-10
- [x] TECH2: Developer modifies parent construction in single location (vs 5 duplications) ✅ 2025-11-10
- [x] TEST1: Parent filtering verified through integration tests (vs unit-only coverage) ✅ 2025-11-10
- [x] TECH4: System validates sort_by values explicitly (vs silent failures) ✅ 2025-11-10
- [x] DEPLOY1: Developer deploys to Scaleway Containers in <5min (vs manual local setup) ✅ 2025-11-10
  - Note: Implemented Containers; Functions deployment tracked in DEPLOY2
- [x] CI1: Developer receives automated test feedback on every push (vs manual local testing) ✅ 2025-11-10
- [x] TECH3: Developer adds complex items via builder pattern (vs 9-parameter functions) ✅ 2025-11-10
- [x] TECH5: Developer adds new tools without modifying routing (vs hardcoded match) ✅ 2025-11-10
- [x] AUTH3: User completes OAuth flow in browser from Claude Desktop (vs manual token management) ✅ 2025-11-10
- [x] AUTH4: Developer adds OAuth state via encrypted cookies (vs in-memory HashMap) ✅ 2025-11-10
- [x] AUTH5: User's access token stored in encrypted cookies (vs server-side storage) ✅ 2025-11-10
  - Note: Implemented but superseded by ADR-003 (HTTP Resource Server mode)
- [x] AUTH6: Claude discovers OAuth via metadata endpoint (vs manual configuration) ✅ 2025-11-10
- [x] AUTH7: Server extracts Bearer tokens from Authorization header (vs cookies) ✅ 2025-11-10
- [x] AUTH8: Server validates tokens with Miro introspection API (vs trusting Claude) ✅ 2025-11-10
- [x] AUTH9: Token validation cached with 5-minute TTL (vs 100ms latency per request) ✅ 2025-11-10
- [x] PROTO1: Claude.ai connects via MCP protocol (vs REST-only server) ✅ 2025-11-10
- [x] DEPLOY2: System deploys to Scaleway Containers successfully ✅ 2025-11-10
  - Note: Deployed with MCP protocol support; Container: miro-mcp at flyagileapipx8njvei-miro-mcp.functions.fnc.fr-par.scw.cloud
- [x] OBS1: Developer diagnoses production auth failures through structured logging ✅ 2025-11-11
  - Note: Implemented correlation IDs, structured logging with JSON format, and emergency debugging runbook (DEBUGGING.md)

## In Progress

⚠️ **CONTEXT NEEDED**: User mentioned "3 P0 blockers in progress" but no items marked [⏳] below.
Please clarify which items are actively being worked on.

## Blocked

⚠️ **RESOLUTION NEEDED**: Blocked items require explicit decision (DROP/DEFER/WORKAROUND)

- [🚫] LAYER1.1: User controls z-order stacking (bring to front, send to back)
  - **Blocker**: Web SDK only feature, not available via REST API
  - **Resolution Options**:
    - DROP: Out of scope for MCP server (API limitation)
    - DEFER: Wait for Miro API v2 support
    - WORKAROUND: Delete and recreate items in desired order
  - **Recommendation**: DROP (API limitation, low user impact)

- [🚫] LAYER1.2: User manages organizational layers (visibility, locking)
  - **Blocker**: UI-only feature, not exposed via REST API
  - **Resolution Options**:
    - DROP: Out of scope for MCP server (UI-only)
    - DEFER: Wait for Miro API expansion
  - **Recommendation**: DROP (UI-centric feature, not programmatic use case)

## Planned

### 🔄 Architectural Change: ADR-002 → ADR-004 (Proxy OAuth Pattern)

**Context**: ADR-002 (HTTP Resource Server pattern) implemented but does not work with Claude.ai web interface.
- **Problem**: Claude.ai web cannot complete OAuth flow with current implementation
- **Root Cause**: Resource Server pattern requires Claude to handle OAuth → Claude.ai doesn't support this
- **Decision**: Switch to Proxy OAuth pattern (ADR-004) where OUR server proxies OAuth between Claude.ai and Miro
- **Impact**: 7 new implementation tasks (AUTH10-14, TEST3, DEPLOY4) + 4 file modifications
- **Previous Work**: AUTH3-9 provided foundation (cookie management, metadata endpoint, token validation)
  - AUTH4/AUTH5 (cookie encryption) remains useful for state management
  - AUTH6-9 (metadata, token validation) gets modified for proxy pattern

**New Implementation Tasks Below** (AUTH10-14, TEST3, DEPLOY4)

---

### Production Readiness Summary

**Timeline to Production**: 2-3 days (Proxy OAuth implementation + validation + deployment)

**Critical Path**:
1. **AUTH10-14**: Implement Proxy OAuth pattern (5 tasks) → P0 blockers
2. **TEST3**: Validate OAuth flow end-to-end with Claude.ai → P0 blocker
3. **DEPLOY4**: Secure secrets in Scaleway Secret Manager → P0 blocker (replaces SEC1)
4. Final deployment validation

**Technical Investment Ratio**: 38% (5 technical / 13 total planned items)
- **New OAuth implementation**: AUTH10-14 (5 items, complexity 2.0+1.5+1.0+0.5+0.5 = 5.5)
- **Testing/validation**: TEST2, TEST3 (2 items, complexity 1.0+1.0 = 2.0)
- **Deployment/security**: DEPLOY3, DEPLOY4 (2 items, complexity 0.5+1.0 = 1.5)
- **Technical optimization**: TECH6 (1 item, complexity 0.5)
- **Documentation**: DOC1, DOC2 (2 items, complexity 0.5+0.5 = 1.0)
**Zone Status**: 🟡 Yellow - Higher technical investment due to architectural pivot, but justified by production blocker
**Note**: Technical ratio elevated due to ADR-004 implementation, not scope creep. OAuth foundation work critical for production.

---

### 🚨 P0 Production Blockers - OAuth Proxy Implementation (MUST complete before production deployment)

**Context**: Current ADR-002 implementation doesn't work with Claude.ai web. Switching to ADR-004 Proxy OAuth pattern.

#### OAuth Core Implementation

- [ ] **AUTH10**: Developer implements OAuth proxy module with Miro integration
  - **Current State**: No OAuth flow implementation (ADR-002 assumed Claude handled it)
  - **Target State**: Complete OAuth proxy between Claude.ai and Miro with PKCE
  - **Value**: Enables Claude.ai web integration (vs current non-functional state)
  - **Acceptance Criteria**:
    - Create `src/oauth/proxy_provider.rs` with `MiroOAuthProvider` struct
    - Implement authorization URL generation with PKCE code challenge
    - Implement token exchange (authorization code → access token)
    - Implement token refresh flow with Miro API
    - Handle Miro-specific scopes (boards:read, boards:write)
    - Return tokens in RFC 6749 format for Claude.ai
    - Add comprehensive error handling for OAuth failures
  - **Dependencies**: None (new module)
  - **Complexity**: 2.0 (OAuth flow implementation)
  - **Priority**: P0 - Current implementation doesn't work

- [ ] **AUTH11**: Developer implements OAuth HTTP endpoints for Claude.ai
  - **Current State**: Server only has MCP endpoints, no OAuth endpoints
  - **Target State**: /oauth/authorize, /oauth/callback, /oauth/token endpoints functional
  - **Value**: Claude.ai can complete OAuth flow (vs redirect failure)
  - **Acceptance Criteria**:
    - Create `src/oauth/endpoints.rs` with three HTTP handlers
    - `/oauth/authorize`: Generate PKCE, redirect to Miro with state cookie
    - `/oauth/callback`: Validate state, exchange code, set token cookie, redirect to Claude.ai
    - `/oauth/token`: Return access token to Claude.ai in RFC 6749 format
    - All endpoints use encrypted cookies for state management (reuse AUTH4 foundation)
    - Wire endpoints to HTTP router in `http_server.rs`
    - Add correlation IDs to all OAuth flows for debugging
  - **Dependencies**: AUTH10 (proxy provider), AUTH12 (cookie manager)
  - **Complexity**: 1.5 (HTTP handlers with cookie management)
  - **Priority**: P0 - Required for OAuth flow

- [ ] **AUTH12**: Developer implements stateless state management with encrypted cookies
  - **Current State**: No state management for OAuth flow (ADR-002 didn't need it)
  - **Target State**: Encrypted cookie-based OAuth state (PKCE verifier, redirect URI)
  - **Value**: Stateless architecture compatible with Scaleway Containers (vs database dependency)
  - **Acceptance Criteria**:
    - Create `src/oauth/cookie_manager.rs` with encryption/decryption utilities
    - Use AES-256-GCM for cookie encryption (reuse AUTH4 crypto patterns)
    - Store: PKCE code_verifier, state nonce, redirect_uri in encrypted cookie
    - Set secure flags: HttpOnly, Secure, SameSite=Lax
    - Implement cookie expiry (5 minutes for OAuth flow)
    - Add `MIRO_ENCRYPTION_KEY` to environment variables
    - Document cookie structure and security properties
  - **Dependencies**: None (utility module, extends AUTH4 foundation)
  - **Complexity**: 1.0 (cookie encryption)
  - **Priority**: P0 - Security requirement

- [ ] **AUTH13**: Developer implements PKCE support for OAuth security
  - **Current State**: No PKCE implementation
  - **Target State**: RFC 7636 compliant PKCE code challenge/verifier
  - **Value**: Protects against authorization code interception attacks
  - **Acceptance Criteria**:
    - Create `src/oauth/pkce.rs` with `generate_pkce_pair()` function
    - Generate cryptographically random code_verifier (43-128 chars, base64url)
    - Compute code_challenge = BASE64URL(SHA256(code_verifier))
    - Use challenge_method = S256
    - Validate verifier in token exchange step
    - Add unit tests for PKCE generation and validation
  - **Dependencies**: None (utility module)
  - **Complexity**: 0.5 (standard PKCE implementation)
  - **Priority**: P0 - Security requirement

- [ ] **AUTH14**: Developer updates OAuth metadata to point to server endpoints
  - **Current State**: Metadata points to Miro endpoints (Resource Server pattern from ADR-002)
  - **Target State**: Metadata points to OUR server endpoints (Proxy OAuth pattern per ADR-004)
  - **Value**: Claude.ai discovers correct OAuth endpoints (vs trying wrong URLs)
  - **Acceptance Criteria**:
    - Update `src/mcp/metadata.rs` `authorization_endpoint` to `https://{BASE_URL}/oauth/authorize`
    - Update `token_endpoint` to `https://{BASE_URL}/oauth/token`
    - Keep `issuer` as "https://miro.com" (actual OAuth provider)
    - Add `BASE_URL` configuration to `config.rs` (from env var)
    - Test metadata endpoint returns correct URLs for deployed environment
    - Update integration tests to verify metadata format
  - **Dependencies**: AUTH10, AUTH11 (endpoints exist)
  - **Complexity**: 0.5 (configuration change)
  - **Priority**: P0 - Discovery requirement

#### OAuth Validation & Security

- [ ] **TEST3**: Developer validates OAuth flow end-to-end with Claude.ai
  - **Current State**: OAuth flow fails (Resource Server not supported by Claude.ai web)
  - **Target State**: Complete OAuth flow from Claude.ai → server → Miro → server → Claude.ai
  - **Value**: Confirms implementation works in production environment
  - **Acceptance Criteria**:
    - Deploy to Scaleway with all AUTH10-14 changes
    - Test from Claude.ai web interface: click "Connect" on MCP server
    - Verify redirect to Miro authorization page
    - Authorize with Miro credentials
    - Verify redirect back to Claude.ai with success message
    - Test MCP tool call (`list_boards`) with obtained token
    - Verify token refresh works after expiry (wait or mock expiry)
    - Document end-to-end flow with screenshots for troubleshooting
  - **Dependencies**: AUTH10-14 (all OAuth components), DEPLOY2 (Scaleway deployment)
  - **Complexity**: 1.0 (end-to-end testing)
  - **Priority**: P0 - Validation requirement

- [ ] **DEPLOY4**: Sensitive credentials protected in Scaleway deployment
  - **Current State**: MIRO_CLIENT_SECRET in environment variables
  - **Target State**: All secrets in Scaleway Secret Manager
  - **Value**: Production security compliance (was SEC1, now more urgent with additional secrets)
  - **Acceptance Criteria**:
    - Add `MIRO_CLIENT_SECRET` to Scaleway Secret Manager
    - Add `MIRO_ENCRYPTION_KEY` to Scaleway Secret Manager (new for AUTH12)
    - Update `deploy_scaleway.sh` to inject secrets at runtime via Scaleway API
    - Verify secrets never appear in logs (even in debug mode)
    - Test OAuth flow with secrets loaded from Secret Manager
    - Document secret rotation procedure for compromise scenarios
    - Test secret access from cold-start container
  - **Dependencies**: AUTH10-14 (OAuth implementation complete)
  - **Complexity**: 1.0 (secret management)
  - **Priority**: P0 - Must complete before production (security)
  - **Note**: Replaces SEC1 with expanded scope (additional encryption key)

#### Legacy Items (Still Relevant)

- [ ] **TEST2**: Claude.ai discovers OAuth capability automatically via metadata endpoint
  - **Current Risk**: OAuth metadata endpoint implemented but not verified with Claude.ai
  - **Target State**: Claude.ai web interface successfully discovers and uses OAuth flow
  - **Value**: Entire integration fails if Claude.ai cannot discover OAuth capability
  - **Acceptance Criteria**:
    - Verify `/.well-known/oauth-protected-resource` accessible from internet (HTTPS)
    - Test Claude.ai web interface discovers OAuth metadata endpoint
    - Confirm Claude.ai initiates OAuth flow automatically (vs manual token input)
    - Validate metadata JSON matches Claude.ai expectations (authorization_endpoint, token_endpoint)
    - Add automated test in `deploy_scaleway.sh` to verify metadata endpoint after deployment
    - Document Claude.ai OAuth discovery process for troubleshooting
  - **Dependencies**: DEPLOY2 (HTTPS endpoint deployed), AUTH14 (metadata updated for proxy pattern)
  - **Complexity**: 1.0 (verification and validation)
  - **Priority**: P0 - Integration failure blocker
  - **Note**: Updated dependency from AUTH6 to AUTH14 (metadata now points to our endpoints)

### Medium Priority (Optimization & Quality)

- [ ] **TECH6**: Developer rebuilds production container in <30s (vs 90s currently)
  - **Current State**: HTTP-only build includes stdio-mcp dependencies (90s compile time, 45MB binary)
  - **Target State**: Optimized feature flags reduce build time 66% and binary size 40%
  - **Value**: Faster deployment iterations during production debugging
  - **Acceptance Criteria**:
    - Measure baseline: cargo build --release (record time and binary size)
    - Change default Cargo feature from `stdio-mcp` to `[]` (no default features)
    - Update deploy_scaleway.sh to build with `--no-default-features --features http-mcp`
    - Measure optimized: cargo build --release --no-default-features --features http-mcp
    - Verify: Build time ≤30s AND binary size ≤27MB
    - Keep `stdio-mcp` feature available for local development
    - Document feature flags in README for different deployment modes
  - **Dependencies**: None (refactoring only)
  - **Complexity**: 0.5 (simple Cargo.toml change)

- [ ] **DEPLOY3**: Developer knows deployment succeeded in <30s (vs manual testing loop)
  - **Current State**: deploy_scaleway.sh completes but requires manual curl testing to verify OAuth endpoint works
  - **Target State**: Deployment script validates critical endpoints automatically, reports success/failure immediately
  - **Value**: Eliminates 5-minute manual testing loop after every deployment
  - **Acceptance Criteria**:
    - Deployment script waits for container to start (poll health endpoint, max 30s timeout)
    - Validate /.well-known/oauth-protected-resource returns HTTP 200 with valid JSON
    - Validate metadata JSON contains required fields (authorization_endpoint, token_endpoint)
    - Script exits with code 0 (success) only if all validations pass
    - Script exits with code 1 (failure) if timeout or validation fails
    - Terminal output shows: "✅ Deployment verified - OAuth endpoint responding" or "❌ Deployment failed - [specific error]"
    - Log deployment URL for easy manual inspection
  - **Dependencies**: DEPLOY2 (deployment script exists)
  - **Complexity**: 0.5 (add verification step to script)

### Documentation (Quality Enablers)

⚠️ **NOTE**: Documentation items reframed as developer capabilities (time-to-understanding metrics)

- [ ] **DOC1**: New developer understands OAuth2 architecture in <30min (vs 4h code tracing)
  - **Current State**: No architecture documentation, requires reading ADR-003 + code + git history to understand design
  - **Target State**: Single README section explains stateless OAuth2 pattern with diagrams and trade-offs
  - **Value**: Onboarding speed for future maintainers or contributors
  - **Acceptance Criteria**:
    - README section "Architecture: Stateless OAuth2 Pattern" added
    - Explains ADR-003 Resource Server pattern (Bearer tokens validated per request)
    - Documents why stateless vs database (no user state, proxy auth compatible)
    - Includes sequence diagram: Claude.ai → Proxy → MCP Server → Miro API
    - Links to ADR-003 for full rationale
    - Test: New developer (unfamiliar with codebase) reads docs and correctly explains pattern in <30min
  - **Dependencies**: AUTH7, AUTH8, AUTH9 (implementation complete)
  - **Complexity**: 0.5 (documentation only)

- [ ] **DOC2**: Developer deploys to production in <10min (vs 45min manual process)
  - **Current State**: No deployment documentation, requires trial-and-error with Scaleway Console + CLI
  - **Target State**: Step-by-step deployment guide with secret management and troubleshooting
  - **Value**: Reduces deployment friction from 45min to 10min for routine updates
  - **Acceptance Criteria**:
    - README section "Deployment: Scaleway Containers" added
    - Documents deploy_scaleway.sh usage and prerequisites
    - Explains secret management via Scaleway Secret Manager (SEC1 implementation)
    - Provides troubleshooting guide: "Container not starting" → check logs via scw CLI
    - Includes cost monitoring section ($1-5/month target per framing.md)
    - Documents cold start behavior (expected <5s for Functions)
    - Test: Developer unfamiliar with Scaleway successfully deploys in <10min following guide
  - **Dependencies**: DEPLOY2, SEC1 (production stack complete)
  - **Complexity**: 0.5 (documentation only)
