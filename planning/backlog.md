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
  - Note: Implemented but superseded by ADR-002 (Resource Server pattern)
- [x] AUTH6: Claude discovers OAuth via metadata endpoint (vs manual configuration) ✅ 2025-11-10
- [x] AUTH7: Server extracts Bearer tokens from Authorization header (vs cookies) ✅ 2025-11-10

## In Progress

## Blocked
- [🚫] LAYER1.1: User controls z-order stacking (bring to front, send to back) ⚠️ Web SDK only
- [🚫] LAYER1.2: User manages organizational layers (visibility, locking) ⚠️ UI-only feature

## Planned

### Critical (ADR-002 Implementation - Resource Server Pattern)

- [ ] **AUTH8**: Server validates tokens with Miro introspection API (vs trusting Claude)
  - **Outcome**: User identity known for audit logging and security
  - **Acceptance Criteria**:
    - Call `GET https://api.miro.com/v1/oauth-token` with Bearer token
    - Parse user info (user_id, team_id, scopes)
    - Return 401 for invalid/expired tokens
    - Log user_id for each request
    - Test with valid/invalid/expired tokens
  - **Dependencies**: AUTH7
  - **Complexity**: 1.0 (API integration)

- [ ] **AUTH9**: Token validation cached with 5-minute TTL (vs 100ms latency per request)
  - **Outcome**: 95% reduction in validation latency (100ms → <1ms)
  - **Acceptance Criteria**:
    - Implement LRU cache (100 token capacity)
    - Cache validated user info for 5 minutes
    - First request: validate with Miro (cache miss)
    - Subsequent requests: serve from cache (cache hit)
    - Test cache hit/miss scenarios
    - Measure latency improvement
  - **Dependencies**: AUTH8
  - **Complexity**: 1.5 (caching logic)

### High Priority (Production Readiness - Scaleway Containers)

- [ ] **DEPLOY2**: System deploys to Scaleway Containers successfully
  - **Outcome**: Production infrastructure configured for stateless container architecture
  - **Acceptance Criteria**:
    - Create Dockerfile for Rust MCP server
    - Configure container with persistent in-memory LRU cache
    - Deploy to Scaleway Containers with HTTPS
    - Verify HTTPS certificate auto-configuration
    - Test OAuth2 redirect URI (https://[container-name].containers.scw.cloud/...)
    - Validate container responds to MCP protocol requests
  - **Dependencies**: AUTH8, AUTH9 (token validation with caching complete)
  - **Complexity**: 1.5 (platform deployment)
  - **Note**: Platform choice made 2025-11-10 (see framing.md); Containers selected for LRU cache persistence

- [ ] **SEC1**: Developer configures secrets securely via Scaleway Secret Manager
  - **Outcome**: Sensitive credentials isolated from application code and logs
  - **Acceptance Criteria**:
    - Store MIRO_CLIENT_SECRET in Secret Manager
    - Store TOKEN_ENCRYPTION_KEY in Secret Manager
    - Configure function to access secrets at runtime via environment injection
    - Verify secrets never logged or exposed in function output
    - Document secret rotation procedure
    - Test secret access from cold-start function
  - **Dependencies**: DEPLOY2 (functions infrastructure ready)
  - **Complexity**: 1.0 (secret management setup)

- [ ] **TEST2**: Stateless authentication verified through comprehensive integration tests
  - **Outcome**: Prevent regressions in security-critical stateless cookie implementation
  - **Acceptance Criteria**:
    - Test PKCE validation (wrong verifier rejected)
    - Test state validation (CSRF attack blocked)
    - Test expired state (10-min timeout enforced)
    - Test expired access token (1-hour refresh)
    - Test cold start simulation (state persists in cookies)
    - Test concurrent auth flows (no state collision)
  - **Dependencies**: AUTH4, AUTH5
  - **Complexity**: 1.5 (complex security test scenarios)

### Medium Priority (Operational Excellence)

- [ ] **OBS1**: Developer monitors production OAuth2 flow via Scaleway Cockpit
  - **Outcome**: Audit trail and debugging capability for authentication events
  - **Acceptance Criteria**:
    - Implement structured logging for auth events (initiate, callback, refresh, errors)
    - Log session/request IDs for correlation across function invocations
    - Configure Cockpit log collection from Serverless Functions
    - Create Cockpit queries for: failed auth, token refresh rate, error patterns
    - Document emergency debugging procedures (e.g., trace failed auth by user)
    - Test log visibility during OAuth2 flow (authorize → callback → token use)
  - **Dependencies**: DEPLOY2 (Cockpit available for functions)
  - **Complexity**: 1.0 (observability setup)

### Documentation

- [ ] **DOC1**: Developer understands stateless OAuth2 pattern through comprehensive documentation
  - **Outcome**: Future maintainers grasp architecture and security trade-offs
  - **Acceptance Criteria**:
    - Document Pattern B architecture (PKCE + encrypted cookies)
    - Explain why stateless vs database (ADR-001 rationale)
    - Document cookie encryption implementation
    - Provide example flows (authorization, callback, token use)
    - Document migration path to database (if needed >100 users)
    - Link to ADR-001 and industry references
  - **Dependencies**: AUTH4, AUTH5 (implementation complete)
  - **Complexity**: 0.5 (documentation only)

- [ ] **DOC2**: Developer understands Scaleway Functions deployment and operations
  - **Outcome**: Clear deployment documentation for production maintenance
  - **Acceptance Criteria**:
    - Document Scaleway Functions deployment steps (from source to running function)
    - Explain stateless architecture compatibility (ADR-001 Pattern B + Functions)
    - Document secret management via Scaleway Secret Manager
    - Provide Cockpit query examples for troubleshooting auth issues
    - Document cold start mitigation strategies (if needed)
    - Include cost monitoring guidance ($1-5/month target per framing.md)
    - Link to Scaleway Functions best practices
  - **Dependencies**: DEPLOY2, SEC1, OBS1 (production stack complete)
  - **Complexity**: 0.5 (documentation only)
