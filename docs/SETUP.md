# Miro MCP Server - Setup Guide

This guide walks you through setting up the Miro MCP Server for use with Claude Desktop or Claude Code.

## Prerequisites

- Rust toolchain installed (https://rustup.rs/)
- Miro account with OAuth2 app configured
- Claude Desktop or Claude Code installed

## Step 1: Build the MCP Server

```bash
# Clone the repository
git clone https://github.com/duquesnay/miro-rust-remote-mcp.git
cd miro-rust-remote-mcp

# Build release binary
cargo build --release

# Binary will be at: target/release/miro-mcp-server
```

Note the absolute path to the binary - you'll need it for configuration.

## Step 2: Configure Miro OAuth2 Credentials

### Create Miro OAuth2 App

1. Visit https://developers.miro.com/
2. Click **Your Apps** → **Create new app**
3. Note your **Client ID** and **Client Secret**
4. Add redirect URI: `http://localhost:3010/oauth/callback`
5. Configure required scopes:
   - `boards:read`
   - `boards:write`

### Create Configuration File

```bash
# Create config directory
mkdir -p ~/.config/mcp/miro-rust

# Create config file
nano ~/.config/mcp/miro-rust/config.json
```

**Configuration content:**

```json
{
  "client_id": "YOUR_MIRO_CLIENT_ID",
  "client_secret": "YOUR_MIRO_CLIENT_SECRET",
  "redirect_uri": "http://localhost:3010/oauth/callback",
  "encryption_key": "GENERATE_WITH_OPENSSL_BELOW",
  "port": 3010
}
```

**Generate encryption key:**

```bash
openssl rand -hex 32
```

Copy the output and paste it as the `encryption_key` value in `config.json`.

## Step 3: Configure Claude Desktop

### Location

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

### Configuration

Edit your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "miro-rust": {
      "command": "/absolute/path/to/miro-rust-remote-mcp/target/release/miro-mcp-server",
      "args": [],
      "env": {}
    }
  }
}
```

**Replace `/absolute/path/to/miro-rust-remote-mcp/` with your actual path!**

Example for macOS:
```json
{
  "mcpServers": {
    "miro-rust": {
      "command": "/Users/yourname/dev/miro-rust-remote-mcp/target/release/miro-mcp-server",
      "args": [],
      "env": {}
    }
  }
}
```

### Restart Claude Desktop

After saving the configuration:
1. Quit Claude Desktop completely
2. Restart Claude Desktop
3. The Miro MCP server should now be available

## Step 4: Configure Claude Code (VSCode Extension)

### Location

- **macOS**: `~/Library/Application Support/Code/User/globalStorage/anthropic.claude-code/settings.json`
- **Linux**: `~/.config/Code/User/globalStorage/anthropic.claude-code/settings.json`
- **Windows**: `%APPDATA%\Code\User\globalStorage\anthropic.claude-code\settings.json`

Alternatively, use VSCode settings:
1. Open VSCode
2. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Windows/Linux)
3. Search for: **Claude Code: Open MCP Settings**

### Configuration

Add to your MCP settings:

```json
{
  "mcpServers": {
    "miro-rust": {
      "command": "/absolute/path/to/miro-rust-remote-mcp/target/release/miro-mcp-server",
      "args": [],
      "env": {}
    }
  }
}
```

### Reload VSCode

After saving:
1. Reload VSCode window
2. The Miro MCP server should be available in Claude Code

## Step 5: Initial Authentication

On first use, you'll need to authenticate with Miro:

### Using Claude Desktop or Claude Code:

```
User: Start Miro authentication
Claude: [Uses start_auth tool]
       Please visit this URL to authorize:
       https://miro.com/oauth/authorize?client_id=...&redirect_uri=...
```

1. Open the authorization URL in your browser
2. Log in to Miro and authorize the application
3. You'll be redirected to `http://localhost:3010/oauth/callback?code=...`
4. The server automatically exchanges the code for tokens
5. Tokens are encrypted and stored in `~/.miro-mcp/tokens.enc`

Authentication is complete! The server will automatically refresh tokens when needed.

## Verification

### Test in Claude Desktop

