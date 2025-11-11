# Miro MCP Server - User Stories

### AUTH1: User authenticates with Miro securely via OAuth2

**User**: Claude AI assistant accessing Miro on behalf of human user
**Outcome**: Establishes secure authenticated session with Miro API
**Context**: Currently no MCP server exists for Miro with OAuth2 support, only static token implementations

**Acceptance Criteria**:
- OAuth2 authorization code flow implemented correctly
- Authorization URL generated with proper client_id and scopes
- Token exchange endpoint working (code → access_token)
- Access token stored securely
- All Miro API requests use Bearer token authentication
- Error handling for invalid/expired tokens

**Implementation Notes**:
- Use authorization endpoint: https://miro.com/oauth/authorize
- Token exchange endpoint: https://api.miro.com/v1/oauth/token
- Client credentials provided in requirements
- Follow MCP OAuth2 specification for remote servers

**Source**: PROJECT_REQUIREMENTS

---

### AUTH2: System refreshes tokens automatically (vs manual re-auth)

**User**: Claude AI assistant maintaining long-running session
**Outcome**: Continues working without user re-authentication when token expires
**Context**: Access tokens expire after 3600 seconds, causing session interruption

**Acceptance Criteria**:
- Refresh token stored securely alongside access token
- System detects 401 Unauthorized responses
- Automatic token refresh triggered before/on expiration
- New access_token obtained using refresh_token
- Failed operations retried automatically after refresh
- User not interrupted for re-authentication

**Implementation Notes**:
- Monitor token expiry time (expires_in field)
- Implement refresh flow: grant_type=refresh_token
- Handle refresh token expiration gracefully (fallback to full re-auth)

**Source**: PROJECT_REQUIREMENTS

---

### AUTH3: User completes OAuth flow in browser from Claude Desktop (vs manual token management)

**User**: Human user connecting Claude Desktop to Miro
**Outcome**: Seamlessly authorizes access through browser OAuth flow without manual token handling
**Context**: Claude Desktop requires local HTTP server for OAuth callback handling

**Acceptance Criteria**:
- HTTP server runs on localhost:3010 for OAuth callbacks
- `start_auth` MCP tool generates authorization URL
- User clicks URL, opens browser, authorizes on Miro
- Miro redirects to http://localhost:3010/oauth/callback?code=xxx
- HTTP server exchanges code for tokens automatically
- Tokens saved to encrypted storage
- Success/error feedback displayed in browser
- MCP server uses saved tokens for subsequent API calls

**Implementation Notes**:
- Concurrent server architecture: stdio MCP + HTTP OAuth server
- HTTP server spawned in background tokio task
- Shared OAuth client and token store between servers
- Beautiful HTML pages for user feedback
- Docker port mapping: -p 3010:3010 for callback access

**Source**: VAULT_SERVER_OAUTH_LEARNINGS

---

### BOARD1: User lists accessible Miro boards programmatically

**User**: Claude AI assistant helping user find existing Miro boards
**Outcome**: Retrieves list of boards user can access via simple prompt
**Context**: User needs to discover boards before adding content to them

**Acceptance Criteria**:
- MCP tool `list_boards` exposed to Claude
- GET /v2/boards endpoint called successfully
- Returns board id, name, description, creation date
- Pagination supported for users with many boards
- Error handling for API failures

**Implementation Notes**:
- Endpoint: GET https://api.miro.com/v2/boards
- Response includes board metadata
- May need cursor-based pagination

**Source**: PROJECT_REQUIREMENTS (Miro API v2)

---

### BOARD2: User creates new boards via Claude prompts

**User**: Claude AI assistant creating visualization workspace for user
**Outcome**: New Miro board created with custom name and description
**Context**: User wants to start fresh board for agile squad visualization

**Acceptance Criteria**:
- MCP tool `create_board` exposed to Claude
- POST /v2/boards endpoint called successfully
- Accepts name and description parameters
- Returns new board id for subsequent operations
- Board accessible in user's Miro account

**Implementation Notes**:
- Endpoint: POST https://api.miro.com/v2/boards
- Minimal required: name field
- Optional: description, team_id

**Source**: PROJECT_REQUIREMENTS

---

### VIS1: User creates sticky notes with custom content and styling

