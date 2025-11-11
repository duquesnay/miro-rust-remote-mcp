# Miro MCP Server - Project Guidelines

## Project Overview

MCP server for Miro board manipulation via OAuth2 authentication, built in Rust. Enables Claude AI to create agile squad organizational visualizations programmatically.

**Primary Use Case**: Visualize agile team structures (squads, roles, reporting lines) from natural language prompts in <5 minutes.

**Key Differentiators**:
- First Miro MCP with OAuth2 support (vs static tokens)
- Rust implementation (vs existing TypeScript servers)
- Remote MCP deployment for Claude.ai web interface

---

## Team Structure

**See**: [planning/framing.md](planning/framing.md) for complete team structure and collaboration patterns.

**Core Team** (Consulted Systematically):
- `solution-architect` - Before each epic, complex features
- `developer` - ALL code implementation (MANDATORY)
- `security-specialist` - Auth implementation, token storage, pre-production
- `integration-specialist` - MCP compliance, API integration

**Quick Reference**:
```bash
# When starting new feature
Task(subagent_type="solution-architect", prompt="Plan implementation for [ITEM_ID]")

# When implementing code (ALWAYS)
Task(subagent_type="developer", prompt="Implement [ITEM_ID] per plan")

# When committing (ALWAYS)
Skill(skill="git-workflow")

# Every 2-3 features
Skill(skill="quality-orchestrator")
```

---

## Rust-Specific Coding Standards

### Code Organization

**Project Structure**:
```
miro-mcp-server/
├── src/
│   ├── main.rs              # MCP server entry point
│   ├── auth/                # OAuth2 flow implementation
│   │   ├── mod.rs
│   │   ├── oauth.rs         # Authorization code flow
│   │   └── token_store.rs   # Secure token storage
│   ├── mcp/                 # MCP protocol implementation
│   │   ├── mod.rs
│   │   ├── server.rs        # MCP server core
│   │   └── tools.rs         # Tool definitions
│   ├── miro/                # Miro API client
│   │   ├── mod.rs
│   │   ├── client.rs        # HTTP client + auth
│   │   ├── boards.rs        # Board operations
│   │   ├── items.rs         # Item CRUD
│   │   └── types.rs         # Miro API types
│   └── lib.rs               # Library exports
├── tests/
│   ├── integration/         # Integration tests
│   └── fixtures/            # Test data
├── Cargo.toml
└── planning/                # Agile artifacts
```

### Naming Conventions

**Rust Standard Library Style** (follow rustfmt defaults):
- **Modules**: `snake_case` (e.g., `token_store`, `oauth_flow`)
- **Types**: `PascalCase` (e.g., `OAuthClient`, `MiroBoard`)
- **Functions**: `snake_case` (e.g., `refresh_token`, `create_sticky_note`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BULK_ITEMS`, `TOKEN_EXPIRY_SECONDS`)
- **Lifetimes**: Single letter lowercase (e.g., `'a`, `'b`)

**MCP Tool Naming** (external API):
- Snake_case with underscores: `list_boards`, `create_sticky_note`, `bulk_create_items`
- Matches MCP specification conventions
- Maps to Rust functions via macro/conversion

### Error Handling

**Use `Result<T, E>` everywhere**:
```rust
// Good
pub async fn fetch_boards(&self) -> Result<Vec<Board>, MiroError> {
    // ...
}

// Bad - avoid unwrap() in production code
pub async fn fetch_boards(&self) -> Vec<Board> {
    api_call().await.unwrap()  // ❌ Will panic
}
```

**Custom error types with thiserror**:
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MiroError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("API request failed: {status}")]
    ApiError { status: u16, message: String },

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}
```

**Error propagation with `?` operator**:
```rust
pub async fn create_board(&self, name: &str) -> Result<Board, MiroError> {
    let token = self.get_valid_token().await?;  // Propagate auth errors
    let response = self.http_client
        .post(BOARDS_URL)
        .bearer_auth(token)
        .json(&json!({ "name": name }))
        .send()
        .await?;  // Propagate HTTP errors

    Ok(response.json().await?)  // Propagate JSON parse errors
}
```

### Async Patterns

**Use tokio runtime** (standard for async Rust):
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Server initialization
    Ok(())
}
```

**Prefer async/await over manual Future implementation**:
```rust
// Good - clean async/await
async fn fetch_and_process(&self) -> Result<Vec<Item>, MiroError> {
    let boards = self.list_boards().await?;
    let items = self.fetch_items(&boards[0].id).await?;
    Ok(items)
}

