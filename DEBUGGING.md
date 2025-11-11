# Emergency Debugging Procedures - Production Auth Failures

## Overview

This document provides procedures for debugging authentication failures in the Miro MCP Server deployed on Scaleway Containers. All authentication events are logged with correlation IDs and structured fields for traceability.

---

## Quick Diagnosis Flowchart

```
Auth Failure Reported
    ↓
1. Find request_id from user/error message
    ↓
2. Search logs in Scaleway Cockpit for request_id
    ↓
3. Identify auth_stage where failure occurred
    ├─ token_extraction → Missing/malformed Authorization header
    ├─ token_validation → Invalid token or Miro API error
    └─ http_request     → Network issue contacting Miro API
    ↓
4. Follow stage-specific procedure below
```

---

## Log Structure

All logs include structured fields for filtering:

### Request Lifecycle Fields
- `request_id`: Unique UUID for each HTTP request (correlation ID)
- `method`: HTTP method (GET, POST, etc.)
- `uri`: Request URI path

### Authentication Fields
- `auth_stage`: Stage where auth processing occurred
  - `token_extraction`: Bearer token parsing from Authorization header
  - `token_validation`: Token validation with Miro API
- `user_id`: Miro user ID (after successful auth)
- `team_id`: Miro team ID (after successful auth)
- `scopes`: OAuth scopes granted to token
- `error_type`: Classification of error for filtering
  - `invalid_token`: Token rejected by Miro API (401)
  - `http_request_failed`: Network error calling Miro API
  - `api_error`: Miro API returned non-2xx status
  - `json_parse_failed`: Failed to parse Miro API response

---

## Accessing Logs in Scaleway Cockpit

### Step 1: Access Scaleway Console

1. Navigate to: https://console.scaleway.com
2. Login with Scaleway credentials
3. Go to: **Containers** → **miro-mcp-server** namespace
4. Click on deployed container: **server**
5. Click **Logs** tab

### Step 2: Filter by Request ID

If you have a `request_id` from the error report:

```
Filter: request_id="<uuid-from-error>"
Time range: Last 1 hour (or adjust based on error timestamp)
```

Example:
```
request_id="a1b2c3d4-e5f6-7890-abcd-ef1234567890"
```

This shows ALL log entries for that specific request across its entire lifecycle.

### Step 3: Filter by Error Type

To find all recent auth failures:

```
Filter: auth_stage AND error_type
Time range: Last 24 hours
```

Common filters:
- `error_type="invalid_token"` - Expired or revoked tokens
- `error_type="http_request_failed"` - Network issues to Miro API
- `auth_stage="token_extraction"` - Malformed Authorization headers

---

## Common Failure Scenarios

### Scenario 1: Missing Authorization Header

**Symptoms:**
- Error message: "Bearer token extraction failed"
- `auth_stage="token_extraction"`
- No `user_id` in logs

**Log Example:**
```json
{
  "timestamp": "2025-11-11T10:30:45Z",
  "level": "WARN",
  "request_id": "a1b2c3d4-...",
  "auth_stage": "token_extraction",
  "error": "Missing Authorization header",
  "message": "Bearer token extraction failed"
}
```

**Root Causes:**
1. Client not sending Authorization header
2. Header name misspelled (e.g., "Authorisation")
3. Missing "Bearer " prefix in header value

**Debugging Steps:**
1. Check client code for header construction
2. Verify header format: `Authorization: Bearer <token>`
3. Test with curl:
   ```bash
   curl -H "Authorization: Bearer <token>" https://<container-url>/health
   ```

**Resolution:**
- Fix client to send proper Authorization header
- Ensure "Bearer " prefix (with space) before token

---

### Scenario 2: Invalid or Expired Token

**Symptoms:**
- Error message: "Token validation failed"
- `auth_stage="token_validation"`
- `error_type="invalid_token"`
- Miro API returned 401 Unauthorized

**Log Example:**
```json
{
  "timestamp": "2025-11-11T10:35:12Z",
  "level": "WARN",
  "request_id": "b2c3d4e5-...",
  "auth_stage": "token_validation",
  "error_type": "invalid_token",
  "status": "401",
  "message": "Token validation failed: 401 Unauthorized from Miro API"
}
```

**Root Causes:**
1. Token expired (OAuth2 access tokens typically expire after 1 hour)
2. Token revoked by user in Miro settings
3. Token invalidated after password change
4. Token belongs to different Miro team/workspace

**Debugging Steps:**
1. Search logs for recent successful auth from same user:
   ```
   Filter: user_id="<user-id>" AND message="Request authenticated successfully"
   Time range: Last 7 days
   ```
2. Check if token was valid previously (indicates expiry vs revocation)
3. Verify token refresh mechanism in client