**User**: Claude AI assistant building agile team visualization
**Outcome**: Sticky notes appear on board with role names, colors, and positions
**Context**: Sticky notes represent team members in organizational structure

**Acceptance Criteria**:
- MCP tool `create_sticky_note` exposed to Claude
- POST /v2/boards/{board_id}/sticky_notes endpoint called successfully
- Accepts: content (HTML), position (x, y), color, size
- Returns created item id
- Sticky note visible on specified board
- Supports all available colors (light_yellow, green, blue, etc.)

**Implementation Notes**:
- Endpoint: POST /v2/boards/{board_id}/sticky_notes
- Content supports HTML formatting (<p><strong>Name</strong></p><p>Role</p>)
- Position origin: "center"
- Default size: 200x200

**Source**: PROJECT_REQUIREMENTS (Primary use case)

---

### VIS2: User creates shapes for organizational structure

**User**: Claude AI assistant visualizing squad boundaries
**Outcome**: Shapes (rectangles, circles) created to represent squads or roles
**Context**: Shapes define organizational containers and structure

**Acceptance Criteria**:
- MCP tool `create_shape` exposed to Claude
- POST /v2/boards/{board_id}/shapes endpoint called successfully
- Supports shape types: rectangle, circle
- Accepts: content, position, color, border style, size
- Returns created item id
- Shape appears on board with correct styling

**Implementation Notes**:
- Endpoint: POST /v2/boards/{board_id}/shapes
- More color options than sticky notes (white, black, gradients)
- Border width and color configurable
- Common use: squad containers (rectangles)

**Source**: PROJECT_REQUIREMENTS

---

### VIS4: User creates frames for grouping related content

**User**: Claude AI assistant organizing squad visualizations
**Outcome**: Frames created to group team members and structure
**Context**: Frames provide visual containers for entire squad

**Acceptance Criteria**:
- MCP tool `create_frame` exposed to Claude
- POST /v2/boards/{board_id}/frames endpoint called successfully
- Accepts: title, position, size, fill color
- Returns created item id
- Frame contains items positioned within boundaries
- Frame visible with title

**Implementation Notes**:
- Endpoint: POST /v2/boards/{board_id}/frames
- Large size recommended (e.g., 1000x800) for squad containers
- Items placed inside frame bounds automatically grouped

**Source**: PROJECT_REQUIREMENTS

---

### REL1: User connects items with styled arrows/lines

**User**: Claude AI assistant showing reporting relationships
**Outcome**: Connectors drawn between items to show organizational hierarchy
**Context**: Visual lines indicate "reports to" or "depends on" relationships

**Acceptance Criteria**:
- MCP tool `create_connector` exposed to Claude
- POST /v2/boards/{board_id}/connectors endpoint called successfully
- Accepts: start_item_id, end_item_id, style (color, width, end cap)
- Returns created connector id
- Line/arrow appears connecting specified items
- Supports various end caps (arrow, none, diamond)

**Implementation Notes**:
- Endpoint: POST /v2/boards/{board_id}/connectors
- Style options: stroke color, width, cap style
- Caption support for labeling relationships

**Source**: PROJECT_REQUIREMENTS (Primary use case)

---

### REL2: User adds captions to connectors

**User**: Claude AI assistant labeling relationship types
**Outcome**: Text captions appear on connectors explaining relationships
**Context**: "reports to", "depends on", "collaborates with" need clear labels

**Acceptance Criteria**:
- Caption text configurable when creating connector
- Caption position adjustable (0.0 to 1.0 along line)
- Multiple captions per connector supported
- Text readable and styled appropriately

**Implementation Notes**:
- Captions array in connector creation request
- Position: 0.5 = middle of line
- Each caption has content and position

**Source**: PROJECT_REQUIREMENTS

---

### BULK1: User creates multiple items efficiently (vs individual API calls)

**User**: Claude AI assistant creating complex visualizations
**Outcome**: Multiple items created in single transaction reducing latency
**Context**: Creating 20+ items individually is slow and hits rate limits

**Acceptance Criteria**:
- MCP tool `bulk_create_items` exposed to Claude
- Accepts array of items (max 20 per call per API limit)
- Single API transaction creates all items atomically
- Returns array of created item ids
- Significantly faster than individual calls (>50% time reduction)

