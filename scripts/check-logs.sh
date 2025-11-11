#!/bin/bash
# Query production logs from Scaleway Cockpit

set -e

# Load Cockpit token from .env.local
if [ -f .env.local ]; then
    source .env.local
fi

if [ -z "$SCW_COCKPIT_TOKEN" ]; then
    echo "Error: SCW_COCKPIT_TOKEN not set"
    echo "Make sure .env.local exists with the token"
    exit 1
fi

# Get container info
CONTAINER_ID=$(scw container container list region=fr-par -o json | jq -r '.[] | select(.name == "miro-mcp-server") | .id')
CONTAINER_STATUS=$(scw container container list region=fr-par -o json | jq -r '.[] | select(.name == "miro-mcp-server") | .status')
CONTAINER_URL=$(scw container container list region=fr-par -o json | jq -r '.[] | select(.name == "miro-mcp-server") | .domain_name')

echo "=== Miro MCP Server Production Status ==="
echo "Container ID: $CONTAINER_ID"
echo "Status: $CONTAINER_STATUS"
echo "URL: https://$CONTAINER_URL"
echo ""

# Check health endpoint
echo "=== Health Check ==="
HEALTH_RESPONSE=$(curl -s "https://${CONTAINER_URL}/health")
echo "$HEALTH_RESPONSE"
echo ""

# Query logs via Loki API
echo "=== Recent Logs (last 100 lines) ==="
# Scaleway Loki endpoint for fr-par region
LOKI_URL="https://logs.cockpit.fr-par.scw.cloud/loki/api/v1/query_range"

# Time range: last hour
START_TIME=$(date -u -v-1H '+%s')000000000  # nanoseconds
END_TIME=$(date -u '+%s')000000000

# Query logs for our container
QUERY="{container_id=\"$CONTAINER_ID\"}"

# Make request to Loki
curl -s -G "$LOKI_URL" \
    -H "Authorization: Bearer $SCW_COCKPIT_TOKEN" \
    --data-urlencode "query=$QUERY" \
    --data-urlencode "start=$START_TIME" \
    --data-urlencode "end=$END_TIME" \
    --data-urlencode "limit=100" \
    | jq -r '.data.result[]?.values[]? | .[1]' 2>/dev/null || echo "No logs found or API error"

echo ""
echo "=== Access Full Logs ==="
echo "Grafana UI: https://cockpit.fr-par.scw.cloud/grafana"
echo "Use the Cockpit token from .env.local to authenticate"
