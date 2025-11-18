use clap::{Parser, Subcommand};
use rust_socks::auth::{MultiAuth, UserPassAuth};
use rust_socks::client::{ClientBuilder, SocksClient};
use rust_socks::config::Config;
use rust_socks::protocol::{SocksAddr, TargetAddr};
use rust_socks::server::ServerBuilder;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

/// SOCKS5 - A comprehensive SOCKS5 server, client, and forwarding tool
#[derive(Parser)]
#[command(name = "socks5")]
#[command(author, version, about = "SOCKS5 server, client, and port forwarding tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// SOCKS5 server operations
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

    /// SOCKS5 client operations
    Client {
        /// SOCKS5 proxy address (e.g., 127.0.0.1:1080)
        #[arg(short, long, default_value = "127.0.0.1:1080")]
        proxy: String,

        /// Target address (domain or IP)
        #[arg(short, long)]
        target: String,

        /// Target port
        #[arg(short = 'P', long)]
        port: u16,

        /// Username for authentication
        #[arg(short, long)]
        username: Option<String>,

        /// Password for authentication
        #[arg(short = 'w', long)]
        password: Option<String>,

        /// HTTP request mode - send HTTP GET request
        #[arg(long)]
        http: bool,

        /// Custom HTTP path (default: /)
        #[arg(long, default_value = "/")]
        http_path: String,

        /// Read response (number of bytes, 0 for all)
        #[arg(short, long, default_value = "4096")]
        read: usize,

        /// Send custom data
        #[arg(short, long)]
        send: Option<String>,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Test connection only (don't send/receive data)
        #[arg(long)]
        test_only: bool,
    },

    /// Port forwarding through SOCKS5 proxy
    Forward {
        /// Local address to listen on (e.g., 127.0.0.1:8080)
        #[arg(short, long)]
        local: String,

        /// Remote target address (domain or IP)
        #[arg(short, long)]
        remote: String,

        /// Remote target port
        #[arg(short = 'P', long)]
        port: u16,

        /// SOCKS5 proxy address
        #[arg(short = 's', long, default_value = "127.0.0.1:1080")]
        proxy: String,

        /// Username for SOCKS5 authentication
        #[arg(short, long)]
        username: Option<String>,

        /// Password for SOCKS5 authentication
        #[arg(short = 'w', long)]
        password: Option<String>,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Start the SOCKS5 server
    Start {
        /// Path to configuration file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Bind address (overrides config file)
        #[arg(short, long)]
        bind: Option<String>,

        /// Disable authentication (overrides config file)
        #[arg(long)]
        no_auth: bool,

        /// Set log level (trace, debug, info, warn, error)
        #[arg(short, long, default_value = "info")]
        log_level: String,
    },

    /// Generate a default configuration file
    GenerateConfig {
        /// Output path for the configuration file
        #[arg(short, long, default_value = "socks5.toml")]
        output: PathBuf,

        /// Overwrite existing file
        #[arg(short, long)]
        force: bool,
    },

    /// Validate a configuration file
    Validate {
        /// Path to configuration file
        #[arg(value_name = "FILE")]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { action } => match action {
            ServerAction::Start {
                config,
                bind,
                no_auth,
                log_level,
            } => {
                start_server(config, bind, no_auth, log_level).await?;
            }
            ServerAction::GenerateConfig { output, force } => {
                generate_config(output, force)?;
            }
            ServerAction::Validate { config } => {
                validate_config(config)?;
            }
        },

        Commands::Client {
            proxy,
            target,
            port,
            username,
            password,
            http,
            http_path,
            read,
            send,
            verbose,
            test_only,
        } => {
            run_client(
                proxy, target, port, username, password, http, http_path, read, send, verbose,
                test_only,
            )
            .await?;
        }

        Commands::Forward {
            local,
            remote,
            port,
            proxy,
            username,
            password,
            verbose,
        } => {
            run_forward(local, remote, port, proxy, username, password, verbose).await?;
        }
    }

    Ok(())
}

