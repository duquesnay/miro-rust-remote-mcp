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

## Blocked
- [🚫] LAYER1.1: User controls z-order stacking (bring to front, send to back) ⚠️ Web SDK only
- [🚫] LAYER1.2: User manages organizational layers (visibility, locking) ⚠️ UI-only feature

## Planned

### Production Readiness Summary

**Timeline to Production**: 1-2 days (if P0 blockers addressed)

**Critical Path**:
1. SEC1 (secrets in Secret Manager) → P0 blocker
2. TEST2 (Claude.ai OAuth discovery) → P0 blocker
3. OBS1 (observability setup) → P1 issue
4. Final deployment validation

**Technical Investment Ratio**: ~25% (3 technical capabilities / 12 total items in planned backlog)
**Zone Status**: 🟢 Green - Healthy balance of technical and user-facing work

---

### 🚨 P0 Production Blockers (MUST complete before production deployment)

- [ ] **SEC1**: Sensitive credentials protected from exposure in production environment
  - **Current Risk**: Client secret and encryption keys in environment variables without secure storage
  - **Target State**: Secrets in Scaleway Secret Manager, zero exposure in logs/code
  - **Value**: Prevents unauthorized API access and data breaches
  - **Acceptance Criteria**:
    - MIRO_CLIENT_SECRET stored in Scaleway Secret Manager (not env var)
    - TOKEN_ENCRYPTION_KEY stored in Scaleway Secret Manager (not env var)
    - Function accesses secrets at runtime via secure injection
    - Verify secrets NEVER appear in function logs (even debug mode)
    - Document secret rotation procedure for compromise scenarios
    - Test secret access from cold-start container
  - **Dependencies**: DEPLOY2 (Scaleway infrastructure ready)
  - **Complexity**: 1.0 (secret management setup)
  - **Priority**: P0 - Security vulnerability blocker

- [ ] **TEST2**: Claude.ai discovers OAuth capability automatically via metadata endpoint
  - **Current Risk**: OAuth metadata endpoint implemented but not verified with Claude.ai
  - **Target State**: Claude.ai web interface successfully discovers and uses OAuth flow
  - **Value**: Entire integration fails if Claude.ai cannot discover OAuth capability
  - **Acceptance Criteria**:
    - Verify /.well-known/oauth-protected-resource accessible from internet (HTTPS)
    - Test Claude.ai web interface discovers OAuth metadata endpoint
    - Confirm Claude.ai initiates OAuth flow automatically (vs manual token input)
    - Validate metadata JSON matches Claude.ai expectations (authorization_endpoint, token_endpoint)
    - Add automated test in deploy_scaleway.sh to verify metadata endpoint after deployment
    - Document Claude.ai OAuth discovery process for troubleshooting
  - **Dependencies**: DEPLOY2 (HTTPS endpoint deployed), AUTH6 (metadata endpoint implemented)
  - **Complexity**: 1.0 (verification and validation)
  - **Priority**: P0 - Integration failure blocker

### Medium Priority (Optimization & Quality)

- [ ] **TECH6**: Developer builds faster with optimized Cargo features
  - **Current State**: Default feature includes stdio-mcp dependencies in HTTP mode
  - **Target State**: Minimal binary size and faster compilation for HTTP-only deployments
  - **Value**: Faster CI/CD pipeline and smaller container images
  - **Acceptance Criteria**:
    - Change default Cargo feature from `stdio-mcp` to `[]` (no default features)
    - Keep `stdio-mcp` feature available for local development
    - Update deploy_scaleway.sh to build with `--no-default-features --features http-mcp`
    - Verify binary size reduction (measure before/after)
    - Document feature flags in README for different deployment modes
  - **Dependencies**: None (refactoring only)
  - **Complexity**: 0.5 (simple Cargo.toml change)

- [ ] **DEPLOY3**: Deployment failures detected automatically via metadata validation
  - **Current State**: deploy_scaleway.sh missing OAuth metadata endpoint verification
  - **Target State**: Deployment script validates OAuth capability immediately after deploy
  - **Value**: Catches failed deployments before manual testing
  - **Acceptance Criteria**:
    - Add curl verification step in deploy_scaleway.sh after deployment
    - Validate /.well-known/oauth-protected-resource returns HTTP 200
    - Validate metadata JSON contains required fields (authorization_endpoint, token_endpoint)
    - Fail deployment script if metadata endpoint unreachable or malformed
    - Log deployment URL for easy manual verification
  - **Dependencies**: DEPLOY2 (deployment script exists)
  - **Complexity**: 0.5 (add verification step to script)

### Documentation

- [ ] **DOC1**: Developer understands stateless OAuth2 pattern through comprehensive documentation
  - **Outcome**: Future maintainers grasp architecture and security trade-offs
  - **Acceptance Criteria**:
    - Document Pattern B architecture (PKCE + encrypted cookies)
    - Explain why stateless vs database (ADR-003 rationale)
    - Document cookie encryption implementation
    - Provide example flows (authorization, callback, token use)
    - Document migration path to database (if needed >100 users)
    - Link to ADR-003 and industry references
  - **Dependencies**: AUTH4, AUTH5 (implementation complete)
  - **Complexity**: 0.5 (documentation only)

- [ ] **DOC2**: Developer understands Scaleway Functions deployment and operations
  - **Outcome**: Clear deployment documentation for production maintenance
  - **Acceptance Criteria**:
    - Document Scaleway Functions deployment steps (from source to running function)
    - Explain stateless architecture compatibility (ADR-003 HTTP mode + Functions)
    - Document secret management via Scaleway Secret Manager
    - Provide Cockpit query examples for troubleshooting auth issues
    - Document cold start mitigation strategies (if needed)
    - Include cost monitoring guidance ($1-5/month target per framing.md)
    - Link to Scaleway Functions best practices
  - **Dependencies**: DEPLOY2, SEC1, OBS1 (production stack complete)
  - **Complexity**: 0.5 (documentation only)
