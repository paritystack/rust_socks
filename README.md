# Rust SOCKS5 Library

A comprehensive, high-performance SOCKS5 implementation in Rust providing both server and client functionality with async/await support.

## Features

- **Full SOCKS5 Protocol Support**
  - CONNECT command for TCP connections
  - BIND command for accepting incoming connections
  - UDP ASSOCIATE command for UDP relay

- **Multiple Authentication Methods**
  - No authentication
  - Username/Password authentication (RFC 1929)
  - Extensible authentication system via traits

- **Async/Await Architecture**
  - Built on Tokio for high-performance async I/O
  - Efficient connection handling
  - Non-blocking operations

- **Type-Safe Protocol Implementation**
  - Compile-time safety for SOCKS5 messages
  - Comprehensive error handling
  - Zero-copy where possible

- **Flexible Configuration**
  - Builder pattern for easy setup
  - Configurable timeouts
  - Selective command enabling
  - Custom authentication handlers

- **Unified CLI Application**
  - Single `socks5` binary with subcommands for server, client, and forwarding
  - Easy to install and deploy
  - Backwards-compatible individual binaries also available

## CLI Application

This library includes a comprehensive command-line application `socks5` that provides server, client, and port forwarding functionality in a single binary. See [APPLICATIONS.md](APPLICATIONS.md) for detailed documentation.

### Quick Start

```bash
# Build the unified binary
cargo build --release --bin socks5

# Or install it
cargo install --path . --bin socks5
```

### Server

Start a SOCKS5 proxy server with configuration file support.

```bash
# Generate default config
socks5 server generate-config --output my-config.toml

# Start with config
socks5 server start --config my-config.toml

# Quick start with no authentication
socks5 server start --no-auth

# Start with custom bind address
socks5 server start --bind 0.0.0.0:1080 --no-auth
```

### Client

Make connections through a SOCKS5 proxy.

```bash
# HTTP request
socks5 client --target example.com --port 80 --http

# With authentication
socks5 client \
  --target example.com \
  --port 443 \
  --http \
  --username admin \
  --password secret

# Test connection only
socks5 client --target google.com --port 443 --test-only
```

### Port Forwarding

Forward local ports to remote destinations through SOCKS5.

```bash
# Forward local port to remote server
socks5 forward \
  --local 127.0.0.1:8080 \
  --remote api.example.com \
  --port 443

# With authentication
socks5 forward \
  --local 127.0.0.1:5432 \
  --remote db.internal \
  --port 5432 \
  --username admin \
  --password secret
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust_socks = "0.1.0"
tokio = { version = "1.35", features = ["full"] }
```

## Quick Start

### Server

```rust
use rust_socks::server::ServerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ServerBuilder::new()
        .bind_addr("127.0.0.1:1080".parse()?)
        .no_auth()
        .build();

    server.run().await?;
    Ok(())
}
```

### Client

```rust
use rust_socks::client::SocksClient;
use rust_socks::protocol::{SocksAddr, TargetAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = SocksClient::new("127.0.0.1:1080".to_string());

    let target = TargetAddr::new(
        SocksAddr::Domain("example.com".to_string()),
        80
    );

    let mut stream = client.connect(target).await?;
    // Use stream for communication
    Ok(())
}
```

## Usage Examples

### Server with Authentication

```rust
use rust_socks::auth::UserPassAuth;
use rust_socks::server::ServerBuilder;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create authenticator with multiple users
    let auth = UserPassAuth::with_users(vec![
        ("alice".to_string(), "password123".to_string()),
        ("bob".to_string(), "secret456".to_string()),
    ]);

    let server = ServerBuilder::new()
        .bind_addr("127.0.0.1:1080".parse()?)
        .authenticator(Arc::new(auth))
        .build();

    server.run().await?;
    Ok(())
}
```

### Client with Authentication

```rust
use rust_socks::client::ClientBuilder;

let client = ClientBuilder::new("127.0.0.1:1080".to_string())
    .with_credentials("alice".to_string(), "password123".to_string())
    .build();

let target = TargetAddr::new(
    SocksAddr::Domain("example.com".to_string()),
    443
);

let stream = client.connect(target).await?;
```

### Multiple Authentication Methods

```rust
use rust_socks::auth::{MultiAuth, UserPassAuth};
use rust_socks::server::ServerBuilder;

let auth = UserPassAuth::single_user(
    "admin".to_string(),
    "admin123".to_string()
);

let multi_auth = MultiAuth::new()
    .with_no_auth()
    .with_user_pass(auth);

let server = ServerBuilder::new()
    .bind_addr("127.0.0.1:1080".parse()?)
    .multi_auth(multi_auth)
    .build();
```

### Custom Server Configuration

```rust
use rust_socks::server::ServerBuilder;

let server = ServerBuilder::new()
    .bind_addr("0.0.0.0:1080".parse()?)
    .no_auth()
    .enable_connect(true)    // Enable CONNECT command
    .enable_bind(false)       // Disable BIND command
    .enable_udp(false)        // Disable UDP ASSOCIATE
    .timeout(300)             // 5 minute timeout
    .build();
```

### Client Connection Methods

