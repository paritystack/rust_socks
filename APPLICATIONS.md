# SOCKS5 Applications

This document describes the command-line applications included with the Rust SOCKS5 library.

## Unified Application: socks5

**Recommended:** Use the unified `socks5` binary which combines all functionality in one convenient tool.

### Installation

```bash
# Build the unified binary
cargo build --release --bin socks5

# Install to your system
cargo install --path . --bin socks5

# The binary will be available as: socks5
```

### Usage

The `socks5` binary has three main subcommands:

#### Server

```bash
# Generate config
socks5 server generate-config --output my-config.toml

# Start server
socks5 server start --config my-config.toml

# Quick start (no auth)
socks5 server start --no-auth

# Validate config
socks5 server validate my-config.toml
```

#### Client

```bash
# HTTP request
socks5 client --target example.com --port 80 --http

# With authentication
socks5 client \
  --target example.com \
  --port 80 \
  --http \
  --username admin \
  --password secret

# Test connection
socks5 client --target google.com --port 443 --test-only
```

#### Forward

```bash
# Forward local port to remote
socks5 forward \
  --local 127.0.0.1:8080 \
  --remote api.example.com \
  --port 443

# With authentication
socks5 forward \
  --local 0.0.0.0:3306 \
  --remote db.internal \
  --port 3306 \
  --username dbuser \
  --password dbpass
```

### Why Use the Unified Binary?

- **Single tool**: One binary for all SOCKS5 operations
- **Easier deployment**: Just copy one executable
- **Consistent interface**: All features accessible through subcommands
- **Smaller footprint**: Share code between components

---

## Individual Applications (Legacy)

For backwards compatibility, individual binaries are also available.

### 1. socks-server - SOCKS5 Proxy Server

A full-featured SOCKS5 proxy server with configuration file support.

#### Features

- TOML-based configuration
- Multiple authentication modes (none, password, multi)
- Command-line overrides
- Comprehensive logging
- Configuration validation
- Production-ready

#### Usage

```bash
# Start server with default configuration
cargo run --bin socks-server start

# Start with custom configuration file
cargo run --bin socks-server start --config configs/socks5-auth.toml

# Start with command-line overrides
cargo run --bin socks-server start --bind 0.0.0.0:8080 --no-auth

# Generate a default configuration file
cargo run --bin socks-server generate-config --output my-config.toml

# Validate a configuration file
cargo run --bin socks-server validate configs/socks5-auth.toml
```

#### Configuration File

See the `configs/` directory for example configuration files:

- `socks5-noauth.toml` - No authentication
- `socks5-auth.toml` - Username/password authentication
- `socks5-multi.toml` - Multiple authentication methods
- `socks5-production.toml` - Production deployment example

#### Command-Line Options

```
socks-server start [OPTIONS]

Options:
  -c, --config <FILE>         Path to configuration file
  -b, --bind <ADDRESS>        Bind address (overrides config)
      --no-auth               Disable authentication (overrides config)
  -l, --log-level <LEVEL>     Log level [default: info]
  -h, --help                  Print help
```

### 2. socks-client - SOCKS5 Client Tool

A command-line tool for making connections through SOCKS5 proxies.

#### Features

- HTTP request mode
- Custom data sending
- Authentication support
- Verbose logging
- Binary and text response handling
- Connection testing

#### Usage

```bash
# Simple HTTP request
cargo run --bin socks-client -- \
  --proxy 127.0.0.1:1080 \
  --target example.com \
  --port 80 \
  --http

# HTTP request with authentication
cargo run --bin socks-client -- \
  --proxy 127.0.0.1:1080 \
  --target httpbin.org \
  --port 80 \
  --http \
  --http-path /ip \
  --username admin \
  --password admin123

# Test connection only
cargo run --bin socks-client -- \
  --proxy 127.0.0.1:1080 \
  --target google.com \
  --port 443 \
  --test-only

# Send custom data
cargo run --bin socks-client -- \
  --proxy 127.0.0.1:1080 \
  --target example.com \
  --port 80 \
  --send "GET / HTTP/1.1\r\nHost: example.com\r\n\r\n" \
  --read 2048

# Connect to IP address
cargo run --bin socks-client -- \
  --proxy 127.0.0.1:1080 \
  --target 1.1.1.1 \
  --port 80 \
  --http
```

#### Command-Line Options

```
socks-client [OPTIONS]

Options:
  -p, --proxy <ADDRESS>       SOCKS5 proxy address [default: 127.0.0.1:1080]
  -t, --target <ADDRESS>      Target address (domain or IP)
  -P, --port <PORT>           Target port
  -u, --username <USER>       Username for authentication
  -w, --password <PASS>       Password for authentication
      --http                  Send HTTP GET request
      --http-path <PATH>      HTTP path [default: /]
  -r, --read <BYTES>          Read response bytes [default: 4096]
  -s, --send <DATA>           Send custom data
  -v, --verbose               Enable verbose logging
      --test-only             Test connection only
  -h, --help                  Print help
```

### 3. socks-forward - Port Forwarding Tool

Forward a local port to a remote destination through a SOCKS5 proxy.

#### Features

- Bidirectional port forwarding
- Multiple concurrent connections
- Connection statistics
- Authentication support
- Automatic reconnection
- Traffic logging

#### Usage

