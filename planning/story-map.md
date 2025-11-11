# Miro MCP Server - Story Map

## Purpose

This document shows the **actual decomposition history** of user stories as they evolved during development. Unlike the backlog (single priority list), this map shows parent-child relationships and how epics decomposed into implementable items.

---

## Epic 1: Authentication Foundation (Completed ✅ 2025-11-10)

**Original Goal**: Enable secure programmatic access to Miro API via OAuth2
**Evolution**: Expanded from simple OAuth (AUTH1-2) to stateless architecture (AUTH3-9) and now pivoting to Proxy OAuth (AUTH10-14)

### Phase 1: Basic OAuth (Completed ✅)
```
AUTH1: User authenticates with Miro securely via OAuth2
├── Implemented authorization code flow
└── Foundation for all subsequent auth work

AUTH2: System refreshes tokens automatically (vs manual re-auth)
├── Prevents session interruption
└── Enables long-running operations
```

### Phase 2: Stateless Architecture - ADR-002 (Completed ✅, Superseded by ADR-004)
```
AUTH3: User completes OAuth flow in browser from Claude Desktop
├── HTTP server for OAuth callbacks
└── Browser-based authorization

AUTH4: Developer adds OAuth state via encrypted cookies
├── AES-256-GCM encryption
└── Foundation for stateless pattern

AUTH5: User's access token stored in encrypted cookies
├── Client-side token storage
└── Note: Superseded by Bearer token pattern (AUTH7)

AUTH6: Claude discovers OAuth via metadata endpoint
├── /.well-known/oauth-protected-resource endpoint
└── Auto-discovery per MCP spec
```

### Phase 3: HTTP Resource Server Pattern - ADR-003 (Completed ✅, Superseded by ADR-004)
```
AUTH7: Server extracts Bearer tokens from Authorization header
├── Replaces cookie-based auth
└── Standard OAuth2 Resource Server pattern

AUTH8: Server validates tokens with Miro introspection API
├── Security: prevents token forgery
└── 100ms latency per validation

AUTH9: Token validation cached with 5-minute TTL
├── Reduces latency to <10ms for cache hits
└── Balances security vs performance
```

### Phase 4: Proxy OAuth Pattern - ADR-004 (In Progress ⏳)
**Context**: ADR-002/003 don't work with Claude.ai web - switching to proxy pattern where OUR server handles OAuth

**Decomposition**:
```
AUTH10: Developer implements OAuth proxy module with Miro integration
├── Complete PKCE OAuth flow
├── Authorization URL generation
├── Token exchange and refresh
└── Complexity: 2.0 (core OAuth implementation)

AUTH11: Developer implements OAuth HTTP endpoints for Claude.ai
├── /oauth/authorize (redirect to Miro)
├── /oauth/callback (exchange code, set token)
├── /oauth/token (return token to Claude.ai)
└── Complexity: 1.5 (HTTP handlers + cookie management)

AUTH12: Developer implements stateless state management with encrypted cookies
├── PKCE verifier storage
├── AES-256-GCM encryption (reuses AUTH4 foundation)
└── Complexity: 1.0 (cookie encryption)

AUTH13: Developer implements PKCE support for OAuth security
├── RFC 7636 compliance
├── Code challenge/verifier generation
└── Complexity: 0.5 (standard implementation)

AUTH14: Developer updates OAuth metadata to point to server endpoints
├── Metadata now points to OUR proxy endpoints
├── vs Miro endpoints (ADR-002 pattern)
└── Complexity: 0.5 (configuration change)
```

**Total AUTH Effort**: 8 items completed (AUTH1-9), 5 in progress (AUTH10-14), 13 total

---

## Epic 2: Board Management (Completed ✅ 2025-11-10)

**Goal**: Enable Claude to discover and create Miro boards programmatically

```
BOARD1: User lists accessible Miro boards programmatically
├── Board discovery before content creation
└── 3h implementation

BOARD2: User creates new boards via Claude prompts
├── Fresh workspace on-demand
└── 3h implementation
```

**Total Effort**: 2 items, 6 hours

---

## Epic 3: Visual Element Creation (Completed ✅ 2025-11-10)

**Goal**: Enable creation of all visual elements needed for organizational diagrams

```
VIS1: User creates sticky notes with custom content and styling
├── Core element for team member representation
└── 4h implementation

VIS2: User creates shapes for organizational structure
├── Squad boundaries and structural elements
└── 4h implementation

VIS3: User creates text elements on boards
├── Labels and standalone descriptions
└── 2h implementation

VIS4: User creates frames for grouping related content
├── Organizes entire squads in visual containers
└── 3h implementation
```

**Total Effort**: 4 items, 13 hours

