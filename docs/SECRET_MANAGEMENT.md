# Secret Management for Miro MCP Server

## Overview

This document describes the secret management strategy for the Miro MCP Server deployed on Scaleway Containers. All sensitive credentials are managed via **Scaleway Secret Manager** to ensure secure storage, access control, and auditability.

**Security Principle**: Secrets NEVER appear in code, environment files, logs, or git history.

---

## Secrets Inventory

### Managed Secrets (Scaleway Secret Manager)

| Secret Name | Type | Purpose | Rotation Frequency |
|-------------|------|---------|-------------------|
| `MIRO_CLIENT_SECRET` | OAuth2 Credential | Miro API authentication | 90 days |
| `MIRO_ENCRYPTION_KEY` | Encryption Key | Token storage encryption (AES-256-GCM) | 90 days |

### Non-Secret Configuration (Environment Variables)

| Variable Name | Value Example | Purpose |
|---------------|---------------|---------|
| `MIRO_CLIENT_ID` | `3458764647516852398` | Miro OAuth2 application ID (public) |
| `MIRO_REDIRECT_URI` | `https://...scw.cloud/oauth/callback` | OAuth2 callback URL (public) |
| `TOKEN_STORAGE_PATH` | `/app/data/tokens.enc` | Token storage file path |

**Rationale**: Client IDs and redirect URIs are public OAuth2 identifiers and do not require secret management.

---

## Initial Secret Setup

### Prerequisites

- Scaleway CLI installed and configured (`scw init`)
- Access to Miro Developer Portal (for client secret)
- Permissions to create secrets in Scaleway project

### Step 1: Generate Encryption Key

```bash
# Generate 32-byte hex encryption key for token storage
openssl rand -hex 32

# Example output:
# 3f9a8b2c5d7e1f4a6b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a
```

**IMPORTANT**: Store this key securely - loss of this key means all stored tokens become unrecoverable.

### Step 2: Create Secrets in Scaleway

```bash
# Set your region (fr-par, nl-ams, pl-waw)
REGION=fr-par

# Create MIRO_CLIENT_SECRET
scw secret secret create \
  region=${REGION} \
  name=MIRO_CLIENT_SECRET \
  description="Miro OAuth2 client secret for MCP server"

# Get the secret ID from output (format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
MIRO_CLIENT_SECRET_ID=<secret-id-from-output>

# Add the secret value
scw secret version create \
  region=${REGION} \
  secret-id=${MIRO_CLIENT_SECRET_ID} \
  data="<your-miro-client-secret>"

# Create MIRO_ENCRYPTION_KEY
scw secret secret create \
  region=${REGION} \
  name=MIRO_ENCRYPTION_KEY \
  description="AES-256-GCM encryption key for token storage"

MIRO_ENCRYPTION_KEY_ID=<secret-id-from-output>

scw secret version create \
  region=${REGION} \
  secret-id=${MIRO_ENCRYPTION_KEY_ID} \
  data="<hex-key-from-step-1>"
```

### Step 3: Verify Secret Creation

```bash
# List all secrets in region
scw secret secret list region=${REGION}

# Expected output:
# ID                                    NAME                    STATUS
# xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  MIRO_CLIENT_SECRET      ready
# xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx  MIRO_ENCRYPTION_KEY     ready
```

### Step 4: Deploy with Secrets

The deployment script (`scripts/deploy.sh`) automatically validates secrets and injects them into the container:

```bash
./scripts/deploy.sh --env=production
```

**Automatic Validation**: Deployment fails if required secrets are missing, preventing deployments without proper configuration.

---

## Secret Rotation Procedure

**Rotation Schedule**: Every 90 days (recommended best practice for OAuth2 credentials and encryption keys)

**Rotation Strategy**: Blue-Green approach (create new, validate, switch over, delete old)

### Rotating MIRO_CLIENT_SECRET

**Frequency**: 90 days
**Downtime**: Zero (if done correctly)
**Prerequisites**: Access to Miro Developer Portal

#### Step 1: Generate New Client Secret in Miro

