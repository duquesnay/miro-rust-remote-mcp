# Miro Rust Remote MCP Server

A production-ready Model Context Protocol (MCP) server for Miro board manipulation, built in Rust with OAuth2 authentication. Enables Claude AI to programmatically create and manage Miro boards, with special focus on visualizing agile squad organizational structures.

## Features

- **OAuth2 Authentication**: Secure authorization code flow with PKCE and CSRF protection
- **Automatic Token Refresh**: Seamless token renewal without user intervention
- **Board Operations**: List and create Miro boards programmatically
- **Visual Elements**: Create sticky notes, shapes, text, and frames
- **Item Management**: List, update, and delete board items (coming soon)
- **Connectors**: Create styled arrows and lines showing relationships (coming soon)
- **Squad Visualization**: Rapid agile team structure visualization (coming soon)

## Key Differentiators

- **First OAuth2-enabled Miro MCP**: Unlike existing TypeScript implementations using static tokens
- **Rust Performance**: Memory-safe, concurrent, and fast
- **Remote MCP**: Accessible from Claude.ai web interface via HTTPS
- **Production-Ready**: AES-256-GCM token encryption, comprehensive error handling

## Prerequisites

- Rust 1.70+ (2021 edition)
- Miro Developer Account
- OpenSSL (for generating encryption keys)

## Setup

### 1. Create Miro OAuth2 App

1. Visit https://developers.miro.com/
2. Click "Your Apps" → "Create new app"
3. Note your **Client ID** and **Client Secret**
4. Add redirect URI: `http://localhost:3010/oauth/callback` (for local development)

### 2. Configure Application

Create the configuration directory and file:

```bash
# Create config directory
mkdir -p ~/.config/mcp/miro-rust

# Copy example config
cp config.example.json ~/.config/mcp/miro-rust/config.json

# Generate encryption key
openssl rand -hex 32

# Edit config.json with your credentials
nano ~/.config/mcp/miro-rust/config.json
```

Configuration file (`~/.config/mcp/miro-rust/config.json`):

```json
{
  "client_id": "your_client_id_here",
  "client_secret": "your_client_secret_here",
  "redirect_uri": "http://localhost:3010/oauth/callback",
  "encryption_key": "output_from_openssl_rand_hex_32",
  "port": 3010
}
```

**Configuration Fields:**
- `client_id`: Your Miro OAuth2 Client ID (from Miro Developer Portal)
- `client_secret`: Your Miro OAuth2 Client Secret (from Miro Developer Portal)
- `redirect_uri`: OAuth2 callback URL (must match Miro app configuration)
  - Development: `http://localhost:3010/oauth/callback`
  - Production: `https://your-domain.com/oauth/callback`
- `encryption_key`: 32-byte hex string for token encryption (generate with `openssl rand -hex 32`)
- `port`: Server port (3010 for development)

### 3. Build and Run

```bash
# Install dependencies and build
cargo build --release

# Run the server
cargo run --release

# Or run in development mode
cargo run
```

### 4. Start OAuth Flow

The MCP server will provide a `start_auth` tool that returns an authorization URL. Open this URL in your browser to authorize the application.

## Project Structure

```
miro-mcp-server/
├── src/
│   ├── auth/           # OAuth2 implementation
│   │   ├── oauth.rs    # Authorization code flow with PKCE
│   │   ├── token_store.rs  # AES-256-GCM encrypted token storage
│   │   └── types.rs    # Authentication types and errors
│   ├── mcp/            # MCP protocol implementation
│   │   ├── server.rs   # MCP server with rmcp framework
│   │   └── auth_handler.rs  # OAuth callback handling
│   ├── miro/           # Miro API client
│   │   ├── client.rs   # HTTP client with auto-refresh
│   │   └── types.rs    # Miro API types (boards, items, etc.)
│   ├── config.rs       # Environment configuration
│   ├── lib.rs          # Library exports
│   └── main.rs         # Entry point
├── planning/           # Agile planning artifacts
└── tests/             # Integration tests
```

## Available MCP Tools

### Board Operations
- `list_boards`: List all accessible Miro boards
- `create_board`: Create a new board with name and description

### Visual Elements
- `create_sticky_note`: Create sticky notes with custom content, position, and color
- `create_shape`: Create shapes (rectangle, circle, triangle) for org structures
- `create_text`: Create text elements on boards
- `create_frame`: Create frames for grouping related content

### Coming Soon
- `list_items`: List board items filtered by type
- `update_item`: Update item properties dynamically
- `delete_item`: Remove items from boards
- `create_connector`: Connect items with styled arrows/lines
- Squad visualization tools for rapid org chart creation

## Security

- **OAuth2 Security**: PKCE prevents authorization code interception, state parameter prevents CSRF
- **Token Encryption**: AES-256-GCM encryption for tokens at rest
- **Secrets Management**: All credentials loaded from environment variables
- **No Unsafe Code**: 100% safe Rust, memory safety guaranteed
- **Comprehensive Error Handling**: Result types throughout, no production panics

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Code Quality

```bash
# Lint with clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

### Git Workflow

This project follows the Feature Branch Workflow with atomic commits:

```bash
# Create feature branch
git checkout -b feat/feature-name

