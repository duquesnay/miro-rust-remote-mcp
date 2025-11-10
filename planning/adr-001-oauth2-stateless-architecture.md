# ADR-001: OAuth2 Stateless Architecture for Remote MCP Server

**Status:** Accepted
**Date:** 2025-11-10
**Context:** Implementing OAuth2 authentication for remote MCP server deployment
**Decision Makers:** Solution Architect, Security Specialist, Architecture Reviewer, Integration Specialist

---

## Context and Problem Statement

We need to implement OAuth2 authentication for a remote MCP (Model Context Protocol) server with the following constraints:

- **Scale:** Personal use initially (1 user), potentially shared later (~10-100 users)
- **Deployment:** Serverless (AWS Lambda, Cloudflare Workers, or similar)
- **Cost:** Minimize infrastructure costs (avoid always-on services like Redis/databases)
- **Security:** Non-negotiable - must follow OAuth2 security best practices
- **Timeline:** MVP first, production-ready but not over-engineered

**Key Question:** Can we implement stateless remote MCP with OAuth2 securely?

---

## Decision

**Use Pattern B: PKCE + Encrypted State Cookies (Stateless Compute)**

We will implement OAuth2 authentication using:
- PKCE (Proof Key for Code Exchange) for authorization code protection
- Encrypted state parameters stored in httpOnly cookies
- Short-lived JWT access tokens (1 hour)
- OAuth provider-managed code validation
- No database/Redis for authorization code tracking

**This is the industry-standard pattern used by Auth0, Supabase, Vercel, and Netlify.**

---

## Considered Options

### Option A: Naive Stateless (REJECTED)
- No PKCE, no state tracking
- **Rejected:** Vulnerable to CSRF and code replay attacks

### Option B: PKCE + Encrypted Cookies (SELECTED)
- PKCE for code protection
- Encrypted cookies for state management
- OAuth provider handles code validation
- **Selected:** Industry standard, secure, zero infrastructure

### Option C: Stateful with Database (DEFERRED)
- Store authorization codes in DynamoDB/Redis
- Track active sessions in database
- **Deferred:** Over-engineering for MVP, adds complexity without security benefit for our scale

### Option D: Hybrid with Minimal State (CONSIDERED)
- JWT access tokens (stateless)
- Database tracking for authorization codes only
- **Not chosen:** Unnecessary complexity - PKCE provides same protection

---

## Decision Rationale

### Security Analysis

**Pattern B provides complete security through:**

1. **PKCE (RFC 7636)** - Prevents authorization code interception
   - Code verifier stored client-side (never transmitted)
   - Code challenge in authorization request
   - Server validates verifier matches challenge
   - Stolen code is useless without verifier

2. **Encrypted State Parameter** - Prevents CSRF attacks
   - Cryptographically signed state in httpOnly cookie
   - Validates callback came from our initiated flow
   - 10-minute expiration

3. **OAuth Provider Enforcement** - Single-use codes
   - Google/GitHub/etc. track code usage
   - Codes expire in 30-60 seconds
   - Automatic one-time use enforcement

4. **Transport Security** - HTTPS mandatory
   - Prevents man-in-the-middle attacks
   - Protects code and token transmission

**What we DON'T get (acceptable for MVP):**
- Immediate token revocation (1-hour window until expiry)
  - Mitigation: Short access token TTL + OAuth provider revocation API
- Session management UI (no "view active sessions")
  - Mitigation: CloudWatch Insights for audit queries
- Concurrent session limits
  - Not a security requirement for personal use

### Industry Evidence

**Real-world implementations that validate this pattern:**

- **Auth0** - Official recommendation for serverless applications
- **Supabase** - Default auth pattern via @supabase/ssr package
- **Vercel** - NextAuth.js default implementation
- **Netlify** - Identity service uses this pattern
- **Clerk** - Serverless-first auth service

**None of these services use database tracking for authorization codes.**

### Cost-Benefit Analysis

| Factor | Pattern B | Pattern C (Database) |
|--------|-----------|---------------------|
| **Infrastructure** | $0-5/month | $25-50/month |
| **Complexity** | ~200 LOC | ~800 LOC + migrations |
| **Deployment** | Single serverless function | Function + database + cleanup jobs |
| **Security** | Industry standard | Same (no additional benefit) |
| **Scalability** | 0-100 users effortlessly | 0-100 users (overkill) |
| **Audit Trail** | CloudWatch logs | Queryable database |

**Verdict:** Pattern B is appropriate for our scale and requirements.

---

## Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. User clicks "Login with Google"                          │
│    ↓                                                         │
│ 2. Server generates PKCE (verifier + challenge)             │
│    Stores verifier in encrypted cookie (10-min TTL)         │
│    ↓                                                         │
│ 3. Redirect to Google with code_challenge                   │
│    ↓                                                         │
│ 4. User approves, Google redirects to callback              │
│    with authorization code + state                          │
│    ↓                                                         │
│ 5. Server validates state (CSRF check)                      │
│    Retrieves code_verifier from cookie                      │
│    ↓                                                         │
│ 6. Exchange code + verifier for access token                │
│    Google validates PKCE + marks code as used               │
│    ↓                                                         │
│ 7. Store access token in httpOnly cookie                    │
│    Clear oauth_state cookie                                 │
│    ↓                                                         │
│ 8. User authenticated                                        │
└─────────────────────────────────────────────────────────────┘
```

### Security Properties

| Threat | Mitigation |
|--------|------------|
| **Authorization code interception** | PKCE - stolen code unusable without verifier |
| **CSRF attacks** | Encrypted state parameter validation |
| **Code replay attacks** | PKCE + OAuth provider single-use enforcement |
| **Token theft (XSS)** | HttpOnly, Secure, SameSite cookies |
| **Token theft (network)** | HTTPS mandatory, short TTL (1 hour) |
| **Session fixation** | Cryptographically signed state |

### Implementation Skeleton

```typescript
// Step 1: Authorization initiation
export async function initiateOAuth(req: Request, res: Response) {
  // Generate PKCE parameters
  const codeVerifier = crypto.randomBytes(32).toString('base64url');
  const codeChallenge = crypto
    .createHash('sha256')
    .update(codeVerifier)
    .digest('base64url');

  // Generate state
  const state = crypto.randomBytes(32).toString('hex');

  // Store in encrypted cookie (10-min TTL)
  const cookieValue = encrypt({
    state,
    codeVerifier,
    returnTo: req.query.returnTo || '/',
    expiresAt: Date.now() + 600000 // 10 minutes
  });

  res.cookie('oauth_state', cookieValue, {
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
    maxAge: 600000
  });

  // Redirect to OAuth provider
  const authUrl = new URL('https://accounts.google.com/o/oauth2/v2/auth');
  authUrl.searchParams.set('client_id', process.env.GOOGLE_CLIENT_ID);
  authUrl.searchParams.set('redirect_uri', `${process.env.BASE_URL}/auth/callback`);
  authUrl.searchParams.set('response_type', 'code');
  authUrl.searchParams.set('scope', 'openid email profile');
  authUrl.searchParams.set('state', state);
  authUrl.searchParams.set('code_challenge', codeChallenge);
  authUrl.searchParams.set('code_challenge_method', 'S256');

  res.redirect(authUrl.toString());
}

// Step 2: Callback handler
export async function handleOAuthCallback(req: Request, res: Response) {
  const { code, state: returnedState } = req.query;

  // Decrypt and validate state
  const stored = decrypt(req.cookies.oauth_state);

  if (!stored || stored.state !== returnedState) {
    throw new Error('Invalid state - CSRF detected');
  }

  if (Date.now() > stored.expiresAt) {
    throw new Error('State expired');
  }

  // Exchange code for tokens with PKCE
  const tokenResponse = await fetch('https://oauth2.googleapis.com/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      code,
      client_id: process.env.GOOGLE_CLIENT_ID,
      client_secret: process.env.GOOGLE_CLIENT_SECRET,
      redirect_uri: `${process.env.BASE_URL}/auth/callback`,
      grant_type: 'authorization_code',
      code_verifier: stored.codeVerifier // PKCE proof
    })
  });

  const tokens = await tokenResponse.json();

  // Store access token
  res.cookie('access_token', encrypt(tokens.access_token), {
    httpOnly: true,
    secure: true,
    sameSite: 'strict',
    maxAge: tokens.expires_in * 1000
  });

  // Clear oauth_state cookie
  res.clearCookie('oauth_state');

  // Redirect to original destination
  res.redirect(stored.returnTo);
}

