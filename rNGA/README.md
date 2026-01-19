# rNGA

A Rust client library for the NGA (艾泽拉斯国家地理) forum.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rnga = { git = "https://github.com/peitaosu/rNGA" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use rnga::{NGAClient, Result, ForumIdKind};

#[tokio::main]
async fn main() -> Result<()> {
    // Create an anonymous client
    let client = NGAClient::builder().build()?;

    // List forum categories
    let categories = client.forums().list().await?;
    for cat in &categories {
        println!("{}: {} forums", cat.name, cat.forums.len());
    }

    // Browse topics in a forum
    let result = client.topics()
        .list(ForumIdKind::fid("310"))
        .page(1)
        .send()
        .await?;
    
    for topic in result.topics {
        println!("{}: {}", topic.id, topic.subject.content);
    }

    Ok(())
}
```

## Authentication

For operations requiring login:

```rust
let client = NGAClient::builder()
    .auth("your_token", "your_uid")
    .build()?;

// Check notifications
let counts = client.notifications().counts().await?;
println!("Unread: {}", counts.total());

// Post a reply
client.posts()
    .reply("12345678")
    .content("Hello world!")
    .send()
    .await?;
```

## API Overview

### Forums

```rust
// List categories
let categories = client.forums().list().await?;

// Search forums
let results = client.forums().search("game").await?;

// Manage favorites (requires auth)
let favorites = client.forums().favorites().await?;
client.forums().add_favorite("310").await?;
```

### Topics

```rust
// List topics with options
let result = client.topics()
    .list(ForumIdKind::fid("310"))
    .page(2)
    .order(TopicOrder::PostDate)
    .recommended_only(true)
    .send()
    .await?;

// View topic details
let details = client.topics()
    .details("12345678")
    .page(1)
    .author("user_id")  // Filter by author
    .send()
    .await?;

// Search topics
let results = client.topics()
    .search(ForumIdKind::fid("310"), "keyword")
    .send()
    .await?;
```

### Posts

```rust
// Vote on a post
let vote = client.posts()
    .vote_up("12345678", "87654321")
    .await?;

// Reply to a topic
client.posts()
    .reply("12345678")
    .content("Your reply")
    .quote("post_to_quote")  // Optional
    .anonymous(true)         // Optional
    .send()
    .await?;

// View comments on a post
let comments = client.posts()
    .comments("12345678", "87654321")
    .send()
    .await?;
```

### Users

```rust
// Get user by ID
let user = client.users().get("12345").await?;

// Get user by username
let user = client.users().get_by_name("Username").await?;

// Get current user (requires auth)
let me = client.users().me().await?;

// Search users
let results = client.users().search("query").await?;
```

### Notifications

```rust
// Get notification counts
let counts = client.notifications().counts().await?;

// List notifications
let list = client.notifications()
    .list(NotificationType::Reply)
    .send()
    .await?;

// Mark as read
client.notifications().mark_read("notification_id").await?;
```

### Messages

```rust
// List conversations
let conversations = client.messages().list().send().await?;

// Read a conversation
let messages = client.messages()
    .conversation("conversation_id")
    .send()
    .await?;

// Send a message
client.messages()
    .send("recipient_name", "Subject", "Content")
    .await?;
```

## Caching

Enable caching with the built-in memory cache or implement your own:

```rust
use rnga::cache::MemoryCache;
use std::sync::Arc;

let client = NGAClient::builder()
    .cache(Arc::new(MemoryCache::new()))
    .build()?;
```

Custom cache implementation:

```rust
use rnga::cache::CacheStorage;
use async_trait::async_trait;

#[async_trait]
impl CacheStorage for MyCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> { /* ... */ }
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) { /* ... */ }
    async fn remove(&self, key: &str) { /* ... */ }
    async fn clear(&self) { /* ... */ }
    async fn scan_prefix(&self, prefix: &str) -> Vec<String> { /* ... */ }
}
```

## Error Handling

All operations return `Result<T, Error>`:

```rust
use rnga::{Error, Result};

match client.topics().details("123").send().await {
    Ok(details) => println!("Topic: {}", details.topic.subject.content),
    Err(Error::NGAApi { code, message }) => {
        eprintln!("API error {}: {}", code, message);
    }
    Err(Error::AuthRequired) => {
        eprintln!("Login required");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `native-tls` | ✓ | Use native TLS for HTTPS |
| `rustls` | | Use rustls for HTTPS (pure Rust TLS) |

```toml
[dependencies]
rnga = { git = "...", default-features = false, features = ["rustls"] }
```

## License

MIT