**Resolution:**
- Client must implement token refresh flow
- Use refresh token to obtain new access token
- Never hardcode tokens (always use OAuth2 flow)

---

### Scenario 3: Miro API Unavailable

**Symptoms:**
- Error message: "Failed to call Miro token endpoint"
- `auth_stage="token_validation"`
- `error_type="http_request_failed"`

**Log Example:**
```json
{
  "timestamp": "2025-11-11T10:40:30Z",
  "level": "WARN",
  "request_id": "c3d4e5f6-...",
  "auth_stage": "token_validation",
  "error_type": "http_request_failed",
  "endpoint": "https://api.miro.com/v1/oauth-token",
  "error": "connection timeout",
  "message": "Failed to call Miro token endpoint"
}
```

**Root Causes:**
1. Miro API experiencing downtime
2. Network connectivity issue from Scaleway
3. DNS resolution failure
4. Firewall blocking outbound HTTPS

**Debugging Steps:**
1. Check Miro API status: https://status.miro.com
2. Test connectivity from container:
   ```bash
   # Via Scaleway Console → Container → Terminal
   curl -v https://api.miro.com/v1/oauth-token
   ```
3. Check for pattern of failures:
   ```
   Filter: error_type="http_request_failed"
   Time range: Last 1 hour
   ```
   - Isolated failures → Client network issue
   - Widespread failures → Miro API or container networking issue

**Resolution:**
- If Miro API issue: Wait for Miro to resolve
- If network issue: Check Scaleway network configuration
- If DNS issue: Verify container DNS settings
- Implement retry logic with exponential backoff in client

---

### Scenario 4: Cache-Related Issues

**Symptoms:**
- Token works initially, then fails despite being valid
- Inconsistent auth behavior
- `cached_at` timestamp in logs shows stale cache

**Log Example:**
```json
{
  "timestamp": "2025-11-11T10:45:00Z",
  "level": "DEBUG",
  "message": "Cached token expired, revalidating"
}
```

**Root Causes:**
1. LRU cache eviction (100 entry limit)
2. Cache TTL expired (5 minutes)
3. Container restart cleared in-memory cache

**Debugging Steps:**
1. Search for cache hit/miss patterns:
   ```
   Filter: message="Token validation cache hit" OR message="Token validation cache miss"
   Time range: Last 1 hour
   ```
2. Look for container restarts:
   ```
   Filter: message="Starting ADR-002 Resource Server"
   Time range: Last 24 hours
   ```

**Resolution:**
- Expected behavior: Cache misses trigger Miro API validation
- If excessive cache misses → Consider increasing TTL (requires code change)
- If container restarts → Investigate deployment stability

---

## Advanced Debugging Techniques

### Tracing Full Request Lifecycle

Use `request_id` to see complete request flow:

```
Filter: request_id="<uuid>"
Sort: Timestamp ascending
```

Expected log sequence for successful auth:
1. "Request started" (correlation_id_middleware)
2. "Calling Miro token validation endpoint" (token_validator)
3. "Miro API returned valid token information" (token_validator)
4. "Token validated successfully" (token_validator)
5. "Request authenticated successfully" (bearer_auth_middleware)
6. "Processing MCP request" (mcp_endpoint)
7. "Request completed" (correlation_id_middleware)

Missing steps indicate where failure occurred.

### Analyzing Error Patterns

Find all auth failures in last 24 hours:

```
Filter: auth_stage AND level="WARN"
Time range: Last 24 hours
Group by: error_type
```

High counts per error_type indicate:
- `invalid_token` (>10/hour) → Token refresh issue
- `http_request_failed` (>5/hour) → Miro API or network issue
- Token extraction errors → Client implementation bug

### Correlating with User Reports

User reports: "Auth failed at 10:30 AM"

1. Convert to UTC (Scaleway logs use UTC)
2. Search ±15 minutes around reported time
3. Filter by method/uri if known:
   ```
   Filter: method="POST" AND uri="/mcp"
   Time range: 2025-11-11 10:15:00 to 10:45:00
   ```

---

## Configuring Log Levels

### Development (Local)

Default: `RUST_LOG=miro_mcp_server=info`

Enable debug logging:
```bash
RUST_LOG=miro_mcp_server=debug cargo run --bin server --no-default-features
```

Enable trace logging (very verbose):
```bash
RUST_LOG=miro_mcp_server=trace cargo run --bin server --no-default-features
```

### Production (Scaleway)

Set via Scaleway Console:
1. Go to: Containers → miro-mcp-server → server
2. Click: Environment Variables
3. Add/Update:
   - `RUST_LOG=miro_mcp_server=debug` (temporary for debugging)
   - `LOG_FORMAT=json` (structured logs for Cockpit)
4. Click: Deploy (restarts container with new settings)

