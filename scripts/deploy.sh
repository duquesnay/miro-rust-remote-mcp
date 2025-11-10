#!/usr/bin/env bash
#
# Scaleway Container deployment script for Miro MCP Server
#
# Usage:
#   ./scripts/deploy.sh --env=production  # Deploy to Scaleway
#   ./scripts/deploy.sh --env=local       # Local Docker build (default)
#   ./scripts/deploy.sh --dry-run         # Simulate deployment
#
# Prerequisites:
#   - Docker installed and running
#   - For Scaleway deployment:
#     - Scaleway CLI installed (`scw` command available)
#     - Scaleway credentials configured (`scw init`)
#     - .env.production configured with registry and credentials
#
# Configuration:
#   - .env (default): Local Docker configuration
#   - .env.production: Scaleway deployment configuration
#

set -euo pipefail

# Default environment
ENVIRONMENT="local"
DRY_RUN=false

# Load environment configuration
load_environment() {
    local env_file=".env.${ENVIRONMENT}"

    # Fallback to .env for "local" environment
    if [[ "${ENVIRONMENT}" == "local" && ! -f "${env_file}" ]]; then
        env_file=".env"
    fi

    if [[ ! -f "${env_file}" ]]; then
        echo -e "${RED}Error: Environment file '${env_file}' not found${NC}"
        echo "Available environments:"
        echo "  - local (default): .env"
        echo "  - production: .env.production"
        exit 1
    fi

    echo -e "${YELLOW}Loading configuration from ${env_file}...${NC}"

    # Source the environment file
    set -a
    source "${env_file}"
    set +a

    # Extract namespace from registry if not provided
    if [[ -z "${NAMESPACE:-}" && -n "${REGISTRY:-}" ]]; then
        NAMESPACE=$(basename "${REGISTRY}")
    fi
}

# Colors for output
RED='\033[0.31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --env=*)
      ENVIRONMENT="${1#*=}"
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    *)
      echo "Unknown option: $1"
      echo "Usage: $0 [--env=local|production] [--dry-run]"
      exit 1
      ;;
  esac
done

# Load environment configuration
load_environment

# Validate configuration based on deployment target
if [[ "${DEPLOY_TARGET}" == "scaleway" ]]; then
  if [[ -z "${REGISTRY:-}" ]]; then
    echo -e "${RED}Error: REGISTRY is required for Scaleway deployment${NC}"
    echo "Configure REGISTRY in .env.production"
    echo "Example: REGISTRY=rg.fr-par.scw.cloud/miro-mcp"
    exit 1
  fi
  # Build full image name with registry
  IMAGE_NAME="${REGISTRY}/${PROJECT_NAME}:${TAG}"
else
  # Local deployment - no registry needed
  IMAGE_NAME="${PROJECT_NAME}:${TAG}"
fi

echo -e "${GREEN}=== Miro MCP Server Deployment ===${NC}"
echo "Environment: ${ENVIRONMENT}"
echo "Deploy target: ${DEPLOY_TARGET}"
echo "Project: ${PROJECT_NAME}"
echo "Image: ${IMAGE_NAME}"
if [[ "${DEPLOY_TARGET}" == "scaleway" ]]; then
  echo "Region: ${REGION}"
  echo "Registry: ${REGISTRY}"
  echo "Namespace: ${NAMESPACE}"
fi
echo "Dry run: ${DRY_RUN}"
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

if ! command -v docker &> /dev/null; then
  echo -e "${RED}Error: Docker is not installed${NC}"
  exit 1
fi

if ! docker info &> /dev/null; then
  echo -e "${RED}Error: Docker is not running${NC}"
  exit 1
fi

# Check Scaleway CLI only for Scaleway deployment
if [[ "${DEPLOY_TARGET}" == "scaleway" ]]; then
  if ! command -v scw &> /dev/null; then
    echo -e "${RED}Error: Scaleway CLI is not installed${NC}"
    echo "Install: https://www.scaleway.com/en/docs/developer-tools/scaleway-cli/"
    exit 1
  fi
fi

echo -e "${GREEN}✓ Prerequisites OK${NC}"
echo ""

# Build Docker image
echo -e "${YELLOW}Building Docker image...${NC}"
if [[ "$DRY_RUN" == "true" ]]; then
  echo "[DRY RUN] Would run: docker build -t $IMAGE_NAME ."
else
  docker build -t "$IMAGE_NAME" .
  echo -e "${GREEN}✓ Docker build complete${NC}"
fi
echo ""

