# Miro MCP Server - Scaleway Deployment Guide

## Overview

This document describes the deployment process for the Miro MCP Server to Scaleway Containers. The deployment enables Claude.ai web interface integration via remote MCP server.

**Deployment Target**: Scaleway Managed Containers (serverless)
**Deployment Time**: <5 minutes (automated)
**Runtime**: Rust 1.90 + Debian Bookworm Slim

---

## Prerequisites

### 1. Local Environment

- Docker installed and running
- Scaleway CLI installed (`scw` command)
- Scaleway account with credentials configured

### 2. Scaleway Resources

- **Container Registry** created in `fr-par` region
- **Container Namespace** created (e.g., `miro-mcp`)
- **Secrets** configured for sensitive environment variables

### 3. Miro Developer Portal

- OAuth2 application registered
- Client ID and Client Secret obtained
- Redirect URI configured (will be updated post-deployment)

---

## Configuration

### Environment Variables

The MCP server requires the following environment variables:

#### Required Secrets (Configure in Scaleway)

```bash
MIRO_CLIENT_ID=<your-client-id>
MIRO_CLIENT_SECRET=<your-client-secret>
MIRO_REDIRECT_URI=https://<container-url>/oauth/callback
MIRO_ENCRYPTION_KEY=<32-byte-hex-string>
TOKEN_STORAGE_PATH=/app/data/tokens.enc
```

#### Optional Environment Variables

```bash
RUST_LOG=info                    # Logging level (info, debug, trace)
LOG_FORMAT=json                  # Log format: json (production) or pretty (dev)
MCP_SERVER_PORT=3000            # Server port (default)
```

**Log Configuration (OBS1):**
- `LOG_FORMAT=json` - Structured JSON logs for Scaleway Cockpit (recommended for production)
- `LOG_FORMAT=pretty` - Human-readable logs for development (default if unset)
- `RUST_LOG=info` - Standard verbosity (recommended)
- `RUST_LOG=debug` - Detailed auth logging (temporary debugging only)
- `RUST_LOG=trace` - Very verbose (not recommended for production)

### Generate Encryption Key

```bash
# Generate 32-byte hex encryption key for token storage
openssl rand -hex 32
```

### Scaleway Secrets Configuration

```bash
# Create secrets in Scaleway (via Console or CLI)
scw container secret create \
  region=fr-par \
  namespace-id=<namespace-id> \
  name=MIRO_CLIENT_ID \
  value=<your-client-id>

scw container secret create \
  region=fr-par \
  namespace-id=<namespace-id> \
  name=MIRO_CLIENT_SECRET \
  value=<your-client-secret>

scw container secret create \
  region=fr-par \
  namespace-id=<namespace-id> \
  name=MIRO_ENCRYPTION_KEY \
  value=<hex-key>

scw container secret create \
  region=fr-par \
  namespace-id=<namespace-id> \
  name=TOKEN_STORAGE_PATH \
  value=/app/data/tokens.enc
```

---

## Deployment Steps

### Step 1: Build and Test Locally (Optional)

```bash
# Build Docker image
docker build -t miro-mcp-server:test .

# Verify binary was created
docker run --rm miro-mcp-server:test ls -lh /app/miro-mcp-server

# Test with environment variables
docker run --rm \
  -e MIRO_CLIENT_ID=test \
  -e MIRO_CLIENT_SECRET=test \
  -e MIRO_REDIRECT_URI=http://localhost:3000/callback \
  -e MIRO_ENCRYPTION_KEY=$(openssl rand -hex 32) \
  -e TOKEN_STORAGE_PATH=/app/data/tokens.enc \
  miro-mcp-server:test \
  /app/miro-mcp-server --help
```

### Step 2: Configure Scaleway

```bash
# Install Scaleway CLI if not installed
curl -o /tmp/scw-installer.sh https://raw.githubusercontent.com/scaleway/scaleway-cli/master/scripts/get.sh
bash /tmp/scw-installer.sh

# Initialize Scaleway CLI
scw init

# Create Container Registry namespace (if not exists)
scw container namespace create \
  name=miro-mcp \
  region=fr-par \
  description="Miro MCP Server registry"

# Configure secrets (see Configuration section above)
```