1. Go to Miro Developer Portal: https://miro.com/app/settings/user-profile/apps
2. Select your OAuth2 application
3. Navigate to **Client Credentials** section
4. Click **Regenerate Client Secret**
5. Copy the new client secret (shown only once)

**IMPORTANT**: Do NOT delete the old secret yet - both will work during transition.

#### Step 2: Create New Secret Version in Scaleway

```bash
REGION=fr-par

# Get existing secret ID
MIRO_CLIENT_SECRET_ID=$(scw secret secret list region=${REGION} name=MIRO_CLIENT_SECRET -o json | jq -r '.[0].id')

# Create new version with new secret
scw secret version create \
  region=${REGION} \
  secret-id=${MIRO_CLIENT_SECRET_ID} \
  data="<new-miro-client-secret>"

# Verify new version created
scw secret version list region=${REGION} secret-id=${MIRO_CLIENT_SECRET_ID}
```

#### Step 3: Deploy with New Secret

```bash
# Redeploy container to pick up new secret
./scripts/deploy.sh --env=production
```

**Validation**: The container will restart and use the new client secret. OAuth2 flows will use the new credential.

#### Step 4: Test OAuth Flow

```bash
# Monitor logs during OAuth test
scw container container logs region=${REGION} container-id=<container-id> --follow

# Test steps:
# 1. Initiate OAuth flow via Claude.ai or MCP Inspector
# 2. Complete authorization in browser
# 3. Verify successful token exchange in logs
# 4. Confirm MCP tools work with new credentials
```

#### Step 5: Clean Up Old Secret Version (Optional)

```bash
# List versions to find old version ID
scw secret version list region=${REGION} secret-id=${MIRO_CLIENT_SECRET_ID}

# Disable old version (keeps for audit, prevents use)
scw secret version access \
  region=${REGION} \
  secret-id=${MIRO_CLIENT_SECRET_ID} \
  revision=<old-version-number> \
  --disable
```

**Recommendation**: Keep old versions disabled for 30 days before deletion to allow rollback if issues arise.

---

### Rotating MIRO_ENCRYPTION_KEY

**Frequency**: 90 days
**Complexity**: HIGH (requires token re-encryption)
**Downtime**: Minimal (users must re-authenticate)

**WARNING**: Encryption key rotation invalidates all stored tokens. Users will need to re-authenticate via OAuth2.

#### Step 1: Generate New Encryption Key

```bash
# Generate new 32-byte hex key
NEW_ENCRYPTION_KEY=$(openssl rand -hex 32)
echo "New encryption key: ${NEW_ENCRYPTION_KEY}"

# Store securely (password manager, secure note)
```

#### Step 2: Create New Secret Version

```bash
REGION=fr-par

# Get existing secret ID
MIRO_ENCRYPTION_KEY_ID=$(scw secret secret list region=${REGION} name=MIRO_ENCRYPTION_KEY -o json | jq -r '.[0].id')

# Create new version
scw secret version create \
  region=${REGION} \
  secret-id=${MIRO_ENCRYPTION_KEY_ID} \
  data="${NEW_ENCRYPTION_KEY}"
```

#### Step 3: Clear Token Storage

Since the encryption key changed, existing tokens cannot be decrypted. Options:

**Option A: Let tokens expire naturally** (recommended for scheduled rotation)
- Deploy new key
- Users re-authenticate when tokens expire (1 hour for access tokens)
- Gradual transition

**Option B: Force immediate re-authentication** (for security incidents)
```bash
# SSH into container (if accessible) or redeploy with volume reset
# Delete token storage file
rm -f /app/data/tokens.enc

# Or redeploy container with fresh volume
scw container container deploy region=${REGION} container-id=<container-id>
```

#### Step 4: Deploy and Notify Users

```bash
./scripts/deploy.sh --env=production
```

**User Communication**:
```
Subject: Miro MCP Server - Re-authentication Required

Due to scheduled security maintenance (encryption key rotation),
please re-authenticate with Miro via Claude.ai:

1. Start any Miro MCP tool operation
2. Follow OAuth2 flow when prompted
3. Resume normal usage

Tokens issued after [DEPLOYMENT_TIME] will work normally.
```