**Implementation Notes**:
- Endpoint: POST /v2/boards/{board_id}/items with array payload
- Error handling: partial success vs total failure
- May need to batch if user requests >20 items

**Source**: PROJECT_REQUIREMENTS (Bulk Operations)

---

### ITEM1: User lists board items filtered by type

**User**: Claude AI assistant discovering existing board content
**Outcome**: Retrieves specific item types (frames, sticky notes, shapes) for analysis
**Context**: User wants to understand current board state or modify specific items

**Acceptance Criteria**:
- MCP tool `list_items` exposed to Claude
- GET /v2/boards/{board_id}/items endpoint called successfully
- Supports type filtering: ?type=frame,sticky_note,shape,text,connector
- Returns item metadata (id, type, position, content)
- Pagination supported for boards with many items

**Implementation Notes**:
- Endpoint: GET /v2/boards/{board_id}/items
- Query parameter: type for filtering
- Cursor-based pagination likely needed

**Source**: PROJECT_REQUIREMENTS

---

### ITEM2: User updates item properties dynamically

**User**: Claude AI assistant modifying board content based on feedback
**Outcome**: Item position, content, or style updated without recreation
**Context**: User wants to adjust visualization after initial creation

**Acceptance Criteria**:
- MCP tool `update_item` exposed to Claude
- PATCH /v2/boards/{board_id}/items/{item_id} endpoint called successfully
- Supports updating: position, content, style, geometry
- Partial updates supported (only changed fields)
- Item reflects changes immediately

**Implementation Notes**:
- Endpoint: PATCH /v2/boards/{board_id}/items/{item_id}
- JSON patch format
- Not all properties may be mutable

**Source**: PROJECT_REQUIREMENTS

---

### ITEM3: User removes items from boards

**User**: Claude AI assistant cleaning up or reorganizing board
**Outcome**: Specified items deleted from board
**Context**: User wants to remove incorrect or obsolete items

**Acceptance Criteria**:
- MCP tool `delete_item` exposed to Claude
- DELETE /v2/boards/{board_id}/items/{item_id} endpoint called successfully
- Item removed from board permanently
- Returns success confirmation
- Error handling for non-existent items

**Implementation Notes**:
- Endpoint: DELETE /v2/boards/{board_id}/items/{item_id}
- 204 No Content on success
- Consider cascade effects (deleting frame with items inside)

**Source**: PROJECT_REQUIREMENTS

---

### VIS3: User creates text elements on boards

**User**: Claude AI assistant adding labels or descriptions
**Outcome**: Text items created for titles, labels, or explanations
**Context**: Additional text needed beyond sticky note/shape content

**Acceptance Criteria**:
- MCP tool `create_text` exposed to Claude
- POST /v2/boards/{board_id}/texts endpoint called successfully
- Accepts: content (plain or HTML), position, size
- Returns created item id
- Text appears on board at specified location

**Implementation Notes**:
- Endpoint: POST /v2/boards/{board_id}/texts
- Less commonly used than sticky notes (which can contain text)
- Useful for standalone labels

**Source**: PROJECT_REQUIREMENTS

---

### DEPLOY1: Developer deploys to Scaleway in <5min (vs manual local setup)

**User**: Developer deploying Miro MCP server to production
**Outcome**: Container deployed to Scaleway Container Registry with automated configuration
**Context**: Currently requires manual local setup, environment configuration, and no production deployment option

**Acceptance Criteria**:
- Dockerfile builds MCP server successfully
- Container pushed to Scaleway Container Registry
- Environment variables (MIRO_CLIENT_ID, MIRO_CLIENT_SECRET) configurable via Scaleway secrets
- Container runs MCP server accessible via network
- Deployment documented with single-command setup
- Health check endpoint responds correctly
- OAuth redirect URI configured for production domain

**Implementation Notes**:
- Use scaleway-deployment skill for Scaleway-specific setup
- Follow Scaleway Container best practices
- Consider using Scaleway Serverless Containers for auto-scaling
- Token storage needs persistent volume or Scaleway Object Storage

**Source**: USER_REQUEST

---

### PROTO1: Claude.ai connects via MCP protocol (vs REST-only server)

**User**: Claude.ai (AI assistant) attempting to connect to MCP server from web interface
**Outcome**: Successfully initializes MCP session, discovers tools, and executes tool calls via JSON-RPC 2.0 protocol
**Context**: Server currently implements REST API (GET /health, Bearer token validation) but Claude.ai expects MCP protocol (POST /mcp with JSON-RPC requests). Connection fails because server doesn't speak MCP protocol.