---

## Epic 4: Relationship Visualization (Completed ✅ 2025-11-10)

**Goal**: Enable visual representation of relationships between people and teams

```
REL1: User connects items with styled arrows/lines
├── Reporting hierarchy and dependencies
└── 4h implementation

REL2: User adds captions to connectors
├── Labels relationship types clearly
└── 2h implementation
```

**Total Effort**: 2 items, 6 hours

---

## Epic 5: Item Operations (Completed ✅ 2025-11-10)

**Goal**: Enable Claude to inspect and modify existing board content

```
ITEM1: User lists board items filtered by type
├── Discover existing content for modification
└── 3h implementation

ITEM2: User updates item properties dynamically
├── Adjust visualizations without recreation
└── 4h implementation

ITEM3: User removes items from boards
├── Clean up incorrect or obsolete content
└── 2h implementation
```

**Total Effort**: 3 items, 9 hours

---

## Epic 6: Bulk Operations (Completed ✅ 2025-11-10)

**Goal**: Reduce latency for complex visualizations with many items

```
BULK1: User creates multiple items efficiently (vs individual API calls)
├── Reduces API calls and latency by >50%
└── 5h implementation
```

**Total Effort**: 1 item, 5 hours

---

## Epic 7: Frame Operations (Completed ✅ 2025-11-10)

**Goal**: Advanced content organization within frames

**Decomposition**: Single epic decomposed into 4 capabilities

```
FRAME1: User creates items directly in frames
├── No manual move after creation
└── Foundation for FRAME2-4

FRAME2: User moves items between frames for reorganization
├── Dynamic reorganization
└── Extends ITEM2 (update operation)

FRAME3: User filters items by containing frame
├── Focused squad operations
└── Combines with ITEM1 filtering

FRAME4: User removes items from frames to board root
├── Ungrouping capability
└── Inverse of FRAME1 operation
```

**Total Effort**: 4 items (complexity-weighted effort included in sprint total)

---

## Epic 8: Layering (Partially Blocked)

**Goal**: Control visual stacking order

**Decomposition**:
```
LAYER1.1: User controls z-order stacking [BLOCKED - Web SDK only]
└── Blocked: API limitation, recommend DROP

LAYER1.2: User manages organizational layers [BLOCKED - UI only]
└── Blocked: UI-centric feature, recommend DROP

LAYER2: User understands item stacking order [COMPLETED ✅]
├── Read-only understanding of z-index
└── Documents predictable stacking behavior
```

**Status**: 1 completed (read-only), 2 blocked (programmatic control unavailable via API)

---

## Epic 9: Technical Capabilities (Completed ✅ 2025-11-10)

**Goal**: Developer experience and system quality improvements

**Decomposition**: Quality-driven technical capabilities emerged during implementation

```
TECH1: MCP server responds to protocol requests
├── Fixed crash on tools/list
└── Foundational protocol compliance

TECH2: Developer modifies parent construction in single location
├── Eliminated 5 duplications
└── Found during TEST1 implementation

TECH3: Developer adds complex items via builder pattern
├── Replaces 9-parameter functions
└── Improves readability

TECH4: System validates sort_by values explicitly
├── Fail-fast principle
└── Prevents silent failures

TECH5: Developer adds new tools without modifying routing
├── Tool registry pattern
└── Enables plugin architecture
```

**Total Effort**: 5 items (technical investment)

---

## Epic 10: Deployment & Infrastructure (Completed ✅ 2025-11-10)

**Goal**: Production-ready deployment to Scaleway

```
DEPLOY1: Developer deploys to Scaleway Containers in <5min
├── Automated deployment script
└── Foundation for DEPLOY2

DEPLOY2: System deploys to Scaleway Containers successfully
├── Production endpoint: flyagileapipx8njvei-miro-mcp.functions.fnc.fr-par.scw.cloud
└── MCP protocol integration (PROTO1)

CI1: Developer receives automated test feedback on every push
├── GitHub Actions configured
└── Automated quality gate
```

**Total Effort**: 3 items

---

## Epic 11: Testing & Quality (Completed ✅ 2025-11-10)

**Goal**: Comprehensive test coverage and validation

```
TEST1: Parent filtering verified through integration tests
├── Exposed TECH2 duplication issue
└── Real Miro API validation

PROTO1: Claude.ai connects via MCP protocol
├── JSON-RPC 2.0 implementation
└── Critical for web interface integration
```

**Total Effort**: 2 items

---

## Epic 12: Observability (Completed ✅ 2025-11-11)

**Goal**: Production debugging and operational visibility

