#!/bin/bash
# Setup Scaleway API key with proper permissions for deployment automation
# Based on IAM hierarchy: API Keys → Applications → Policies → Rules (Permission Sets + Scopes)

set -e

PROJECT_ID="b60e8eef-d494-4cc0-b42a-f7c700a9dd3b"
ORG_ID="69097242-d77c-46f1-95d8-77c7a079162c"

echo "=== Scaleway API Key Setup for Miro MCP Deployment ==="
echo ""
echo "Current status: API key 'SCWV7M0EN4Y62MHKF580' is invalid/deleted"
echo "Need: API key with SecretManagerFullAccess + ContainerRegistryFullAccess + ContainersFullAccess"
echo ""
echo "=== Step 1: Create IAM Application ==="
echo "Creating application 'miro-mcp-deployment'..."

APP_ID=$(scw iam application create \
  name=miro-mcp-deployment \
  description="Deployment automation for Miro MCP server" \
  -o json | jq -r '.id')

echo "✓ Application created: $APP_ID"
echo ""

echo "=== Step 2: Create Policy ==="
echo "Creating policy 'MiroMcpDeploymentPolicy'..."

POLICY_ID=$(scw iam policy create \
  name=MiroMcpDeploymentPolicy \
  description="Secrets + Container Registry + Containers access for miro-mcp-server deployment" \
  -o json | jq -r '.id')

echo "✓ Policy created: $POLICY_ID"
echo ""

echo "=== Step 3: Add Rules to Policy (Project-scoped for security) ==="

echo "Adding SecretManagerFullAccess rule..."
RULE1_ID=$(scw iam rule create \
  policy-id="$POLICY_ID" \
  permission-set-names.0=SecretManagerFullAccess \
  project-ids.0="$PROJECT_ID" \
  -o json | jq -r '.id')
echo "✓ Rule created: $RULE1_ID"

echo "Adding ContainerRegistryFullAccess rule..."
RULE2_ID=$(scw iam rule create \
  policy-id="$POLICY_ID" \
  permission-set-names.0=ContainerRegistryFullAccess \
  project-ids.0="$PROJECT_ID" \
  -o json | jq -r '.id')
echo "✓ Rule created: $RULE2_ID"

echo "Adding ContainersFullAccess rule..."
RULE3_ID=$(scw iam rule create \
  policy-id="$POLICY_ID" \
  permission-set-names.0=ContainersFullAccess \
  project-ids.0="$PROJECT_ID" \
  -o json | jq -r '.id')
echo "✓ Rule created: $RULE3_ID"
echo ""

echo "=== Step 4: Attach Policy to Application ==="
scw iam policy attach \
  application-id="$APP_ID" \
  policy-id="$POLICY_ID"
echo "✓ Policy attached to application"
echo ""

echo "=== Step 5: Create API Key for Application ==="
API_KEY_OUTPUT=$(scw iam api-key create \
  application-id="$APP_ID" \
  description="Deployment automation - created $(date +%Y-%m-%d)" \
  -o json)

ACCESS_KEY=$(echo "$API_KEY_OUTPUT" | jq -r '.access_key')
SECRET_KEY=$(echo "$API_KEY_OUTPUT" | jq -r '.secret_key')

echo "✓ API key created:"
echo "  Access Key: $ACCESS_KEY"
echo "  Secret Key: $SECRET_KEY"
echo ""

echo "=== Step 6: Update scw CLI Configuration ==="
echo "Updating ~/.config/scw/config.yaml with new credentials..."

scw config set access-key="$ACCESS_KEY"
scw config set secret-key="$SECRET_KEY"

echo "✓ Configuration updated"
echo ""

echo "=== Step 7: Verify New API Key ==="
echo "Testing API key permissions..."

# Test Secret Manager access
echo -n "  Secret Manager access: "
if scw secret secret list project-id="$PROJECT_ID" -o json &>/dev/null; then
  echo "✓"
else
  echo "✗ (might need a few seconds for permissions to propagate)"
fi

# Test Container Registry access
echo -n "  Container Registry access: "
if scw registry namespace list -o json &>/dev/null; then
  echo "✓"
else
  echo "✗ (might need a few seconds for permissions to propagate)"
fi

# Test Containers access
echo -n "  Containers access: "
if scw container container list -o json &>/dev/null; then
  echo "✓"
else
  echo "✗ (might need a few seconds for permissions to propagate)"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Summary:"
echo "  Application ID: $APP_ID"
echo "  Policy ID: $POLICY_ID"
echo "  Access Key: $ACCESS_KEY"
echo ""
echo "Next steps:"
echo "  1. Wait 30-60 seconds for permissions to fully propagate"
echo "  2. Run: ./scripts/create-scaleway-secrets.sh"
echo "  3. Run: ./scripts/deploy.sh --env=production"
echo ""