```rust
use rust_socks::client::SocksClient;
use rust_socks::protocol::{SocksAddr, TargetAddr};

let client = SocksClient::new("127.0.0.1:1080".to_string());

// Method 1: Using TargetAddr
let target = TargetAddr::new(
    SocksAddr::Domain("example.com".to_string()),
    80
);
let stream = client.connect(target).await?;

// Method 2: Using domain helper
let stream = client.connect_domain("example.com".to_string(), 80).await?;

// Method 3: Using IP address
let addr = "93.184.216.34:80".parse()?;
let stream = client.connect_addr(addr).await?;
```

### BIND Command

```rust
use rust_socks::client::SocksClient;
use rust_socks::protocol::{SocksAddr, TargetAddr};

let client = SocksClient::new("127.0.0.1:1080".to_string());

let target = TargetAddr::new(
    SocksAddr::Domain("0.0.0.0".to_string()),
    0
);

let (stream, incoming_addr) = client.bind(target).await?;
println!("Incoming connection from: {}", incoming_addr);
```

### UDP ASSOCIATE

```rust
use rust_socks::client::SocksClient;
use rust_socks::protocol::{SocksAddr, TargetAddr};

let client = SocksClient::new("127.0.0.1:1080".to_string());

let target = TargetAddr::new(
    SocksAddr::Domain("0.0.0.0".to_string()),
    0
);

let (tcp_stream, udp_relay_addr) = client.udp_associate(target).await?;
println!("UDP relay at: {}", udp_relay_addr);
// Keep tcp_stream alive to maintain UDP association
```

## Architecture

```
rust_socks/
├── src/
│   ├── lib.rs           # Library entry point and public API
│   ├── protocol.rs      # SOCKS5 protocol definitions
│   ├── message.rs       # Message parsing and serialization
│   ├── auth.rs          # Authentication handlers
│   ├── server.rs        # Server implementation
│   ├── client.rs        # Client implementation
│   └── error.rs         # Error types
├── examples/
│   ├── server.rs        # Server examples
│   ├── client.rs        # Client examples
│   └── proxy_chain.rs   # Proxy chaining example
└── Cargo.toml
```

## Protocol Support

### Commands
- ✅ CONNECT - Establish a TCP connection
- ✅ BIND - Accept incoming TCP connections
- ✅ UDP ASSOCIATE - Setup UDP relay

### Authentication Methods
- ✅ No Authentication (0x00)
- ✅ Username/Password (0x02)
- ⚠️ GSSAPI (0x01) - Not implemented

### Address Types
- ✅ IPv4 (0x01)
- ✅ Domain Name (0x03)
- ✅ IPv6 (0x04)

## Running Examples

### Start a basic server

```bash
cargo run --example server noauth
```

### Start a server with authentication

```bash
cargo run --example server userpass
```

### Run client examples

```bash
# Simple CONNECT
cargo run --example client connect

# With authentication
cargo run --example client auth

# Connect to domain
cargo run --example client domain

# BIND command
cargo run --example client bind

# UDP ASSOCIATE
cargo run --example client udp
```

### Proxy chaining example

```bash
cargo run --example proxy_chain
```

This example demonstrates routing traffic through multiple SOCKS5 proxies:
```
Client → Proxy1 → Proxy2 → Proxy3 → Destination
```

## Error Handling

The library provides comprehensive error types:

```rust
use rust_socks::error::{Result, SocksError};

match client.connect(target).await {
    Ok(stream) => {
        // Handle successful connection
    }
    Err(SocksError::AuthenticationFailed(msg)) => {
        eprintln!("Auth failed: {}", msg);
    }
    Err(SocksError::ConnectionRefused) => {
        eprintln!("Target refused connection");
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Custom Authentication

Implement the `Authenticator` trait for custom authentication:

```rust
use rust_socks::auth::Authenticator;
use async_trait::async_trait;

struct MyAuth;

#[async_trait]
impl Authenticator for MyAuth {
    async fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        // Custom authentication logic
        Ok(username == "admin" && password == "secret")
    }

    fn supported_methods(&self) -> Vec<AuthMethod> {
        vec![AuthMethod::UserPass]
    }
}
```

## Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Build all examples
cargo build --examples
```

## Performance Considerations

- Uses Tokio for async I/O - efficient for handling many concurrent connections
- Zero-copy message parsing where possible
- Minimal allocations in hot paths
- Connection pooling recommended for high-throughput clients

## Security Notes

- Always use authentication in production environments
- Consider using TLS/SSL wrapper for encrypted proxy connections
- Implement rate limiting for public-facing servers
- Validate and sanitize all user inputs
- Keep credentials secure (use environment variables, not hardcoded)

## Logging

The library uses the `tracing` crate for logging. Enable logging in your application:

```rust
use tracing_subscriber;

tracing_subscriber::fmt::init();
```

Set log level via environment variable:
```bash
RUST_LOG=debug cargo run --example server
RUST_LOG=info cargo run --example client
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## References

- [RFC 1928](https://www.rfc-editor.org/rfc/rfc1928.txt) - SOCKS Protocol Version 5
- [RFC 1929](https://www.rfc-editor.org/rfc/rfc1929.txt) - Username/Password Authentication for SOCKS V5

## Changelog

### 0.1.0 (Initial Release)
- Full SOCKS5 protocol implementation
- Server and client functionality
- Multiple authentication methods
- Comprehensive examples
- Full documentation