### Step 3: Deploy to Scaleway

```bash
# Dry run (test without deploying)
./scripts/deploy.sh \
  --project=miro-mcp-server \
  --region=fr-par \
  --registry=rg.fr-par.scw.cloud/miro-mcp \
  --dry-run

# Actual deployment
./scripts/deploy.sh \
  --project=miro-mcp-server \
  --region=fr-par \
  --registry=rg.fr-par.scw.cloud/miro-mcp
```

### Step 4: Configure Miro OAuth Redirect URI

```bash
# Get container URL
scw container container list \
  region=fr-par \
  namespace-id=<namespace-id> \
  name=miro-mcp-server \
  -o json | jq -r '.[0].domain_name'

# Update Miro Developer Portal
# 1. Go to https://miro.com/app/settings/user-profile/apps
# 2. Select your OAuth2 application
# 3. Update Redirect URI: https://<container-url>/oauth/callback
# 4. Save changes
```

### Step 5: Update Scaleway Secret

```bash
# Update redirect URI secret with production URL
scw container secret update \
  region=fr-par \
  secret-id=<MIRO_REDIRECT_URI-secret-id> \
  value=https://<container-url>/oauth/callback

# Restart container to pick up new secret
scw container container deploy \
  region=fr-par \
  container-id=<container-id>
```

### Step 6: Verify Deployment

```bash
# Check container status
scw container container get \
  region=fr-par \
  container-id=<container-id>

# View logs
scw container container logs \
  region=fr-par \
  container-id=<container-id> \
  --follow

# Expected log output:
# INFO Starting Miro MCP Server
# INFO Configuration loaded successfully
# INFO MCP server initialized
```

---

## Architecture

### Container Specifications

- **Base Image**: `debian:bookworm-slim`
- **Runtime User**: `mcp` (UID 1000, non-root)
- **Memory**: 256Mi
- **CPU**: 0.25 vCPU
- **Scaling**: min=1, max=1 (single instance for personal use)
- **Port**: 3000 (MCP uses stdio transport, HTTP for health checks)

### Multi-Stage Build

1. **Builder Stage**: `rust:1.90-bookworm`
   - Compiles Rust binary with optimizations
   - Uses `cargo build --release --locked`
   - ~2-5 minutes build time

2. **Runtime Stage**: `debian:bookworm-slim`
   - Minimal Debian base (~80MB)
   - Installs only runtime dependencies (ca-certificates, libssl3)
   - Copies binary from builder stage
   - Final image size: ~150-200MB

### Storage

- **Token Storage**: Persistent volume mounted at `/app/data`
- **Path**: `/app/data/tokens.enc` (encrypted OAuth2 tokens)
- **Encryption**: AES-256-GCM with key from environment variable

---

## Troubleshooting

### Container Fails to Start

```bash
# Check logs for error messages
scw container container logs region=fr-par container-id=<id>

# Common issues:
# 1. Missing environment variables
#    → Check secrets configuration
# 2. Invalid encryption key
#    → Regenerate with: openssl rand -hex 32
# 3. Binary not found
#    → Rebuild Docker image
```

### OAuth Flow Fails

```bash
# Check redirect URI configuration
# 1. Verify Miro Developer Portal settings match container URL
# 2. Ensure HTTPS is used (Scaleway provides HTTPS by default)
# 3. Check MIRO_REDIRECT_URI secret value matches registered URI
```

### Token Storage Issues

```bash
# Check volume mount and permissions
scw container container get region=fr-par container-id=<id>

# Verify TOKEN_STORAGE_PATH points to /app/data/tokens.enc
# Volume should be mounted at /app/data
# Container runs as user 'mcp' (UID 1000) with write access
```

### Build Timeouts

```bash
# Rust compilation can take time (2-5 minutes)
# If timeout occurs during deployment:
#
# 1. Build and push image manually first:
docker build -t rg.fr-par.scw.cloud/miro-mcp/miro-mcp-server:latest .
docker push rg.fr-par.scw.cloud/miro-mcp/miro-mcp-server:latest

# 2. Then deploy pre-built image:
scw container container update \
  region=fr-par \
  container-id=<id> \
  registry-image=rg.fr-par.scw.cloud/miro-mcp/miro-mcp-server:latest
```