---

## Emergency Procedures

### Secret Compromised (Immediate Response)

If a secret is compromised (leaked in logs, code, or external breach):

#### Immediate Actions (< 15 minutes)

1. **Revoke compromised credential**:
   - **MIRO_CLIENT_SECRET**: Regenerate in Miro Developer Portal immediately
   - **MIRO_ENCRYPTION_KEY**: Generate new key (`openssl rand -hex 32`)

2. **Rotate secret in Scaleway**:
```bash
# Create new version with emergency credential
scw secret version create \
  region=${REGION} \
  secret-id=<compromised-secret-id> \
  data="<new-emergency-value>"
```

3. **Deploy immediately**:
```bash
./scripts/deploy.sh --env=production
```

4. **Verify deployment**:
```bash
scw container container logs region=${REGION} container-id=<id> --follow
# Look for "Configuration loaded successfully"
```

#### Post-Incident Actions (< 24 hours)

1. **Audit logs**: Check Scaleway Cockpit for unauthorized access attempts
2. **Review access control**: Verify who has permissions to secrets
3. **Update incident log**: Document what was compromised, when, how detected, remediation
4. **Notify stakeholders**: If user data potentially affected, follow disclosure policy

---

### Secret Lost (Encryption Key)

**Scenario**: MIRO_ENCRYPTION_KEY lost and no backup available.

**Impact**: All stored tokens become unrecoverable.

**Recovery**:
1. Generate new encryption key
2. Rotate key following standard procedure (Step 2-4 in rotation section)
3. All users must re-authenticate (tokens cannot be migrated)

**Prevention**: Store encryption keys in password manager or secure vault with backup.

---

## Access Control

### Who Can Access Secrets?

**Scaleway IAM Permissions** required:
- `SecretManagerSecretsRead` - Read secret metadata
- `SecretManagerVersionsRead` - Read secret values
- `SecretManagerVersionsWrite` - Create/update secret versions

**Recommended Roles**:
- **Platform Admin**: Full access (create, read, update, delete secrets)
- **Developer**: Read-only access (view secrets for debugging)
- **CI/CD Pipeline**: Read-only access (deploy containers with secrets)

**Least Privilege**: Production secrets should only be accessible to platform admins and automated deployment pipelines.

### Audit Secret Access

```bash
# View secret access logs via Scaleway Cockpit
# https://console.scaleway.com/cockpit/logs

# Query for secret access events
# Filter: service=secret-manager AND action=version.read
```

**Monitor for**:
- Unexpected access times (outside business hours)
- Access from unknown IP addresses
- Bulk secret reads (potential exfiltration)

---

## Logging and Monitoring

### What Gets Logged?

**ALWAYS Logged** (safe):
- Secret names (e.g., "MIRO_CLIENT_SECRET")
- Secret IDs (UUID format)
- Deployment events ("Secrets configured")
- Rotation events ("New version created")

**NEVER Logged** (dangerous):
- Secret values (actual client secret, encryption key)
- Decrypted tokens
- OAuth2 authorization codes

### Log Filtering for Secrets

The application uses structured logging with automatic secret filtering:

```rust
// Example: Secrets are redacted in logs
tracing::info!(
    client_id = %config.miro_client_id,  // ✓ Safe to log
    client_secret = "[REDACTED]",        // ✓ Explicitly redacted
    "OAuth2 configuration loaded"
);
```

**Validation**: Check production logs to ensure no secrets appear:

```bash
# Search for potential secret leaks
scw container container logs region=${REGION} container-id=<id> | grep -i "client_secret"

# Expected: Only "[REDACTED]" or "client_secret" (key name), never actual values
```

---

## Scaleway Secret Manager Best Practices

### Secret Naming Convention

- Use `SCREAMING_SNAKE_CASE` matching environment variable names
- Prefix with application: `MIRO_MCP_<SECRET_NAME>` (if managing multiple apps)
- Example: `MIRO_MCP_CLIENT_SECRET`, `MIRO_MCP_ENCRYPTION_KEY`

### Version Management