// Step 3: Authentication middleware
export async function authenticateRequest(req: Request, res: Response, next: NextFunction) {
  const encryptedToken = req.cookies.access_token;

  if (!encryptedToken) {
    return res.status(401).json({ error: 'Unauthorized' });
  }

  try {
    const accessToken = decrypt(encryptedToken);

    // Validate token with OAuth provider (optional - for immediate revocation)
    // const userInfo = await fetch('https://www.googleapis.com/oauth2/v3/userinfo', {
    //   headers: { Authorization: `Bearer ${accessToken}` }
    // });

    req.user = { accessToken }; // Attach to request
    next();
  } catch (err) {
    res.status(401).json({ error: 'Invalid token' });
  }
}
```

---

## Implementation Checklist

### Phase 1: Core OAuth2 Flow (Week 1)

- [ ] Set up environment variables (CLIENT_ID, CLIENT_SECRET, BASE_URL)
- [ ] Implement PKCE generation (code_verifier, code_challenge)
- [ ] Implement cookie encryption/decryption (using server secret)
- [ ] Create `/auth/google` endpoint (authorization initiation)
- [ ] Create `/auth/callback` endpoint (token exchange)
- [ ] Validate state parameter (CSRF protection)
- [ ] Exchange authorization code with PKCE verification

### Phase 2: Security Hardening (Week 2)

- [ ] HTTPS enforcement (reject HTTP in production)
- [ ] Cookie security (httpOnly, secure, sameSite)
- [ ] Short access token lifetime (1 hour)
- [ ] Implement refresh token rotation (if using refresh tokens)
- [ ] Add rate limiting (optional but recommended)
- [ ] Error handling (don't expose internal details)

### Phase 3: Observability (Week 3)

- [ ] Structured logging for auth events
- [ ] Log session IDs for audit trail
- [ ] Monitor token validation failures
- [ ] Set up CloudWatch Insights queries
- [ ] Document emergency revocation procedure

### Phase 4: Testing

- [ ] Test PKCE validation (wrong verifier should fail)
- [ ] Test state validation (CSRF attack should be blocked)
- [ ] Test expired state (10-min timeout)
- [ ] Test token expiration (1-hour access token)
- [ ] Test refresh token rotation (if implemented)
- [ ] Load test (concurrent auth flows)

---

## Migration Path

### Current: Pattern B (Stateless)
- Appropriate for 1-100 users
- Zero infrastructure cost
- CloudWatch-based audit trail

### Trigger for Database Migration

Add database tracking when ANY of these occur:

1. **Scale:** >100 active users (log analysis becomes slow)
2. **Compliance:** SOC2 Type 2, HIPAA, or similar audit requirements
3. **Feature:** Need session management UI ("view active sessions")
4. **Security:** Security incident requires immediate revocation capability

### Migration Strategy (Non-Breaking)

**Phase 1: Passive logging**
```typescript
// Add session tracking without enforcement
app.use(async (req, res, next) => {
  if (req.user) {
    await db.sessions.upsert({ /* session data */ })
      .catch(err => logger.error('Session tracking failed', err));
  }
  next(); // Don't block on DB failure
});
```

**Phase 2: Enforcement (if needed)**
```typescript
// Make DB validation mandatory
const session = await db.sessions.findOne({ userId, tokenHash });
if (!session || session.revoked) {
  return res.status(401).json({ error: 'Session revoked' });
}
```

**Key insight:** Database can be added incrementally without changing auth flow.

---

## Consequences

### Positive

- ✅ **Zero infrastructure cost** ($0-5/month vs $25-50/month)
- ✅ **Simple deployment** (single serverless function, no database)
- ✅ **Industry-proven** (Auth0, Supabase, Vercel use this pattern)
- ✅ **Scales effortlessly** (stateless compute, horizontal scaling)
- ✅ **Secure by default** (PKCE + HTTPS + httpOnly cookies)
- ✅ **Fast iteration** (200 LOC vs 800 LOC with database)

### Negative

- ⚠️ **Revocation latency** (1-hour window until token expiry)
  - Acceptable for personal use
  - Mitigated by OAuth provider revocation API
- ⚠️ **Audit via logs** (CloudWatch Insights vs SQL queries)
  - Sufficient for <100 users
  - Structured logs provide queryable audit trail
- ⚠️ **No session UI** (can't "view active sessions" in admin panel)
  - Not needed for MVP
  - Can be added later with database migration

### Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Token theft | Low | Medium | Short TTL (1h), httpOnly cookies, HTTPS |
| Compromised token used | Low | Medium | OAuth provider revocation API, monitoring |
| Need immediate revocation | Low | Low | Can add database incrementally if needed |
| Scale exceeds log analysis | Low | Low | Migrate to database when >100 users |

---

## References

### Standards
- [RFC 6749 - OAuth 2.0 Authorization Framework](https://datatracker.ietf.org/doc/html/rfc6749)
- [RFC 7636 - PKCE (Proof Key for Code Exchange)](https://datatracker.ietf.org/doc/html/rfc7636)
- [RFC 9068 - JWT Access Token Profile](https://datatracker.ietf.org/doc/html/rfc9068)
- [OAuth 2.1 Draft](https://oauth.net/2.1/)

### Industry Implementations
- [Auth0 - Authorization Code Flow with PKCE](https://auth0.com/docs/get-started/authentication-and-authorization-flow/authorization-code-flow-with-pkce)
- [Supabase - Server-Side Auth](https://supabase.com/docs/guides/auth/server-side/overview)
- [Vercel - NextAuth.js](https://next-auth.js.org/)
- [Cloudflare Workers - OAuth Example](https://developers.cloudflare.com/workers/examples/auth-with-headers/)

### Related ADRs
- None yet (this is the first architecture decision record)

---

## Review and Update

**Next review:** After MVP deployment (estimated 3 weeks)
**Review triggers:**
- Scale exceeds 50 active users
- Security incident occurs
- Compliance requirements change
- Cost exceeds $10/month

**Decision can be revisited if:**
- Real-world usage reveals security gaps
- Performance/scalability issues emerge
- New OAuth2 best practices are published