**Acceptance Criteria**:
- POST /mcp endpoint accepts JSON-RPC 2.0 requests
- Request with `initialize` method returns ServerCapabilities:
  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
      "protocolVersion": "2024-11-05",
      "capabilities": { "tools": {} },
      "serverInfo": { "name": "miro-mcp-server", "version": "0.1.0" }
    }
  }
  ```
- Request with `tools/list` method returns 2 tools (list_boards, get_board) with proper JSON schemas
- Request with `tools/call` method executes list_boards and returns Miro API response
- Bearer token extracted from Authorization header and validated before processing
- All responses follow JSON-RPC 2.0 format (id matches request, jsonrpc="2.0", result or error)
- Error responses use JSON-RPC error codes:
  - -32700: Parse error (invalid JSON)
  - -32600: Invalid request (missing jsonrpc/method)
  - -32601: Method not found (unknown method)
  - -32602: Invalid params
  - 401: Unauthorized (missing/invalid Bearer token)
- Integration test verifies full flow: initialize → tools/list → tools/call

**Implementation Notes**:
- Manual implementation (no rmcp library, stable Rust only per user decision)
- Estimated 200 LOC, 6-8h implementation (per solution-architect plan)
- POST /mcp handles all JSON-RPC methods (single endpoint, method routing)
- JSON-RPC request format:
  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {...} }
  }
  ```
- Tool call format:
  ```json
  {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": { "name": "list_boards", "arguments": {} }
  }
  ```
- Reuse existing Bearer token validation from AUTH7/AUTH9
- Tool execution delegates to existing Miro API client
- Error handling must distinguish protocol errors (JSON-RPC) from application errors (Miro API)

**Technical Approach**:
1. Add `POST /mcp` route to Axum router
2. Parse JSON-RPC request (jsonrpc, id, method, params)
3. Route by method: initialize → capabilities, tools/list → tool schemas, tools/call → tool execution
4. Validate Bearer token before processing (reuse existing middleware)
5. Execute tool via existing MiroCient
6. Format response as JSON-RPC (success or error)
7. Integration test with real JSON-RPC requests

**Source**: USER_REQUEST

---
### AUTH4: Developer adds OAuth state via encrypted cookies (vs in-memory HashMap)

**User**: Developer implementing stateless OAuth flow
**Outcome**: OAuth state persisted across requests without server-side storage
**Context**: In-memory HashMap loses state on container restart, blocking OAuth completion

**Acceptance Criteria**:
- Encrypted cookie stores OAuth state (PKCE verifier, nonce)
- AES-256-GCM encryption implemented
- HttpOnly, Secure, SameSite=Lax flags set
- Cookie expiry: 5 minutes (OAuth flow duration)
- State validated on callback to prevent CSRF

**Implementation Notes**:
- Foundation for stateless architecture (ADR-003)
- Enables horizontal scaling on Scaleway Containers
- Reusable for ACCESS_TOKEN cookie storage (AUTH5)

**Source**: ADR-003 decision (stateless OAuth)

---

### AUTH5: User's access token stored in encrypted cookies (vs server-side storage)

**User**: Claude Desktop/Web maintaining authenticated session
**Outcome**: Access token available across requests without database
**Context**: Stateless architecture requires client-side token storage

**Acceptance Criteria**:
- Encrypted cookie stores access_token securely
- Token encrypted with AES-256-GCM
- HttpOnly flag prevents JavaScript access
- Cookie expiry matches token expiry (3600s)
- Refresh token stored separately (higher security)

**Implementation Notes**:
- Note: Superseded by ADR-003 (HTTP Resource Server mode uses Bearer tokens)
- Cookie approach still useful for local development (Claude Desktop)
- Production uses Authorization: Bearer header (AUTH7)

**Source**: ADR-003 evolution

---

### AUTH6: Claude discovers OAuth via metadata endpoint (vs manual configuration)

**User**: Claude.ai attempting to connect to MCP server
**Outcome**: Automatically discovers OAuth endpoints and initiates flow
**Context**: MCP spec requires /.well-known/oauth-protected-resource endpoint

