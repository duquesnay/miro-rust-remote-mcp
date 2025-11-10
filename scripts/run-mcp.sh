#!/usr/bin/env bash
#
# Wrapper script to run Miro MCP Server via Docker with credentials from .env
# This script is referenced by Claude Desktop config to avoid hardcoding credentials
#

set -euo pipefail

# Load credentials from .env file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "${SCRIPT_DIR}")"
ENV_FILE="${PROJECT_DIR}/.env"

if [[ ! -f "${ENV_FILE}" ]]; then
    echo "Error: .env file not found at ${ENV_FILE}" >&2
    exit 1
fi

# Source environment variables
set -a
source "${ENV_FILE}"
set +a

# Run Docker container with environment variables
# Map port 3010 for OAuth callbacks from browser
exec docker run -i --rm \
    -v miro-mcp-data:/app/data \
    -p 3010:3010 \
    -e MIRO_CLIENT_ID \
    -e MIRO_CLIENT_SECRET \
    -e MIRO_REDIRECT_URI \
    -e MIRO_ENCRYPTION_KEY \
    -e RUST_LOG \
    miro-mcp-server:test
