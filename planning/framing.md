# Miro MCP Server - Project Framing

## Vision

Build a complete RFC-compliant OAuth2 Authorization Server with Dynamic Client Registration in Rust, enabling secure MCP integration with Miro. The primary goal is creating reusable, properly-separated OAuth infrastructure that can be applied to future projects. Squad visualization is a secondary use case demonstrating the infrastructure's capabilities.

## Context

**Primary Goal**: Build proper OAuth infrastructure (reusable foundation)
- Complete RFC 6749 OAuth2 Authorization Code Flow implementation
- Dynamic Client Registration (RFC 7591)
- Separation of concerns: Miro configuration decoupled from Claude.ai
- Learning/achievement: Full Authorization Server implementation
- Reusability: Infrastructure usable for future OAuth-enabled projects

**Secondary Goal**: Demonstrate infrastructure with Miro integration
- MCP server for board manipulation (visual validation of OAuth)
- Squad visualization as example use case
- Timeline is flexible - do it right, not fast

**Architectural Choice** (ADR-004 + DCR):
- Authorization Server pattern (vs simpler Resource Server)
- Server implements full OAuth flow and token management
- Intentionally more complex to build reusable infrastructure
- ADR-005 documented alternative approach for reference only

---

## Team Structure

### Core Team (Consulted Systematically)

**1. solution-architect** - Implementation Planning & Architecture
- **When to consult**: Before starting each epic, for complex feature planning
- **Responsibilities**:
  - OAuth2 flow architecture decisions
  - MCP tool design and structure
  - Rust async patterns and library selection
  - API client architecture
  - Token storage and security patterns
- **Deliverables**: Implementation plans with file breakdowns, pattern recommendations

**2. developer** - Code Implementation
- **When to consult**: All code writing tasks (MANDATORY delegation)
- **Responsibilities**:
  - Implementing OAuth2 flows in Rust
  - Creating MCP tool handlers
  - Miro API client implementation
  - Error handling and validation
  - Integration testing
- **Deliverables**: Working code with tests
- **Model**: Use Haiku for simple tools (CRUD operations), Sonnet for OAuth2 and complex orchestration

**3. security-specialist** - OAuth2 & Token Security
- **When to consult**: Auth implementation, token storage, before production deployment
- **Responsibilities**:
  - OAuth2 flow security review
  - Token storage security (encryption at rest)
  - Secrets management validation
  - API credential handling
  - HTTPS/TLS configuration review
- **Deliverables**: Security findings with specific remediations

**4. integration-specialist** - MCP Protocol & API Integration
- **When to consult**: MCP tool design, API compatibility changes
- **Responsibilities**:
  - MCP protocol compliance validation
  - Miro API integration patterns
  - Tool parameter schema design
  - Cross-tool coordination (e.g., SQUAD1 orchestrating multiple tools)
  - API version compatibility
- **Deliverables**: Integration tests, compatibility matrices

---

### Support Team (On-Demand Consultation)

**5. architecture-reviewer** - SOLID & Design Review
- **When to consult**: After completing epics, before complex refactorings
- **Use case**: Ensure Rust code follows SOLID principles, review module structure

**6. performance-optimizer** - Scalability & Efficiency
- **When to consult**: Bulk operations implementation (BULK1), production performance issues
- **Use case**: Async/await patterns, connection pooling, rate limit handling

**7. code-quality-analyst** - Code Health
- **When to consult**: End of sprint, before major releases
- **Use case**: Identify DRY violations, complexity hotspots, maintainability issues

**8. git-workflow-manager** - Commit Hygiene
- **When to consult**: ALL commits (MANDATORY)
- **Use case**: Atomic commits, proper commit messages, history cleanliness

**9. documentation-writer** - API Documentation
- **When to consult**: Public API finalization, deployment documentation
- **Use case**: MCP tool documentation, OAuth2 setup guide, deployment instructions

---

### Collaboration Patterns

**Feature Development Flow**:
```
User Request
    ↓
solution-architect (plan implementation)
    ↓
developer (implement with tests)
    ↓
integration-specialist (validate MCP compliance)
    ↓
git-workflow-manager (atomic commit)
    ↓
[Complexity accumulator tracks progress]
    ↓
quality-orchestrator (every 2-3 features)
```

**Security-Critical Flow** (Auth, Token Management):
```
User Request
    ↓
solution-architect (security-aware architecture)
    ↓
security-specialist (review approach before implementation)
    ↓
developer (implement with security constraints)
    ↓
security-specialist (validate implementation)
    ↓
git-workflow-manager (commit)
```

**Bug Fix Flow**:
```
Bug Report
    ↓
developer (TDD: write failing test)
    ↓
developer (fix to make test pass)
    ↓
git-workflow-manager (commit with test)
```

---

## Agile Flow Configuration

