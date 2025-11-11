#!/bin/bash
# Simple production monitoring script

set -e

CONTAINER_URL="flyagileapipx8njvei-miro-mcp-server.functions.fnc.fr-par.scw.cloud"

echo "=== Miro MCP Server - Production Monitor ==="
echo "URL: https://$CONTAINER_URL"
echo ""

# Check health
echo "=== Health Check ==="
HEALTH=$(curl -s -w "\nHTTP Status: %{http_code}\nResponse Time: %{time_total}s" "https://${CONTAINER_URL}/health")
echo "$HEALTH"
echo ""

# Check root endpoint (MCP server info)
echo "=== MCP Server Info ==="
ROOT=$(curl -s -w "\nHTTP Status: %{http_code}" "https://${CONTAINER_URL}/")
echo "$ROOT"
echo ""

# Container status via Scaleway CLI
echo "=== Container Status (Scaleway) ==="
scw container container list region=fr-par -o json | jq -r '.[] | select(.name == "miro-mcp-server") | {
    status,
    cpu_limit,
    memory_limit,
    min_scale,
    max_scale,
    updated_at
}'
echo ""

echo "=== Logs Access ==="
echo "Grafana UI: https://cockpit.fr-par.scw.cloud/grafana"
echo "  1. Click 'Explore' in left sidebar"
echo "  2. Select 'Logs' datasource"
echo "  3. Query: {container_name=\"miro-mcp-server\"}"
echo ""
echo "Or use token from .env.local to authenticate via API"