```bash
# Forward local port 8080 to example.com:80 via proxy
cargo run --bin socks-forward -- \
  --local 127.0.0.1:8080 \
  --remote example.com \
  --port 80 \
  --proxy 127.0.0.1:1080

# Forward with authentication
cargo run --bin socks-forward -- \
  --local 0.0.0.0:3306 \
  --remote db.example.com \
  --port 3306 \
  --proxy 127.0.0.1:1080 \
  --username admin \
  --password secretpass

# Forward SSH connection
cargo run --bin socks-forward -- \
  --local 127.0.0.1:2222 \
  --remote remote-server.com \
  --port 22 \
  --proxy 127.0.0.1:1080 \
  --verbose

# Forward to IP address
cargo run --bin socks-forward -- \
  --local 127.0.0.1:8888 \
  --remote 192.168.1.100 \
  --port 80 \
  --proxy 127.0.0.1:1080
```

#### Command-Line Options

```
socks-forward [OPTIONS]

Options:
  -l, --local <ADDRESS>       Local address to listen on
  -r, --remote <ADDRESS>      Remote target address
  -P, --port <PORT>           Remote target port
  -s, --proxy <ADDRESS>       SOCKS5 proxy address [default: 127.0.0.1:1080]
  -u, --username <USER>       Username for authentication
  -w, --password <PASS>       Password for authentication
  -v, --verbose               Enable verbose logging
  -h, --help                  Print help
```

## Building Release Binaries

Build optimized release binaries:

```bash
# Build all binaries
cargo build --release --bins

# Build specific binary
cargo build --release --bin socks-server
cargo build --release --bin socks-client
cargo build --release --bin socks-forward

# Binaries will be in target/release/
./target/release/socks-server --help
./target/release/socks-client --help
./target/release/socks-forward --help
```

## Installation

Install binaries to your system:

```bash
# Install all binaries
cargo install --path .

# Install specific binary
cargo install --path . --bin socks-server

# Binaries will be installed to ~/.cargo/bin/
socks-server --help
socks-client --help
socks-forward --help
```

## Use Cases

### 1. Development Proxy

```bash
# Start a local SOCKS5 proxy for development
socks-server start --no-auth --log-level debug
```

### 2. Authenticated Corporate Proxy

```bash
# Start with authentication for team use
socks-server start --config configs/socks5-auth.toml
```

### 3. Database Access Through Bastion

```bash
# Forward local port to remote database through SOCKS5 proxy
socks-forward \
  --local 127.0.0.1:5432 \
  --remote prod-db.internal \
  --port 5432 \
  --proxy bastion.company.com:1080 \
  --username yourname \
  --password yourpass

# Then connect to localhost:5432
psql -h 127.0.0.1 -p 5432 -U dbuser production
```

### 4. Testing API Through Proxy

```bash
# Test HTTP API through SOCKS5
socks-client \
  --target api.example.com \
  --port 443 \
  --http \
  --http-path /v1/status \
  --username api-user \
  --password api-pass
```

### 5. Proxy Chain

```bash
# Terminal 1: Start first proxy
socks-server start --bind 127.0.0.1:1081 --no-auth

# Terminal 2: Forward through first proxy to second
socks-forward \
  --local 127.0.0.1:2080 \
  --remote 127.0.0.1 \
  --port 1082 \
  --proxy 127.0.0.1:1081

# Now you have: Client -> :2080 -> :1081 -> :1082 -> Destination
```

## Environment Variables

All applications respect these environment variables:

- `RUST_LOG` - Set log level (overrides config file)
  ```bash
  RUST_LOG=debug socks-server start
  ```

- `RUST_BACKTRACE` - Enable backtraces for errors
  ```bash
  RUST_BACKTRACE=1 socks-server start
  ```

## Logging

All applications use structured logging with the `tracing` framework.

### Log Levels

- `error` - Only errors
- `warn` - Warnings and errors
- `info` - Informational messages (default)
- `debug` - Detailed debugging information
- `trace` - Very verbose tracing

### Log Format

Standard format:
```
2024-01-15T10:30:45.123Z INFO socks_server: Starting SOCKS5 server...
```

JSON format (production):
```json
{"timestamp":"2024-01-15T10:30:45.123Z","level":"INFO","target":"socks_server","fields":{"message":"Starting SOCKS5 server..."}}
```

## Security Considerations

### Production Deployment

1. **Always use authentication** in production
2. **Use strong passwords** - avoid default credentials
3. **Bind to specific interfaces** - use 127.0.0.1 for local only
4. **Use firewall rules** - restrict access to authorized IPs
5. **Enable TLS** - consider TLS wrapper like stunnel
6. **Monitor logs** - watch for suspicious activity
7. **Rotate credentials** - change passwords regularly
8. **Limit commands** - disable BIND and UDP if not needed

### Network Security

```bash
# Good: Local development
socks-server start --bind 127.0.0.1:1080 --no-auth

# Good: Production with auth and limited commands
socks-server start --config production-config.toml

# Bad: Public access without auth
socks-server start --bind 0.0.0.0:1080 --no-auth  # DON'T DO THIS
```

## Troubleshooting

### Connection Refused

```bash
# Check if server is running
netstat -tlnp | grep 1080

# Check firewall
sudo iptables -L | grep 1080

# Test with verbose logging
socks-client --target example.com --port 80 --test-only --verbose
```

### Authentication Failures

```bash
# Verify credentials in config
socks-server validate configs/socks5-auth.toml

# Check server logs
socks-server start --log-level debug

# Test authentication
socks-client \
  --target example.com \
  --port 80 \
  --username admin \
  --password admin123 \
  --test-only
```

### Performance Issues

```bash
# Enable debug logging
RUST_LOG=debug socks-server start

# Monitor connections
watch -n 1 'netstat -an | grep 1080 | wc -l'

# Check system limits
ulimit -n
```

## License

MIT License - See LICENSE file for details
