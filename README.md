# rNGA

A Rust implementation for interacting with the NGA (艾泽拉斯国家地理) forum.

## Overview

This repository contains a Rust workspace with two crates:

| Crate | Description |
|-------|-------------|
| [`rNGA`](./rNGA) | Core client library for NGA API |
| [`rNGA-CLI`](./rNGA-CLI) | Command-line interface for NGA |

## Quick Start

### Library Usage

```rust
use rnga::{NGAClient, Result, ForumIdKind};

#[tokio::main]
async fn main() -> Result<()> {
    let client = NGAClient::builder()
        .auth("your_token", "your_uid")
        .build()?;

    // Browse forums
    let categories = client.forums().list().await?;
    
    // List topics
    let topics = client.topics()
        .list(ForumIdKind::fid("310"))
        .send()
        .await?;

    // Check notifications
    let counts = client.notifications().counts().await?;
    println!("Unread: {}", counts.total());

    Ok(())
}
```

### CLI Usage

```bash
# Install
cargo install --path rNGA-CLI

# Authenticate
rnga auth login --token YOUR_TOKEN --uid YOUR_UID

# Browse
rnga forum list
rnga topic list 310
rnga topic read 12345678

# Interact
rnga post reply 12345678 "Hello!"
rnga notification counts
```

## Building

```bash
# Build all crates
cargo build

# Build release
cargo build --release

# Run tests
cargo test

# Install CLI
cargo install --path rNGA-CLI
```

## API Coverage

| Category | Operations |
|----------|------------|
| **Forums** | List categories, search, favorites management |
| **Topics** | List, search, details, favorites, subforum filtering |
| **Posts** | Vote, reply, comment, quote, hot replies |
| **Users** | Profile lookup, search, topic/post history |
| **Notifications** | Counts, list by type, mark read |
| **Messages** | Conversations, send, reply |

## Documentation

- [Library README](./rNGA/README.md) - API usage and examples
- [CLI README](./rNGA-CLI/README.md) - Command reference and usage

## License

MIT