- **Enable versioning**: Keep old versions disabled (not deleted) for rollback
- **Retention period**: 30 days (allows rollback window)
- **Latest version**: Always use latest by default (Scaleway behavior)

### Tagging Secrets

```bash
# Add tags for organization
scw secret secret update \
  region=${REGION} \
  secret-id=<secret-id> \
  tags.0="env=production" \
  tags.1="app=miro-mcp" \
  tags.2="rotation-freq=90d"
```

### Cost Optimization

- **Secret Manager Pricing** (as of 2025):
  - Secrets: Free (up to 1000 secrets)
  - Version storage: €0.40/1000 version-months
  - API calls: €0.40/10,000 requests

**Cost for this deployment**: ~€0.05/month (2 secrets × 2 versions average)

---

## Troubleshooting

### Deployment Fails: "Missing required secrets"

**Symptom**:
```
Error: Missing required secrets in Scaleway Secret Manager:
  - MIRO_CLIENT_SECRET
```

**Resolution**:
1. Verify secrets exist: `scw secret secret list region=${REGION}`
2. Check secret names match exactly (case-sensitive)
3. Ensure secrets are in correct region (`fr-par`)
4. Create missing secrets following "Initial Secret Setup" section

---

### Container Logs Show "Failed to load configuration"

**Symptom**:
```
ERROR Failed to load configuration: Environment variable MIRO_CLIENT_SECRET not found
```

**Resolution**:
1. Check secret injection: `scw container container get region=${REGION} container-id=<id> -o json | jq .secret_environment_variables`
2. Verify secret IDs are valid: `scw secret secret get region=${REGION} secret-id=<id>`
3. Redeploy to refresh secret mapping: `./scripts/deploy.sh --env=production`

---

### OAuth Flow Fails After Secret Rotation

**Symptom**: "Invalid client credentials" error during OAuth2 token exchange

**Cause**: Mismatch between Miro's expected client secret and Scaleway's stored secret

**Resolution**:
1. Verify Miro Developer Portal shows correct client secret
2. Check Scaleway secret version: `scw secret version list region=${REGION} secret-id=<id>`
3. Ensure latest version contains correct value
4. Redeploy to pick up latest secret version

---

## Compliance and Security

### Security Standards Met

- ✅ **Encryption at rest**: Scaleway Secret Manager encrypts all secrets with AES-256
- ✅ **Encryption in transit**: Secrets injected via secure container startup (not over network)
- ✅ **Access control**: IAM-based permissions with audit logs
- ✅ **Separation of concerns**: Secrets separate from code and configuration
- ✅ **Principle of least privilege**: Containers only access assigned secrets

### Audit Requirements

For compliance audits, provide:
1. **Secret inventory**: List of all secrets (names, not values)
2. **Access logs**: Scaleway Cockpit query results for secret access
3. **Rotation schedule**: 90-day rotation for all secrets
4. **Incident response plan**: Emergency rotation procedures (documented above)

---

## Quick Reference

### Create New Secret

```bash
scw secret secret create region=fr-par name=SECRET_NAME description="Purpose"
scw secret version create region=fr-par secret-id=<id> data="<value>"
```

### Rotate Secret

```bash
scw secret version create region=fr-par secret-id=<id> data="<new-value>"
./scripts/deploy.sh --env=production
```

### List Secrets

```bash
scw secret secret list region=fr-par
scw secret version list region=fr-par secret-id=<id>
```

### Delete Secret (Careful!)

```bash
# Disable first (reversible)
scw secret version access region=fr-par secret-id=<id> revision=<version> --disable

# Delete permanently (irreversible after 7 days)
scw secret version delete region=fr-par secret-id=<id> revision=<version>
```

---

## Related Documentation

- [Scaleway Secret Manager Docs](https://www.scaleway.com/en/docs/identity-and-access-management/secret-manager/)
- [Deployment Guide](../planning/deployment.md)
- [OAuth2 Security Checklist](../CLAUDE.md#security-checklist)

---

## Changelog

| Date | Change | Author |
|------|--------|--------|
| 2025-11-11 | Initial secret management documentation | System |