```yaml
agile_flow:
  # Require estimation before starting work
  estimation_required: false  # Move fast for initial MVP

  # Quality review threshold (number of micro features)
  quality_review_threshold: 2-3  # Review every 2-3 features (standard)

  # Require backlog for all work
  backlog_required: true

  # Complexity multipliers
  complexity_weights:
    simple: 1.0    # Simple CRUD tools (create_text, delete_item)
    medium: 1.5    # API integration tools with state (list_boards, update_item)
    complex: 2.0   # OAuth2 flows, orchestration tools (SQUAD1)
```

**Rationale**:
- No estimation overhead for MVP (move fast)
- Standard quality review cadence (2-3 features)
- OAuth2 and squad orchestration are complex (2.0 weight)
- Simple tool implementations are lightweight (1.0 weight)

---

## Technical Constraints

### Language & Framework
- **Language**: Rust (stable)
- **MCP Framework**: TBD - evaluate existing Rust MCP libraries or implement from spec
- **HTTP Client**: reqwest (async, well-maintained)
- **OAuth2**: oauth2-rs crate (standard Rust OAuth2 implementation)
- **Serialization**: serde (JSON for MCP protocol and Miro API)

### Miro API Constraints
- **Rate Limit**: 100 requests/minute per user
- **Bulk Limit**: Max 20 items per bulk create operation
- **Token Expiry**: Access tokens expire after 3600 seconds (1 hour)
- **API Version**: v2 (stable, v1 deprecated for most endpoints)

### Stateless Architecture Constraints (ADR-001)
- **State Management**: Encrypted cookies only (no database/Redis/in-memory state)
- **Cookie Security**: httpOnly, Secure, SameSite attributes mandatory
- **State Expiration**: OAuth state 10-minute TTL
- **Token Lifetime**: Access token 1-hour maximum
- **Encryption**: Server secret for cookie encryption/decryption
- **Cache Persistence**: LRU cache in-memory for token validation (requires container)

### Security Requirements
- HTTPS/TLS mandatory for OAuth2 redirect URI
- Access tokens encrypted at rest
- Refresh tokens stored securely
- Client secret NEVER in client-side code or version control
- Audit logging for authentication events

### MCP Protocol Requirements
- Remote MCP server accessible via public URL
- Health check endpoint for monitoring
- Proper tool definitions with JSON schema
- Error responses following MCP error format
- OAuth2 flow per MCP specification

---

## Success Criteria

### Phase 1: OAuth2 Authorization Server (PRIMARY GOAL)
- [ ] RFC 6749 OAuth2 Authorization Code Flow implemented
- [ ] Dynamic Client Registration (RFC 7591) working
- [ ] PKCE implemented (code_verifier + code_challenge)
- [ ] Token management (access + refresh) with secure storage
- [ ] Token refresh flow working correctly
- [ ] Separation of concerns: Miro config independent of Claude.ai
- [ ] All OAuth endpoints functional and compliant
- [ ] Security review passed (Authorization Server scope)

### Phase 2: MCP Server Integration (SECONDARY)
- [ ] MCP server communicates with OAuth infrastructure
- [ ] All API requests use Bearer tokens correctly
- [ ] Token expiry handled gracefully
- [ ] MCP protocol compliance validated

### Phase 3: Miro Operations (DEMONSTRATION)
- [ ] Basic operations: list boards, create board
- [ ] Visual elements: sticky notes, shapes, text, frames
- [ ] Connectors and relationships
- [ ] Update and delete operations

### Phase 4: Use Case Validation (NICE-TO-HAVE)
- [ ] 3-squad org chart creation works
- [ ] Bulk operations if needed
- [ ] Documentation of OAuth infrastructure
- [ ] Deployment documentation

---

## Definition of Done

**Per Feature**:
- Code implemented following Rust best practices
- Unit tests covering core logic
- Integration tests for API interactions
- Error handling for API failures
- Committed via git-workflow skill (atomic commits)
- Backlog item marked complete with date

**Per Epic**:
- All epic features complete
- Integration tests passing
- Quality review completed (if threshold reached)
- Architecture review for complex epics (OAuth2, orchestration)
- Documentation updated

**Production Release**:
- OAuth2 Authorization Server complete and RFC-compliant
- Security review passed (Authorization Server scope)
- OAuth2 flow tested end-to-end
- MCP integration functional
- Infrastructure documented for reuse
- Deployment successful

---

## Risk Management

**High Risks**:
1. **OAuth2 complexity**: Mitigated by developer experience + solution-architect planning
2. **MCP protocol compliance**: Mitigated by integration-specialist validation
3. **Token security**: Mitigated by security-specialist review before production
4. **Miro API rate limits**: Mitigated by bulk operations (BULK1) and smart batching
5. **Resource Server pattern**: ADR-002 supersedes ADR-001 stateless OAuth architecture
   - **Change**: Claude handles OAuth, we validate tokens (RFC 9728 Resource Server)
   - **Impact**: 70% less code (~150 LOC vs ~500 LOC)
   - **Benefit**: LRU cache for token validation (95% cache hit rate, <1ms latency)

