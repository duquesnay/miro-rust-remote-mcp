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