**Acceptance Criteria**:
- GET /.well-known/oauth-protected-resource returns valid JSON
- Metadata includes: authorization_endpoint, token_endpoint, issuer
- Endpoints point to correct URLs (Miro or proxy depending on pattern)
- Claude.ai successfully discovers and uses endpoints
- HTTPS accessible from internet

**Implementation Notes**:
- RFC 8414 OAuth 2.0 Authorization Server Metadata compliance
- Critical for Claude.ai web integration
- Updated in ADR-004 to point to proxy endpoints (AUTH14)

**Source**: MCP OAuth2 specification

---

### AUTH7: Server extracts Bearer tokens from Authorization header (vs cookies)

**User**: Claude.ai sending authenticated requests to MCP server
**Outcome**: Server validates requests using Bearer token from header
**Context**: HTTP Resource Server pattern (ADR-003) requires Bearer tokens

**Acceptance Criteria**:
- Extract Authorization: Bearer <token> from HTTP headers
- Validate token format before introspection
- Return 401 Unauthorized if header missing
- Return 401 Unauthorized if token malformed
- Pass valid tokens to introspection (AUTH8)

**Implementation Notes**:
- Replaces cookie-based auth from AUTH5
- Standard OAuth2 Resource Server pattern
- Middleware applied to all MCP endpoints

**Source**: ADR-003 (HTTP Resource Server mode)

---

### AUTH8: Server validates tokens with Miro introspection API (vs trusting Claude)

**User**: MCP server verifying token authenticity
**Outcome**: Only valid Miro tokens accepted for API requests
**Context**: Cannot trust tokens from Claude.ai without validation

**Acceptance Criteria**:
- POST to Miro /v1/oauth/token/introspect endpoint
- Validate active=true in response
- Verify scopes match required permissions
- Return 401 if introspection fails or token inactive
- Handle Miro API errors gracefully

**Implementation Notes**:
- Security critical: prevents token forgery
- 100ms latency per validation (addressed in AUTH9)
- Uses MIRO_CLIENT_ID for introspection auth

**Source**: ADR-003 security requirement

---

### AUTH9: Token validation cached with 5-minute TTL (vs 100ms latency per request)

**User**: Developer experiencing fast MCP tool responses
**Outcome**: Sub-10ms authentication overhead (vs 100ms per request)
**Context**: Introspection API adds 100ms latency to every MCP call

**Acceptance Criteria**:
- In-memory cache for introspection results
- TTL: 5 minutes (balance security vs performance)
- Cache key: SHA-256 hash of token
- Cache invalidation on 401 from Miro API
- Performance: ≤10ms for cache hit

**Implementation Notes**:
- Uses DashMap for concurrent access
- Cache miss: 100ms (introspection API call)
- Cache hit: <1ms (in-memory lookup)
- Acceptable security trade-off: 5min window

**Source**: Performance optimization (ADR-003)

---

### FRAME1: User creates items directly in frames (vs manual move after creation)

**User**: Claude AI creating organized squad visualizations
**Outcome**: Items appear inside frames without extra move operation
**Context**: Cleaner API, reduces round-trips for grouped content

**Acceptance Criteria**:
- Create item with parent.id = frame_id in request
- Item appears inside specified frame automatically
- No separate move operation needed
- Works for sticky_note, shape, text, connector types
- Frame boundary enforced by Miro

**Implementation Notes**:
- Uses parent field in POST /v2/boards/{board_id}/items
- Simplifies bulk creation of squad structures
- Foundation for FRAME2-4 operations

**Source**: Miro API v2 feature

---

### FRAME2: User moves items between frames for reorganization

**User**: Claude AI reorganizing squad structures
**Outcome**: Items repositioned to different frames dynamically
**Context**: Users want to reorganize without recreating items

**Acceptance Criteria**:
- PATCH /v2/boards/{board_id}/items/{item_id} with parent.id
- Item moves from current frame to target frame
- Item position updated relative to new parent
- Moving to board root: set parent to null
- Works across all item types

**Implementation Notes**:
- Extends ITEM2 (update operation)
- Parent change triggers Miro re-layout
- Critical for iterative squad refinement

**Source**: Miro API v2 feature

---

### FRAME3: User filters items by containing frame

**User**: Claude AI analyzing specific squad content
**Outcome**: Lists only items within specified frame
**Context**: Users want to inspect/modify specific squad without noise

