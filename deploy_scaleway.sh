#!/usr/bin/env bash
#
# Deploy Miro MCP Server to Scaleway Containers
#

set -e

# Configuration
PROJECT_NAME="miro-mcp"
REGION="${SCW_DEFAULT_REGION:-fr-par}"
REGISTRY_REGION="${SCW_DEFAULT_REGION:-fr-par}"
IMAGE_TAG="${IMAGE_TAG:-latest}"  # Accept tag from environment or default to 'latest'

echo "🚀 Deploying Miro MCP Server (ADR-002) to Scaleway Containers"
echo "📌 Image tag: ${IMAGE_TAG}"
echo ""

# Step 1: Build Docker image (only if IMAGE_TAG is 'latest')
if [ "$IMAGE_TAG" = "latest" ]; then
    echo "📦 Building Docker image..."
    docker build -t ${PROJECT_NAME}:latest .
else
    echo "⏭️  Skipping build (using pre-built image from GitHub Container Registry)"
fi

# Step 2: Get Scaleway registry endpoint (use existing container namespace's registry)
echo ""
echo "🔍 Getting Scaleway Container Registry endpoint..."

# Get the registry from an existing container namespace
CONTAINER_NAMESPACE_ID=$(scw container namespace list -o json | jq -r '.[0].id')
if [ -z "$CONTAINER_NAMESPACE_ID" ] || [ "$CONTAINER_NAMESPACE_ID" = "null" ]; then
    echo "❌ No Container Namespace found. Please create one in Scaleway Console first."
    exit 1
fi

REGISTRY_NAMESPACE_ID=$(scw container namespace list -o json | jq -r '.[0].registry_namespace_id')
REGISTRY_ENDPOINT=$(scw registry namespace get ${REGISTRY_NAMESPACE_ID} -o json | jq -r '.endpoint')

echo "   Registry: $REGISTRY_ENDPOINT"

# Step 3: Tag and push image
echo ""
if [ "$IMAGE_TAG" = "latest" ]; then
    echo "📤 Pushing image to Scaleway Registry..."
    docker tag ${PROJECT_NAME}:latest ${REGISTRY_ENDPOINT}/${PROJECT_NAME}:latest
    docker push ${REGISTRY_ENDPOINT}/${PROJECT_NAME}:latest
else
    echo "📤 Pulling and pushing versioned image..."
    docker pull ghcr.io/${GITHUB_REPOSITORY}:${IMAGE_TAG}
    docker tag ghcr.io/${GITHUB_REPOSITORY}:${IMAGE_TAG} ${REGISTRY_ENDPOINT}/${PROJECT_NAME}:${IMAGE_TAG}
    docker push ${REGISTRY_ENDPOINT}/${PROJECT_NAME}:${IMAGE_TAG}
fi

# Step 4: Deploy container
echo ""
echo "🚢 Deploying container..."

# Check if container already exists
CONTAINER_ID=$(scw container container list -o json | jq -r ".[] | select(.name == \"${PROJECT_NAME}\") | .id")

# Use IMAGE_TAG for the registry image
REGISTRY_IMAGE="${REGISTRY_ENDPOINT}/${PROJECT_NAME}:${IMAGE_TAG}"

if [ -z "$CONTAINER_ID" ] || [ "$CONTAINER_ID" = "null" ]; then
    echo "   Creating new container..."
    CONTAINER_ID=$(scw container container create \
        name=${PROJECT_NAME} \
        region=${REGION} \
        namespace-id=${CONTAINER_NAMESPACE_ID} \
        registry-image=${REGISTRY_IMAGE} \
        port=3010 \
        min-scale=1 \
        max-scale=1 \
        cpu-limit=250 \
        memory-limit=256 -o json | jq -r '.id')
else
    echo "   Updating existing container (ID: ${CONTAINER_ID})..."
    scw container container update ${CONTAINER_ID} \
        registry-image=${REGISTRY_IMAGE}
fi

# Step 5: Deploy (start) the container
echo ""
echo "▶️  Starting container..."
scw container container deploy ${CONTAINER_ID} --wait

# Step 6: Get container URL
CONTAINER_URL=$(scw container container get ${CONTAINER_ID} -o json | jq -r '.domain_name')

echo ""
echo "✅ Deployment complete!"
echo ""
echo "🔗 Container URL: https://${CONTAINER_URL}"
echo "🔍 OAuth metadata: https://${CONTAINER_URL}/.well-known/oauth-protected-resource"
echo "🏥 Health check: https://${CONTAINER_URL}/health"
echo ""
echo "📝 Next steps:"
echo "   1. Test health endpoint: curl https://${CONTAINER_URL}/health"
echo "   2. Configure custom connector in claude.ai"
echo "   3. Test OAuth flow"