```
OBS1: Developer diagnoses production auth failures through structured logging
├── Correlation IDs for request tracing
├── Structured JSON logging
├── DEBUGGING.md runbook
└── <2min root cause identification
```

**Total Effort**: 1 item

---

## Epic 13: Production Readiness (In Progress ⏳)

**Goal**: Complete OAuth proxy implementation and secure deployment

### Testing & Validation
```
TEST2: Claude.ai discovers OAuth capability automatically
├── Metadata endpoint validation
├── Auto-discovery verification
└── Dependency: AUTH14 (metadata updated for proxy pattern)
└── Complexity: 1.0

TEST3: Developer validates OAuth flow end-to-end with Claude.ai
├── Full proxy flow verification
├── Production environment testing
└── Dependencies: AUTH10-14, DEPLOY2
└── Complexity: 1.0
```

### Security & Deployment
```
DEPLOY4: Sensitive credentials protected in Scaleway deployment
├── MIRO_CLIENT_SECRET in Secret Manager
├── MIRO_ENCRYPTION_KEY in Secret Manager (new for AUTH12)
├── Secret rotation procedure
└── Dependencies: AUTH10-14
└── Complexity: 1.0
└── Priority: P0 - Replaces SEC1 with expanded scope
```

### Optimization
```
TECH6: Developer rebuilds production container in <30s (vs 90s currently)
├── Optimized Cargo feature flags
├── 66% build time reduction
├── 40% binary size reduction
└── Complexity: 0.5

DEPLOY3: Developer knows deployment succeeded in <30s
├── Automated endpoint validation
├── Eliminates 5-min manual testing loop
└── Complexity: 0.5
```

### Documentation
```
DOC1: New developer understands OAuth2 architecture in <30min
├── Architecture section in README
├── Sequence diagrams
├── ADR-004 proxy pattern explanation
└── Complexity: 0.5

DOC2: Developer deploys to production in <10min
├── Deployment guide
├── Secret management procedures
├── Troubleshooting runbook
└── Complexity: 0.5
```

**Total Effort**: 7 items planned (5 P0, 2 optimization, 2 documentation)

---

## Historical Summary

**Completed Work** (27 items):
- Authentication: AUTH1-9 (9 items) - Evolved through 3 architectural patterns
- Board Management: BOARD1-2 (2 items)
- Visual Elements: VIS1-4 (4 items)
- Relationships: REL1-2 (2 items)
- Item Operations: ITEM1-3 (3 items)
- Frame Operations: FRAME1-4 (4 items)
- Layering: LAYER2 (1 item, 2 blocked)
- Bulk Operations: BULK1 (1 item)
- Technical: TECH1-5 (5 items)
- Deployment: DEPLOY1-2, CI1 (3 items)
- Testing: TEST1, PROTO1 (2 items)
- Observability: OBS1 (1 item)

**In Progress** (7 items):
- Authentication Proxy: AUTH10-14 (5 items) - ADR-004 pivot
- Testing: TEST2, TEST3 (2 items)

**Planned** (6 items):
- Security: DEPLOY4 (1 item, replaces SEC1)
- Optimization: TECH6, DEPLOY3 (2 items)
- Documentation: DOC1, DOC2 (2 items)

**Total**: 40 items (27 completed, 7 in progress, 6 planned)

---

## Architectural Evolution

**Phase 1**: Basic OAuth (AUTH1-2)
- Standard authorization code flow
- Token refresh

**Phase 2**: Stateless Architecture - ADR-002 (AUTH3-6)
- Encrypted cookie storage
- Horizontal scaling compatible
- **Result**: Good for Claude Desktop, NOT compatible with Claude.ai web

**Phase 3**: HTTP Resource Server - ADR-003 (AUTH7-9)
- Bearer token pattern
- Token introspection
- Performance caching
- **Result**: Assumes Claude handles OAuth, but Claude.ai doesn't support this

**Phase 4**: Proxy OAuth - ADR-004 (AUTH10-14) ⏳
- OUR server proxies OAuth between Claude.ai and Miro
- Combines stateless patterns (AUTH4 cookies) with proxy endpoints
- **Result**: Should work with Claude.ai web (validation pending TEST3)

**Key Learning**: OAuth integration evolved through 3 major architectural shifts as requirements clarified (Claude Desktop → Claude.ai web → production compatibility)

---

## Next Steps

1. **Complete AUTH10-14**: Implement Proxy OAuth pattern
2. **Validate with TEST2-3**: End-to-end testing with Claude.ai
3. **Secure with DEPLOY4**: Secret Manager integration
4. **Optimize with TECH6, DEPLOY3**: Build and deployment improvements
5. **Document with DOC1-2**: Architecture and deployment guides

**Expected Timeline**: 2-3 days to production readiness
