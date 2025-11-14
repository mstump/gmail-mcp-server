# Gmail MCP Server

A Model Context Protocol (MCP) server that provides tools for searching, reading, and managing Gmail emails. Built with Rust and designed to run as a persistent HTTP server.

## Features

- **Search Gmail threads** - Full Gmail search capabilities with query strings
- **Create and manage drafts** - Create email drafts with thread awareness
- **Extract attachment text** - Safely extract text from PDF, DOCX, and TXT attachments
- **Fetch email bodies** - Retrieve full email content for threads
- **Download attachments** - Download attachments to local filesystem
- **Forward emails** - Forward emails with original content
- **Send drafts** - Send existing draft emails

## Prerequisites

- Rust 1.91 or later
- Google Cloud Project with Gmail API enabled
- OAuth 2.0 credentials (Client ID and Client Secret)

## Setup

### 1. Google Cloud Project Setup

#### Create a Google Cloud Project

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Click "Select a project" dropdown at the top
3. Click "New Project"
4. Enter a project name (e.g., "Gmail MCP Server")
5. Click "Create"

#### Enable Gmail API

1. In your project, go to "APIs & Services" → "Library"
2. Search for "Gmail API"
3. Click on "Gmail API" and click "Enable"

#### Create OAuth 2.0 Credentials

1. Go to "APIs & Services" → "Credentials"
2. Click "Create Credentials" → "OAuth Client ID"
3. If prompted, configure the OAuth consent screen:
   - Choose "External" user type
   - Fill in required fields (App name, User support email, Developer email)
   - Add your email to "Test users" section
   - Save and continue through all steps
4. Back in Credentials, click "Create Credentials" → "OAuth Client ID"
5. Choose "Desktop application" as the application type
6. Enter a name (e.g., "Gmail MCP Client")
7. Click "Create"
8. **Important**: Copy the **Client ID** and **Client Secret** from the confirmation dialog

#### OAuth Scopes

The server requests the following OAuth scopes:

- `https://www.googleapis.com/auth/gmail.readonly` - Read Gmail messages
- `https://www.googleapis.com/auth/gmail.compose` - Create and send drafts

## Building

### Using Make (Recommended)

The project includes a Makefile with convenient build targets:

```bash
# Build the release binary (default)
make

# Or explicitly
make build

# Build debug binary
make build-dev

# Run tests
make test

# Build Docker image
make docker

# Clean build artifacts
make clean

# Run all: clean, deps, test, build, docker
make all
```

### Manual Build

```bash
# Get dependencies
cargo fetch

# Build release binary
cargo build --release

# Build debug binary
cargo build

# Run tests
cargo test
```

The binary will be located at `target/release/gmail-mcp-server` (or `target/debug/gmail-mcp-server` for debug builds).

## Configuration

The server can be configured via command-line flags and environment variables.

### Command-Line Flags

Run `gmail-mcp-server --help` for a full list of commands and flags.

- `--gmail-client-id`: Your Google OAuth Client ID
- `--gmail-client-secret`: Your Google OAuth Client Secret
- `--app-data-dir`: Custom directory for storing application data (e.g., tokens)

**HTTP Server Flags (`http` command):**

- `--port`: HTTP server port (default: 8080)
- `--oauth-redirect-url`: Custom OAuth redirect URL
- `--metrics-route`: Metrics endpoint path
- `--http-stream-route`: HTTP stream endpoint path
- `--sse-prefix`: SSE router prefix path
- `--login-route`: Login endpoint path
- `--callback-route`: OAuth callback endpoint path
- `--health-route`: Health check endpoint path
- `--root-route`: Root endpoint path

### Environment Variables

The server also supports environment variables, which correspond to the CLI flags.

- `GMAIL_CLIENT_ID`
- `GMAIL_CLIENT_SECRET`
- `APP_DATA_DIR`
- `PORT`
- `OAUTH_REDIRECT_URL`
- `METRICS_ROUTE`
- `HTTP_STREAM_ROUTE`
- `SSE_PREFIX`
- `LOGIN_ROUTE`
- `CALLBACK_ROUTE`
- `HEALTH_ROUTE`
- `ROOT_ROUTE`

### Using a `.env` File

Create a `.env` file in the project root:

```bash
# Required
GMAIL_CLIENT_ID=your_client_id_here.apps.googleusercontent.com
GMAIL_CLIENT_SECRET=your_client_secret_here

# Optional
PORT=8080
```

The server automatically loads environment variables from a `.env` file if it exists.

### File Storage Locations

The server stores authentication tokens in the following locations:

- **Windows**: `%APPDATA%\\gmail-mcp-server-data\\`
- **macOS/Linux**: `~/.gmail-mcp-server-data/`

The token file is stored as `token.json` in this directory.

## Running the Server

The server is now managed via CLI commands.

### `http` Command

Run the HTTP server. All flags are optional.

```bash
# Run with default settings
gmail-mcp-server http

# Run with custom port
gmail-mcp-server http --port 3000

# Run with custom client ID and secret
gmail-mcp-server --gmail-client-id "YOUR_ID" --gmail-client-secret "YOUR_SECRET" http
```

### `tools` Command

Access MCP tools directly from the command line.

**Note:** All `tools` subcommands require `--gmail-client-id` and `--gmail-client-secret` to be set, either as flags or environment variables.

#### `search-threads`

Search Gmail threads.

```bash
gmail-mcp-server tools search-threads "from:test@example.com" --max-results 5
```

#### `create-draft`

Create a new draft.

```bash
gmail-mcp-server tools create-draft "recipient@example.com" "Subject" "Body" --thread-id "thread123"
```

#### `extract-attachment`