**Acceptance Criteria**:
- GET /v2/boards/{board_id}/items?parent_id=<frame_id>
- Returns items contained in specified frame
- Excludes items in other frames or board root
- Supports pagination for large frames
- Works with type filtering (?type=sticky_note&parent_id=X)

**Implementation Notes**:
- Combines with ITEM1 filtering
- Enables focused squad operations
- Foundation for squad-level analytics

**Source**: Miro API v2 feature

---

### FRAME4: User removes items from frames to board root

**User**: Claude AI extracting items from squad containers
**Outcome**: Items moved to board root (ungrouped)
**Context**: Users want to ungroup items for reorganization

**Acceptance Criteria**:
- PATCH /v2/boards/{board_id}/items/{item_id} with parent=null
- Item moves from frame to board root
- Position preserved (or updated to absolute coordinates)
- Frame no longer owns item
- Item remains on board (not deleted)

**Implementation Notes**:
- Special case of FRAME2 (move to null parent)
- Inverse of FRAME1 operation
- Completes frame lifecycle

**Source**: Miro API v2 feature

---

### LAYER2: User understands item stacking order when reading/creating items

**User**: Claude AI creating layered visualizations
**Outcome**: Created items stack predictably (new items on top)
**Context**: Visual hierarchy matters for diagrams (backgrounds vs foregrounds)

**Acceptance Criteria**:
- GET /v2/boards/{board_id}/items returns items with position.z_index
- z_index indicates stacking order (higher = on top)
- Newly created items have highest z_index (appear on top)
- Reading order documented for users
- Note: Cannot programmatically change z-order (API limitation)

**Implementation Notes**:
- Read-only understanding (z-order control is Web SDK only)
- Inform users of stacking behavior
- Workaround: Delete and recreate in desired order

**Source**: Miro API v2 limitation (LAYER1.1/1.2 blocked)

---

### TECH2: Developer modifies parent construction in single location (vs 5 duplications)

**User**: Developer adding new item types or parent logic
**Outcome**: Parent field logic centralized for maintainability
**Context**: Parent construction duplicated across 5 item creation functions

**Acceptance Criteria**:
- Create `src/miro/parent.rs` module with `build_parent()` function
- All item creation calls `build_parent(parent_id: Option<String>)`
- Function returns: Some(Parent { id }) if parent_id provided, None otherwise
- Delete duplicated logic from sticky_note, shape, text, connector, frame
- Add unit tests for parent construction edge cases

**Implementation Notes**:
- DRY principle application
- Reduces bug surface (single source of truth)
- Enables future parent validation logic

**Source**: Code refactoring (TEST1 discovered duplication)

---

### TEST1: Parent filtering verified through integration tests (vs unit-only coverage)

**User**: Developer ensuring frame operations work end-to-end
**Outcome**: Integration tests validate real Miro API behavior
**Context**: Unit tests mocked parent behavior, missed API quirks

**Acceptance Criteria**:
- Integration test: Create frame, create item with parent_id
- Verify item appears inside frame (GET items?parent_id=<frame>)
- Test move between frames (PATCH parent.id)
- Test move to root (PATCH parent=null)
- Test filtering by parent_id
- All tests use real Miro API (test board)

**Implementation Notes**:
- Exposed TECH2 duplication during implementation
- Integration tests catch API contract changes
- Run against test Miro account

**Source**: Quality assurance (found during FRAME1-4 implementation)

---

### TECH4: System validates sort_by values explicitly (vs silent failures)

**User**: Developer catching invalid sort parameter early
**Outcome**: Clear error message for invalid sort_by instead of silent failure
**Context**: Miro API silently ignores invalid sort values

**Acceptance Criteria**:
- Validate sort_by against allowed values: ["created_at", "modified_at"]
- Return 400 Bad Request if invalid value provided
- Error message: "sort_by must be one of: created_at, modified_at"
- Add unit test for validation logic
- Document allowed values in tool schema

**Implementation Notes**:
- Fail-fast principle
- Improves developer experience
- Prevents silent bugs

**Source**: Code quality improvement

---

### DEPLOY2: System deploys to Scaleway Containers successfully