// Avoid - manual Future unless necessary for performance
```

**Use `tokio::spawn` for concurrent operations**:
```rust
// Concurrent API calls for bulk operations
let handles: Vec<_> = items.iter()
    .map(|item| tokio::spawn(create_item(item.clone())))
    .collect();

let results = futures::future::join_all(handles).await;
```

### Security Standards

**Token Storage**:
- Access tokens encrypted at rest using `ring` or `rustls`
- Refresh tokens stored separately with higher security
- NEVER log tokens (even in debug mode)
- Use `SecretString` or similar for sensitive data

**Secrets Management**:
```rust
// Good - secrets from environment or secure store
let client_secret = env::var("MIRO_CLIENT_SECRET")
    .expect("MIRO_CLIENT_SECRET not set");

// Bad - hardcoded secrets
const CLIENT_SECRET: &str = "AKoo...";  // ❌ NEVER
```

**OAuth2 Security**:
- Validate `state` parameter to prevent CSRF
- Use PKCE (Proof Key for Code Exchange) if supported by Miro
- Verify redirect URI matches registered URI exactly
- Implement token rotation (use new refresh token each refresh)

---

## Testing Standards

### Test Organization

**Unit tests** (in same file):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expiry_detection() {
        // Test pure logic without I/O
    }
}
```

**Integration tests** (tests/integration/):
```rust
// tests/integration/miro_api_test.rs
#[tokio::test]
async fn test_create_board_flow() {
    // Test actual API interactions (with test credentials)
}
```

### TDD Workflow

**See**: `tdd-workflow` skill for complete Red-Green-Refactor protocol.

**For bug fixes** (MANDATORY):
1. Write failing test reproducing bug
2. Run test to confirm it fails
3. Fix bug to make test pass
4. Verify no regressions
5. Commit test + fix together via `git-workflow` skill

**For new features** (RECOMMENDED):
1. Write test defining expected behavior
2. Implement minimal code to pass test
3. Refactor for quality
4. Commit

### Integration Test Requirements

**All MCP tools MUST have integration tests**:
```rust
#[tokio::test]
async fn test_create_sticky_note_tool() {
    let client = MiroClient::new_with_test_token();
    let board = client.create_board("Test Board").await.unwrap();

    let sticky_note = client.create_sticky_note(
        &board.id,
        "Test Content",
        Position { x: 0.0, y: 0.0 },
        Color::LightYellow,
    ).await.unwrap();

    assert_eq!(sticky_note.data.content, "Test Content");
    assert_eq!(sticky_note.style.fill_color, "light_yellow");
}
```

**OAuth2 flow testing** (use Miro test environment or mocking):
```rust
#[tokio::test]
async fn test_token_refresh_flow() {
    // Test token refresh without manual intervention
}
```

### Mock vs Real API

**Use real API for integration tests** (preferred when possible):
- Tests actual integration behavior
- Catches API changes early
- Use test Miro account with cleanup

**Use mocks for** (when needed):
- Rate limit testing
- Error condition simulation
- Offline development
- CI/CD pipelines (if API access restricted)

**NEVER**:
- Add graceful degradation logic in production code to handle missing test setup
- Skip failing integration tests with `--exclude` flags
- Use mocks in integration tests when real dependencies are available

---

## Git Workflow

### Commit Protocol

**MANDATORY**: ALL commits go through `git-workflow` skill.

**Never commit directly** - always delegate to git specialist:
```rust
// After implementing feature and tests passing
Skill(skill="git-workflow")
```

