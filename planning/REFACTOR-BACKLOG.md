# Resource Server Pattern Refactor - Backlog

**Context**: Switch from ADR-004 (Proxy OAuth) to ADR-005 (Resource Server with Claude OAuth)

**Branch**: `feat/resource-server-pattern`

**Estimated Effort**: 0.5-1 day (vs 2-3 days remaining for ADR-004)

---

## Critical Path (P0 - Production Blockers)

### Phase 1: Miro Configuration (15 minutes)

- [ ] **CONFIG1**: Register Claude's callback URL in Miro Developer Portal
  - **Action**: Update OAuth2.0 Redirect URI in Miro app settings
  - **Old URI**: `https://miro-mcp.fly-agile.com/oauth/callback`
  - **New URIs** (add both):
    - `https://claude.ai/api/mcp/auth_callback`
    - `https://claude.com/api/mcp/auth_callback` (future-proof)
  - **Test**: Verify URIs saved successfully in Miro Developer Portal
  - **Complexity**: 0.1 (configuration only)

### Phase 2: Code Cleanup (30 minutes)

- [ ] **REMOVE1**: Delete Proxy OAuth implementation files
  - **Remove files**:
    - `src/auth/oauth.rs` (if exists)
    - `src/auth/token_store.rs` (if exists)
    - `src/auth/cookie_manager.rs` (if exists)
    - `src/auth/pkce.rs` (if exists)
  - **Remove from `src/auth/mod.rs`**: Module declarations for above
  - **Verify**: `cargo check` passes after removal
  - **Complexity**: 0.2 (file deletion + module cleanup)

- [ ] **REMOVE2**: Remove OAuth dependencies from Cargo.toml
  - **Remove** (if present as dedicated OAuth deps):
    - `ring` (unless used elsewhere)
    - `aes-gcm` (unless used elsewhere)
    - Dedicated OAuth cookie/PKCE crates
  - **Keep**:
    - `rmcp` (MCP protocol)
    - `reqwest` (Miro API)
    - `serde`, `serde_json` (serialization)
  - **Verify**: `cargo build` succeeds
  - **Complexity**: 0.1 (dependency cleanup)

### Phase 3: Protected Resource Metadata (1-2 hours)

- [ ] **OAUTH1**: Implement Protected Resource Metadata endpoint
  - **File**: `src/auth/metadata.rs`
  - **Endpoint**: `GET /.well-known/oauth-protected-resource`
  - **Response** (RFC 9728):
    ```json
    {
      "resource": "https://miro-mcp.fly-agile.com",
      "authorization_servers": [
        "https://miro.com"
      ]
    }
    ```
  - **Wire to router**: Add route in `src/http_server.rs` or equivalent
  - **Test**: `curl https://miro-mcp.fly-agile.com/.well-known/oauth-protected-resource`
  - **Acceptance**:
    - Returns HTTP 200
    - Valid JSON with `resource` and `authorization_servers` fields
    - `authorization_servers` points to Miro's OAuth server
  - **Complexity**: 0.5 (simple JSON endpoint)

- [ ] **OAUTH2**: Return 401 with WWW-Authenticate header for unauthenticated requests
  - **File**: `src/auth/middleware.rs` (or create if needed)
  - **Logic**: If `Authorization` header missing or invalid
  - **Response**:
    ```http
    HTTP/1.1 401 Unauthorized
    WWW-Authenticate: Bearer realm="miro-mcp",
                      as_uri="https://miro-mcp.fly-agile.com/.well-known/oauth-protected-resource"
    Content-Type: application/json

    {"error": "unauthorized", "message": "Bearer token required"}
    ```
  - **Test**: `curl -v https://miro-mcp.fly-agile.com/mcp/v1/tools` (no token)
  - **Acceptance**:
    - HTTP 401 status
    - WWW-Authenticate header present with correct `as_uri`
    - Clear error message in JSON body
  - **Complexity**: 0.3 (middleware modification)

### Phase 4: Token Validation (2-3 hours)

- [ ] **OAUTH3**: Validate Bearer tokens from Claude
  - **File**: `src/auth/token_validation.rs`
  - **Function**: `validate_token(token: &str) -> Result<Claims, AuthError>`
  - **Logic**:
    1. Extract Bearer token from `Authorization` header
    2. Decode JWT (use `jsonwebtoken` crate or simple base64 decode if unsigned)
    3. Verify token not expired (`exp` claim)
    4. **Critical**: Verify audience claim includes our server URL
    5. Optional: Cache validation results (5min TTL, reuse AUTH9 pattern if exists)
  - **Error handling**:
    - Missing token → 401
    - Invalid token → 401
    - Expired token → 401
    - Wrong audience → 401
  - **Test cases**:
    - Valid token → allow request
    - Expired token → 401
    - Token for different audience → 401
    - Malformed token → 401
  - **Acceptance**:
    - All test cases pass
    - Token validation adds <10ms latency (with caching)
    - Clear error messages for different failure modes
  - **Dependencies**: OAUTH2 (middleware structure)
  - **Complexity**: 1.0 (JWT validation with proper error handling)

### Phase 5: Configuration Updates (30 minutes)

