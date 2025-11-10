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

### Environment Variables

The server supports the following environment variables:

#### Required

- `GMAIL_CLIENT_ID` - Your Google OAuth Client ID (required)
- `GMAIL_CLIENT_SECRET` - Your Google OAuth Client Secret (required)

#### Optional

- `PORT` - HTTP server port (default: `8080`)
- `OAUTH_REDIRECT_URL` - OAuth redirect URL (default: `http://localhost:{PORT}/callback`)
- `METRICS_ROUTE` - Metrics endpoint path (default: `/metrics`)
- `MCP_ROUTE` - MCP endpoint path (default: `/mcp`)
- `LOGIN_ROUTE` - Login endpoint path (default: `/login`)
- `APP_DATA_DIR` - Application data directory (default: platform-specific location)

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

- **Windows**: `%APPDATA%\gmail-mcp-server-data\`
- **macOS/Linux**: `~/.gmail-mcp-server-data/`

The token file is stored as `token.json` in this directory.

## Running the Server

### Local Development

```bash
# Build the server
make build

# Run the server
./gmail-mcp-server

# Or with custom port
PORT=3000 ./gmail-mcp-server
```

The server will:

1. Start on `http://localhost:8080` (or your configured port)
2. Display login URL: `http://localhost:8080/login`
3. Visit the login URL in your browser to authenticate
4. After authentication, the server is ready to accept MCP connections

### Server Endpoints

- **Root** (`/`) - Server information page
- **Health Check** (`/health`) - Health check endpoint
- **Login** (`/login`) - OAuth authentication initiation
- **Callback** (`/callback`) - OAuth callback handler
- **Metrics** (`/metrics`) - Prometheus metrics endpoint
- **MCP** (`/mcp`) - MCP protocol endpoint

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

## Available MCP Tools

### `search_threads`

Search Gmail threads using a query string.

**Parameters:**

- `query` (string, required) - Gmail search query (e.g., "from:example@gmail.com", "subject:meeting")
- `max_results` (number, optional) - Maximum number of results to return (default: 10)

**Example:**

```json
{
  "query": "from:example@gmail.com subject:meeting",
  "max_results": 20
}
```

### `create_draft`

Create a Gmail draft email.

**Parameters:**

- `to` (string, required) - Recipient email address
- `subject` (string, required) - Email subject
- `body` (string, required) - Email body text
- `thread_id` (string, optional) - Thread ID to reply to

**Example:**

```json
{
  "to": "recipient@example.com",
  "subject": "Meeting Follow-up",
  "body": "Thank you for the meeting today.",
  "thread_id": "thread_id_here"
}
```

### `extract_attachment_by_filename`

Extract text from an email attachment by filename.

**Parameters:**

- `message_id` (string, required) - Gmail message ID
- `filename` (string, required) - Attachment filename

**Example:**

```json
{
  "message_id": "message_id_here",
  "filename": "document.pdf"
}
```

### `fetch_email_bodies`

Fetch email bodies for multiple thread IDs.

**Parameters:**

- `thread_ids` (array of strings, required) - List of thread IDs to fetch

**Example:**

```json
{
  "thread_ids": ["thread_id_1", "thread_id_2"]
}
```

### `download_attachment`

Download an attachment to the local filesystem.

**Parameters:**

- `message_id` (string, required) - Gmail message ID
- `filename` (string, required) - Attachment filename
- `download_dir` (string, optional) - Download directory (default: current directory)

**Example:**

```json
{
  "message_id": "message_id_here",
  "filename": "document.pdf",
  "download_dir": "/tmp"
}
```

### `forward_email`

Forward an email.

**Parameters:**

- `message_id` (string, required) - Gmail message ID to forward
- `to` (string, required) - Recipient email address
- `subject` (string, required) - Forward subject
- `body` (string, required) - Forward body text

**Example:**

```json
{
  "message_id": "message_id_here",
  "to": "recipient@example.com",
  "subject": "Fwd: Original Subject",
  "body": "Please see the forwarded email below."
}
```

### `send_draft`

Send an existing draft email.

**Parameters:**

- `draft_id` (string, required) - Gmail draft ID to send

**Example:**

```json
{
  "draft_id": "draft_id_here"
}
```

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
