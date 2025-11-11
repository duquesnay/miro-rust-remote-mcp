#!/bin/bash
# Comprehensive log access script for Miro MCP Server production
# Usage: ./scripts/access-logs.sh [method]
#   method: grafana | check-config | help (default: grafana)

set -e

CONTAINER_NAME="miro-mcp-server"
REGION="fr-par"

# Load Cockpit token if available
if [ -f .env.local ]; then
    source .env.local
fi

show_help() {
    cat << EOF
=== Miro MCP Server - Log Access Guide ===

METHODS:

1. Grafana UI (Recommended)
   ./scripts/access-logs.sh grafana

   Opens Grafana dashboard where you can explore logs visually.

2. Check Configuration
   ./scripts/access-logs.sh check-config

   Verifies if logs are being sent to Cockpit.

3. Container Status
   ./scripts/access-logs.sh status

   Shows current container health and basic metrics.

TROUBLESHOOTING:

If no logs appear in Grafana:
- Scaleway Containers may not send logs to Cockpit by default
- Check container environment variables for log configuration
- Verify application is actually writing to stdout/stderr
- Container logs may have retention limits

TOKEN:
Cockpit token stored in .env.local: $([ -n "$SCW_COCKPIT_TOKEN" ] && echo "✓ Found" || echo "✗ Not found")

EOF
}

grafana_access() {
    echo "=== Access Logs via Grafana ==="
    echo ""
    echo "1. Open Grafana: https://cockpit.fr-par.scw.cloud/grafana"
    echo ""
    echo "2. Login:"
    echo "   - Username: Use your Scaleway email/credentials"
    echo "   - Or use Cockpit token: ${SCW_COCKPIT_TOKEN:0:20}..."
    echo ""
    echo "3. Navigate to Explore (left sidebar)"
    echo ""
    echo "4. Select 'Logs' datasource"
    echo ""
    echo "5. Query examples:"
    echo "   {job=\"serverless-containers\"}"
    echo "   {container_name=\"$CONTAINER_NAME\"}"
    echo "   {namespace=\"fly-agile-api\"}"
    echo ""
    echo "6. Time range: Select last 1h, 6h, or custom"
    echo ""

    # Try to open in browser (macOS)
    if command -v open &> /dev/null; then
        echo "Opening Grafana in browser..."
        open "https://cockpit.fr-par.scw.cloud/grafana"
    fi
}

check_config() {
    echo "=== Checking Log Configuration ==="
    echo ""

    # Get container details
    CONTAINER_JSON=$(scw container container list region=$REGION -o json | jq -r ".[] | select(.name == \"$CONTAINER_NAME\")")

    if [ -z "$CONTAINER_JSON" ]; then
        echo "✗ Container not found: $CONTAINER_NAME"
        exit 1
    fi

    CONTAINER_ID=$(echo "$CONTAINER_JSON" | jq -r '.id')
    NAMESPACE_ID=$(echo "$CONTAINER_JSON" | jq -r '.namespace_id')
    STATUS=$(echo "$CONTAINER_JSON" | jq -r '.status')

    echo "Container ID: $CONTAINER_ID"
    echo "Namespace ID: $NAMESPACE_ID"
    echo "Status: $STATUS"
    echo ""

    # Get full container details
    echo "=== Container Configuration ==="
    scw container container get region=$REGION container-id=$CONTAINER_ID -o json | jq '{
        name,
        status,
        registry_image,
        environment_variables: (.environment_variables | length),
        cpu_limit,
        memory_limit,
        min_scale,
        max_scale
    }'
    echo ""

    # Check if Cockpit is enabled
    echo "=== Cockpit Integration Status ==="
    if [ -n "$SCW_COCKPIT_TOKEN" ]; then
        echo "✓ Cockpit token configured"
        echo "  Token: ${SCW_COCKPIT_TOKEN:0:20}..."
    else
        echo "✗ Cockpit token not found in .env.local"
        echo "  Run: scw cockpit token create name=miro-mcp-logs region=$REGION"
    fi
    echo ""

    # Test Loki API
    echo "=== Testing Loki API Access ==="
    if [ -n "$SCW_COCKPIT_TOKEN" ]; then
        HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
            -H "Authorization: Bearer $SCW_COCKPIT_TOKEN" \
            "https://logs.cockpit.fr-par.scw.cloud/loki/api/v1/labels")

        if [ "$HTTP_STATUS" = "200" ]; then
            echo "✓ Loki API accessible (HTTP $HTTP_STATUS)"
        else
            echo "✗ Loki API returned HTTP $HTTP_STATUS"
            echo "  This may mean:"
            echo "  - Logs not yet ingested (container just started)"
            echo "  - Token needs additional permissions"
            echo "  - Logs not configured to send to Cockpit"
        fi
    else
        echo "⊘ Skipped (no token)"
    fi
    echo ""

    echo "=== Recommendations ==="
    echo "1. Ensure container writes logs to stdout/stderr"
    echo "2. Check Grafana UI for available log streams"
    echo "3. Container logs may take 1-2 minutes to appear after startup"
    echo "4. Use 'grafana' method for visual log exploration"
}

container_status() {
    echo "=== Container Status & Metrics ==="
    echo ""

    # Get container info
    CONTAINER_JSON=$(scw container container list region=$REGION -o json | jq -r ".[] | select(.name == \"$CONTAINER_NAME\")")

    if [ -z "$CONTAINER_JSON" ]; then
        echo "✗ Container not found: $CONTAINER_NAME"
        exit 1
    fi

    CONTAINER_URL=$(echo "$CONTAINER_JSON" | jq -r '.domain_name')

    echo "$CONTAINER_JSON" | jq '{
        name,
        status,
        domain_name,
        cpu_limit,
        memory_limit,
        min_scale,
        max_scale,
        created_at,
        updated_at
    }'
    echo ""

    # Health check
    echo "=== Health Check ==="
    HEALTH_RESPONSE=$(curl -s -w "\nHTTP %{http_code} | %{time_total}s" "https://${CONTAINER_URL}/health")
    echo "$HEALTH_RESPONSE"
    echo ""

    echo "=== Quick Access ==="
    echo "Health: https://${CONTAINER_URL}/health"
    echo "Logs: ./scripts/access-logs.sh grafana"
}

# Main
METHOD=${1:-grafana}

case "$METHOD" in
    grafana)
        grafana_access
        ;;
    check-config)
        check_config
        ;;
    status)
        container_status
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Unknown method: $METHOD"
        echo ""
        show_help
        exit 1
        ;;
esac