# Deployment target-specific actions
if [[ "${DEPLOY_TARGET}" == "scaleway" ]]; then
  # Push to Scaleway Container Registry
  echo -e "${YELLOW}Pushing image to Scaleway Container Registry...${NC}"
  if [[ "${DRY_RUN}" == "true" ]]; then
    echo "[DRY RUN] Would run: docker push ${IMAGE_NAME}"
  else
    docker push "${IMAGE_NAME}"
    echo -e "${GREEN}✓ Image pushed${NC}"
  fi
  echo ""

  # Deploy to Scaleway Container
  echo -e "${YELLOW}Deploying to Scaleway Container...${NC}"
  if [[ "${DRY_RUN}" == "true" ]]; then
    echo "[DRY RUN] Would run: scw container deploy ..."
    echo "  --region=${REGION}"
    echo "  --namespace-id=\$(scw container namespace list region=${REGION} name=${NAMESPACE} -o json | jq -r '.[0].id')"
    echo "  --registry-image=${IMAGE_NAME}"
    echo "  --name=${PROJECT_NAME}"
  else
    # Get namespace ID
    NAMESPACE_ID=$(scw container namespace list region="${REGION}" name="${NAMESPACE}" -o json | jq -r '.[0].id')

    if [[ -z "${NAMESPACE_ID}" || "${NAMESPACE_ID}" == "null" ]]; then
      echo -e "${RED}Error: Namespace '${NAMESPACE}' not found in region '${REGION}'${NC}"
      echo "Create namespace first: scw container namespace create name=${NAMESPACE} region=${REGION}"
      exit 1
    fi

    echo "Namespace ID: ${NAMESPACE_ID}"

    # Check if container already exists
    EXISTING_CONTAINER=$(scw container container list region="${REGION}" namespace-id="${NAMESPACE_ID}" name="${PROJECT_NAME}" -o json | jq -r '.[0].id // empty')

    if [[ -n "${EXISTING_CONTAINER}" ]]; then
      echo "Updating existing container: ${EXISTING_CONTAINER}"
      scw container container update \
        region="${REGION}" \
        container-id="${EXISTING_CONTAINER}" \
        registry-image="${IMAGE_NAME}"
    else
      echo "Creating new container"
      scw container container create \
        region="${REGION}" \
        namespace-id="${NAMESPACE_ID}" \
        name="${PROJECT_NAME}" \
        registry-image="${IMAGE_NAME}" \
        min-scale="${MIN_SCALE}" \
        max-scale="${MAX_SCALE}" \
        memory-limit="${MEMORY_LIMIT}" \
        cpu-limit="${CPU_LIMIT}"
    fi

    echo -e "${GREEN}✓ Container deployed${NC}"
  fi
  echo ""

  # Deployment complete message for Scaleway
  if [[ "${DRY_RUN}" == "false" ]]; then
    echo -e "${YELLOW}Deployment complete!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Configure Scaleway secrets:"
    echo "   - MIRO_CLIENT_ID"
    echo "   - MIRO_CLIENT_SECRET"
    echo "   - MIRO_REDIRECT_URI"
    echo "   - MIRO_ENCRYPTION_KEY"
    echo "   - TOKEN_STORAGE_PATH=/app/data/tokens.enc"
    echo ""
    echo "2. Update Miro Developer Portal redirect URI:"
    echo "   - Get container URL: scw container container list region=${REGION} namespace-id=${NAMESPACE_ID} name=${PROJECT_NAME}"
    echo "   - Register redirect URI: https://<container-url>/oauth/callback"
    echo ""
    echo "3. Test deployment:"
    echo "   scw container container logs region=${REGION} container-id=<container-id>"
  else
    echo -e "${YELLOW}[DRY RUN] Scaleway deployment simulated successfully${NC}"
  fi
else
  # Local deployment complete
  echo -e "${GREEN}✓ Local Docker build complete!${NC}"
  echo ""
  echo "Next steps:"
  echo "1. Run the container:"
  echo "   docker run -d --name ${PROJECT_NAME} \\"
  echo "     -p 3010:3010 \\"
  echo "     -e MIRO_CLIENT_ID=\${MIRO_CLIENT_ID} \\"
  echo "     -e MIRO_CLIENT_SECRET=\${MIRO_CLIENT_SECRET} \\"
  echo "     -e MIRO_REDIRECT_URI=http://localhost:3010/oauth/callback \\"
  echo "     -e MIRO_ENCRYPTION_KEY=\${MIRO_ENCRYPTION_KEY} \\"
  echo "     -v miro-mcp-data:/app/data \\"
  echo "     ${IMAGE_NAME}"
  echo ""
  echo "2. View logs:"
  echo "   docker logs -f ${PROJECT_NAME}"
  echo ""
  echo "3. Test MCP server:"
  echo "   Configure in Claude desktop app or use MCP Inspector"
fi