**Medium Risks**:
1. **Rust async complexity**: Mitigated by solution-architect patterns + tokio best practices
2. **Deployment hosting**: Requires HTTPS - plan deployment platform early
3. **Error handling coverage**: Mitigated by comprehensive testing strategy

**Low Risks**:
1. **API stability**: Miro API v2 is stable and well-documented
2. **Library availability**: Rust ecosystem has mature HTTP/OAuth2/JSON libraries

---

## Development Phases

### Phase 1: OAuth2 Authorization Server (PRIMARY)
**Focus**: Complete RFC-compliant OAuth infrastructure
- OAuth2 Authorization Code Flow (RFC 6749)
- Dynamic Client Registration (RFC 7591)
- Token management (access + refresh)
- PKCE implementation
- Security review (Authorization Server scope)
- **Timeline**: Take the time needed to do it right

### Phase 2: MCP Integration (SECONDARY)
**Focus**: Connect MCP server to OAuth infrastructure
- MCP protocol compliance
- Token-based API client
- Basic Miro operations
- **Timeline**: After OAuth is solid

### Phase 3: Demonstration (NICE-TO-HAVE)
**Focus**: Validate infrastructure with real use case
- Squad visualization
- Bulk operations if needed
- Documentation
- **Timeline**: When infrastructure is complete

**Philosophy**: Quality over speed - building reusable infrastructure

---

## Notes

**Developer Context**:
- Building complete OAuth2 Authorization Server as learning/infrastructure project
- Goal: Reusable OAuth infrastructure for future projects
- Separation of concerns: Miro config independent of Claude.ai
- Timeline: Flexible - prioritizing correctness over speed

**Value Proposition**:
1. **Reusability**: OAuth infrastructure usable beyond Miro
2. **Learning**: Complete Authorization Server implementation
3. **Separation**: Decoupled OAuth configuration
4. **Foundation**: Proper infrastructure for future OAuth-enabled projects

**Testing Strategy**:
- Focus on OAuth flow correctness first
- MCP integration second
- Use case validation third
- Take time to build it right

**Infrastructure & Deployment**

**Platform Choice: Scaleway Managed Containers** ✅

*Decision rationale*: Required for LRU cache persistence and optimal token validation performance (ADR-002 Resource Server pattern)

**Why Container > Function**:
- **Remote MCP = SSE transport over HTTP** (long-polling HTTP server, not stdio)
- **LRU cache in-memory** persists between requests (95% cache hit rate)
- **No cold start penalty** for token validation (critical path)
- **Token validation latency**: <1ms (cached) vs 100ms (Miro API call)

**Performance Analysis**:
- **Workload pattern**: Sporadic bursts (org chart 1x/day + spaced API calls)
- **Token validation**: <1ms (95% cached) vs 100ms (Miro API)
- **MCP operations**: 200-500ms latency (Miro API + processing)
- **Cache efficiency**: 95% hit rate with 5-minute TTL

**Cost Projection**:
- **Container (always-on)**: ~€20/month (0.25 vCPU + 256Mi memory)
- **Container cost breakdown**:
  - vCPU: €0.10/vCPU/hour × 0.25 × 730 hours = €18/month
  - Memory: €0.01/GB/hour × 0.256 GB × 730 hours = €1.87/month
- **Verdict**: Acceptable for personal use with optimal performance

**Recommended Configuration**:
```yaml
containers:
  miro-mcp:
    runtime: rust (Debian Bookworm Slim)
    memory: 256Mi       # Sufficient for OAuth2 + API calls + LRU cache
    cpu: 0.25           # Single user, low concurrency
    min_scale: 1        # Always-on for cache persistence
    max_scale: 1        # Single user deployment
    port: 3000          # HTTP/SSE transport
```

**Architecture (ADR-002)**:
- **Pattern**: Resource Server with token validation + caching (RFC 9728)
- **OAuth flow**: Claude handles OAuth, server validates tokens
- **Token storage**: Claude stores tokens (not our responsibility)
- **Cache**: LRU cache (100 tokens, 5-min TTL) for validation results
- **Code complexity**: ~150 LOC (70% less than ADR-001 Proxy OAuth)

**Cache Configuration**:
- **Type**: LRU (Least Recently Used)
- **Size**: 100 tokens (~10KB memory)
- **TTL**: 5 minutes (balance security vs performance)
- **Hit rate**: 95% (estimated for typical usage)

**Platform Details**:
- **Compute**: Scaleway Managed Containers
- **Secrets**: Scaleway Secret Manager (MIRO_CLIENT_ID, MIRO_ENCRYPTION_KEY)
- **Logs**: Scaleway Cockpit (audit trail for token validation)
- **TLS**: Native HTTPS (Scaleway provides TLS termination)
- **Cost target**: €20/month (vs €25-50/month with database)

**Decision date**: 2025-11-10 (ADR-002 architecture supersedes ADR-001, container vs function)