**WARNING:** Debug/trace logging increases volume. Return to `info` after debugging.

---

## Log Persistence

### Scaleway Cockpit Retention

- **Default retention**: 7 days
- **Logs persist across container restarts**: Yes (Scaleway Cockpit is external to container)
- **Logs lost on container deletion**: Yes (if namespace deleted)

### Backup Recommendations

For long-term retention or compliance:

1. **Scaleway Logs Export** (if available):
   - Configure log forwarding to S3/Object Storage
   - Retention: Indefinite (until manually deleted)

2. **Manual Export**:
   - Cockpit UI: Export logs → JSON/CSV
   - Frequency: Weekly (for audit trail)

3. **Alerting** (future enhancement):
   - Configure alerts for `error_type="invalid_token"` threshold
   - Proactive notification before user reports issue

---

## Escalation Procedures

### When to Escalate

1. **Widespread auth failures** (>50% of requests failing)
2. **Miro API downtime** (confirmed via status.miro.com)
3. **Container networking issues** (cannot reach Miro API)
4. **Unknown error patterns** (new error_type not documented)

### Escalation Steps

1. **Document evidence**:
   - Export relevant logs (JSON format)
   - Screenshot Scaleway Cockpit filters
   - Note request_id examples
   - Record timeline (when started, frequency)

2. **Check external status**:
   - Miro API: https://status.miro.com
   - Scaleway status: https://status.scaleway.com

3. **Contact support**:
   - **Miro API issues**: Miro developer support
   - **Scaleway issues**: Scaleway support ticket
   - Provide: Container ID, region, exported logs

---

## Prevention and Monitoring

### Proactive Measures

1. **Client-side token refresh**:
   - Implement refresh before expiry (58 minutes for 1-hour tokens)
   - Graceful handling of 401 errors (auto-refresh + retry)

2. **Health monitoring**:
   - Regular `/health` checks (every 60 seconds)
   - Alert on consecutive failures (3+ in 5 minutes)

3. **Log monitoring**:
   - Daily review of `error_type` counts
   - Investigate spikes in auth failures

### Useful Queries for Daily Monitoring

**Auth failure summary (last 24h):**
```
Filter: level="WARN" AND auth_stage
Time range: Last 24 hours
Group by: error_type
Count
```

**Unique users authenticated (last 24h):**
```
Filter: message="Request authenticated successfully"
Time range: Last 24 hours
Count distinct: user_id
```

**Cache hit rate (last 1h):**
```
Filter: message="Token validation cache hit" OR message="Token validation cache miss"
Time range: Last 1 hour
Count each
Calculate: hit_rate = hits / (hits + misses)
```

---

## Appendix: Log Field Reference

### Structured Fields (Always Present)

| Field       | Type   | Example                              | Description                     |
|-------------|--------|--------------------------------------|---------------------------------|
| timestamp   | string | "2025-11-11T10:30:45Z"               | UTC timestamp                   |
| level       | string | "INFO", "WARN", "ERROR", "DEBUG"     | Log level                       |
| message     | string | "Request authenticated successfully" | Human-readable message          |

### HTTP Request Fields (request lifecycle)

| Field      | Type   | Example              | Description           |
|------------|--------|----------------------|-----------------------|
| request_id | string | "a1b2c3d4-..."       | Correlation ID (UUID) |
| method     | string | "POST"               | HTTP method           |
| uri        | string | "/mcp"               | Request URI path      |

### Authentication Fields (auth flow)

| Field      | Type   | Example                    | Description                        |
|------------|--------|----------------------------|------------------------------------|
| auth_stage | string | "token_validation"         | Auth processing stage              |
| error_type | string | "invalid_token"            | Error classification               |
| user_id    | string | "3458764647516852398"      | Miro user ID (post-auth)           |
| team_id    | string | "3458764647516852399"      | Miro team ID (post-auth)           |
| scopes     | array  | ["boards:read", "boards:write"] | OAuth scopes granted          |

### API Call Fields (Miro API validation)

| Field    | Type   | Example                                  | Description              |
|----------|--------|------------------------------------------|--------------------------|
| endpoint | string | "https://api.miro.com/v1/oauth-token"    | Miro API endpoint called |
| status   | number | 401                                      | HTTP status code         |
| error    | string | "connection timeout"                     | Error detail             |

---

## Changelog

| Date       | Change                                  | Author     |
|------------|-----------------------------------------|------------|
| 2025-11-11 | Initial version (OBS1 implementation)   | System     |

---

## Related Documentation

- [Deployment Guide](planning/deployment.md) - Scaleway deployment procedures
- [Architecture Decision Record 003](planning/adr-003-dual-mode-architecture.md) - Dual-mode architecture
- [Miro OAuth2 Documentation](https://developers.miro.com/docs/oauth-20) - Miro auth reference