**Commit message format** (enforced by git-workflow):
```
feat: implement OAuth2 authorization code flow
fix: correct token expiry detection logic
refactor: consolidate Miro API error types
test: add integration tests for board operations
docs: document MCP tool usage examples
```

**Atomic commits** (git-workflow ensures):
- All related files included in commit
- Tests included with implementation
- Commit compiles and tests pass standalone
- No partial features committed

### Branch Strategy

**Feature Branch Workflow**:
```bash
# Create branch on task start (not code start)
git checkout -b feat/AUTH1-oauth2-flow

# Work on feature, commit via git-workflow skill

# When complete, merge to main
git checkout main
git merge feat/AUTH1-oauth2-flow
```

**Branch naming**:
- `feat/[ID]-short-description` - New features
- `fix/[ID]-short-description` - Bug fixes
- `refactor/[ID]-short-description` - Refactoring
- `test/[ID]-short-description` - Test additions

**ALWAYS check current branch before code changes**:
```bash
git branch --show-current
```

---

## Quality Standards

### Code Quality Targets

**Rust-specific quality**:
- Clippy warnings resolved: `cargo clippy -- -D warnings`
- Rustfmt applied: `cargo fmt --check`
- No unsafe code without explicit safety documentation
- Public APIs documented with `///` doc comments

**Test coverage** (aspirational):
- Critical paths: 100% (OAuth2 flow, token refresh)
- MCP tools: 90%+ (all tools have integration tests)
- Utility functions: 80%+

**Performance targets**:
- Single item creation: <200ms (network + API processing)
- Bulk 20 items: <1000ms (50ms per item amortized)
- Token refresh: <500ms

### Quality Review Triggers

**See**: Agile Flow Configuration in [planning/framing.md](planning/framing.md)

**Trigger quality-orchestrator when**:
- 2-3 features completed (complexity-weighted)
- Architectural changes made (OAuth2, orchestration tools)
- Before sprint completion
- Before production deployment

**Complexity weights**:
- Simple CRUD tools: 1.0 (create_text, delete_item)
- API integration: 1.5 (list_boards, update_item)
- OAuth2/orchestration: 2.0 (AUTH1, AUTH2, SQUAD1)

---

## Development Workflow

### Starting New Work

1. **Check backlog context**:
   ```bash
   cat planning/backlog.md
   ```

2. **Identify backlog item**: User specifies item ID (e.g., "Work on AUTH1")

3. **Create branch**:
   ```bash
   git checkout -b feat/AUTH1-oauth2-flow
   ```

4. **Consult solution-architect** (for medium/complex items):
   ```
   Task(subagent_type="solution-architect", prompt="Plan implementation for AUTH1")
   ```

5. **Implement via developer agent** (MANDATORY):
   ```
   Task(subagent_type="developer", model="sonnet", prompt="Implement AUTH1 per plan")
   ```
   - Use Sonnet for OAuth2, complex orchestration
   - Use Haiku for simple CRUD tools

6. **Run tests**:
   ```bash
   cargo test
   cargo clippy
   cargo fmt --check
   ```

7. **Commit via git-workflow**:
   ```
   Skill(skill="git-workflow")
   ```

8. **Update backlog**:
   - Mark item [x] in planning/backlog.md
   - Add completion date ✅ YYYY-MM-DD

9. **Check complexity accumulator**:
   - If threshold reached → trigger quality-orchestrator

### Bug Fix Workflow

1. **Use TDD approach** (MANDATORY for bugs):
   ```
   Skill(skill="tdd-workflow")
   ```

2. **Protocol**:
   - Write failing test reproducing bug
   - Fix bug to make test pass
   - Verify no regressions
   - Commit test + fix together

3. **Why didn't tests catch this earlier?**
   - Review existing test coverage
   - Add missing tests before fixing

---

## Security Checklist

### Before Committing Auth Code

- [ ] No hardcoded secrets (client_id, client_secret, tokens)
- [ ] Tokens encrypted at rest
- [ ] State parameter validated (CSRF prevention)
- [ ] Redirect URI matches registered URI exactly
- [ ] Token expiry checked before API calls
- [ ] Refresh token rotated on use
- [ ] Sensitive data not logged (even in debug mode)

