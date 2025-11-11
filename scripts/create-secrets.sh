#!/bin/bash

# Scaleway Secret Manager Setup Script for Miro MCP Server
# This script provides instructions for creating secrets via Scaleway Console
# API key lacks CLI permissions - must use web console

set -euo pipefail

echo "============================================"
echo "Scaleway Secret Manager Setup"
echo "============================================"
echo ""
echo "API key lacks permissions to create secrets via CLI."
echo "Please create secrets manually via Scaleway Console:"
echo ""
echo "1. Go to: https://console.scaleway.com/secret-manager/secrets"
echo "2. Select region: fr-par"
echo "3. Create the following secrets:"
echo ""

# Read values from .env
if [ -f ".env" ]; then
    source .env

    echo "Secret 1: MIRO_CLIENT_SECRET"
    echo "  Name: MIRO_CLIENT_SECRET"
    echo "  Value: $MIRO_CLIENT_SECRET"
    echo "  Region: fr-par"
    echo ""

    echo "Secret 2: MIRO_ENCRYPTION_KEY"
    echo "  Name: MIRO_ENCRYPTION_KEY"
    echo "  Value: $MIRO_ENCRYPTION_KEY"
    echo "  Region: fr-par"
    echo ""
else
    echo "ERROR: .env file not found"
    echo "Cannot read secret values"
    exit 1
fi

echo "============================================"
echo "After creating secrets in Console:"
echo "============================================"
echo ""
echo "1. Verify secrets exist:"
echo "   scw secret secret list region=fr-par"
echo ""
echo "2. Re-run deployment:"
echo "   ./scripts/deploy.sh --env=production"
echo ""
echo "============================================"
echo ""
echo "Note: The deployment script will automatically inject"
echo "these secrets as environment variables into the container."
echo "============================================"