Extract text from an attachment.

```bash
gmail-mcp-server tools extract-attachment "message123" "report.pdf"
```

#### `fetch-email-bodies`

Fetch email bodies for one or more thread IDs.

```bash
gmail-mcp-server tools fetch-email-bodies "thread123" "thread456"
```

#### `download-attachment`

Download an attachment.

```bash
gmail-mcp-server tools download-attachment "message123" "invoice.pdf" --download-dir "/tmp/downloads"
```

#### `forward-email`

Forward an email.

```bash
gmail-mcp-server tools forward-email "message123" "forward-to@example.com" "Fwd: Subject" "Please see this"
```

#### `send-draft`

Send a draft.

```bash
gmail-mcp-server tools send-draft "draft123"
```

## Server Endpoints

The server exposes the following HTTP endpoints:

- **Root** (`GET /`) - Server information page with endpoint documentation
- **Health Check** (`GET /health`) - Health check endpoint (returns `200 OK`)
- **Login** (`GET /login`) - OAuth authentication initiation (redirects to Google OAuth)
- **Callback** (`GET /callback`) - OAuth callback handler (processes OAuth response)
- **Metrics** (`GET /metrics`) - Prometheus metrics endpoint (returns Prometheus-formatted metrics)
- **HTTP Stream** (`POST /stream`) - MCP protocol endpoint via HTTP streaming
- **SSE** (`GET /sse/sse`) - Server-Sent Events endpoint for MCP protocol
- **SSE POST** (`POST /sse/message`) - POST endpoint for SSE-based MCP protocol

**Note:** All route paths can be customized via environment variables (see [Configuration](#configuration) section).

## Docker Deployment

### Build the Docker Image

```bash
make docker

# Or manually
docker build -t gmail-mcp-server:latest .
```

### Run the Container

```bash
docker run -d \
  --name gmail-mcp-server \
  -p 8080:8080 \
  -e GMAIL_CLIENT_ID=your_client_id_here.apps.googleusercontent.com \
  -e GMAIL_CLIENT_SECRET=your_client_secret_here \
  gmail-mcp-server:latest
```

### Using Environment File

Create a `.env` file:

```bash
GMAIL_CLIENT_ID=your_client_id_here.apps.googleusercontent.com
GMAIL_CLIENT_SECRET=your_client_secret_here
PORT=8080
```

Then run:

```bash
docker run -d \
  --name gmail-mcp-server \
  -p 8080:8080 \
  --env-file .env \
  gmail-mcp-server:latest
```

### Persistent Token Storage

To persist OAuth tokens across container restarts, mount a volume:

```bash
docker run -d \
  --name gmail-mcp-server \
  -p 8080:8080 \
  -v $(pwd)/.gmail-mcp-server-data:/root/.gmail-mcp-server-data \
  -e GMAIL_CLIENT_ID=your_client_id_here.apps.googleusercontent.com \
  -e GMAIL_CLIENT_SECRET=your_client_secret_here \
  gmail-mcp-server:latest
```

### Custom Port

```bash
docker run -d \
  --name gmail-mcp-server \
  -p 3000:3000 \
  -e PORT=3000 \
  -e GMAIL_CLIENT_ID=your_client_id_here.apps.googleusercontent.com \
  -e GMAIL_CLIENT_SECRET=your_client_secret_here \
  gmail-mcp-server:latest
```

### Container Management

```bash
# View logs
docker logs gmail-mcp-server

# Stop the container
docker stop gmail-mcp-server

# Start the container
docker start gmail-mcp-server

# Remove the container
docker rm gmail-mcp-server
```

## MCP Client Configuration

### Cursor

1. Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (Mac)
2. Type "MCP" and select "MCP: Add new server"
3. Edit the configuration file

Add the following configuration:

```json
{
  "mcpServers": {
    "gmail": {
      "url": "http://localhost:8080/mcp",
      "transport": "http"
    }
  }
}
```

### Claude Desktop

1. Go to File > Settings > Developer > Edit Config
2. Edit the configuration file

Add the following configuration:

```json
{
  "mcpServers": {
    "gmail": {
      "url": "http://localhost:8080/mcp",
      "transport": "http"
    }
  }
}
```

### Manual Configuration

You can edit these config files directly:

- **Cursor**: `~/.cursor/mcp.json` (macOS/Linux) or `%APPDATA%\Cursor\mcp.json` (Windows)
- **Claude Desktop**: `%APPDATA%\Claude\claude_desktop_config.json` (Windows) or `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)

## Development

### Running Tests

```bash
# Run all tests
make test

# Or with cargo
cargo test

# Run with verbose output
cargo test --verbose
```

### Logging

The server uses the `tracing` crate for logging. Set the `RUST_LOG` environment variable to control log levels:

```bash
# Debug logging
RUST_LOG=debug ./gmail-mcp-server

# Info logging (default)
RUST_LOG=info ./gmail-mcp-server

# Error logging only
RUST_LOG=error ./gmail-mcp-server
```

## Troubleshooting

### Authentication Issues

If you encounter authentication errors:

1. Delete the token file: `rm ~/.gmail-mcp-server-data/token.json` (or equivalent on Windows)
2. Restart the server
3. Visit the login URL again: `http://localhost:8080/login`

### Port Already in Use

If port 8080 is already in use:

```bash
# Use a different port
PORT=3000 ./gmail-mcp-server
```

### OAuth Redirect URL Mismatch

Ensure the OAuth redirect URL in Google Cloud Console matches:

- `http://localhost:8080/callback` (default)
- Or your configured `OAUTH_REDIRECT_URL`

## License

MIT License - see LICENSE file for details.