---

## Updating the Deployment

### Code Changes

```bash
# 1. Make code changes locally
# 2. Test locally with Docker
docker build -t miro-mcp-server:test .
docker run --rm ... miro-mcp-server:test

# 3. Deploy updated image
./scripts/deploy.sh \
  --project=miro-mcp-server \
  --region=fr-par \
  --registry=rg.fr-par.scw.cloud/miro-mcp \
  --tag=v1.0.1  # Optional version tag
```

### Environment Variable Changes

```bash
# Update secrets via Scaleway Console or CLI
scw container secret update \
  region=fr-par \
  secret-id=<secret-id> \
  value=<new-value>

# Restart container to pick up changes
scw container container deploy \
  region=fr-par \
  container-id=<container-id>
```

---

## Cost Estimation

**Scaleway Containers Pricing** (fr-par region):

- **Registry Storage**: ~€0.02/GB/month (Docker image ~200MB = ~€0.004/month)
- **Container vCPU**: ~€0.10/vCPU/hour × 0.25 vCPU × 730 hours = ~€18/month
- **Container Memory**: ~€0.01/GB/hour × 0.256 GB × 730 hours = ~€1.87/month
- **Bandwidth**: First 100GB free, then €0.01/GB

**Total Estimated Cost**: ~€20/month (single instance, always-on)

**Note**: For personal use with low traffic, costs may be lower with auto-scaling (min-scale=0) but requires cold start handling.

---

## Security Considerations

### Container Security

- ✅ Runs as non-root user (`mcp`, UID 1000)
- ✅ Minimal attack surface (Debian Slim base)
- ✅ No SSH access (container-only deployment)
- ✅ Secrets managed via Scaleway Secrets (not in environment)
- ✅ HTTPS enforced (Scaleway provides TLS termination)

### Token Security

- ✅ Tokens encrypted at rest (AES-256-GCM)
- ✅ Encryption key stored as Scaleway secret
- ✅ Token storage persisted to volume (survives container restarts)
- ✅ No tokens logged (tracing configured to avoid sensitive data)

### OAuth2 Security

- ✅ State parameter validation (CSRF prevention)
- ✅ Redirect URI exact match (no open redirects)
- ✅ Token refresh rotation (use new refresh token)
- ✅ HTTPS-only redirect URIs (prevent token interception)

---

## Next Steps

1. **Monitor Deployment**: Set up monitoring/alerting for container health
2. **Backup Strategy**: Configure automated backups of token storage volume
3. **CI/CD Integration**: Automate deployment via GitHub Actions or GitLab CI
4. **Load Testing**: Test OAuth flow and API calls under load
5. **Documentation**: Document MCP tools and usage examples for Claude

---

## References

- **Scaleway Containers Docs**: https://www.scaleway.com/en/docs/serverless/containers/
- **Scaleway CLI Reference**: https://www.scaleway.com/en/docs/developer-tools/scaleway-cli/
- **Miro OAuth2 Docs**: https://developers.miro.com/docs/oauth-20
- **MCP Specification**: https://spec.modelcontextprotocol.io/

---

## Appendix: scaleway-config.json

The project includes `scaleway-config.json` for reference:

```json
{
  "project": "miro-mcp-server",
  "type": "container",
  "runtime": "rust",
  "region": "fr-par",
  "registry": "rg.fr-par.scw.cloud/miro-mcp",
  "resources": {
    "memory": "256Mi",
    "cpu": "0.25",
    "minScale": "1",
    "maxScale": "1"
  },
  "secrets": [
    "MIRO_CLIENT_ID",
    "MIRO_CLIENT_SECRET",
    "MIRO_REDIRECT_URI",
    "MIRO_ENCRYPTION_KEY",
    "TOKEN_STORAGE_PATH"
  ]
}
```

This configuration is used for documentation and future Infrastructure-as-Code (IaC) migration to Terraform/Pulumi.
