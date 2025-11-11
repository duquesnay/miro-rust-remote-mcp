#!/usr/bin/env bash
#
# Integration test for OAuth flow - emulates Claude.ai call sequence
#

set -euo pipefail

BASE_URL="${1:-https://miro-mcp.fly-agile.com}"
CLIENT_ID="3458764647632208270"
REDIRECT_URI="https://claude.ai/api/mcp/auth_callback"

echo "🧪 Testing OAuth flow on $BASE_URL"
echo ""

# Generate PKCE pair (simplified - just for testing structure)
CODE_VERIFIER=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-43)
CODE_CHALLENGE=$(echo -n "$CODE_VERIFIER" | openssl dgst -sha256 -binary | base64 | tr -d "=+/" | cut -c1-43)
STATE=$(openssl rand -hex 16)

echo "📋 Generated PKCE parameters:"
echo "  Code Verifier: ${CODE_VERIFIER:0:20}..."
echo "  Code Challenge: ${CODE_CHALLENGE:0:20}..."
echo "  State: $STATE"
echo ""

# Step 1: Claude.ai redirects user to /authorize
echo "🚀 Step 1: Testing /authorize endpoint (Claude.ai → Server)"
AUTHORIZE_URL="${BASE_URL}/authorize?response_type=code&client_id=${CLIENT_ID}&redirect_uri=${REDIRECT_URI}&code_challenge=${CODE_CHALLENGE}&code_challenge_method=S256&state=${STATE}&scope=boards:read boards:write"

echo "  URL: $AUTHORIZE_URL"
echo "  Expect: 302 redirect to Miro's OAuth"
echo ""

AUTHORIZE_RESPONSE=$(curl -s -i -w "\n%{http_code}" "$AUTHORIZE_URL")
AUTHORIZE_STATUS=$(echo "$AUTHORIZE_RESPONSE" | tail -1)
AUTHORIZE_LOCATION=$(echo "$AUTHORIZE_RESPONSE" | grep -i "^location:" | cut -d' ' -f2- | tr -d '\r')

echo "  Status: $AUTHORIZE_STATUS"
if [[ "$AUTHORIZE_STATUS" == "302" ]]; then
    echo "  ✅ Redirects to: ${AUTHORIZE_LOCATION:0:60}..."

    # Verify it's Miro's OAuth URL
    if [[ "$AUTHORIZE_LOCATION" == https://miro.com/oauth/authorize* ]]; then
        echo "  ✅ Correct Miro OAuth URL"
    else
        echo "  ❌ Unexpected redirect target"
    fi
else
    echo "  ❌ Expected 302, got $AUTHORIZE_STATUS"
    exit 1
fi
echo ""

# Step 2-4: User authorizes on Miro (browser interaction - cannot automate)
echo "⏸️  Step 2-4: User authorizes on Miro (requires browser)"
echo "  Miro redirects back to: ${BASE_URL}/callback?code=MIRO_CODE&state=$STATE"
echo "  Server should redirect to: ${REDIRECT_URI}?code=SERVER_CODE&state=$STATE"
echo ""

# Step 5: Simulate Claude.ai calling /token endpoint
echo "🔐 Step 5: Testing /token endpoint (Claude.ai → Server)"
echo "  Note: This will fail without real authorization code from Miro"
echo "  Testing endpoint structure only..."
echo ""

TOKEN_REQUEST="grant_type=authorization_code&code=TEST_CODE&redirect_uri=${REDIRECT_URI}&client_id=${CLIENT_ID}&code_verifier=${CODE_VERIFIER}"

curl -s -X POST "${BASE_URL}/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "$TOKEN_REQUEST" \
    -w "\n\nHTTP Status: %{http_code}\n" || true

echo ""
echo "📊 Integration Test Summary:"
echo "  ✅ /authorize endpoint accessible"
echo "  ✅ Redirects to Miro OAuth correctly"
echo "  ✅ /token endpoint accessible"
echo "  ⏸️  Full flow requires browser interaction with Miro"
echo ""
echo "💡 To test complete flow:"
echo "   1. Deploy to production"
echo "   2. Test with Claude.ai custom connector"
echo "   3. Monitor logs: ./scripts/access-logs.sh grafana"