### Before Production Deployment

- [ ] security-specialist review passed
- [ ] HTTPS/TLS configured correctly
- [ ] Redirect URI registered in Miro Developer Portal
- [ ] Environment variables for all secrets
- [ ] Token storage encryption tested
- [ ] Audit logging for auth events
- [ ] Rate limiting implemented

---

## MCP Protocol Compliance

### Tool Definition Format

**All tools must have proper JSON schema**:
```rust
{
    "name": "create_sticky_note",
    "description": "Create a sticky note on a Miro board with custom content and styling",
    "parameters": {
        "type": "object",
        "properties": {
            "board_id": {
                "type": "string",
                "description": "ID of the board to create sticky note on"
            },
            "content": {
                "type": "string",
                "description": "Content of sticky note (supports HTML)"
            },
            "x": {
                "type": "number",
                "description": "X coordinate (center of note)"
            },
            "y": {
                "type": "number",
                "description": "Y coordinate (center of note)"
            },
            "color": {
                "type": "string",
                "enum": ["light_yellow", "yellow", "orange", ...],
                "description": "Fill color of sticky note"
            }
        },
        "required": ["board_id", "content", "x", "y"]
    }
}
```

### Error Response Format

**Follow MCP error specification**:
```json
{
    "error": {
        "code": "AUTHENTICATION_FAILED",
        "message": "Access token expired, please re-authenticate",
        "details": {
            "expires_at": "2025-11-10T15:30:00Z"
        }
    }
}
```

---

## Dependencies

### Required Crates

**Core**:
- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde` + `serde_json` - Serialization

**OAuth2**:
- `oauth2` - OAuth2 client implementation
- `url` - URL parsing

**Security**:
- `ring` or `rustls` - Encryption for token storage
- `base64` - Encoding

**Error Handling**:
- `thiserror` - Custom error types
- `anyhow` - Error context (for binary, not library)

**Testing**:
- `mockito` - HTTP mocking (if needed)
- `tokio-test` - Async test utilities

### Cargo.toml Template

```toml
[package]
name = "miro-mcp-server"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
oauth2 = "4.4"
thiserror = "1.0"
url = "2.5"

[dev-dependencies]
tokio-test = "0.4"
```

---

## Deployment Notes

### Hosting Requirements

**HTTPS Required**: OAuth2 redirect URI must use HTTPS

**Platform: Scaleway Containers** ✅ (Selected 2025-11-10)

**Rationale**:
- Container-based deployment for MCP server
- Native HTTPS support (required for OAuth2)
- Stateless architecture (ADR-002) compatible
- Predictable pricing and resource allocation
- Supports Resource Server pattern with token validation

**Configuration**: See [planning/framing.md](planning/framing.md) for deployment details

**Alternative Platforms** (if requirements change):
- **Railway**: Simple deployment alternative
- **Self-hosted VPS**: Full control but requires Nginx for HTTPS

### Environment Variables

```bash
# Required for production
MIRO_CLIENT_ID=3458764647516852398
MIRO_CLIENT_SECRET=<from secure store>
MIRO_REDIRECT_URI=https://[container-name].containers.scw.cloud/oauth/callback
MCP_SERVER_PORT=3000
TOKEN_ENCRYPTION_KEY=<generated securely>
```

### Health Check Endpoint

```rust
// GET /health
{
    "status": "healthy",
    "version": "0.1.0",
    "auth": {
        "token_valid": true,
        "expires_in": 3245
    }
}
```

---

## Success Metrics

**Development Velocity**:
- Sprint 1 (Foundation): 3 days
- Sprint 2 (Visualization): 3 days
- Sprint 3 (Production): 2 days

**Code Quality**:
- Zero clippy warnings
- 90%+ test coverage on critical paths
- All integration tests passing

**User Experience**:
- 3-squad org chart created in <5 minutes via Claude prompt
- OAuth2 flow completes in <2 minutes
- Token refresh transparent to user

**Production Readiness**:
- Security review passed
- Deployed to HTTPS endpoint
- Claude.ai web integration verified
- Documentation complete

---

## Learnings & Evolution

**This section documents project-specific learnings as they emerge.**

### 2025-11-11 - MCP OAuth Patterns: Resource Server vs Authorization Server

**Context**: During Scaleway deployment investigation, compared our OAuth implementation with vault-server project.

**Finding**: **Two distinct MCP OAuth patterns exist**, and we chose the more complex one unnecessarily:

1. **Authorization Server Pattern** (what we built in ADR-004):
   - MCP server implements full OAuth flow (`/oauth/authorize`, `/oauth/callback`, `/oauth/token`)
   - Server handles authorization code exchange with external API (Miro)
   - Server stores encrypted tokens
   - Server manages PKCE, state, token refresh
   - Redirect URI: `https://our-server.com/oauth/callback`
   - **Complexity**: ~1000 LOC, 3 secrets to manage
   - **Use case**: When you need OAuth control (rate limiting, custom flows, multi-provider)