- [ ] **CONFIG2**: Update environment configuration
  - **File**: `.env.production`
  - **Remove**:
    - `MIRO_CLIENT_SECRET` (Claude uses it, not us)
    - `MIRO_ENCRYPTION_KEY` (no token storage)
    - `MIRO_REDIRECT_URI` (Claude's callback, not ours)
  - **Keep**:
    - `MIRO_CLIENT_ID=3458764647632208270`
    - `BASE_URL=https://miro-mcp.fly-agile.com`
    - `MCP_SERVER_PORT=8080`
  - **File**: `src/config.rs`
  - **Remove**: `encryption_key` field and parsing logic
  - **Remove**: `client_secret` if loaded
  - **Keep**: `client_id`, `base_url`, `port`
  - **Test**: `cargo build` succeeds with updated config
  - **Complexity**: 0.3 (config file cleanup)

- [ ] **CONFIG3**: Update Scaleway deployment scripts
  - **File**: `scripts/deploy.sh` or `scripts/deploy-container.sh`
  - **Remove** from secret injection:
    - `MIRO_CLIENT_SECRET`
    - `MIRO_ENCRYPTION_KEY`
  - **Remove** from Scaleway Secret Manager:
    - Delete `MIRO_CLIENT_SECRET` secret
    - Delete `MIRO_ENCRYPTION_KEY` secret
  - **Keep**: Environment variables for `MIRO_CLIENT_ID`, `BASE_URL`
  - **Test**: Dry-run deployment script validates configuration
  - **Complexity**: 0.2 (script modification)

### Phase 6: Integration Testing (1-2 hours)

- [ ] **TEST4**: End-to-end OAuth flow with Claude.ai
  - **Prerequisites**: OAUTH1-3 complete, deployed to Scaleway
  - **Test Steps**:
    1. Open Claude.ai web interface
    2. Add MCP server: `https://miro-mcp.fly-agile.com`
    3. Claude discovers metadata endpoint (verify in network tab)
    4. Claude initiates OAuth flow → redirects to Miro
    5. Authorize with Miro credentials
    6. Claude redirects to `claude.ai/api/mcp/auth_callback`
    7. Claude stores token internally
    8. Test MCP tool call: `list_boards`
    9. Verify request includes `Authorization: Bearer <token>` header
    10. Verify MCP server validates token successfully
    11. Verify Miro API call succeeds
  - **Success Criteria**:
    - Complete OAuth flow without errors
    - MCP tools work with Claude-provided tokens
    - No 401 errors after authorization
    - Token refresh handled transparently by Claude
  - **Document**: Screenshots and logs for troubleshooting
  - **Complexity**: 1.0 (end-to-end validation)
  - **Priority**: P0 - Must work before production

- [ ] **TEST5**: Token validation edge cases
  - **Test Cases**:
    - Missing Authorization header → 401 with WWW-Authenticate
    - Malformed Bearer token → 401 with clear error
    - Expired token → 401 with "token expired" message
    - Token for wrong audience → 401 with "invalid audience"
    - Valid token → request succeeds
  - **Automated**: Add integration tests in `tests/` directory
  - **Manual**: Test with curl commands
  - **Acceptance**: All edge cases handled gracefully
  - **Complexity**: 0.5 (test coverage)

### Phase 7: Documentation (1 hour)

- [ ] **DOC3**: Update CLAUDE.md project guidelines
  - **Section**: "OAuth2 Security Standards"
  - **Update**: Remove references to Authorization Server pattern
  - **Add**: Document Resource Server pattern
  - **Key Points**:
    - We delegate OAuth to Claude.ai (no token storage)
    - Token validation only (verify audience claim)
    - Simpler security model (fewer secrets to manage)
    - Miro redirect URI: `claude.ai/api/mcp/auth_callback`
  - **Complexity**: 0.3 (documentation update)

- [ ] **DOC4**: Update README with simplified architecture
  - **Section**: "Authentication"
  - **Remove**: Complex OAuth proxy flow diagram
  - **Add**: Simple Resource Server diagram:
    ```
    Claude.ai → Miro OAuth → claude.ai/callback
                          ↓
    Claude.ai → MCP Server (with token) → Miro API
    ```
  - **Explain**: Claude handles OAuth, we only validate tokens
  - **Complexity**: 0.2 (README update)

---

## Success Metrics

**Before (ADR-004 Proxy OAuth)**:
- ~1000 LOC for OAuth implementation
- 3 secrets to manage (client_secret, encryption_key, tokens)
- 2-3 days remaining implementation
- Complex debugging (cookie encryption, PKCE, state management)

**After (ADR-005 Resource Server)**:
- ~150 LOC for metadata + validation
- 0 secrets on our server (Claude manages them)
- 0.5-1 day implementation
- Simple debugging (token validation only)

**Code Reduction**: 85% less OAuth-related code
**Security Surface**: 66% fewer secrets to protect
**Implementation Time**: 60-70% faster to production

---

## Dependencies

**Miro Developer Portal Access**: Required for CONFIG1 (redirect URI change)
**Scaleway Access**: Required for CONFIG3 (secret cleanup) and deployment
**Claude.ai Pro/Team/Enterprise**: Required for MCP OAuth flow (user already has)

---

## Rollback Plan

If Resource Server pattern doesn't work:
1. Revert to `main` branch (ADR-004 implementation)
2. Continue with AUTH10-14 from original backlog
3. Document why Resource Server failed (for future reference)

**Risk**: Low - Multiple projects successfully use this pattern (vault-server, etc.)

---

## Next Steps

1. ✅ ADR-005 documented (this file)
2. ✅ Worktree created: `feat/resource-server-pattern`
3. ⏳ Start with CONFIG1 (Miro redirect URI update)
4. ⏳ Implement OAUTH1-3 (metadata + validation)
5. ⏳ Test end-to-end with Claude.ai (TEST4)
6. ⏳ Merge to main and deprecate ADR-004

**Estimated Completion**: 2025-11-11 or 2025-11-12
