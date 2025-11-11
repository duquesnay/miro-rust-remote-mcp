#!/bin/bash
# Create Scaleway secrets for Miro MCP deployment
# Requires: API key with SecretManagerFullAccess permissions

set -e

# Load local secrets from .env
if [ ! -f .env ]; then
  echo "Error: .env file not found"
  echo "Need MIRO_CLIENT_SECRET and MIRO_ENCRYPTION_KEY values"
  exit 1
fi

source .env

PROJECT_ID="b60e8eef-d494-4cc0-b42a-f7c700a9dd3b"

echo "=== Creating Scaleway Secrets ==="
echo ""
echo "Project: $PROJECT_ID"
echo ""

# Create MIRO_CLIENT_SECRET
echo "Creating secret: MIRO_CLIENT_SECRET..."
SECRET1_ID=$(scw secret secret create \
  project-id="$PROJECT_ID" \
  name=MIRO_CLIENT_SECRET \
  -o json | jq -r '.id')
echo "✓ Secret created: $SECRET1_ID"

# Add version with actual value
echo "Adding secret value..."
scw secret version create \
  secret-id="$SECRET1_ID" \
  data="$MIRO_CLIENT_SECRET" \
  > /dev/null
echo "✓ Secret value added"
echo ""

# Create MIRO_ENCRYPTION_KEY
echo "Creating secret: MIRO_ENCRYPTION_KEY..."
SECRET2_ID=$(scw secret secret create \
  project-id="$PROJECT_ID" \
  name=MIRO_ENCRYPTION_KEY \
  -o json | jq -r '.id')
echo "✓ Secret created: $SECRET2_ID"

# Add version with actual value
echo "Adding secret value..."
scw secret version create \
  secret-id="$SECRET2_ID" \
  data="$MIRO_ENCRYPTION_KEY" \
  > /dev/null
echo "✓ Secret value added"
echo ""

echo "=== Secrets Created Successfully ==="
echo ""
echo "Summary:"
echo "  MIRO_CLIENT_SECRET: $SECRET1_ID"
echo "  MIRO_ENCRYPTION_KEY: $SECRET2_ID"
echo ""
echo "Verify secrets:"
echo "  scw secret secret list project-id=$PROJECT_ID"
echo ""
echo "Next step:"
echo "  ./scripts/deploy.sh --env=production"
echo ""
