# rNGA-CLI

Command-line interface and MCP server for NGA forum, built with the `rnga` library.

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary is at target/release/rnga
```

## Usage

### Authentication

Before using commands that require login, authenticate first:

```bash
# Login with your NGA token and user ID
rnga auth login --token YOUR_TOKEN --uid YOUR_UID

# Steps to get NGA Token and UID 
# 1. Login to https://nga.178.com/
# 2. Right Click -> Inspect (F12) to open Developer Tools
# 3. Application -> Storage -> Cookies
# ngaPassportUid = UID
# ngaPassportCid = Token

# Check auth status
rnga auth status

# Logout
rnga auth logout
```

### Forum Commands

```bash
# List all forum categories
rnga forum list

# List with all forums shown (verbose)
rnga forum list -v

# Search forums
rnga forum search "gaming"

# List favorite forums (requires auth)
rnga forum favorites

# Add forum to favorites
rnga forum fav-add 310

# Remove forum from favorites
rnga forum fav-remove 310
```

### Topic Commands

```bash
# List topics in a forum (use forum ID)
rnga topic list 310

# With pagination and sorting
rnga topic list 310 --page 2 --order postdate

# View a topic (with posts)
rnga topic read 12345678

# View with full post content
rnga topic read 12345678 -v

# View specific page
rnga topic read 12345678 --page 2

# Filter by author
rnga topic read 12345678 --author 9876543

# Search topics in a forum
rnga topic search 310 "keyword"

# Search in content (not just titles)
rnga topic search 310 "keyword" --content

# List recent topics in a forum (default: last hour)
rnga topic recent 310

# Show all individual posts/replies from recent topics
rnga topic recent 310 --with-posts

# List recent topics with time range (second/minute/hour/day/week/month/year)
rnga topic recent 310 --range minute
rnga topic recent 310 --range hour
rnga topic recent 310 --range day

# Use custom time ranges with units (s/m/h/d)
rnga topic recent 310 --range 30s     # Topics from last 30 seconds
rnga topic recent 310 --range 5m      # Topics from last 5 minutes
rnga topic recent 310 --range 2h      # Topics from last 2 hours
rnga topic recent 310 --range 3d      # Topics from last 3 days

# Show all posts from recent topics with time range
rnga topic recent 310 --range 5m --with-posts

# With sorting and pagination
rnga topic recent 310 --range 12h --order postdate --page 1

# Output in JSON format for processing
rnga topic recent 310 --range 1h --format json

# List favorite topic folders
rnga topic folders

# List favorite topics
rnga topic favorites

# Add topic to favorites
rnga topic fav-add 12345678
```

### Post Commands

```bash
# Upvote a post
rnga post up --topic 12345678 --post 87654321

# Downvote a post
rnga post down --topic 12345678 --post 87654321

# View hot replies
rnga post hot --topic 12345678 --post 87654321

# View comments
rnga post comments --topic 12345678 --post 87654321

# Post a reply (requires auth)
rnga post reply 12345678 "Your reply content"

# Quote another post when replying
rnga post reply 12345678 "Your reply" --quote 87654321

# Post anonymously
rnga post reply 12345678 "Anonymous reply" --anonymous

# Comment on a post
rnga post comment --topic 12345678 --post 87654321 "Your comment"

# Get quote content for a post
rnga post quote --topic 12345678 --post 87654321
```

### User Commands

```bash
# View user by ID
rnga user get 12345

# View user by username
rnga user name "SomeUser"

# View your own profile (requires auth)
rnga user me

# Search users
rnga user search "keyword"

# View user's topics
rnga user topics 12345

# View user's posts
rnga user posts 12345
```

### Notification Commands

```bash
# Show unread notification counts
rnga notification counts

# List reply notifications
rnga notification list --kind reply

# List mention notifications
rnga notification list --kind mention

# Mark notification as read
rnga notification read NOTIFICATION_ID

# Mark all notifications of a type as read
rnga notification read-all --kind reply
```

### Message Commands

```bash
# List message conversations
rnga message list

# Read a conversation
rnga message read CONVERSATION_ID

# Send a new message
rnga message send --to "Username" --subject "Hello" "Message content"

# Reply to a conversation
rnga message reply CONVERSATION_ID "Reply content"
```

## Output Formats

All commands support different output formats:

```bash
# Table format (default)
rnga forum list

# JSON format
rnga forum list --format json

# Plain text format
rnga forum list --format plain
```

## Language

Output language can be changed using `--lang` or `-l`:

```bash
# English (default)
rnga --lang en topic list 7

# Simplified Chinese
rnga --lang zh-CN topic list 7

# Short form
rnga -l zh-CN auth status
```

Supported languages:
- `en` - English (default)
- `zh-CN` - Simplified Chinese (简体中文)

## Aliases

Short aliases are available for common commands:

- `rnga f` → `rnga forum`
- `rnga t` → `rnga topic`
- `rnga p` → `rnga post`
- `rnga u` → `rnga user`
- `rnga n` → `rnga notification`
- `rnga m` → `rnga message`
- `rnga forum ls` → `rnga forum list`
- `rnga topic ls` → `rnga topic list`
- `rnga topic view` → `rnga topic read`

## Configuration

Configuration is stored as `rnga.toml` in the same directory as the executable.

View current config location:

```bash
rnga config
```

## MCP Server

rNGA-CLI can run as a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server, allowing AI assistants like Claude, Cursor, and others to interact with NGA forum.

### Running as MCP Server

```bash
# Start MCP server over stdio
rnga --mcp
```

### Available Tools

The MCP server exposes the following tools:

| Tool | Description | Auth Required |
|------|-------------|---------------|
| `forum_list` | List all forum categories and forums | No |
| `forum_search` | Search forums by name | No |
| `topic_list` | List topics in a forum | No |
| `topic_read` | Read a topic with its posts | No |
| `topic_search` | Search topics in a forum | No |
| `user_get` | Get user profile by ID | No |
| `user_by_name` | Get user profile by username | No |
| `post_reply` | Reply to a topic | Yes |
| `notification_counts` | Get unread notification counts | Yes |

### IDE Configurations

Add to your Cursor MCP settings:

```json
{
  "mcpServers": {
    "rnga": {
      "command": "/path/to/rnga",
      "args": ["--mcp"]
    }
  }
}
```

### Authentication for MCP

The MCP server uses the same authentication as the CLI. Login via CLI first:

```bash
rnga auth login --token YOUR_TOKEN --uid YOUR_UID
```

The credentials are stored in `rnga.toml` and shared between CLI and MCP server.

## Examples

### Browse a forum

```bash
# List topics in the Genshin Impact forum
rnga topic list 650

# With pagination
rnga topic list 650 -p 2

# View recent topics from the last hour (default)
rnga topic recent 650

# View all individual posts/replies from the last 30 minutes
rnga topic recent 650 --range 30m --with-posts

# View recent topics from the last 24 hours
rnga topic recent 650 --range day
```

### Read a topic

```bash
# View topic with posts
rnga topic read 12345678 -v

# Navigate pages
rnga topic read 12345678 -p 3
```

### Check notifications

```bash
# Quick check
nga n counts

# View replies
nga n list -k reply
```

## License

MIT