# Make changes, run tests
cargo test && cargo clippy

# Commit (all commits reviewed for completeness)
git add .
git commit -m "feat: description of change"

# Merge to main
git checkout main
git merge feat/feature-name
```

## Architecture Decisions

### Why Rust?
- Memory safety without garbage collection
- Excellent async/await support via tokio
- Strong type system prevents many bugs at compile time
- Fast performance for production deployment

### Why OAuth2 vs Static Tokens?
- Better security (tokens can be revoked)
- User-specific permissions (each user authorizes individually)
- Automatic refresh (no manual token renewal)
- Required for Claude.ai web interface integration

### Why AES-256-GCM for Token Storage?
- Authenticated encryption (confidentiality + integrity)
- Industry standard, well-audited
- Efficient in Rust via `ring` crate
- Prevents token tampering

## Deployment

### Local Development
1. Configure `~/.config/mcp/miro-rust/config.json` with localhost redirect URI
2. Run `cargo run`
3. OAuth callback works on localhost:3010

### Production (HTTPS Required)
1. Deploy to Scaleway Containers (or alternative platform)
2. Configure HTTPS/TLS certificate
3. Update redirect URI in `~/.config/mcp/miro-rust/config.json` and Miro Developer Portal
4. Deploy with `cargo build --release`

**Configuration on Production Server:**
```bash
# On your production server
mkdir -p ~/.config/mcp/miro-rust
nano ~/.config/mcp/miro-rust/config.json

# Add your production configuration with HTTPS redirect URI:
# "redirect_uri": "https://your-domain.com/oauth/callback"
```

**Recommended Platforms:**
- **Scaleway Containers** (Selected): Container-based deployment, native HTTPS, predictable pricing
- **Railway**: Simple deployment, automatic HTTPS, environment-based config
- **Self-hosted**: Full control, requires Nginx for HTTPS, manual config management

## Primary Use Case: Agile Squad Visualization

Example prompt to Claude:

> "Create a Miro board showing 3 agile squads (Alpha, Beta, Gamma). Each squad has 1 Product Owner, 1 Scrum Master, and 5 developers. Show reporting lines from team members to Scrum Master."

The MCP server will:
1. Create a new board
2. Create frames for each squad
3. Create color-coded sticky notes for team members (PO=yellow, SM=green, Dev=blue)
4. Create connectors showing reporting relationships
5. Return the board URL

All in <5 minutes via natural language!

## API Documentation

### Miro API v2
- **Base URL**: `https://api.miro.com/v2/`
- **Authentication**: Bearer token (OAuth2)
- **Rate Limit**: 100 requests/minute per user
- **Documentation**: https://developers.miro.com/docs/rest-api-reference

### MCP Protocol
- **Specification**: https://modelcontextprotocol.io/
- **Transport**: stdio (Claude.ai web interface) or HTTP
- **Framework**: rmcp (official Rust SDK)

## Troubleshooting

### "Configuration file not found"
- Ensure config directory exists: `mkdir -p ~/.config/mcp/miro-rust`
- Copy example config: `cp config.example.json ~/.config/mcp/miro-rust/config.json`
- Edit config with your credentials: `nano ~/.config/mcp/miro-rust/config.json`

### "Authentication failed"
- Check `client_id` and `client_secret` in `~/.config/mcp/miro-rust/config.json`
- Verify `redirect_uri` matches Miro app configuration exactly
- Check token hasn't expired (refresh should be automatic)

### "Token encryption failed"
- Ensure `encryption_key` is exactly 64 hex characters (32 bytes)
- Generate new key: `openssl rand -hex 32`
- Update it in `~/.config/mcp/miro-rust/config.json`

### "Rate limit exceeded"
- Miro API limit: 100 requests/minute
- Wait 60 seconds and retry
- Consider bulk operations for large visualizations

## Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feat/amazing-feature`)
3. Follow Rust conventions (run `cargo fmt` and `cargo clippy`)
4. Write tests for new features
5. Ensure all tests pass (`cargo test`)
6. Commit with conventional format (`feat:`, `fix:`, `refactor:`)
7. Push and create Pull Request

## Roadmap

### Sprint 1 (Complete) ✅
- [x] OAuth2 authorization with PKCE
- [x] Automatic token refresh
- [x] Board list and creation
- [x] Visual element creation (sticky notes, shapes, text, frames)

### Sprint 2 (In Progress)
- [ ] Item management (list, update, delete)
- [ ] Connector creation with captions
- [ ] Squad visualization orchestration

### Sprint 3 (Planned)
- [ ] Bulk operations for performance
- [ ] Production deployment guide
- [ ] Claude.ai web interface integration testing

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Miro API v2 for comprehensive board manipulation
- MCP protocol for AI-native tool integration
- rmcp Rust SDK for clean MCP implementation
- oauth2-rs for robust OAuth2 client
- Anthropic for Claude Code development platform

## Support

- **Issues**: https://github.com/yourusername/miro-rust-remote-mcp/issues
- **Miro API Docs**: https://developers.miro.com/docs
- **MCP Specification**: https://modelcontextprotocol.io/

---

**Built with ❤️ in Rust | Powered by OAuth2 | Secured by AES-256-GCM**