2. **Resource Server Pattern** (vault-server approach, simpler):
   - Claude.ai handles OAuth flow directly with external API
   - Server only validates tokens passed in `Authorization` headers
   - No OAuth endpoints needed on server
   - No token storage needed
   - Redirect URI: `https://claude.ai/api/mcp/auth_callback`
   - **Complexity**: ~150 LOC, 0 secrets on server
   - **Use case**: Standard MCP servers (most cases)

**Key Discovery**: Miro accepts external redirect URIs including Claude's callback URL. The MCP OAuth 2.1 specification (RFC 9728) explicitly supports Resource Server pattern via Protected Resource Metadata.

**Implication**:
- **85% code reduction** possible by switching patterns
- **66% fewer secrets** to manage (client_secret stays with Claude)
- **60-70% faster** to production (0.5-1 day vs 2-3 days remaining)
- **Simpler maintenance** (token validation only vs full OAuth lifecycle)

**Action**:
- Created **ADR-005** documenting Resource Server pattern decision
- Created `feat/resource-server-pattern` worktree for refactor
- New **REFACTOR-BACKLOG.md** with simplified implementation plan (OAUTH1-3 vs AUTH10-14)
- Pattern validated: vault-server, multiple other MCP servers use this successfully

**Architectural Lesson**: Always check for simpler patterns before implementing complex solutions. The MCP specification supports multiple OAuth patterns - choose based on actual requirements, not assumptions.

**Reference**: [planning/ADR-005-resource-server-with-claude-oauth.md](planning/ADR-005-resource-server-with-claude-oauth.md)

---

### [Date] - [Learning Title]

**Context**: [What situation led to this learning]
**Finding**: [What we discovered]
**Implication**: [How this changes our approach]
**Action**: [What we're doing differently going forward]

---

## Quick Reference

**Start new feature**:
```bash
# 1. Check backlog
cat planning/backlog.md

# 2. Create branch
git checkout -b feat/[ID]-description

# 3. Plan (if complex)
Task(subagent_type="solution-architect", prompt="Plan [ID]")

# 4. Implement
Task(subagent_type="developer", prompt="Implement [ID]")

# 5. Test
cargo test && cargo clippy

# 6. Commit
Skill(skill="git-workflow")

# 7. Update backlog (mark [x] with date)
```

**Fix bug**:
```bash
Skill(skill="tdd-workflow")  # Ensures test-first approach
```

**Quality review**:
```bash
Skill(skill="quality-orchestrator")  # Every 2-3 features
```

---

## Philosophy

**This project prioritizes**:
- Security over convenience (proper OAuth2, token encryption)
- Correctness over speed (comprehensive testing, TDD for bugs)
- Rust idioms over shortcuts (Result types, async/await, no unwrap)
- User experience over features (5-minute squad visualization goal)
- Agile discipline over ad-hoc development (backlog-first, quality reviews)

**Remember**: "Production-ready OAuth2 MCP in Rust requires systematic quality - security and correctness are non-negotiable."