Try this prompt:
```
List my Miro boards
```

Expected response:
```
Found 3 board(s):
- Project Planning (ID: o9J_k1...)
- Team Retro (ID: o9J_k2...)
- Architecture Design (ID: o9J_k3...)
```

### Test in Claude Code

Try this prompt:
```
Create a new Miro board called "Test Board"
```

Expected response:
```
Successfully created board: Test Board
Board ID: o9J_k4...
```

## Available MCP Tools

Once configured, Claude can use these Miro operations:

### Authentication
- `start_auth` - Initiate OAuth2 authentication

### Board Operations
- `list_boards` - List all accessible boards
- `create_board` - Create new board with name/description

### Visual Elements
- `create_sticky_note` - Add sticky notes (content, position, color)
- `create_shape` - Add shapes (rectangles, circles, etc.)
- `create_text` - Add text elements
- `create_frame` - Add frames for grouping

### Item Management
- `list_items` - List items on a board (with optional type filter)
- `update_item` - Update item properties (position, content, style)
- `delete_item` - Remove items from board

### Connectors
- `create_connector` - Connect items with arrows/lines and captions

### Bulk Operations
- `bulk_create_items` - Create up to 20 items in a single API call

## Troubleshooting

### "Configuration file not found"

Ensure config exists:
```bash
ls -la ~/.config/mcp/miro-rust/config.json
```

If missing, recreate following Step 2.

### "Command not found" or "Permission denied"

Check binary path and permissions:
```bash
# Verify binary exists
ls -l /path/to/miro-mcp-server/target/release/miro-mcp-server

# Make executable if needed
chmod +x /path/to/miro-mcp-server/target/release/miro-mcp-server
```

### "Authentication failed"

1. Verify `client_id` and `client_secret` in `~/.config/mcp/miro-rust/config.json`
2. Ensure `redirect_uri` matches exactly: `http://localhost:3010/oauth/callback`
3. Check Miro Developer Portal that redirect URI is registered
4. Restart authentication: remove `~/.miro-mcp/tokens.enc` and re-run `start_auth`

### "Token encryption failed"

Ensure `encryption_key` is exactly 64 hex characters (32 bytes):
```bash
# Generate new key
openssl rand -hex 32

# Update in config.json
nano ~/.config/mcp/miro-rust/config.json
```

### Server not appearing in Claude

1. **Check config file syntax**: Ensure valid JSON (no trailing commas, proper quotes)
2. **Verify absolute path**: Must be full path, not relative (e.g., `~/` won't work, use `/Users/yourname/`)
3. **Restart Claude**: Completely quit and restart Claude Desktop or reload VSCode
4. **Check logs**:
   - Claude Desktop: Check Console.app (macOS) or VSCode Developer Tools
   - Look for MCP server startup errors

## Advanced Configuration

### Custom Port

If port 3010 is already in use, change in `config.json`:

```json
{
  "port": 3011
}
```

Also update `redirect_uri`:
```json
{
  "redirect_uri": "http://localhost:3011/oauth/callback"
}
```

And register new redirect URI in Miro Developer Portal.

### Production Deployment

For remote/production use:
1. Deploy to server with HTTPS
2. Update config with HTTPS redirect URI
3. Register HTTPS redirect URI in Miro Developer Portal
4. Configure Claude to connect via network (beyond scope of this guide)

## Example Usage

### Create Agile Squad Visualization

```
Create a Miro board called "Team Structure".
Add 3 frames labeled "Squad Alpha", "Squad Beta", "Squad Gamma".
In each frame, add:
- 1 yellow sticky note labeled "Product Owner"
- 1 green sticky note labeled "Scrum Master"
- 5 blue sticky notes labeled "Developer 1" through "Developer 5"
Connect each developer to the Scrum Master with arrows.
```

Claude will orchestrate multiple MCP tool calls to create this visualization in under 30 seconds.

## Support

- **Issues**: https://github.com/duquesnay/miro-rust-remote-mcp/issues
- **Documentation**: https://github.com/duquesnay/miro-rust-remote-mcp/blob/main/README.md
- **Miro API Docs**: https://developers.miro.com/docs/web-sdk-reference