**User**: Developer deploying production MCP server
**Outcome**: MCP server running on Scaleway with HTTPS and OAuth
**Context**: DEPLOY1 planned Containers, DEPLOY2 executes deployment

**Acceptance Criteria**:
- Container image built and pushed to Scaleway registry
- Container deployed to Scaleway Containers platform
- HTTPS endpoint accessible: https://[container].scw.cloud
- Environment variables configured: MIRO_CLIENT_ID, MIRO_CLIENT_SECRET
- OAuth redirect URI points to production domain
- Health check endpoint responds (GET /health)
- MCP protocol endpoint responds (POST /mcp)

**Implementation Notes**:
- Uses deploy_scaleway.sh script
- Container: miro-mcp at flyagileapipx8njvei-miro-mcp.functions.fnc.fr-par.scw.cloud
- MCP protocol support added (PROTO1)
- Foundation for OAuth flow testing (TEST2, TEST3)

**Source**: Production deployment

---

### CI1: Developer receives automated test feedback on every push (vs manual local testing)

**User**: Developer pushing code changes
**Outcome**: GitHub Actions runs tests automatically, reports failures
**Context**: Manual testing slow and error-prone

**Acceptance Criteria**:
- .github/workflows/ci.yml configured
- Triggers on push to main and pull requests
- Runs: cargo test, cargo clippy, cargo fmt --check
- Reports test failures as GitHub status check
- Fast feedback: <5 minutes for standard PR
- Blocks merge if tests fail (optional branch protection)

**Implementation Notes**:
- Standard Rust CI setup
- Caches dependencies for speed
- Runs on ubuntu-latest
- Foundation for CD pipeline

**Source**: Development automation

---

### TECH3: Developer adds complex items via builder pattern (vs 9-parameter functions)

**User**: Developer creating items with many optional fields
**Outcome**: Readable, maintainable item creation code
**Context**: Functions with 9+ parameters unreadable and error-prone

**Acceptance Criteria**:
- Builder structs for StickyNote, Shape, Connector, Frame
- Fluent API: `StickyNote::new(board_id).content("text").color(Color::Yellow).build()`
- Required fields enforced at build time (compile-time safety)
- Optional fields have sensible defaults
- All existing item creation refactored to use builders
- Documentation examples use builder pattern

**Implementation Notes**:
- Rust builder pattern (derive-builder crate or manual)
- Improves readability of bulk creation (BULK1)
- Type-safe alternative to HashMap<String, Value>

**Source**: Code quality improvement

---

### TECH5: Developer adds new tools without modifying routing (vs hardcoded match)

**User**: Developer adding new MCP tools
**Outcome**: Tools registered declaratively, router auto-discovers
**Context**: Hardcoded match statement in mcp/server.rs requires edit for each tool

**Acceptance Criteria**:
- Tool registry pattern: `registry.register(ListBoardsTool)`
- Router iterates registry for tools/list
- Router dispatches tools/call via registry lookup
- Adding tool = implement trait + register (no router changes)
- All existing tools refactored to registry pattern

**Implementation Notes**:
- Trait-based tool abstraction
- Dynamic dispatch or macro-generated registry
- Reduces coupling between tools and router
- Enables plugin architecture (future)

**Source**: Architecture improvement

---

### OBS1: Developer diagnoses production auth failures through structured logging

**User**: Developer troubleshooting OAuth failures in production
**Outcome**: Quickly identifies root cause via correlation IDs and structured logs
**Context**: Production auth errors opaque without context

**Acceptance Criteria**:
- Correlation ID generated for each request
- Correlation ID propagated through OAuth flow (/authorize → /callback → /token)
- Structured JSON logging for all auth events:
  - OAuth initiation (correlation_id, timestamp)
  - Miro redirect (correlation_id, state, pkce_challenge)
  - Callback received (correlation_id, code, state_valid)
  - Token exchange (correlation_id, success/failure)
- Logs never contain tokens/secrets (even in debug mode)
- DEBUGGING.md runbook for common failure modes
- Test: Induce failure, grep logs by correlation_id, identify cause in <2 minutes

**Implementation Notes**:
- Uses tracing/slog for structured logging
- Correlation ID in X-Correlation-ID header
- Log levels: ERROR (failures), INFO (flow steps), DEBUG (internals)
- Production log retention: 7 days minimum

**Source**: Production debugging (completed 2025-11-11)

---