// ===== SERVER FUNCTIONS =====

async fn start_server(
    config_path: Option<PathBuf>,
    bind_override: Option<String>,
    no_auth: bool,
    log_level: String,
) -> anyhow::Result<()> {
    let config = if let Some(path) = config_path {
        info!("Loading configuration from: {}", path.display());
        Config::from_file(path)?
    } else {
        info!("Using default configuration");
        Config::default()
    };

    let filter = EnvFilter::try_new(&log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    if config.logging.json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    info!("Starting SOCKS5 server...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    let bind_addr = if let Some(addr) = bind_override {
        addr
    } else {
        config.server.bind_addr.clone()
    };

    info!("Bind address: {}", bind_addr);

    let mut builder = ServerBuilder::new()
        .bind_addr(bind_addr.parse()?)
        .enable_connect(config.server.enable_connect)
        .enable_bind(config.server.enable_bind)
        .enable_udp(config.server.enable_udp)
        .timeout(config.server.timeout_secs);

    if no_auth {
        info!("Authentication: None (disabled via command line)");
        builder = builder.no_auth();
    } else {
        match config.auth.mode.as_str() {
            "none" => {
                info!("Authentication: None");
                builder = builder.no_auth();
            }
            "password" => {
                info!("Authentication: Username/Password");
                info!("Configured users: {}", config.auth.users.len());
                let auth = UserPassAuth::with_users(
                    config
                        .auth
                        .users
                        .iter()
                        .map(|(u, p)| (u.clone(), p.clone()))
                        .collect(),
                );
                builder = builder.authenticator(Arc::new(auth));
            }
            "multi" => {
                info!("Authentication: Multiple methods (NoAuth + Username/Password)");
                let auth = UserPassAuth::with_users(
                    config
                        .auth
                        .users
                        .iter()
                        .map(|(u, p)| (u.clone(), p.clone()))
                        .collect(),
                );
                let multi_auth = MultiAuth::new().with_no_auth().with_user_pass(auth);
                builder = builder.multi_auth(multi_auth);
            }
            _ => {
                error!("Invalid authentication mode: {}", config.auth.mode);
                anyhow::bail!("Invalid authentication mode");
            }
        }
    }

    info!("Commands enabled:");
    info!("  CONNECT: {}", config.server.enable_connect);
    info!("  BIND: {}", config.server.enable_bind);
    info!("  UDP ASSOCIATE: {}", config.server.enable_udp);
    info!("Connection timeout: {} seconds", config.server.timeout_secs);

    let server = builder.build();
    info!("Server ready, accepting connections...");

    if let Err(e) = server.run().await {
        error!("Server error: {}", e);
        return Err(e.into());
    }

    Ok(())
}

fn generate_config(output: PathBuf, force: bool) -> anyhow::Result<()> {
    if output.exists() && !force {
        anyhow::bail!(
            "Configuration file already exists: {}\nUse --force to overwrite",
            output.display()
        );
    }

    let config = Config::generate_default();
    config.to_file(&output)?;

    println!(
        "✓ Generated default configuration file: {}",
        output.display()
    );
    println!("\nConfiguration details:");
    println!("  Bind address: {}", config.server.bind_addr);
    println!("  Authentication: {}", config.auth.mode);
    println!("  Users configured: {}", config.auth.users.len());
    println!("\nEdit the file to customize your server settings.");

    Ok(())
}

fn validate_config(path: PathBuf) -> anyhow::Result<()> {
    println!("Validating configuration file: {}", path.display());

    let config = Config::from_file(&path)?;

    println!("✓ Configuration is valid!\n");
    println!("Server settings:");
    println!("  Bind address: {}", config.server.bind_addr);
    println!("  CONNECT: {}", config.server.enable_connect);
    println!("  BIND: {}", config.server.enable_bind);
    println!("  UDP ASSOCIATE: {}", config.server.enable_udp);
    println!("  Timeout: {} seconds", config.server.timeout_secs);

    println!("\nAuthentication settings:");
    println!("  Mode: {}", config.auth.mode);
    println!("  Users: {}", config.auth.users.len());
    if !config.auth.users.is_empty() {
        println!(
            "  Usernames: {}",
            config
                .auth
                .users
                .keys()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!("\nLogging settings:");
    println!("  Level: {}", config.logging.level);
    println!("  JSON format: {}", config.logging.json);

    Ok(())
}

// ===== CLIENT FUNCTIONS =====

async fn run_client(
    proxy: String,
    target: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    http: bool,
    http_path: String,
    read: usize,
    send: Option<String>,
    verbose: bool,
    test_only: bool,
) -> anyhow::Result<()> {
    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    info!("SOCKS5 Client");
    info!("Proxy: {}", proxy);
    info!("Target: {}:{}", target, port);

    let client = if let (Some(username), Some(password)) = (username, password) {
        info!("Using authentication: {}", username);
        ClientBuilder::new(proxy.clone())
            .with_credentials(username, password)
            .build()
    } else {
        info!("No authentication");
        SocksClient::new(proxy.clone())
    };

    let target_addr = if target.parse::<std::net::IpAddr>().is_ok() {
        let ip: std::net::IpAddr = target.parse()?;
        match ip {
            std::net::IpAddr::V4(ipv4) => TargetAddr::new(SocksAddr::V4(ipv4), port),
            std::net::IpAddr::V6(ipv6) => TargetAddr::new(SocksAddr::V6(ipv6), port),
        }
    } else {
        info!("Resolving domain: {}", target);
        TargetAddr::new(SocksAddr::Domain(target.clone()), port)
    };

    info!("Connecting to {}...", target_addr);
    let mut stream = client.connect(target_addr).await?;
    info!("✓ Connected successfully!");

    if test_only {
        info!("Test completed successfully");
        return Ok(());
    }

    if http {
        let http_request = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            http_path, target
        );
        info!("Sending HTTP GET request for {}", http_path);
        debug!("Request:\n{}", http_request);
        stream.write_all(http_request.as_bytes()).await?;
        stream.flush().await?;
        info!("✓ Request sent");
    } else if let Some(data) = send {
        info!("Sending custom data ({} bytes)", data.len());
        stream.write_all(data.as_bytes()).await?;
        stream.flush().await?;
        info!("✓ Data sent");
    }

    if read > 0 {
        info!("Reading response...");
        let mut buffer = if read == 0 {
            Vec::new()
        } else {
            vec![0u8; read]
        };

        let bytes_read = if read == 0 {
            stream.read_to_end(&mut buffer).await?
        } else {
            stream.read(&mut buffer).await?
        };

        info!("✓ Received {} bytes", bytes_read);

        match String::from_utf8(buffer[..bytes_read].to_vec()) {
            Ok(text) => {
                println!("\n--- Response ({} bytes) ---", bytes_read);
                println!("{}", text);
                println!("--- End Response ---");
            }
            Err(_) => {
                println!("\n--- Binary Response ({} bytes) ---", bytes_read);
                println!("{:02X?}", &buffer[..bytes_read.min(256)]);
                if bytes_read > 256 {
                    println!("... ({} more bytes)", bytes_read - 256);
                }
                println!("--- End Response ---");
            }
        }
    }

    info!("Connection completed successfully");
    Ok(())
}

// ===== FORWARD FUNCTIONS =====

async fn run_forward(
    local: String,
    remote: String,
    port: u16,
    proxy: String,
    username: Option<String>,
    password: Option<String>,
    verbose: bool,
) -> anyhow::Result<()> {
    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    info!("SOCKS5 Port Forwarder");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("Local: {}", local);
    info!("Remote: {}:{}", remote, port);
    info!("Proxy: {}", proxy);

    let client_template = if let (Some(username), Some(password)) = (&username, &password) {
        info!("Using authentication: {}", username);
        ClientBuilder::new(proxy.clone())
            .with_credentials(username.clone(), password.clone())
            .build()
    } else {
        info!("No authentication");
        SocksClient::new(proxy.clone())
    };

    let target_addr = if remote.parse::<std::net::IpAddr>().is_ok() {
        let ip: std::net::IpAddr = remote.parse()?;
        match ip {
            std::net::IpAddr::V4(ipv4) => TargetAddr::new(SocksAddr::V4(ipv4), port),
            std::net::IpAddr::V6(ipv6) => TargetAddr::new(SocksAddr::V6(ipv6), port),
        }
    } else {
        TargetAddr::new(SocksAddr::Domain(remote.clone()), port)
    };

    let listener = TcpListener::bind(&local).await?;
    info!("✓ Listening on {}", local);
    info!("Forwarding to {} via {}", target_addr, proxy);
    info!("Ready to accept connections...");

    let mut connection_count = 0u64;

    loop {
        match listener.accept().await {
            Ok((local_stream, peer_addr)) => {
                connection_count += 1;
                let conn_id = connection_count;
                info!("[{}] New connection from {}", conn_id, peer_addr);

                let client = client_template.clone();
                let target = target_addr.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(conn_id, local_stream, client, target).await
                    {
                        error!("[{}] Connection error: {}", conn_id, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    conn_id: u64,
    mut local_stream: TcpStream,
    client: SocksClient,
    target: TargetAddr,
) -> anyhow::Result<()> {
    debug!("[{}] Connecting to {} via proxy...", conn_id, target);

    let mut remote_stream = match client.connect(target.clone()).await {
        Ok(stream) => {
            info!("[{}] ✓ Connected to {}", conn_id, target);
            stream
        }
        Err(e) => {
            error!("[{}] Failed to connect to {}: {}", conn_id, target, e);
            return Err(e.into());
        }
    };

    let (mut local_read, mut local_write) = local_stream.split();
    let (mut remote_read, mut remote_write) = remote_stream.split();

    let client_to_server = async {
        match copy_with_stats(&mut local_read, &mut remote_write, conn_id, "upstream").await {
            Ok(bytes) => {
                debug!("[{}] Upstream closed ({} bytes)", conn_id, bytes);
                Ok(bytes)
            }
            Err(e) => {
                debug!("[{}] Upstream error: {}", conn_id, e);
                Err(e)
            }
        }
    };

    let server_to_client = async {
        match copy_with_stats(&mut remote_read, &mut local_write, conn_id, "downstream").await {
            Ok(bytes) => {
                debug!("[{}] Downstream closed ({} bytes)", conn_id, bytes);
                Ok(bytes)
            }
            Err(e) => {
                debug!("[{}] Downstream error: {}", conn_id, e);
                Err(e)
            }
        }
    };

    let (upstream_result, downstream_result) = tokio::join!(client_to_server, server_to_client);

    let upstream_bytes = upstream_result.unwrap_or(0);
    let downstream_bytes = downstream_result.unwrap_or(0);

    info!(
        "[{}] Connection closed (↑ {} bytes, ↓ {} bytes)",
        conn_id, upstream_bytes, downstream_bytes
    );

    Ok(())
}

async fn copy_with_stats<R, W>(
    reader: &mut R,
    writer: &mut W,
    conn_id: u64,
    direction: &str,
) -> io::Result<u64>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; 8192];
    let mut total = 0u64;

    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        };

        writer.write_all(&buf[..n]).await?;
        writer.flush().await?;
        total += n as u64;

        if total % 1048576 == 0 {
            debug!(
                "[{}] {} transferred: {} MB",
                conn_id,
                direction,
                total / 1048576
            );
        }
    }

    Ok(total)
}
